use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "Bonfida vesting cli")]
#[clap(version, author)]
pub struct CliArgs {
    /// RPC endpoint.
    #[clap(long, long, default_value_t = String::from("https://api.mainnet-beta.solana.com"), value_name = "URL")]
    pub url: String,

    /// Keypair in base 58 encoding
    #[clap(long, value_name = "BASE 58")]
    pub payer_keypair: String,

    /// Bonfida program key
    #[clap(long, default_value_t = String::from("CChTq6PthWU82YZkbveA3WDf7s97BWhBK4Vx9bmsT743"), value_name = "PUBKEY")]
    pub program_id: String,

    #[clap(subcommand)]
    pub command: Commands,
}

/// CLI sub-commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    ///
    Create {
        #[clap(long, value_name = "PUBKEY")]
        source_token_address: Option<String>,
        #[clap(long, value_name = "FILE")]
        vesting_data_file: String,
    },
    ///
    Unlock {
        #[clap(long, value_name = "PUBKEY")]
        vesting_seed: String,
    },
}
