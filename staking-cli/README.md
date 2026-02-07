# Espresso staking CLI

WARNING: This CLI is intended for testing as well as validator registrations purposes only. Stakers should use the
staking UI:

- decaf testnet staking UI: https://staking.decaf.testnet.espresso.network/
- mainnet staking UI: not yet available

This CLI helps users interact with the Espresso staking contract, either as a delegator or a node operator.

Contracts:

- Decaf stake table (on Sepolia):
  [0x40304fbe94d5e7d1492dd90c53a2d63e8506a037](https://sepolia.etherscan.io/address/0x40304fbe94d5e7d1492dd90c53a2d63e8506a037)
- Mainnet stake table:
  [0xCeF474D372B5b09dEfe2aF187bf17338Dc704451](https://etherscan.io/address/0xcef474d372b5b09defe2af187bf17338dc704451)

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc-refresh-toc -->

**Table of Contents**

- [Espresso staking CLI](#espresso-staking-cli)
  - [Getting Started](#getting-started)
    - [Overview](#overview)
    - [Choose your type of wallet (mnemonic, private key, or Ledger)](#choose-your-type-of-wallet-mnemonic-private-key-or-ledger)
    - [Initialize the configuration file (optional)](#initialize-the-configuration-file-optional)
    - [Managing multiple network configurations](#managing-multiple-network-configurations)
    - [Inspect the configuration](#inspect-the-configuration)
    - [View the stake table](#view-the-stake-table)
  - [Calldata Export (for Multisig Wallets)](#calldata-export-for-multisig-wallets)
    - [Calldata Simulation](#calldata-simulation)
  - [Delegators (or stakers)](#delegators-or-stakers)
    - [Delegating](#delegating)
    - [Undelegating](#undelegating)
    - [Recovering funds after a validator exit](#recovering-funds-after-a-validator-exit)
    - [Claiming staking rewards](#claiming-staking-rewards)
  - [Node operators](#node-operators)
    - [Registering a validator](#registering-a-validator)
      - [Validator Metadata](#validator-metadata)
      - [Registration Command](#registration-command)
    - [Updating your commission](#updating-your-commission)
    - [Updating your metadata URL](#updating-your-metadata-url)
      - [Metadata JSON Schema (for custom hosting)](#metadata-json-schema-for-custom-hosting)
    - [De-registering your validator](#de-registering-your-validator)
    - [Rotating your consensus keys](#rotating-your-consensus-keys)
    - [Exporting Node Signatures](#exporting-node-signatures)
    - [Native Demo Staking](#native-demo-staking)

<!-- markdown-toc end -->

To run the staking-cli using Docker:

```bash
docker run -it ghcr.io/espressosystems/espresso-sequencer/staking-cli:main staking-cli --help
```

To build and run from source:

```bash
cargo run --bin staking-cli -p staking-cli -- --help
```

For brevity what follows assumes the `staking-cli` binary is in the `PATH` (or aliased to the Docker command).

To show help for a command run `staking-cli COMMAND --help`, for example `staking-cli delegate --help`.

If you run into any problems please open an issue on https://github.com/EspressoSystems/espresso-network.

To build tools that interact with the stake table contract the ABI can be found at
[../contracts/artifacts/abi/StakeTable.json](../contracts/artifacts/abi/StakeTable.json).

## Getting Started

### Overview

You can get help for the CLI by running:

```bash
staking-cli --help
```

Which will show all the available commands and options shared by commands:

```text
A CLI to interact with the Espresso stake table contract

Usage: staking-cli [OPTIONS] [COMMAND]

Commands:
  version                 Display version information of the staking-cli
  config                  Display the current configuration
  init                    Initialize the config file with deployment and wallet info
  purge                   Remove the config file
  stake-table             Show the stake table in the Espresso stake table contract
  account                 Print the signer account address
  register-validator      Register to become a validator
  update-consensus-keys   Update a validators Espresso consensus signing keys
  deregister-validator    Deregister a validator
  update-commission       Update validator commission rate
  update-metadata-uri     Update validator metadata URL
  approve                 Approve stake table contract to move tokens
  delegate                Delegate funds to a validator
  undelegate              Initiate a withdrawal of delegated funds from a validator
  claim-withdrawal        Claim withdrawal after an undelegation
  claim-validator-exit    Claim withdrawal after validator exit
  claim-rewards           Claim staking rewards
  unclaimed-rewards       Check unclaimed staking rewards
  token-balance           Check ESP token balance
  token-allowance         Check ESP token allowance of stake table contract
  transfer                Transfer ESP tokens
  stake-for-demo          Register the validators and delegates for the local demo
  export-node-signatures  Export validator node signatures for address validation
  preview-metadata        Preview metadata from a URL without registering
  help                    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG_PATH>
          Config file

      --rpc-url <RPC_URL>
          L1 Ethereum RPC

          [env: L1_PROVIDER=]

      --stake-table-address <STAKE_TABLE_ADDRESS>
          Deployed stake table contract address

          [env: STAKE_TABLE_ADDRESS=]

      --espresso-url [<ESPRESSO_URL>]
          Espresso sequencer API URL for reward claims

          [env: ESPRESSO_URL=]

      --mnemonic <MNEMONIC>
          The mnemonic to use when deriving the key

          [env: MNEMONIC=]

      --private-key <PRIVATE_KEY>
          Raw private key (hex-encoded with or without 0x prefix)

          [env: PRIVATE_KEY=]

      --account-index <ACCOUNT_INDEX>
          The mnemonic account index to use when deriving the key

          [env: ACCOUNT_INDEX=]

      --ledger
          Use a ledger device to sign transactions.

          NOTE: ledger must be unlocked, Ethereum app open and blind signing must be enabled in the Ethereum app settings.

          [env: USE_LEDGER=]

      --export-calldata
          Export calldata for multisig wallets instead of sending transaction

          [env: EXPORT_CALLDATA=]

      --sender-address [<SENDER_ADDRESS>]
          Sender address for calldata export (required for simulation)

          [env: SENDER_ADDRESS=]

      --skip-simulation
          Skip eth_call validation when exporting calldata

          [env: SKIP_SIMULATION=]

      --skip-metadata-validation
          Skip metadata URI validation (fetch and schema check)

          [env: SKIP_METADATA_VALIDATION=]

      --output <OUTPUT>
          Output file path. If not specified, outputs to stdout

      --format <FORMAT>
          Output format

          [possible values: json, toml]

  -h, --help
          Print help (see a summary with '-h')
```

or by passing `--help` to a command, for example `delegate`:

```bash
staking-cli delegate --help
```

which will show the options specific to the command:

```text
Delegate funds to a validator

Usage: staking-cli delegate --validator-address <VALIDATOR_ADDRESS> --amount <AMOUNT>

Options:
    --validator-address <VALIDATOR_ADDRESS>
    --amount <AMOUNT>
-h, --help                                   Print help
```

### Choose your type of wallet (mnemonic, private key, or Ledger)

**Security** Utmost care must be taken to avoid leaking the Ethereum private key used for staking or registering
validators. There is currently no built-in key rotation feature for Ethereum keys.

First, determine which signing method you would like to use:

1. **Ledger hardware wallet** - (recommended) sign transactions with a Ledger device
1. **Mnemonic phrase** - derive keys from a BIP-39 mnemonic with account index
1. **Private key** - use a raw hex-encoded private key directly

**Security recommendations:** For managing significant funds on mainnet, we recommend using a hardware wallet (Ledger)
for extra security. Hardware wallets keep your private keys isolated from your computer, offering some protection
against malware and phishing attacks. If you need support for other hardware signers, please open an issue at
https://github.com/EspressoSystems/espresso-network.

For mnemonics and private keys, to avoid passing secrets on the command line, use environment variables:

- `MNEMONIC` for mnemonic phrase
- `PRIVATE_KEY` for raw private key

If using a ledger or mnemonic and you don't know which account index to use, you can find it by running:

```bash
staking-cli --mnemonic MNEMONIC --account-index 0 account
staking-cli --mnemonic MNEMONIC --account-index 1 account
# etc, or
staking-cli --ledger --account-index 0 account
staking-cli --ledger --account-index 1 account
# etc
```

Repeat with different indices until you find the address you want to use.

If using a private key, ensure PRIVATE_KEY env var is set

```bash
staking-cli account
```

Note that for ledger signing to work

1. the ledger needs to be unlocked,
1. the Ethereum app needs to be open,
1. blind signing needs to be enabled in the Ethereum app settings on the ledger.

### Initialize the configuration file (optional)

Once you've identified your desired account index (here 2), initialize a configuration file:

```bash
# For mainnet
staking-cli init --network mainnet --mnemonic MNEMONIC --account-index 2
# For decaf testnet
staking-cli init --network decaf --mnemonic MNEMONIC --account-index 2
# For local development
staking-cli init --network local --mnemonic MNEMONIC --account-index 2

# With ledger
staking-cli init --network mainnet --ledger --account-index 2
# With private key
staking-cli init --network mainnet --private-key 0x1234...abcd
```

The `--network` parameter is **required** and accepts:

- `mainnet` - Espresso mainnet on Ethereum mainnet
- `decaf` - Decaf testnet on Sepolia
- `local` - Local development (localhost RPC)

This creates a TOML config file with the appropriate contract addresses and RPC endpoints. With the config file you
don't need to provide the configuration values every time you run the CLI. If no config file exists, all values must be
provided via command-line arguments or environment variables.

You can also set the network via environment variable: `NETWORK=mainnet staking-cli init --mnemonic MNEMONIC`

NOTE: For this `init` command, wallet flags are specified _after_ the command. The `-c` flag (config path) goes before.

### Managing multiple network configurations

To work with multiple networks (e.g., both mainnet and decaf), use the `-c` flag to specify different config files:

```bash
# Create mainnet config with mnemonic from env var
MNEMONIC='your mnemonic' staking-cli -c mainnet.toml init --network mainnet --account-index 0

# Create decaf config with ledger
staking-cli -c decaf.toml init --network decaf --ledger --account-index 0

# Use specific config for commands
staking-cli -c mainnet.toml stake-table
staking-cli -c decaf.toml delegate --validator-address 0x... --amount 100
```

When no `-c` flag is provided, the CLI uses a platform-specific default path (e.g.,
`~/.config/espresso/espresso-staking-cli/config.toml` on Linux).

### Inspect the configuration

You can inspect the configuration file by running:

```bash
staking-cli config
```

### View the stake table

You can use the following command to display the current L1 stake table:

```bash
staking-cli stake-table
```

## Calldata Export (for Multisig Wallets)

If you're using a multisig wallet (e.g., Safe, Gnosis Safe) or other smart contract wallet, you can export the
transaction calldata instead of signing and sending the transaction directly. This allows you to propose the transaction
through your multisig's interface.

To export calldata for any command, add the `--export-calldata` flag:

```bash
# Export delegate calldata as JSON (default)
staking-cli --export-calldata delegate --validator-address 0x12...34 --amount 100

# Export as TOML
staking-cli --export-calldata --format toml delegate --validator-address 0x12...34 --amount 100

# Save to file
staking-cli --export-calldata --format json --output delegate.json delegate --validator-address 0x12...34 --amount 100
```

The output includes the target contract address and the encoded calldata:

```json
{
  "to": "0x...",
  "data": "0x..."
}
```

This works with all state-changing commands: `approve`, `delegate`, `undelegate`, `claim-withdrawal`,
`claim-validator-exit`, `claim-rewards`, `register-validator`, `update-commission`, `update-metadata-uri`,
`update-consensus-keys`, `deregister-validator`, and `transfer`.

Note: When using `--export-calldata`, no wallet/signer is required since the transaction is not sent.

### Calldata Simulation

By default, the CLI simulates exported calldata via `eth_call` to catch errors before you submit the transaction through
your multisig. Provide `--sender-address` (your multisig address) for accurate simulation:

```bash
staking-cli --export-calldata --sender-address 0xYourSafe... delegate --validator-address 0x12...34 --amount 100
```

To skip simulation (e.g., for batch exports):

```bash
staking-cli --export-calldata --skip-simulation delegate --validator-address 0x12...34 --amount 100
```

Note: The `claim-rewards` command always requires `--sender-address` (even with `--skip-simulation`) because the address
is needed to fetch the reward proof from the Espresso node:

```bash
staking-cli --export-calldata --sender-address 0xYourSafe... --espresso-url https://... claim-rewards
```

## Delegators (or stakers)

This section covers commands for stakers/delegators.

### Delegating

1.  Obtain ESP tokens for staking.
1.  Find the Ethereum address of a validator to delegate to.

    ```bash
    staking-cli stake-table
    ```

1.  Use the `approve` command to allow the stake table to spend your tokens.

    ```bash
    staking-cli approve --amount 123
    ```

1.  Use the `delegate` command to delegate your tokens to a validator.

    ```bash
    staking-cli delegate --validator-address 0x12...34 --amount 123
    ```

### Undelegating

1.  If you would like to undelegate your tokens, use the `undelegate` command.

    ```bash
    staking-cli undelegate --validator-address 0x12...34 --amount 123
    ```

1.  Wait for the exit escrow period to end (currently 1 week), then withdraw to your wallet.

    ```bash
    staking-cli claim-withdrawal --validator-address 0x12...34
    ```

### Recovering funds after a validator exit

1.  Wait for the exit escrow period to elapse after the validator deregistered itself (currently 1 week), then withdraw
    to your wallet by running

    ```bash
    staking-cli claim-validator-exit --validator-address 0x12...34
    ```

### Claiming staking rewards

Delegators and validators can earn staking rewards. To check and claim your rewards:

1.  Check your unclaimed rewards:

    ```bash
    staking-cli unclaimed-rewards
    ```

    This will display the amount of unclaimed rewards in ESP tokens.

2.  Claim your rewards:

    ```bash
    staking-cli claim-rewards
    ```

    This will transfer your unclaimed rewards to your wallet.

Note: You need to set the `espresso_url` in your config file or pass `--espresso-url` flag to use these commands.

## Node operators

This section covers commands for node operators.

### Registering a validator

#### Validator Metadata

Metadata is optional and provides information displayed in the staking UI (name, description, icon, etc.).

Options for `--metadata-uri`:

1. **Node `/status/metrics` endpoint (recommended):**

- `--metadata-uri https://my-validator.example.com/status/metrics`
- Existing nodes already expose this.

2. **Custom JSON file:**

- `--metadata-uri https://example.com/metadata.json`
- See [JSON schema](#metadata-json-schema-for-custom-hosting).

3. **No metadata:** `--no-metadata-uri`

Use `--skip-metadata-validation` if your endpoint isn't ready yet. URL cannot exceed 2048 bytes.

Preview what will be extracted before registering:

```bash
staking-cli preview-metadata --metadata-uri https://my-validator.example.com/status/metrics
```

#### Registration Command

```bash
staking-cli register-validator \
    --consensus-private-key BLS_SIGNING_KEY~... \
    --state-private-key SCHNORR_SIGNING_KEY~... \
    --commission 4.99 \
    --metadata-uri https://my-validator.example.com/status/metrics
```

To avoid keys on the command line, use env vars (`CONSENSUS_PRIVATE_KEY`, `STATE_PRIVATE_KEY`) or pre-signed signatures
(see [Exporting Node Signatures](#exporting-node-signatures)):

```bash
staking-cli register-validator --node-signatures signatures.json --commission 4.99 \
    --metadata-uri https://my-validator.example.com/status/metrics
```

Notes:

- Each Ethereum account needs gas funds (~300k gas for registration)
- Each BLS key can only be registered once
- Each Ethereum account can only register one validator
- Commission can be updated later via `update-commission` (subject to rate limits)
- Metadata URL can be updated anytime via `update-metadata-uri`

### Updating your commission

Validators can update their commission rate, subject to the following rate limits:

- Commission updates are limited to once per week (7 days by default)
- Commission increases are capped at 5% per update (e.g., from 10% to 15%)
- Commission decreases have no limit

To update your commission:

```bash
staking-cli update-commission --new-commission 7.5
```

The commission value is in percent with up to 2 decimal points: from 0.00 to 100.00.

Note: The minimum time interval and maximum increase are contract parameters that may be adjusted by governance.

### Updating your metadata URL

```bash
staking-cli update-metadata-uri --metadata-uri https://my-validator.example.com/status/metrics \
    --consensus-public-key BLS_VER_KEY~...
```

See [Validator Metadata](#validator-metadata) for format options. Use `--no-metadata-uri` to clear.

#### Metadata JSON Schema (for custom hosting)

If hosting a custom JSON file instead of using your node's metrics endpoint:

```json
{
  "pub_key": "BLS_VER_KEY~...",
  "name": "My Validator",
  "description": "Description",
  "company_name": "Acme Inc.",
  "company_website": "https://acme.com/",
  "client_version": "v1.0.0",
  "icon": {
    "14x14": { "@1x": "https://example.com/icon-14.png", "@2x": "...", "@3x": "..." },
    "24x24": { "@1x": "https://example.com/icon-24.png", "@2x": "...", "@3x": "..." }
  }
}
```

Only `pub_key` is required (must match your registered key to prevent impersonation). All other fields are optional.

### De-registering your validator

WARNING: running this command will remove your validator from the stake table and undelegate all the funds delegated to
it.

```bash
staking-cli deregister-validator
```

### Rotating your consensus keys

1.  Obtain your validator's new BLS and state private keys.
1.  Run

    ```bash
    staking-cli update-consensus-keys --consensus-private-key BLS_KEY --state-private-key STATE_KEY
    ```

    The new keys will become active in the 3rd epoch after the command is run.

    To avoid specifying the the keys on the command line they can be set via env vars

    ```text
    CONSENSUS_PRIVATE_KEY=BLS_SIGNING_KEY~...
    STATE_PRIVATE_KEY=SCHNORR_SIGNING_KEY~...
    ```

    Alternatively, you can use pre-signed signatures:

    ```bash
    staking-cli update-consensus-keys --node-signatures signatures.json
    staking-cli update-consensus-keys --node-signatures signatures.toml --format toml
    ```

### Exporting Node Signatures

To avoid mixing Espresso and Ethereum keys on a single host we can pre-sign the validator address for registration and
key updates. The exported payload can later be used to build the Ethereum transaction on another host.

```bash
staking-cli export-node-signatures --address 0x12...34 \
    --consensus-private-key <BLS_KEY> --state-private-key <STATE_KEY>
```

Output formats:

- JSON to stdout (default):
  `staking-cli export-node-signatures --address 0x12...34 --consensus-private-key <BLS_KEY> --state-private-key <STATE_KEY>`
- JSON to file: `--output signatures.json`
- TOML to file: `--output signatures.toml`
- Explicit format override: `--output signatures.json --format toml` (saves TOML content to .json file)

The command will generate a signature payload file that doesn't contain any secrets:

```toml
address = "0x..."
bls_vk = "BLS_VER_KEY~..."
bls_signature = "BLS_SIG~..."
schnorr_vk = "SCHNORR_VER_KEY~..."
schnorr_signature = "SCHNORR_SIG~..."
```

The exported signatures can then be used in validator operations:

```bash
staking-cli register-validator --node-signatures signatures.json --commission 4.99
staking-cli update-consensus-keys --node-signatures signatures.json
```

Format handling:

- File extension auto-detection: `.json` and `.toml` files are automatically parsed in the correct format
- Stdin defaults to JSON: `cat signatures.json | staking-cli register-validator --node-signatures - --commission 4.99`
- Explicit format for stdin:
  `cat signatures.toml | staking-cli register-validator --node-signatures - --format toml --commission 4.99`

### Native Demo Staking

The `stake-for-demo` command is used to set up validators and delegators for testing purposes.

```bash
staking-cli stake-for-demo --num-validators 5
```

Configuration options:

- `--num-validators`: Number of validators to register (default: 5)
- `--num-delegators-per-validator`: Number of delegators per validator (default: random 2-5, max: 100,000)
- `--delegation-config`: Delegation configuration mode (default: variable-amounts)
  - `equal-amounts`: All validators have equal delegation amounts
  - `variable-amounts`: Variable delegation amounts per validator
  - `multiple-delegators`: Multiple delegators per validator
  - `no-self-delegation`: Validators do not self-delegate

Environment variables:

- `NUM_DELEGATORS_PER_VALIDATOR`: Set the number of delegators per validator
- `DELEGATION_CONFIG`: Set the delegation configuration mode

Example usage:

```bash
# Create 10 validators with 50 delegators each
staking-cli stake-for-demo --num-validators 10 --num-delegators-per-validator 50

# Using environment variables with native demo
env NUM_DELEGATORS_PER_VALIDATOR=1000 DELEGATION_CONFIG=no-self-delegation just demo-native-drb-header
```
