# Espresso staking CLI

WARNING: This CLI is intended for use in testnet purposes only.

A CLI to interact with the stake table contract.

To build and run

    cargo run --bin staking-cli -p staking-cli -- --help

Which will show all available commands.

For brevity the following commands assume `staking-cli` is in the `PATH`.

To show help for a command run `staking-cli COMMAND --help`, for example `staking-cli delegate --help`.

# Espresso Staking CLI

This CLI helps users interact with the Espresso staking contract, either as a delegator or a node operator.

## Getting Started

1.  **Getting Help**: You can get help for the CLI by running:

         staking-cli --help

    ```
        A CLI to interact with the Espresso stake table contract

    Usage: staking-cli [OPTIONS] [COMMAND]

    Commands:
    version                Display version information of the staking-cli
    config                 Display the current configuration
    init                   Initialize the config file with deployment and wallet info
    purge                  Remove the config file
    stake-table            Show the stake table in the Espresso stake table contract
    account                Print the signer account address
    register-validator     Register to become a validator
    update-consensus-keys  Update a validators Espresso consensus signing keys
    deregister-validator   Deregister a validator
    approve                Approve stake table contract to move tokens
    delegate               Delegate funds to a validator
    undelegate             Initiate a withdrawal of delegated funds from a validator
    claim-withdrawal       Claim withdrawal after an undelegation
    claim-validator-exit   Claim withdrawal after validator exit
    token-balance          Check ESP token balance
    token-allowance        Check ESP token allowance of stake table contract
    transfer               Transfer ESP tokens
    stake-for-demo         Register the validators and delegates for the local demo
    help                   Print this message or the help of the given subcommand(s)

    Options:
    -c, --config <CONFIG_PATH>
            Config file
        --rpc-url <RPC_URL>
            L1 Ethereum RPC [env: L1_PROVIDER=]
        --token-address <TOKEN_ADDRESS>
            Deployed ESP token contract address [env: ESP_TOKEN_ADDRESS=]
        --stake-table-address <STAKE_TABLE_ADDRESS>
            Deployed stake table contract address [env: STAKE_TABLE_ADDRESS=]
        --mnemonic <MNEMONIC>
            The mnemonic to use when deriving the key [env: MNEMONIC=]
        --account-index <ACCOUNT_INDEX>
            The mnemonic account index to use when deriving the key [env: ACCOUNT_INDEX=]
        --ledger
            The ledger account index to use when deriving the key [env: LEDGER_INDEX=]
        --backtrace-mode <BACKTRACE_MODE>
            [env: RUST_LOG_FORMAT=] [possible values: full, compact, json]
    -h, --help
            Print help
    -V, --version
            Print version
    ```

    or by passing a command, for example

         staking-cli delegate --help

1.  **Choose your type of wallet**:

    First, determine if you would like to use a Mnemonic phrase or ledger hardware wallet.

    If you don't know which account index to use, you can find it by running:
    ```bash
    staking-cli --mnemonic MNEMONIC --account-index 0 account
    staking-cli --mnemonic MNEMONIC --account-index 1 account
    # etc, or
    staking-cli --ledger-index 0 account
    staking-cli --ledger-index 1 account
    # etc
    ```
    Repeat with different indices until you find the address you want to use.

    Note that for ledger signing to work

    1. the ledger needs to be unlocked,
    1. the Ethereum app needs to be open,
    1. blind signing needs to be enabled in the Ethereum app settings on the ledger.

    To avoid passing the mnemonic on the command line, the MNEMONIC env var can be set instead.

1.  **Initialize the configuration**: Once you've identified your desired account index (here 2), initialize a
    configuration file:

         staking-cli init --mnemonic MNEMONIC --account-index 2
         # or
         staking-cli init --ledger-index 2

    This creates a TOML config file with the contracts of our decaf Testnet, deployed on Sepolia. With the config file
    you don't need to provide the configuration values every time you run the CLI.

    Note that only for this command the `--mnemonic` and `--ledger-index` flags are specified _after_ the command.

1.  **Inspect the configuration**: You can inspect the configuration file by running:

         staking-cli config

1.  **View staking info**: You can use the following command to display the current L1 stake table:

         staking-cli info

1.  **Next steps**:
    - **Node operators** can register a validator using `register-validator`.
    - **Delegators** can delegate stake using `delegate` or undelegate using `undelegate`.

## Delegators

### Delegating

1.  Obtain ESP tokens for staking.
1.  Find the Ethereum address of a validator to delegate to.

        staking-cli stake-table

1.  Use the `approve` command to allow the stake table to spend your tokens.

        staking-cli approve --amount 123

1.  Use the `delegate` command to delegate your tokens to a validator.

        staking-cli delegate --validator-address 0x12...34 --amount 123

### Undelegating

1.  If you would like to undelegate your tokens, use the `undelegate` command.

        staking-cli undelegate --validator-address 0x12...34 --amount 123

1.  Wait for the exit escrow period to end (currently 1 week), then withdraw to your wallet.

        staking-cli claim-withdrawal --validator-address 0x12...34

### Recovering funds after a validator exit

1.  Wait for the exit escrow period to elapse after the Validaotr deregistered itself (currently 1 week), then withdraw
    to your wallet by running

         staking-cli claim-validator-exit --validator-address 0x12...34

## Node operators

### Registering a validator

1.  Obtain your validator's BLS and state private keys and choose your commission in percent, with 2 decimals.
1.  Use the `register-validator` command to register your validator.

        staking-cli register-validator --consensus-private-key BLS_KEY --state-private-key STATE_KEY --commission 4.99

    To avoid specifying the the keys on the command line they can be set via env vars

    ```
    CONSENSUS_PRIVATE_KEY=BLS_SIGNING_KEY~...
    STATE_PRIVATE_KEY=SCHNORR_SIGNING_KEY~...
    ```

### De-registering your validator

WARNING: running this command will remove your validator from the stake table and undelegate all the funds delegated to
it.

    staking-cli deregister-validator

### Rotating your consensus keys

1.  Obtain your validator's new BLS and state private keys.
1.  Run

        staking-cli update-consensus-keys --consensus-private-key BLS_KEY --state-private-key STATE_KEY

    The new keys will become active in the 3rd epoch after the command is run.

    To avoid specifying the the keys on the command line they can be set via env vars

    ```
    CONSENSUS_PRIVATE_KEY=BLS_SIGNING_KEY~...
    STATE_PRIVATE_KEY=SCHNORR_SIGNING_KEY~...
    ```
