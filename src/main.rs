use clap::Parser;
use cli_args::{CliArgs, Commands};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_program::{program_pack::Pack, system_program, sysvar};
use solana_sdk::signature::Signature;
use solana_sdk::{
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{
    EncodedTransaction, UiMessage, UiTransactionEncoding,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use token_vesting::{
    instruction::{create, init, unlock, Schedule},
    state::VestingScheduleHeader,
};

mod cli_args;

pub const BONFIDA_KEY: &str = "CChTq6PthWU82YZkbveA3WDf7s97BWhBK4Vx9bmsT743";

#[derive(Serialize, Deserialize, Debug)]
pub struct VestingData {
    pub key: String,
    pub receiver_key_type: ReceiverKeyType,
    pub mint: String,
    pub schedules: Vec<ScheduleCLI>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleCLI {
    pub release_time: u64,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ReceiverKeyType {
    Wallet,
    TokenAcc,
}

fn base58_to_keypair(key: &str) -> Keypair {
    let priv_key_bytes = bs58::decode(key).into_vec().unwrap();

    Keypair::from_bytes(priv_key_bytes.as_ref()).unwrap()
}

fn main() {
    let args = CliArgs::parse();

    let payer = base58_to_keypair(args.payer_keypair.as_ref());

    let program_id = Pubkey::from_str(args.program_id.as_ref()).unwrap();

    let rpc_client = RpcClient::new(args.url);

    match args.command {
        Commands::Create {
            source_token_address,
            vesting_data_file,
        } => {
            let mut file = File::open(vesting_data_file).unwrap();
            let mut data = String::new();
            file.read_to_string(&mut data).unwrap();

            let vesting_data: VestingData = serde_json::from_str(&data).unwrap();

            let mint = Pubkey::from_str(vesting_data.mint.as_ref()).unwrap();

            let source_token_pubkey = {
                if let Some(key) = source_token_address {
                    Pubkey::from_str(key.as_ref()).unwrap()
                } else {
                    get_associated_token_address(&payer.pubkey(), &mint)
                }
            };

            let mut not_found = true;
            let mut vesting_seed: [u8; 32] = [0; 32];
            let mut vesting_pubkey = Pubkey::new_unique();

            while not_found {
                vesting_seed = Pubkey::new_unique().to_bytes();
                let program_id_bump =
                    Pubkey::find_program_address(&[&vesting_seed[..31]], &program_id);
                vesting_pubkey = program_id_bump.0;
                vesting_seed[31] = program_id_bump.1;
                not_found = rpc_client.get_account(&vesting_pubkey).is_ok();
            }

            let vesting_token_pubkey = get_associated_token_address(&vesting_pubkey, &mint);

            let schedules = vesting_data
                .schedules
                .iter()
                .map(|x| Schedule {
                    release_time: x.release_time,
                    amount: x.amount,
                })
                .collect();

            let mut instructions = vec![];

            let destination_token_account = {
                match vesting_data.receiver_key_type {
                    ReceiverKeyType::Wallet => {
                        let ass_t_acc = get_associated_token_address(
                            &Pubkey::from_str(vesting_data.key.as_ref()).unwrap(),
                            &mint,
                        );
                        instructions.push(create_associated_token_account(
                            &payer.pubkey(),
                            &Pubkey::from_str(vesting_data.key.as_ref()).unwrap(),
                            &mint,
                        ));
                        ass_t_acc
                    }
                    ReceiverKeyType::TokenAcc => {
                        Pubkey::from_str(vesting_data.key.as_ref()).unwrap()
                    }
                }
            };

            instructions.append(&mut vec![
                init(
                    &system_program::id(),
                    &sysvar::rent::id(),
                    &program_id,
                    &payer.pubkey(),
                    &vesting_pubkey,
                    vesting_seed,
                    vesting_data.schedules.len() as u32,
                )
                .unwrap(),
                create_associated_token_account(&payer.pubkey(), &vesting_pubkey, &mint),
                create(
                    &program_id,
                    &spl_token::id(),
                    &vesting_pubkey,
                    &vesting_token_pubkey,
                    &payer.pubkey(),
                    &source_token_pubkey,
                    &destination_token_account,
                    &mint,
                    schedules,
                    vesting_seed,
                )
                .unwrap(),
            ]);

            let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            println!(
                "\nThe seed of the contract is: {:?}",
                Pubkey::new_from_array(vesting_seed)
            );
            println!("Please write it down as it is needed to interact with the contract!");

            println!("The vesting account pubkey: {:?}", vesting_pubkey,);

            let tx = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();

            println!("Tx signature: {:?}", tx);
        }
        Commands::Unlock { vesting_seed } => {
            let vesting_seed = Pubkey::from_str(vesting_seed.as_ref()).unwrap().to_bytes();

            let (vesting_pubkey, _) =
                Pubkey::find_program_address(&[&vesting_seed[..31]], &program_id);

            let packed_state = rpc_client.get_account_data(&vesting_pubkey).unwrap();
            let header_state =
                VestingScheduleHeader::unpack(&packed_state[..VestingScheduleHeader::LEN]).unwrap();
            let destination_token_pubkey = header_state.destination_address;

            let vesting_token_pubkey =
                get_associated_token_address(&vesting_pubkey, &header_state.mint_address);

            let unlock_instruction = unlock(
                &program_id,
                &spl_token::id(),
                &sysvar::clock::id(),
                &vesting_pubkey,
                &vesting_token_pubkey,
                &destination_token_pubkey,
                vesting_seed,
            )
            .unwrap();

            let mut transaction =
                Transaction::new_with_payer(&[unlock_instruction], Some(&payer.pubkey()));

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            let tx = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();

            println!("Tx signature: {:?}", tx);
        }
        Commands::Verify { directory } => {
            let path = Path::new(&directory);
            for entry in fs::read_dir(path).expect("Unable to list") {
                let entry = entry.expect("unable to get entry");

                let mut file = File::open(entry.path()).unwrap();
                let mut data = String::new();
                file.read_to_string(&mut data).unwrap();

                let vesting_data: VestingData = serde_json::from_str(&data).unwrap();

                let config = GetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: Some(1),
                    commitment: None,
                };
                let signature = rpc_client
                    .get_signatures_for_address_with_config(
                        &Pubkey::from_str(&vesting_data.key).unwrap(),
                        config,
                    )
                    .unwrap();

                let transaction = rpc_client
                    .get_transaction(
                        &Signature::from_str(&signature.get(0).unwrap().signature).unwrap(),
                        UiTransactionEncoding::Json,
                    )
                    .unwrap();

                let mut keys: HashMap<u8, String> = HashMap::new();

                if let EncodedTransaction::Json(transaction_json) =
                    transaction.transaction.transaction
                {
                    if let UiMessage::Raw(message_raw) = transaction_json.message {
                        for (ind, el) in message_raw.account_keys.iter().enumerate() {
                            keys.insert(ind as u8, String::from(el));
                        }

                        for instruction in message_raw.instructions {
                            if keys.get(&instruction.program_id_index).unwrap() == BONFIDA_KEY {
                                let instruction_data =
                                    bs58::decode(instruction.data).into_vec().unwrap();
                                let deserialized_instruction =
                                    token_vesting::instruction::VestingInstruction::unpack(
                                        &instruction_data,
                                    )
                                    .unwrap();

                                if let token_vesting::instruction::VestingInstruction::Create {
                                    seeds,
                                    mint_address,
                                    destination_token_address,
                                    schedules,
                                } = deserialized_instruction
                                {
                                    let seed_key = Pubkey::new_from_array(seeds);
                                    let expected_mint =
                                        Pubkey::from_str(&vesting_data.mint).unwrap();

                                    if mint_address != expected_mint {
                                        println!(
                                            "Mint key mismatch.\nExpected {:?}\nReceived {:?}",
                                            expected_mint, mint_address
                                        );
                                    }

                                    let expected_schedule: Vec<Schedule> = vesting_data
                                        .schedules
                                        .iter()
                                        .map(|x| Schedule {
                                            release_time: x.release_time,
                                            amount: x.amount,
                                        })
                                        .collect();

                                    if schedules != expected_schedule {
                                        println!(
                                            "Schedule mismatch.\nExpected: {:?}\nReceived: {:?}",
                                            expected_schedule, schedules
                                        );
                                    }

                                    let expected_token_account = {
                                        match vesting_data.receiver_key_type {
                                            ReceiverKeyType::Wallet => {
                                                get_associated_token_address(
                                                    &Pubkey::from_str(vesting_data.key.as_ref())
                                                        .unwrap(),
                                                    &expected_mint,
                                                )
                                            }
                                            ReceiverKeyType::TokenAcc => {
                                                Pubkey::from_str(vesting_data.key.as_ref()).unwrap()
                                            }
                                        }
                                    };

                                    if destination_token_address != expected_token_account {
                                        println!("Destination token account mismatch.\nExpected: {:?}\nReceived: {:?}", expected_token_account, destination_token_address);
                                    }

                                    println!("\n--------------\nReceiver token acc: {:?}\nSeed: {:?}\nFile: {:?}\n--------------", expected_token_account, seed_key, entry.path());
                                }
                            }
                        }
                    } else {
                        panic!("Error while message deserialization");
                    }
                } else {
                    panic!("Error while transaction deserialization");
                }
            }
        }
    }
}
