# Contract Deployer

## Table of Contents

- [Prerequisites](#prerequisites)
- [Assumptions](#assumptions)
- [Fee Contract](#fee-contract)
- [Token](#token)
- [Timelock Proposals](#timelock-proposals)
- [Safe Multisig Proposals](#safe-multisig-proposals)
- [Troubleshooting](#troubleshooting)
- [POS Deployment](#pos-deployment)

## Prerequisites

- Rust and Cargo installed
- Docker and Docker Compose installed
- Foundry (for verification commands)
- Access to an Ethereum RPC endpoint

## Assumptions

- the config in .env file is valid, if not, check the readme or code for the requirements
- if using multisigs, the eth network is supported by Safe SDK

# Fee Contract

If you would like to fork the rpc url, then in one terminal (assuming foundry is installed)

```bash
anvil --fork-url $RPC_URL
```

Your RPC_URL will now be http://localhost:8545

In the terminal where the deployments will occur:

```bash
export RPC_URL=http://localhost:8545
```

## EOA Owner

### Deploying with Cargo

```bash
# Set environment variables
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
unset ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS

# Execute the deployment command
RUST_LOG=info cargo run --bin deploy -- --deploy-fee --rpc-url=$RPC_URL
```

### Transfer Ownership with Cargo

This section covers transferring ownership directly from an EOA (Externally Owned Account) to a new owner address.

#### Prerequisites

- The contract must be deployed and owned by an EOA (not a multisig or timelock)
- The `ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS` environment variable is set to the correct proxy address for your
  current environment.
- You must have access to the current owner's private key/mnemonic
- The new owner address must be a valid, non-zero Ethereum address

#### Transferring Ownership with Cargo

```bash
set -a
source .env
set +a
export ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER=0xNEWOWNERADDRESS
RUST_LOG=info cargo run --bin deploy -- \
--rpc-url=$RPC_URL \
--transfer-ownership-from-eoa \
--target-contract FeeContract \
--transfer-ownership-new-owner $ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER
```

### Transferring Ownership with Docker Compose

```bash
export RPC_URL=http://host.docker.internal:8545
```

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
  deploy-sequencer-contracts \
  deploy --rpc-url=$RPC_URL \
  --transfer-ownership-from-eoa \
  --target-contract FeeContract \
  --transfer-ownership-new-owner $ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER
```

#### Supported Contract Types

The following contract types are supported for EOA ownership transfer:

- `lightclient` or `lightclientproxy` - LightClientProxy
- `feecontract` or `feecontractproxy` - FeeContractProxy
- `esptoken` or `esptokenproxy` - EspTokenProxy
- `staketable` or `staketableproxy` - StakeTableProxy

#### Verification

After the transfer is completed, verify the ownership change on-chain:

```bash
# Verify the new owner is set correctly
cast call $ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER
```

## Multisig Owner

### Deploying with Cargo

```bash
# Set the env vars
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS

# Deploy the fee contract with a multisig owner (requires ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS to be set which occurs in the step above)
RUST_LOG=info cargo run --bin deploy -- --deploy-fee --rpc-url=$RPC_URL
```

## Timelock Owner

### Note:

The code sets the OpsTimelock as the owner of the FeeContract

### Deploying with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --use-timelock-owner --rpc-url=$RPC_URL
```

### Deploying Fee Contract with Docker compose

1. Ensure the deploy image was built, if not, run in the home directory of this repo.

```bash
./scripts/build-docker-images-native --image deploy
```

2. Set the RPC URL env var, example, if it's running on localhost on your host machine

```bash
export RPC_URL=http://host.docker.internal:8545
```

3. Run the docker-compose command. This deploys the contract with the timelock owner and writes the env vars to a file
   called `.env.mydemo`

```bash
docker compose run --rm \
  -e RPC_URL \
  -v $(pwd)/.env.mydemo:/app/.env.mydemo \
  deploy-sequencer-contracts \
  deploy --deploy-ops-timelock --deploy-fee --use-timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
```

# Token

If you would like to fork the rpc url, then in one terminal (assuming foundry is installed)

```bash
anvil --fork-url $RPC_URL
```

Your RPC_URL will now be http://localhost:8545

In the terminal where the deployments will occur:

```bash
export RPC_URL=http://localhost:8545
```

## EOA Owner

### Deploying with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
unset ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-esp-token --rpc-url=$RPC_URL
```

## Multisig Owner

### Deploying Token with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-esp-token --rpc-url=$RPC_URL
```

## Timelock Owner

### Note:

The code sets the OpsTimelock as the owner of the FeeContract

### Deploying with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-safe-exit-timelock --deploy-esp-token --use-timelock-owner --rpc-url=$RPC_URL
```

### Deploying Token with Docker compose

1. Ensure the deploy image was built by running

```bash
./scripts/build-docker-images-native --image deploy
```

2. Set the RPC URL env var, example, if it's running on localhost on your host machine

```bash
export RPC_URL=http://host.docker.internal:8545
```

3. Run the docker-compose command. This deploys the contract with the timelock owner and writes the env vars to a file
   called `.env.mydemo`

```bash
docker compose run --rm \
  -e RPC_URL \
  -v $(pwd)/.env.mydemo:/app/.env.mydemo \
  deploy-sequencer-contracts \
  deploy --deploy-safe-exit-timelock --deploy-esp-token --use-timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
```

Example output file (.env.mydemo) contents after a successful run

```text
ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS=0x0c8e79f3534b00d9a3d4a856b665bf4ebc22f2ba
ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS=0xd04ff4a75edd737a73e92b2f2274cb887d96e110
ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xe1aa25618fa0c7a1cfdab5d6b456af611873b629
ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS=0xe1da8919f262ee86f9be05059c9280142cf23f48
```

# Timelock Proposals

These are demonstration commands and should not be used in production environments

## Transfer Ownership

### Executing with Cargo

Let's first deploy the fee contract and its timelock with scheduler/executer addresses that you control

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_PROPOSERS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPORESS_OPS_TIMELOCK_DELAY=0
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --use-timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
```

The deployed contracts will be written to `.env.mydemo`

Now let's schedule the transfer ownership operation

```bash
set -a
source .env.mydemo
set +a
RUST_LOG=info cargo run --bin deploy -- \
--rpc-url=$RPC_URL \
--perform-timelock-operation \
--timelock-operation-type schedule \
--target-contract FeeContract \
--function-signature "transferOwnership(address)" \
--function-values "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720" \
--timelock-operation-salt 0x \
--timelock-operation-delay 0 \
--timelock-operation-value 0
```

Now let's execute the transfer ownership operation

```bash
RUST_LOG=info cargo run --bin deploy -- \
--rpc-url=$RPC_URL \
--perform-timelock-operation \
--timelock-operation-type execute \
--target-contract FeeContract \
--function-signature "transferOwnership(address)" \
--function-values "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720" \
--timelock-operation-salt 0x \
--timelock-operation-delay 0 \
--timelock-operation-value 0
```

### Executing with Docker Compose

1. Set the roles for the timelock as your deployer account for this demo run

```bash
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_PROPOSERS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
```

2. Follow the deployment steps from the [Docker Compose section](#deploying-fee-contract-with-docker-compose) above.
   Completing this step will deploy the timelock and the fee contract.
3. Use the output file to set the env vars based on the deployment addresses from the step above.

```bash
set -a
source .env.mydemo
set +a
```

4. Schedule the timelock operation

```bash
docker compose run --rm \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  deploy-sequencer-contracts \
  deploy --rpc-url=$RPC_URL \
  --perform-timelock-operation \
  --timelock-operation-type schedule \
  --target-contract FeeContract \
  --function-signature "transferOwnership(address)" \
  --function-values "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720" \
  --timelock-operation-salt 0x \
  --timelock-operation-delay 0 \
  --timelock-operation-value 0
```

5. Execute the timelock operation

```bash
docker compose run --rm \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  deploy-sequencer-contracts \
  deploy --rpc-url=$RPC_URL \
  --perform-timelock-operation \
  --timelock-operation-type execute \
  --target-contract FeeContract \
  --function-signature "transferOwnership(address)" \
  --function-values "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720" \
  --timelock-operation-salt 0x \
  --timelock-operation-delay 0 \
  --timelock-operation-value 0
```

6. Confirm that the contract owner is now the new address (assuming you have Foundry installed)

```bash
cast call $ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS "owner()(address)" --rpc-url http://127.0.0.1:8545
```

## Upgrade To And Call

### Execute via Cargo

Let's first deploy the fee contract and its timelock with scheduler/executer addresses that you control

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_PROPOSERS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPORESS_OPS_TIMELOCK_DELAY=0
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --use-timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
```

The deployed contracts will be written to `.env.mydemo`

Now let's schedule the upgrade to and call operation

```bash
set -a
source .env.mydemo
set +a
RUST_LOG=info cargo run --bin deploy -- \
--rpc-url=$RPC_URL \
--perform-timelock-operation \
--timelock-operation-type schedule \
--target-contract FeeContract \
--function-signature "upgradeToAndCall(address,bytes)" \
--function-values $ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
--function-values "0x" \
--timelock-operation-salt 0x \
--timelock-operation-delay 0 \
--timelock-operation-value 0
```

Now let's execute the upgrade to and call operation

```bash
RUST_LOG=info cargo run --bin deploy -- \
--rpc-url=$RPC_URL \
--perform-timelock-operation \
--timelock-operation-type execute \
--target-contract FeeContract \
--function-signature "upgradeToAndCall(address,bytes)" \
--function-values $ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
--function-values 0x \
--timelock-operation-salt 0x \
--timelock-operation-delay 0 \
--timelock-operation-value 0
```

### Execute via Docker compose

1. Set the roles for the timelock as your deployer account for this demo run

```bash
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_PROPOSERS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
```

2. Follow the deployment steps from the [Docker Compose section](#deploying-fee-contract-with-docker-compose) above.
   Completing this step will deploy the timelock and the fee contract.

3. Use the output file to set the env vars based on the deployment addresses from the step above.

```bash
set -a
source .env.mydemo
set +a
```

4. Schedule the upgrade to and call operation

```bash
docker compose run --rm \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
  deploy-sequencer-contracts \
  deploy --rpc-url=$RPC_URL \
  --perform-timelock-operation \
  --timelock-operation-type schedule \
  --target-contract FeeContract \
  --function-signature "upgradeToAndCall(address,bytes)" \
  --function-values $ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
  --function-values "0x" \
  --timelock-operation-salt 0x \
  --timelock-operation-delay 0 \
  --timelock-operation-value 0
```

5. Execute the upgrade to and call operation

```bash
docker compose run --rm \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
  deploy-sequencer-contracts \
  deploy --rpc-url=$RPC_URL \
  --perform-timelock-operation \
  --timelock-operation-type execute \
  --target-contract FeeContract \
  --function-signature "upgradeToAndCall(address,bytes)" \
  --function-values $ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS \
  --function-values "0x" \
  --timelock-operation-salt 0x \
  --timelock-operation-delay 0 \
  --timelock-operation-value 0
```

6. Confirm that the contract was upgraded (assuming you have Foundry installed)

```bash
cast storage $ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc --rpc-url http://127.0.0.1:8545
```

# Troubleshooting

## Errors

`Error: server returned an error response: error code 3: execution reverted, data: "0xe2517d3f000000000000000000000000a0ee7a142d267c1f36714e4a8f75612f20a79720b09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1"`
That is `error AccessControlUnauthorizedAccount(address account, bytes32 neededRole)` and it occurs when you try to
perform an operation on a timelock using an address that doesn't have that operation privilege. Ensure that address has
the right privilege.

`Error: server returned an error response: error code 3: execution reverted: custom error 0x1425ea42, data: "0x1425ea42"`
That is `error FailedInnerCall()` and it occurs when the timelock operation succeeds but the underlying contract call
fails. This can happen when:

- The function parameters are incorrect
- The target contract doesn't have the function you're trying to call
- The function call would revert for business logic reasons
- The contract is not in the expected state for the operation

`Error: server returned an error response: error code 3: execution reverted: custom error 0x5ead8eb5: ...` That is
`error TimelockUnexpectedOperationState(bytes32 operationId, bytes32 expectedStates)` error and it occurs when the
operation has already been sent to the timelock or is in an unexpected state. This can happen when:

- You try to schedule an operation that's already scheduled
- You try to execute an operation that's not in the pending state
- You try to cancel an operation that's already been executed or cancelled
- The operation ID doesn't match the expected state

Check the operation status and ensure you're performing the correct action for the current state of the operation.

## Common Issues

### Environment Variables Not Set

If you get errors about missing environment variables, ensure all required variables are set:

```bash
# Check if variables are set
echo $ESPRESSO_SEQUENCER_ETH_MNEMONIC
echo $RPC_URL
echo $ESPRESSO_OPS_TIMELOCK_ADMIN

# Set them if missing
export ESPRESSO_SEQUENCER_ETH_MNEMONIC="<your-12-word-mnemonic-phrase>"
export RPC_URL="http://host.docker.internal:8545"
```

# Safe Multisig Proposals

## Upgrading ESP Token to V2 (For demo purposes)

### Prerequisites

Before upgrading to ESP Token V2, ensure you have:

- Deployed ESP Token V1
- The token proxy is owned by the appropriate timelock
- set the multisig as a real multisig address or add `--dry-run` to the commands below if not doing a real run.

```bash
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS="<0x...multisig-address>"
```

### Upgrading with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
# If doing a real run then, export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=YOUR_MULTISIG_ADDRESS
RUST_LOG=info cargo run --bin deploy -- \
  --deploy-esp-token \
  --upgrade-esp-token-v2 \
  --rpc-url=$RPC_URL \
  --use-multisig
  # to simulate, add --dry-run
```

### Upgrading with Docker Compose

```bash
# If doing a real run then, export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=YOUR_MULTISIG_ADDRESS
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -v $(pwd)/.env.mydemo:/app/.env.mydemo \
  deploy-sequencer-contracts \
  deploy --deploy-esp-token --upgrade-esp-token-v2 --rpc-url=$RPC_URL --use-multisig
  # to simulate, add --dry-run
```

You should see the output which says something like:
`EspTokenProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe=YOUR_MULTISIG_ADDRESS`

### Verifying the Upgrade

If the transaction was signed and executed on chain, you can use the following command to check the implementation
address and version number.

```bash
# Check the implementation address
cast storage $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc --rpc-url http://127.0.0.1:8545

# Check V2 specific functions (if available)
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "version()(string)" --rpc-url http://127.0.0.1:8545
```

## Upgrade Verification Checklist

After each upgrade, verify:

1. **Implementation Address**: Check that the proxy points to the new implementation
2. **Functionality**: Test V2-specific functions
3. **Ownership**: Verify ownership hasn't changed unexpectedly
4. **State**: Ensure contract state is preserved correctly

## Transfer ownership from Multisig to Timelock

This section describes how to transfer ownership of contracts from a multisig wallet to a timelock contract using the
deploy binary. This is useful for implementing governance controls where contract upgrades and administrative functions
require timelock approval.

### Prerequisites

- The target contract must be deployed and owned by a multisig wallet
- The timelock contract must be deployed and accessible
- You must have access to the multisig wallet (either through private keys or multisig signing)

### Usage

**Using Cargo:**

```bash
# Set environment variables
set +a
source .env
set -a

# Re-specify the most important vars if needed
export ESPRESSO_SEQUENCER_ETH_MNEMONIC="<your-12-word-mnemonic-phrase>"
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS="<0x...multisig-address>"
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS="0x..."      # timelock contract address, select the correct timelock for the contract
export RPC_URL=""

# Run the ownership transfer proposal
cargo run --bin deploy -- \
    --propose-transfer-ownership-to-timelock \
    --target-contract FeeContract \
    --timelock-address $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
    --fee-contract-proxy $ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS \
    --multisig-address $ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
    --rpc-url $RPC_URL \
```

**Using Docker Compose:**

```bash
# Set environment variables
set +a
source .env
set -a

# Re-specify the most important vars if needed
export ESPRESSO_SEQUENCER_ETH_MNEMONIC="<your-12-word-mnemonic-phrase>"
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS="<0x...multisig-address>"
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS="0x..."      # timelock contract address
export RPC_URL=""

# Run the ownership transfer proposal
docker-compose run --rm deploy-sequencer-contracts \
    --propose-transfer-ownership-to-timelock \
    --target-contract lightclient \
    --timelock-address $ESPRESSO_SEQUENCER_TIMELOCK_ADDRESS \
    --light-client-proxy $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS \
    --multisig-address $ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
    --rpc-url $RPC_URL
```

### Supported Contract Types

The `--target-contract` parameter supports the following values:

- `lightclient` or `lightclientproxy` - for LightClient contracts
- `feecontract` or `feecontractproxy` - for FeeContract contracts
- `esptoken` or `esptokenproxy` - for ESP token contracts
- `staketable` or `staketableproxy` - for StakeTable contracts

### Process Flow

1. **Proposal Creation**: The deployer creates a multisig proposal to transfer ownership from the multisig wallet to the
   timelock contract
2. **Multisig Approval**: The multisig wallet owners must approve the proposal (this may require multiple signatures
   depending on the multisig configuration)
3. **Execution**: Once approved, the proposal can be executed to complete the ownership transfer
4. **Verification**: Verify that the timelock contract is now the owner of the target contract

### Verification

After the ownership transfer is completed, verify the transfer on-chain:

```bash

# Verify the timelock address is loaded correctly
echo "Timelock Address: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"

# Verify the timelock is now the owner of the target contract (after it's been signed and executed)
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL
```

### Troubleshooting

**Common Issues:**

1. **Insufficient Multisig Signatures**: Ensure all required multisig owners have signed the proposal
2. **Invalid Contract Address**: Verify the target contract and timelock addresses are correct
3. **Permission Denied**: Ensure the deployer account has permission to create proposals on the multisig
4. **Network Issues**: Check RPC connectivity and gas settings

**Debug Commands:**

```bash
# Check current owner of the target contract
cast call $TARGET_CONTRACT_ADDRESS "owner()(address)" --rpc-url $RPC_URL

# Check multisig proposal status
cast call $MULTISIG_ADDRESS "getTransactionHash(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,uint256)(bytes32)" \
    $TARGET_CONTRACT_ADDRESS 0 "transferOwnership(address)" 0 0 0 $TIMELOCK_ADDRESS $TIMELOCK_ADDRESS 0 \
    --rpc-url $RPC_URL
```

### RPC Connection Issues

If you can't connect to the RPC endpoint:

- Ensure your L1 node is running
- Check the RPC URL is correct for Docker (use `host.docker.internal` instead of `localhost`)
- Verify the port is accessible from the container

### Contract Not Found

If the deployer can't find deployed contracts:

- Check that the `.env.mydemo` file exists and contains the expected addresses
- Verify the addresses in the file are correct
- Ensure you're using the right network/RPC URL

# POS Deployment

- [Ethereum Mainnet](#ethereum-mainnet)
- [Arbitrum Mainnet](#arbitrum-mainnet)
- [Ethereum Sepolia](#ethereum-sepolia)
- [Arbitrum Sepolia](#arbitrum-sepolia)

**Prerequisites:**

- Docker and Docker Compose installed
- Foundry installed (for verification)
- Access to Ethereum and Arbitrum RPC endpoints
- Foundation Multisig and EspressoSys Multisig addresses
- Espresso Devs addresses for proposer roles

## Ethereum Mainnet

### Step 1: Deploy `SafeExitTimelock`

`Deploy SafeExitTimelock, set Foundation Multisig as the admin, Espresso Devs as proposers and the Foundation Multisig as the executor.`

#### Step 1: Deploy SafeExitTimelock

```bash
export RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.mainnet.safeexittimelock
touch $OUTPUT_FILE
# Set timelock configuration
export ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN=0xFOUNDATION_MULTISIG_ADDRESS
export ESPRESSO_DEVS_ADDRESS_1=0xADDRESS_1
export ESPRESSO_DEVS_ADDRESS_2=0xADDRESS_2
export ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS=0xFOUNDATION_MULTISIG_ADDRESS
export ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY=1209600 #choose a time in seconds that represents a safe exit delay
```

**Note**: If using only one proposer, you can just export the `ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS` address. However,
if you want to set multiple addresses then you can specify each address in the docker command by adding
`--safe-exit-timelock-proposers=$ADDRESS` for each new proposer address.

4. Run the docker-compose command to deploy the SafeExitTimelock

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN \
  -e ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS \
  -e ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  deploy-sequencer-contracts \
  deploy --deploy-safe-exit-timelock --rpc-url=$RPC_URL \
  --safe-exit-timelock-proposers=$ESPRESSO_DEVS_ADDRESS_1 \
  --safe-exit-timelock-proposers=$ESPRESSO_DEVS_ADDRESS_2 \
  --out $OUTPUT_FILE
```

5. Verify the deployment by checking the output file

```bash
cat $OUTPUT_FILE
```

Example output file ($OUTPUT_FILE) contents after a successful SafeExitTimelock deployment:

```text
ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS=0x1234567890123456789012345678901234567890
```

6. Verify the timelock configuration on-chain (assuming you have Foundry installed)

```bash
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "Timelock Address: $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS"
echo "Admin Address: $ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN"

# Check the admin role (DEFAULT_ADMIN_ROLE = 0x00...)
cast call $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0x0000000000000000000000000000000000000000000000000000000000000000 \
  $ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN --rpc-url $RPC_URL

# Check the proposer roles (check each address individually)
cast call $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1 \
  $ESPRESSO_DEVS_ADDRESS_1 --rpc-url $RPC_URL

cast call $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1 \
  $ESPRESSO_DEVS_ADDRESS_2 --rpc-url $RPC_URL

# Check the executor role
cast call $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63 \
  $ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS --rpc-url $RPC_URL

# Check the timelock delay
cast call $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS "getMinDelay()(uint256)" --rpc-url $RPC_URL
```

#### Step 2: Deploy OpsTimelock

```bash
export OUTPUT_FILE=.env.eth.mainnet.opstimelock
touch $OUTPUT_FILE

# Set timelock configuration
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xFOUNDATION_MULTISIG_ADDRESS
export ESPRESSO_DEVS_ADDRESS_1=0xADDRESS_1
export ESPRESSO_DEVS_ADDRESS_2=0xADDRESS_2
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xFOUNDATION_MULTISIG_ADDRESS
export ESPRESSO_OPS_TIMELOCK_DELAY=86400 #choose a time in seconds that represents an operations delay
```

**Note**: If using only one proposer, you can just export the `ESPRESSO_OPS_TIMELOCK_PROPOSERS` address. However, if you
want to set multiple addresses then you can specify each address in the docker command by adding
`--ops-timelock-proposers=$ADDRESS` for each new proposer address.

4. Run the docker-compose command to deploy the OpsTimelock

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_OPS_TIMELOCK_ADMIN \
  -e ESPRESSO_OPS_TIMELOCK_EXECUTORS \
  -e ESPRESSO_OPS_TIMELOCK_DELAY \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  deploy-sequencer-contracts \
  deploy --deploy-ops-timelock --rpc-url=$RPC_URL \
  --ops-timelock-proposers=$ESPRESSO_DEVS_ADDRESS_1 \
  --ops-timelock-proposers=$ESPRESSO_DEVS_ADDRESS_2 \
  --out $OUTPUT_FILE
```

5. Verify the deployment by checking the output file

```bash
cat $OUTPUT_FILE
```

Example output file ($OUTPUT_FILE) contents after a successful OpsTimelock deployment:

```text
ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0x1234567890123456789012345678901234567890
```

6. Verify the timelock configuration on-chain (assuming you have Foundry installed)

```bash
# First, source the output file to load the deployed contract address
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "Timelock Address: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
echo "Admin Address: $ESPRESSO_OPS_TIMELOCK_ADMIN"

# Check the admin role (DEFAULT_ADMIN_ROLE = 0x00...)
cast call $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0x0000000000000000000000000000000000000000000000000000000000000000 \
  $ESPRESSO_OPS_TIMELOCK_ADMIN --rpc-url $RPC_URL

# Check the proposer roles (check each address individually)
cast call $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1 \
  $ESPRESSO_DEVS_ADDRESS_1 --rpc-url $RPC_URL

cast call $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1 \
  $ESPRESSO_DEVS_ADDRESS_2 --rpc-url $RPC_URL

# Check the executor role
cast call $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS "hasRole(bytes32,address)(bool)" \
  0xd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63 \
  $ESPRESSO_OPS_TIMELOCK_EXECUTORS --rpc-url $RPC_URL

# Check the timelock delay
cast call $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS "getMinDelay()(uint256)" --rpc-url $RPC_URL
```

#### Step 3: Upgrade LightClientV2

```bash
export OUTPUT_FILE=.env.eth.mainnet.lightclientv2
touch $OUTPUT_FILE
```

3. Set the environment variables for the LightClientV2 upgrade configuration

```bash
# If doing a real run, set the multisig address
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=YOUR_MULTISIG_ADDRESS

# Set the blocks per epoch and epoch start block
# These can be fetched from the sequencer URL or set manually
export ESPRESSO_SEQUENCER_BLOCKS_PER_EPOCH= #example: 1000
export ESPRESSO_SEQUENCER_EPOCH_START_BLOCK= #example: 1000000

# Set the sequencer URL to fetch config (optional, will use env vars if not set)
export ESPRESSO_SEQUENCER_URL=
```

4. Run the docker-compose command to upgrade to LightClientV2

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
  -e ESPRESSO_SEQUENCER_BLOCKS_PER_EPOCH \
  -e ESPRESSO_SEQUENCER_EPOCH_START_BLOCK \
  -e ESPRESSO_SEQUENCER_URL \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --upgrade-light-client-v2 --rpc-url=$RPC_URL --use-multisig --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The upgrade process will:

- Deploy LightClientV2 implementation
- Create a multisig proposal to upgrade the proxy
- Initialize the V2 contract with the provided epoch configuration
- The timelock address is owned by a multisig currently so we have to send through a multisig proposal to handle the
  upgrade

5. Verify the upgrade proposal was created

You should see output similar to:
`LightClientProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe=YOUR_MULTISIG_ADDRESS`

6. After the multisig signs and executes the proposal, verify the upgrade on-chain (assuming you have Foundry installed)

```bash
# First, source the output file to load the deployed contract address
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "LightClient Proxy Address: $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS"

# Check the implementation address (should point to LightClientV2)
cast storage $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc --rpc-url $RPC_URL

# Check V2 specific functions
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "getVersion()(uint8,uint8,uint8)" --rpc-url $RPC_URL

# Check the epoch configuration
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "blocksPerEpoch()(uint64)" --rpc-url $RPC_URL
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "epochStartBlock()(uint64)" --rpc-url $RPC_URL

# Check if the contract was properly initialized (should return true for V2)
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "isInitialized()(bool)" --rpc-url $RPC_URL
```

### Step 4: Multisig Proposal to change admin of LightClientProxy from EspressoSys multisig to OpsTimelock

#### Creating Multisig Proposal to Transfer LightClientProxy Admin with Docker Compose

**Prerequisites:**

- LightClientProxy must be deployed and owned by the EspressoSys multisig
- OpsTimelock must be deployed (from Step 2)
- The signers on the EspressoSys multisig must be available for signing the proposal and then executing the proposal

1. Ensure you're on the main branch or the release tag branch
2. Set the RPC URL env var for Ethereum mainnet and set the OUTPUT FILE env var to an appropriate location

```bash
export OUTPUT_FILE=.env.eth.mainnet.lightclient.admin.transfer
touch $OUTPUT_FILE
```

3. Set the environment variables for the admin transfer configuration

```bash
# Set the EspressoSys multisig address (current admin)
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=0xESPRESSOSYS_MULTISIG_ADDRESS

# Set the OpsTimelock address (new admin)
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xOPS_TIMELOCK_ADDRESS

# Set the LightClientProxy address
export ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS=0xLIGHT_CLIENT_PROXY_ADDRESS
```

4. Run the docker-compose command to create the multisig proposal for admin transfer

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --propose-transfer-ownership-to-timelock \
  --target-contract lightclient \
  --timelock-address $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  --rpc-url=$RPC_URL \
  --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The admin transfer process will:

- Create a multisig proposal to call `transferOwnership(address)` on the LightClientProxy
- Set the OpsTimelock as the new admin/owner of the LightClientProxy
- Maintain the proxy's functionality while changing administrative control

5. Verify the admin transfer proposal was created

You should see output similar to:
`LightClientProxy ownership transfer proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe=0xESPRESSOSYS_MULTISIG_ADDRESS`

6. After the signer threshold signs the proposal and one executes the proposal, verify the admin transfer on-chain
   (assuming you have Foundry installed)

```bash
# First, source the output file to load the deployed contract addresses
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "OpsTimelock Address: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"

# Verify the OpsTimelock is now the owner
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS
```

#### Step 5: Deploy EspToken with SafeExitTimelock as Admin

```bash
export OUTPUT_FILE=.env.eth.mainnet.esptoken
touch $OUTPUT_FILE
```

3. Set the environment variables for the EspToken deployment configuration

```bash
# Set the SafeExitTimelock address (will be the admin of EspTokenProxy)
export ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS=0xSAFE_EXIT_TIMELOCK_ADDRESS

# Set the initial token supply (optional, will use default if not set)
export ESP_TOKEN_INITIAL_SUPPLY=3590000000000000000000000000 # 3.59 billion tokens with 18 decimals

# Set the token name and symbol (optional, will use defaults if not set)
export ESP_TOKEN_NAME="Espresso Token"
export ESP_TOKEN_SYMBOL="ESP"
export ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS=0xRecipientAddress
```

4. Run the docker-compose command to deploy EspTokenProxy with SafeExitTimelock as admin

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS \
  -e ESP_TOKEN_INITIAL_SUPPLY \
  -e ESP_TOKEN_NAME \
  -e ESP_TOKEN_SYMBOL \
  -e ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --deploy-esp-token --use-timelock-owner --rpc-url=$RPC_URL --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The EspToken deployment process will:

- Deploy the EspTokenV1 implementation contract
- Deploy the EspToken proxy contract with SafeExitTimelock as the initial owner
- Initialize the token with the provided configuration (name, symbol, initial supply)
- Set up the proxy to point to the implementation contract

5. Verify the deployment by checking the output file

```bash
cat $OUTPUT_FILE
```

Example output file ($OUTPUT_FILE) contents after a successful EspToken deployment:

```text
ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS=0x1234567890123456789012345678901234567890
ESPRESSO_SEQUENCER_ESP_TOKEN_ADDRESS=0x0987654321098765432109876543210987654321
ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS=0x5555555555555555555555555555555555555555
```

6. Verify the deployment and ownership were successful

You should see output similar to: `EspTokenProxy deployed successfully` `Ownership set to SafeExitTimelock`

7. After the deployment is completed, verify the EspToken deployment on-chain (assuming you have Foundry installed)

```bash
# First, source the output file to load the deployed contract addresses
source $OUTPUT_FILE

# Check the owner/admin of the EspToken proxy (should be SafeExitTimelock)
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL

# Verify the SafeExitTimelock is the owner
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS

# Check the implementation address (should point to EspTokenV1)
cast storage $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc --rpc-url $RPC_URL

# Check the token name and symbol
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "name()(string)" --rpc-url $RPC_URL
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "symbol()(string)" --rpc-url $RPC_URL

# Check the total supply
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "totalSupply()(uint256)" --rpc-url $RPC_URL

# Check the decimals
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "decimals()(uint8)" --rpc-url $RPC_URL
```

### Step 6: Deploy StakeTableProxy & immediately Upgrade to StakeTableV2, setting the EspressoSys Multisig as the pauser

#### Deploying StakeTableProxy and Upgrading to StakeTableV2 using Docker Compose

**Prerequisites:**

- EspressoSys Multisig must be available for signing proposals
- The deploying account must have permission to create proposals in the EspressoSys multisig

1. Ensure you're on the main branch or the release tag branch
2. Set the RPC URL env var for Ethereum mainnet and set the OUTPUT FILE env var to an appropriate location

```bash
export OUTPUT_FILE=.env.eth.mainnet.staketable
touch $OUTPUT_FILE
```

3. Set the environment variables for the StakeTable deployment and upgrade configuration

```bash
# Set the EspressoSys multisig address (will be the pauser of StakeTableV2)
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=0xESPRESSOSYS_MULTISIG_ADDRESS

# Set the pauser address for StakeTableV2 (same as EspressoSys multisig)
export ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS=0xESPRESSOSYS_MULTISIG_ADDRESS

# Set the EspToken address (required for StakeTableV2)
export ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS=0xESP_TOKEN_PROXY_ADDRESS

# Set the LightClient proxy address (required for StakeTable initialization)
export ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS=0xLIGHT_CLIENT_PROXY_ADDRESS

# Set the OpsTimelock address (will be the admin of StakeTableProxy)
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xOPS_TIMELOCK_ADDRESS

# Set the SafeExitTimelock address (will be the admin of StakeTableProxy)
export ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS=0xSAFE_EXIT_TIMELOCK_ADDRESS
```

4. Run the docker-compose command to deploy StakeTableProxy and upgrade to StakeTableV2

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS \
  -e ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --deploy-stake-table --upgrade-stake-table-v2 --use-timelock-owner --rpc-url=$RPC_URL --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The StakeTable deployment and upgrade process will:

- Deploy the StakeTableV1 implementation contract
- Deploy the StakeTable proxy contract with deployer as initial owner
- Deploy the StakeTableV2 implementation contract
- Upgrade the proxy to StakeTableV2 and set EspressoSys multisig as pauser
- Transfer ownership to OpsTimelock (as specified by --use-timelock-owner)
- Initialize StakeTableV2 with the EspToken address

5. Verify the deployment by checking the output file

```bash
cat $OUTPUT_FILE
```

6. Verify the deployment and upgrade were successful

You should see output similar to: `StakeTable successfully upgraded to 0x...` `Transferring ownership to OpsTimelock`

7. After the deployment and upgrade are completed, verify the StakeTable deployment and upgrade on-chain (assuming you
   have Foundry installed)

```bash
# First, source the output file to load the deployed contract addresses
source $OUTPUT_FILE

# Check the owner/admin of the StakeTable proxy (should be OpsTimelock)
cast call $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL

# Verify the OpsTimelock is the owner
cast call $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS

# Check the implementation address (should point to StakeTableV2)
cast storage $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc --rpc-url $RPC_URL

# Export the PAUSER_ROLE constant
export PAUSER_CONSTANT=$(cast call $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS "PAUSER_ROLE()(bytes32)" --rpc-url $RPC_URL)

# Check the pauser role (should be EspressoSys multisig)
cast call $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS "hasRole(bytes32,address)(bool)" \
  $PAUSER_CONSTANT \
  $ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS --rpc-url $RPC_URL

# Verify the EspToken address matches
cast call $ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS "token()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
```

### Final Verification Checklist

After completing all steps, verify:

1. **Timelocks**: Both timelocks have correct roles and delays
2. **LightClient**: Upgraded to V2, owned by OpsTimelock, properly initialized
3. **EspToken**: Deployed with SafeExitTimelock as owner, correct supply, correct initial recipient
4. **StakeTable**: Upgraded to V2, owned by OpsTimelock, EspressoSys multisig has pauser role
5. **All proxies**: Point to correct implementation addresses

## Arbitrum Mainnet

### Step 1: Deploy the `OpsTimelock`

Follow the same steps as in [Step 2: Deploy OpsTimelock](#step-2-deploy-opstimelock) from the Ethereum Mainnet section
above, but use Arbitrum mainnet RPC URL:

```bash
export RPC_URL=https://arb-mainnet.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.mainnet.opstimelock
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 2: Upgrade LightClientV2

Follow the same steps as in [Step 3: Upgrade LightClientV2](#step-3-upgrade-lightclientv2) from the Ethereum Mainnet
section above, but use Arbitrum mainnet RPC URL:

```bash
export RPC_URL=https://arb-mainnet.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.mainnet.lightclientv2
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 3: Multisig Proposal to change admin of LightClientProxy from EspressoSys multisig to OpsTimelock

Follow the same steps as in
[Step 4: Multisig Proposal to change admin of LightClientProxy from EspressoSys multisig to OpsTimelock](#step-4-multisig-proposal-to-change-admin-of-lightclientproxy-from-espressosys-multisig-to-opstimelock)
from the Ethereum Mainnet section above, but use Arbitrum mainnet RPC URL:

```bash
export RPC_URL=https://arb-mainnet.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.mainnet.lightclient.admin.transfer
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

## Ethereum Sepolia

### Step 1: Deploy `SafeExitTimelock`, set `Foundation Multisig` as the admin, Espresso Devs as proposers and the `Foundation Multisig` as the executor.

Follow the same steps as in [Step 1: Deploy SafeExitTimelock](#step-1-deploy-safeexittimelock) from the Ethereum Mainnet
section above, but use Ethereum Sepolia RPC URL:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.safeexittimelock
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 2: Deploy `OpsTimelock`, set `Foundation Multisig` as the admin, Espresso Devs as proposers and the `Foundation Multisig` as the executor.

Follow the same steps as in [Step 2: Deploy OpsTimelock](#step-2-deploy-opstimelock) from the Ethereum Mainnet section
above, but use Ethereum Sepolia RPC URL:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.opstimelock
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 3: Upgrade `LightClientV2`

Follow the same steps as in [Step 3: Upgrade LightClientV2](#step-3-upgrade-lightclientv2) from the Ethereum Mainnet
section above, but use Ethereum Sepolia RPC URL:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.lightclientv2
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 4: Multisig Proposal to change admin of `LightClientProxy` from `EspressoSys multisig` to `OpsTimelock`

Follow the same steps as in
[Step 4: Multisig Proposal to change admin of LightClientProxy from EspressoSys multisig to OpsTimelock](#step-4-multisig-proposal-to-change-admin-of-lightclientproxy-from-espressosys-multisig-to-opstimelock)
from the Ethereum Mainnet section above, but use Ethereum Sepolia RPC URL:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.lightclient.admin.transfer
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 5: Create a multisig proposal to transfer the owner of the `EspToken`, set `SafeExitTimelock` as the admin

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.esptoken.admin.transfer
touch $OUTPUT_FILE
```

**Prerequisites:**

- EspToken proxy must be deployed and owned by a multisig
- SafeExitTimelock must be deployed (from Step 1)
- The signers on the multisig must be available for signing the proposal

1. **Set the environment variables for the admin transfer configuration:**

```bash
# Set the multisig address (current admin of EspToken proxy)
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=0xMULTISIG_ADDRESS

# Set the SafeExitTimelock address (new admin)
export ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS=0xSAFE_EXIT_TIMELOCK_ADDRESS

# Set the EspToken proxy address
export ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS=0xESP_TOKEN_PROXY_ADDRESS
```

2. **Run the docker-compose command to create the multisig proposal for admin transfer:**

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
  -e ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --propose-transfer-ownership-to-timelock \
  --target-contract esptoken \
  --timelock-address $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS \
  --rpc-url=$RPC_URL \
  --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The admin transfer process will:

- Create a multisig proposal to call `transferOwnership(address)` on the EspToken proxy
- Set the SafeExitTimelock as the new admin/owner of the EspToken proxy
- Maintain the proxy's functionality while changing administrative control

3. **Verify the admin transfer proposal was created:**

You should see output similar to:
`EspTokenProxy ownership transfer proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe=0xMULTISIG_ADDRESS`

4. **After the signer threshold signs the proposal and one executes the proposal, verify the admin transfer on-chain:**

```bash
# First, source the output file to load the deployed contract addresses
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "SafeExitTimelock Address: $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS"

# Verify the SafeExitTimelock is now the owner
cast call $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS
```

### Step 6: Upgrade to `StakeTableV2`. (Sets the `EspressoSys Multisig` as the pauser)

Follow the same steps as in
[Step 6: Deploy StakeTableProxy & immediately Upgrade to StakeTableV2, setting the EspressoSys Multisig as the pauser](#step-6-deploy-staketableproxy--immediately-upgrade-to-staketablev2-setting-the-espressosys-multisig-as-the-pauser)
from the Ethereum Mainnet section above, but use Ethereum Sepolia RPC URL:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.staketable
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 7: Multisig Proposal to change admin of `StakeTableProxy` from multisig to `OpsTimelock`

**Note**: This step is not explicitly covered in the Ethereum Mainnet section above, but follows the same pattern as the
LightClient admin transfer. You would need to create a multisig proposal to call `transferOwnership(address)` on the
StakeTableProxy, setting the OpsTimelock as the new admin/owner.

Use the same approach as in
[Step 4: Multisig Proposal to change admin of LightClientProxy from EspressoSys multisig to OpsTimelock](#step-4-multisig-proposal-to-change-admin-of-lightclientproxy-from-espressosys-multisig-to-opstimelock)
but target the StakeTableProxy instead:

```bash
export RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.eth.sepolia.staketable.admin.transfer
touch $OUTPUT_FILE
```

**Run the docker-compose command to create the multisig proposal for admin transfer:**

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --propose-transfer-ownership-to-timelock \
  --target-contract staketable \
  --timelock-address $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  --rpc-url=$RPC_URL \
  --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

Then create a multisig proposal to transfer ownership of the StakeTableProxy to the OpsTimelock.

## Arbitrum Sepolia

### Step 1: Deploy `OpsTimelock`, set `Espresso admin EOA` as the admin, Espresso Devs as proposers and the `Espresso admin EOA` as the executor.

Follow the same steps as in [Step 2: Deploy OpsTimelock](#step-2-deploy-opstimelock) from the Ethereum Mainnet section
above, but use Arbitrum Sepolia RPC URL:

```bash
export RPC_URL=https://arb-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.sepolia.opstimelock
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 2: Upgrade `LightClientV2`

Follow the same steps as in [Step 3: Upgrade LightClientV2](#step-3-upgrade-lightclientv2) from the Ethereum Mainnet
section above, but use Arbitrum Sepolia RPC URL:

```bash
export RPC_URL=https://arb-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.sepolia.lightclientv2
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 3: Change admin of `LightClientProxy` from `EspressoSys admin EOA` to `OpsTimelock`

**Prerequisites:**

- LightClientProxy must be deployed and owned by the EspressoSys admin EOA
- OpsTimelock must be deployed (from Step 1)
- You must have access to the EspressoSys admin EOA private key/mnemonic

1. Set the RPC URL env var for Arbitrum Sepolia and set the OUTPUT FILE env var to an appropriate location

```bash
export RPC_URL=https://arb-sepolia.g.alchemy.com/v2/YOUR_API_KEY
export OUTPUT_FILE=.env.arb.sepolia.lightclient.admin.transfer
touch $OUTPUT_FILE
```

2. Set the environment variables for the admin transfer configuration

```bash
# Set the OpsTimelock address (new admin)
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xOPS_TIMELOCK_ADDRESS

# Set the LightClientProxy address
export ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS=0xLIGHT_CLIENT_PROXY_ADDRESS

# Set the new owner (OpsTimelock)
export ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER=$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS
```

3. Run the docker-compose command to transfer ownership from EOA to timelock

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS \
  -e ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --transfer-ownership-from-eoa \
  --target-contract lightclient \
  --transfer-ownership-new-owner $ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER \
  --rpc-url=$RPC_URL \
  --out $OUTPUT_FILE
  # to simulate, add --dry-run
```

**Note**: The admin transfer process will:

- Directly call `transferOwnership(address)` on the LightClientProxy using the EOA's private key
- Set the OpsTimelock as the new admin/owner of the LightClientProxy
- Maintain the proxy's functionality while changing administrative control

4. Verify the admin transfer was completed on-chain (assuming you have Foundry installed)

```bash
# First, source the output file to load the deployed contract addresses
source $OUTPUT_FILE

# Verify the variables are loaded correctly
echo "OpsTimelock Address: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"

# Verify the OpsTimelock is now the owner
cast call $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS "owner()(address)" --rpc-url $RPC_URL | grep -i $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS
```

## Water (Ethereum Devnet)

### Step 1: Deploy `SafeExitTimelock`, set `Espresso EOA admin` as the admin, Espresso Devs as proposers and the `Espresso EOA admin` as the executor.

Follow the same steps as in [Step 1: Deploy SafeExitTimelock](#step-1-deploy-safeexittimelock) from the Ethereum Mainnet
section above, but use Water devnet RPC URL and set Espresso EOA admin instead of Foundation Multisig:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.safeexittimelock
touch $OUTPUT_FILE

# Set timelock configuration (use Espresso EOA admin instead of Foundation Multisig)
export ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN=0xESPRESSO_EOA_ADMIN_ADDRESS
export ESPRESSO_DEVS_ADDRESS_1=0xADDRESS_1
export ESPRESSO_DEVS_ADDRESS_2=0xADDRESS_2
export ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS=0xESPRESSO_EOA_ADMIN_ADDRESS
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 2: Deploy `OpsTimelock`, set `Espresso EOA admin` as the admin, Espresso Devs as proposers and the `Espresso EOA admin` as the executor.

Follow the same steps as in [Step 2: Deploy OpsTimelock](#step-2-deploy-opstimelock) from the Ethereum Mainnet section
above, but use Water devnet RPC URL and set Espresso EOA admin instead of Foundation Multisig:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.opstimelock
touch $OUTPUT_FILE

# Set timelock configuration (use Espresso EOA admin instead of Foundation Multisig)
export ESPRESSO_OPS_TIMELOCK_ADMIN=0xESPRESSO_EOA_ADMIN_ADDRESS
export ESPRESSO_DEVS_ADDRESS_1=0xADDRESS_1
export ESPRESSO_DEVS_ADDRESS_2=0xADDRESS_2
export ESPRESSO_OPS_TIMELOCK_EXECUTORS=0xESPRESSO_EOA_ADMIN_ADDRESS
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 3: Upgrade `LightClientV2`

Follow the same steps as in [Step 3: Upgrade LightClientV2](#step-3-upgrade-lightclientv2) from the Ethereum Mainnet
section above, but use Water devnet RPC URL:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.lightclientv2
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 4: Change owner of `LightClientProxy` from `Espresso EOA admin` to `OpsTimelock`

Follow the same steps as in
[Step 3: Change admin of LightClientProxy from EspressoSys admin EOA to OpsTimelock](#step-3-change-admin-of-lightclientproxy-from-espressosys-admin-eoa-to-opstimelock)
from the Arbitrum Sepolia section above, but use Water devnet RPC URL and the Espresso EOA admin instead of EspressoSys
admin EOA:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.lightclient.admin.transfer
touch $OUTPUT_FILE

# Set the OpsTimelock address (new admin)
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xOPS_TIMELOCK_ADDRESS

# Set the LightClientProxy address
export ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS=0xLIGHT_CLIENT_PROXY_ADDRESS

# Set the new owner (OpsTimelock)
export ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER=$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS
```

Then proceed with the same docker-compose command as outlined in the Arbitrum Sepolia section above.

### Step 5: Deploy `EspToken`, set `SafeExitTimelock` as the admin

Follow the same steps as in
[Step 5: Deploy EspToken, set SafeExitTimelock as the admin](#step-5-deploy-esptoken-set-safeexittimelock-as-the-admin)
from the Ethereum Mainnet section above, but use Water devnet RPC URL:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.esptoken
touch $OUTPUT_FILE
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 6: Upgrade to `StakeTableV2`, setting the `Espresso EOA admin` as the pauser

Follow the same steps as in
[Step 6: Deploy StakeTableProxy & immediately Upgrade to StakeTableV2, setting the EspressoSys Multisig as the pauser](#step-6-deploy-staketableproxy--immediately-upgrade-to-staketablev2-setting-the-espressosys-multisig-as-the-pauser)
from the Ethereum Mainnet section above, but use Water devnet RPC URL and set Espresso EOA admin instead of EspressoSys
Multisig:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.staketable
touch $OUTPUT_FILE

# Set StakeTable configuration (use Espresso EOA admin instead of EspressoSys Multisig)
export ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS=0xESPRESSO_EOA_ADMIN_ADDRESS
```

Then proceed with the same deployment steps as outlined in the Ethereum Mainnet section.

### Step 7: Change owner of `StakeTableProxy` to `OpsTimelock`

Follow the same steps as in
[Step 3: Change admin of LightClientProxy from EspressoSys admin EOA to OpsTimelock](#step-3-change-admin-of-lightclientproxy-from-espressosys-admin-eoa-to-opstimelock)
from the Arbitrum Sepolia section above, but use Water devnet RPC URL and target the StakeTableProxy instead:

```bash
export RPC_URL=https://water-devnet.example.com
export OUTPUT_FILE=.env.water.staketable.admin.transfer
touch $OUTPUT_FILE

# Set the OpsTimelock address (new admin)
export ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0xOPS_TIMELOCK_ADDRESS

# Set the StakeTableProxy address
export ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS=0xSTAKE_TABLE_PROXY_ADDRESS

# Set the new owner (OpsTimelock)
export ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER=$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS
```

Then use the same docker-compose command as in Step 4 above, but change the target contract to `staketable`:

```bash
docker compose run --rm \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS \
  -e ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS \
  -e ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER \
  -v $(pwd)/$OUTPUT_FILE:/app/$OUTPUT_FILE \
  \
  deploy-sequencer-contracts \
  deploy --transfer-ownership-from-eoa \
  --target-contract staketable \
  --transfer-ownership-new-owner $ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER \
  --rpc-url=$RPC_URL \
  --out $OUTPUT_FILE
  # to simulate, add --dry-run
```
