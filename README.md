## Installation guide

To run this CLI you need Solana CLI and Rust lang be installed on you machine.

To install Rust lang run next command in the terminal: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

Next we can install Solana CLI: `sh -c "$(curl -sSfL https://release.solana.com/v1.10.8/install)"`

Once you have installed these tools we can move to the next steps.

## Vesting creation guide

This CLI mostly copy original Bonfida vesting program CLI but adds some new useful features like base58 private key support and json file with vesting schedules.

So to use it you need Phantom wallet with created account in it.

To create vesting contract it supposed that you already have token account with some amounts of tokens you want to distribute.

First of all let's fill in `vesting.json` file.

Here is an example of what it might look like:
```
{
    "key": "779ajrhYR7Woez3TbAoBSgYwXLkVmLEvWY6iWcGkk1no",
    "receiver_key_type": "Wallet",
    "mint": "4pM5pFnmhzVCSZnQHq3dkNXYC5SLtkmHYce16VmRghbY",
    "schedules": [
                {"release_time": 1650326333, "amount": 3000000000000}
            ]
}
```

PAY ATTENTION that `amount` is set with precision. It means that amount of tokens you want to distribute should be multiplied by decimals of token mint.

Once json file is ready we can launch command to create vesting contract:
```
cargo run -- --url https://api.testnet.solana.com --payer-keypair MUovFFrR2a3cHD9TwkuiyuXuzmze7SjioUaUGjCYekUwhqiR3QH3qNqq3K9rhxKh6dA6swmfPvAZxZuQ4qgnpeZ --program-id 63spJJWDLFHgnkLTucP9dLECFskMNEPAo8szGdYvsrGy create --vesting-data-file ./vesting.json
```

It's example of creation vesting contract in testnet.

You can see that we set `payer-keypair` value as base58 string. You can find this value in your Phantom wallet. More specifically go to the `settings` -> `Export Private Key`.

And that's it vesting contract is created.