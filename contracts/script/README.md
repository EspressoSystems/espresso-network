# Table of Contents

- [Deploying Upgradeable Smart Contracts](#deploying-upgradeable-smart-contracts)
  - [Prerequisites](#prerequisites)
  - [The Fee Contract](#deploying-the-fee-contract)
  - [The Light Client Contract](#deploying-the-light-client-contract)
- [Upgrading Upgradeable Smart Contracts](#upgrading-upgradeable-smart-contracts)
  - [The Fee Contract](#upgrading-the-fee-contract)
  - [The Light Client Contract](#2-upgrade-the-lightclient-contract)
- [Deploying Upgradeable Contracts without a Multisig Admin](#deploying-upgradable-contracts-without-a-safe-multisig-wallet-admin)
- [Deploying the Plonk Verifier](#deploy-the-plonk-verifier-library)
- [Arbitrum Specific Deploys](#arbitrum-deploys)
- [Solutions to Known Errors](#known-errors)

# Deploying Upgradeable Smart Contracts

## Prerequisites

1. **Create a Multisig Wallet**  
   Use [Safe](https://app.safe.global/welcome/accounts) to create a multisig wallet on the network you'd like to deploy
   to.
2. Install `npx` e.g. `brew install npx`
3. **Enter Nix Shell**  
   In the home folder of this repo, start a nix shell by entering `nix-shell` in the terminal.
4. **Compile Contracts (if necessary)** If the contracts have never been compiled, run `forge build`.
5. **Build diff-test library** Run the following:
   - `cargo build --bin diff-test --release`
6. **Set Environment Variables**  
   Set the following values in the `.env.contracts`
   - `STATE_HISTORY_RETENTION_PERIOD`
   - `NUM_INIT_VALIDATORS`
   - `FEE_CONTRACT_ORIGINAL_NAME`
   - `LIGHT_CLIENT_CONTRACT_ORIGINAL_NAME`
   - `PERMISSIONED_PROVER_ADDRESS`
   - `USE_HARDWARE_WALLET` If you're deploying with a hardware wallet set these variables:
   - `DEPLOYER_HARDWARE_WALLET_ADDRESS` Otherwise, set the mnemonic variables:
   - `DEPLOYER_MNEMONIC`
   - `DEPLOYER_MNEMONIC_OFFSET`

## Deployments - Staging (No-Multisig)

#### Deploy PlonkVerifierV2

In `.env` or `.env.contracts` set the following:

- `MNEMONIC`
- `MNEMONIC_OFFSET`
- `RPC_URL`

```bash
source .env.contracts && forge clean && forge script contracts/script/PlonkVerifierV2.s.sol:DeployPlonkVerifierV2Script --rpc-url $RPC_URL --build-info true --legacy --broadcast
```

#### Deploy LightClient V1

```bash
source .env.contracts &&    forge clean &&  forge script contracts/script/LightClientStaging.s.sol:DeployLightClientContractScript --sig "run(uint32,uint32)" $NUM_INIT_VALIDATORS $STATE_HISTORY_RETENTION_PERIOD --ffi --rpc-url $RPC_URL --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS --libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:$PLONK_VERIFIER_V2_ADDRESS --broadcast
```

#### Upgrade to LightClientV2 (without multisig admin)

In `.env` or `.env.contracts` set the following:

- `MNEMONIC`
- `MNEMONIC_OFFSET`
- `RPC_URL`
- `LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS`
- `PLONK_VERIFIER_ADDRESS`
- `PLONK_VERIFIER_V2_ADDRESS`
- `BLOCKS_PER_EPOCH`

```bash
source .env.contracts &&    forge clean &&  forge script contracts/script/LightClientStaging.s.sol:UpgradeLightClientWithoutMultisigAdminScript --sig "run(address)" $LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS --ffi --rpc-url $RPC_URL --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS --libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:$PLONK_VERIFIER_V2_ADDRESS --broadcast
```

## Deployments (Multisig)

## Deploying the Fee Contract

### 1. Deploy

#### Via a Software Wallet

1. Run the following command in the home directory:

```bash
   source .env.contracts && \
   forge clean && \
   forge script contracts/script/FeeContract.s.sol:DeployFeeContractScript $SAFE_MULTISIG_ADDRESS \
   --sig 'run(address)' \
   --ffi \
   --rpc-url https://ethereum-sepolia.publicnode.com  \
   --build-info true \
   --legacy \
   --broadcast
```

#### Via Ledger Hardware Wallet

- Set the `DEPLOYER_HARDWARE_WALLET_ADDRESS` and `USE_HARDWARE_WALLET=true` in `.env.contracts`

1. Run the following command in the home directory:

```bash
   source .env.contracts && \
   forge clean && \
   forge script contracts/script/FeeContract.s.sol:DeployFeeContractScript $SAFE_MULTISIG_ADDRESS \
   --sig 'run(address)' \
   --ffi \
   --rpc-url https://ethereum-sepolia.publicnode.com  \
   --build-info true \
   --legacy \
   --ledger \
   --broadcast
```

Example successful deployment response

```bash
[⠒] Compiling...
No files changed, compilation skipped
2024-03-11T16:41:29.919133Z  WARN foundry_evm_core::fork::cache: Failed to read cache file err=Os { code: 2, kind: NotFound, message: "No such file or directory" } path="/Users/alysiahuggins/.foundry/cache/rpc/sepolia/5464723"
Script ran successfully.

== Return ==
proxy: address payable 0x61B4C96475B99A6ce01AfF0da7910605D048c125
multisig: address 0xc56fA6505d10bF322e01327e22479DE78C3Bf1cE
```

### 2. Verify the Contract

Set the following variables in `.env.contracts`

- `ETHERSCAN_API_KEY`
- `SOLC_VERSION`
- `FEE_CONTRACT_ADDRESS` (the implementation address)

You can get the SOLC_VERSION by running `solc --version` in command line.

```bash
forge verify-contract --chain-id 11155111 \
--watch --etherscan-api-key $ETHERSCAN_API_KEY \
--compiler-version $SOLC_VERSION \
$FEE_CONTRACT_ADDRESS \
contracts/src/FeeContract.sol:FeeContract
```

### 3. Inform Etherscan about your proxy

Inform Etherscan that it's a Proxy When the proxy is deployed, go to Etherscan. Go to `Contract > Code > More Options`
and select the `is this a proxy?` option. You should then be able to interact with the implementation contract via a
proxy.

---

## Deploying the Light Client Contract

Since the LightClient contract uses the `PlonkVerifier` library, the `PlonkVerifier` library has to be **deployed** and
then referenced at deployment time. Ensure you've deployed the PlonkVerifier
[see steps below](#deploy-the-plonk-verifier-library) and set the`$PLONK_VERIFIER_ADDRESS` variable in the command
below. Each time modifications are made to the Plonk Verifier, contracts that depend on it such as the Light Client
contract have to be upgraded and should use the new PlonkVerifier contract address as part of the deployment.

### Prerequisites:

- Deploy the PlonkVerifier ([see steps below](#deploy-the-plonk-verifier-library)
- Ensure the following are in the `.env.contracts` file.
  - `RPC_URL`
  - `SAFE_MULTISIG_ADDRESS`
  - `PLONK_VERIFIER_ADDRESS`
  - `DEPLOYER_MNEMONIC`
  - `DEPLOYER_MNEMONIC_OFFSET`

### 1. Deploy

#### Via a Software Wallet

In `.env.contracts` set:

- `USE_HARDWARE_WALLET=false`

1. Run the following command in the home directory:

```bash
   source .env.contracts && \
   forge clean && \
   forge script contracts/script/LightClient.s.sol:DeployLightClientScript $NUM_INIT_VALIDATORS $STATE_HISTORY_RETENTION_PERIOD $SAFE_MULTISIG_ADDRESS \
   --sig 'run(uint32, uint32, address)' \
   --ffi \
   --rpc-url https://ethereum-sepolia.publicnode.com  \
   --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS \
   --build-info true \
   --legacy \
   --broadcast
```

#### Via a Hardware Wallet

In `.env.contracts` set:

- `DEPLOYER_HARDWARE_WALLET_ADDRESS`
- `USE_HARDWARE_WALLET=true`

1. Run the following command in the home directory:

```bash
   source .env.contracts && \
   forge clean && \
   forge script contracts/script/LightClient.s.sol:DeployLightClientScript $NUM_INIT_VALIDATORS $STATE_HISTORY_RETENTION_PERIOD $SAFE_MULTISIG_ADDRESS \
   --sig 'run(uint32, uint32, address)' \
   --ffi \
   --rpc-url https://ethereum-sepolia.publicnode.com  \
   --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS \
   --build-info true \
   --legacy \
   --ledger \
   --broadcast
```

### 2. Verify the Contract

Set the following variables in `.env.contracts`

- `ETHERSCAN_API_KEY`
- `SOLC_VERSION`
- `LIGHT_CLIENT_CONTRACT_ADDRESS` (the implementation address)

```bash
forge verify-contract --chain-id 11155111 \
--watch --etherscan-api-key $ETHERSCAN_API_KEY \
--compiler-version $SOLC_VERSION \
$LIGHT_CLIENT_CONTRACT_ADDRESS \
contracts/src/LightClient.sol:LightClient \
--libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS
```

### 3. Inform Etherscan about your proxy

Inform Etherscan that it's a Proxy When the proxy is deployed, go to Etherscan. Go to `Contract > Code > More Options`
and select the `is this a proxy?` option. You should then be able to interact with the implementation contract via a
proxy.

### 4. Set Permissioned Prover

To enable the permissioned prover on the light client contract, ensure that the following environment variables are set
in the `.env.contracts` file:

---

<br/>
<br/>

# Upgrading Upgradeable Smart Contracts

## Upgrading the Fee Contract

### Via a Software Wallet

In `.env.contracts` set:

- `USE_HARDWARE_WALLET=false`

1. Run the following command in the home directory:

```bash
source .env.contracts && \
forge clean && \
forge script contracts/script/FeeContract.s.sol:UpgradeFeeContractScript \
--ffi \
--rpc-url https://ethereum-sepolia.publicnode.com  \
--build-info true \
--legacy \
--broadcast
```

## Upgrading the Light Client Contract (with Multisig owner)

Ensure that you update the version in the `getVersion()` method of the latest implementation contract.

Since the LightClient contract uses the PlonkVerifier & PlonkVerifierV2 libraries, they have to be deployed and then
referenced at deployment time. Thus ensure you've deployed the PlonkVerifier
([see steps below](#deploy-the-plonk-verifier-library)) and set the `$PLONK_VERIFIER_ADDRESS` variable in the command
below. Each time modifications are made to the Plonk Verifier, contracts that depend on it such as the Light Client
contract have to be upgraded and should use the new PlonkVerifier contract address as part of the deployment.

### Via a Software Wallet

### Upgrade to Patch (modify epoch start block)

In `.env.contracts` ensure the following value is updated to the desired value:

- `EPOCH_START_BLOCK`

1. Run the following command in the home directory:

```bash
source .env.contracts && \
forge clean && \
forge script contracts/script/LightClient.s.sol:LightClientContractUpgradeToV2PatchScript \
--ffi \
--rpc-url $RPC_URL \
--libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS \
--libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:$PLONK_VERIFIER_V2_ADDRESS \
--build-info true \
--broadcast
```

2. Go to safe.global and have the signers confirm the transaction and finally execute it

### Via Hardware Wallet

In `.env.contracts` set:

- `USE_HARDWARE_WALLET=true` and add the `--ledger` flag to the `forge script` command

# Deploy the Plonk Verifier Library

## Via a Software Wallet

The Plonk Verifier contract is not upgradeable. Each time modifications are made to the Plonk Verifier, contracts that
depend on it such as the Light Client contract have to be upgraded and should use the new PlonkVerifier contract address
as part of the deployment.

Ensure that you update the salt, `PLONK_VERIFIER_SALT`, in the `.env.contracts` file before each deployment.

```bash
source .env.contracts && \
forge clean && \
forge script contracts/script/PlonkVerifier.s.sol:DeployPlonkVerifierScript \
--ffi \
--rpc-url https://ethereum-sepolia.publicnode.com \
--build-info true \
--legacy \
--broadcast
```

## Via a Hardware Wallet

- Ensure that you update the salt, `PLONK_VERIFIER_SALT`, in the `.env.contracts` file before each deployment.
- Set the `DEPLOYER_HARDWARE_WALLET_ADDRESS` and `USE_HARDWARE_WALLET=true` in `.env.contracts`

```bash
source .env.contracts && \
forge clean && \
forge script contracts/script/PlonkVerifier.s.sol:DeployPlonkVerifierScript \
--ffi \
--rpc-url https://ethereum-sepolia.publicnode.com \
--build-info true \
--legacy \
--ledger \
--broadcast
```

# Arbitrum Deploys

## Deploy LightClientArbitrum (no multisig admin)

1. Ensure the following are in an env file named, `.env.contracts.arbSepolia`

```bash
export RPC_URL=
export MNEMONIC=
export ACCOUNT_INDEX=
export MNEMONIC_OFFSET=
export PLONK_VERIFIER_ADDRESS=
export BLOCKS_PER_EPOCH=
export EPOCH_START_BLOCK=
export STATE_HISTORY_RETENTION_PERIOD=
export NUM_INIT_VALIDATORS=
export USE_HARDWARE_WALLET=
```

2. Then in a terminal, run the following:

```bash
source .env.contracts.arbSepolia && forge clean &&  forge script contracts/script/LightClientArbitrumStaging.s.sol:DeployLightClientArbitrumContractScript --sig "run(uint32,uint32)" $NUM_INIT_VALIDATORS $STATE_HISTORY_RETENTION_PERIOD --ffi --rpc-url $RPC_URL --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS --broadcast
```

## Upgrade to LightClientArbitrumV2 (no multisig admin)

1. Ensure that you've deployed [`PlonkVerifierV2`](#deploy-plonkverifierv2)

2. In the `.env.contracts.arbSepolia` file, add/update the following:

```bash
export LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS=
export PLONK_VERIFIER_V2_ADDRESS=
```

Check this [section](#deploy-lightclientarbitrum-no-multisig-admin) for other fields needed in the environment file

3. Then in a terminal, run the following:

```bash
source .env.contracts.arbSepolia && forge clean && forge script contracts/script/LightClientArbitrumStaging.s.sol:UpgradeLightClientArbitrumV2Script --sig "run(address)" $LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS --ffi --rpc-url $RPC_URL --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS --libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:$PLONK_VERIFIER_V2_ADDRESS --broadcast
```

## Upgrade to LightClientArbitrumV2 Patch (no multisig admin)

This patch adds functionality to modify the `epochStartBlock` and it also calls that method `updateEpochStartBlock(...)`
to set the new epoch start block upon upgrading. So ensure the new `EPOCH_START_BLOCK` value is in the env file.

1. Ensure that you've deployed [`PlonkVerifierV2`](#deploy-plonkverifierv2)

2. In the `.env.contracts.arbSepolia` file, add/update the following:

```bash
export LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS=
export PLONK_VERIFIER_V2_ADDRESS=
export EPOCH_START_BLOCK=
```

Check this [section](#deploy-lightclientarbitrum-no-multisig-admin) for other fields needed in the environment file

3. Then in a terminal, run the following:

```bash
source .env.contracts.arbSepolia && forge clean && forge script contracts/script/LightClientArbitrumStaging.s.sol:UpgradeLightClientArbitrumV2PatchScript --sig "run(address)" $LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS --ffi --rpc-url $RPC_URL --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS --libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:$PLONK_VERIFIER_V2_ADDRESS --broadcast
```

# Known Errors

1. Error Parsing ABI for contract Scenario: You ran `just gen-bindings` Example:

```bash
Error:
error parsing abi for contract '_70c760a3e059d83dbf77da7f6778fbc0': couldn't parse ABI string as either human readable (1) or JSON (2):
1. Illegal abi `{`, expected function
2. data did not match any variant of untagged enum JsonContract
error: Recipe `gen-bindings` failed on line 65 with exit code 1
```

This error occurs when build_info is set to true in the foundry.toml configuration. Ensure that this is false or the
foundry profile is set to default when running commands like `just gen-bindings`.

Solution: `export FOUNDRY_PROFILE=default`

## Deploy LightClientArbitrum

Additional Pre-requisites:

- an API key from arbiscan to verify the contract.

```bash
 source .env.contracts && \
   forge clean && \
   forge script contracts/script/LightClientArb.s.sol:DeployLightClientArbitrum $NUM_INIT_VALIDATORS $STATE_HISTORY_RETENTION_PERIOD $SAFE_MULTISIG_ADDRESS \
   --sig 'run(uint32, uint32, address)' \
   --ffi \
   --rpc-url $RPC_URL \
   --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:$PLONK_VERIFIER_ADDRESS \
   --build-info true \
   --verify --etherscan-api-key $ARBISCAN_API_KEY \
   --broadcast
```
