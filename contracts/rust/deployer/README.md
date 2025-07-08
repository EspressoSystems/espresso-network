# Contract Deployer

## Table of Contents

- [Prerequisites](#prerequisites)
- [Assumptions](#assumptions)
- [Fee Contract](#fee-contract)
- [Token](#token)
- [Timelock Proposals](#timelock-proposals)
- [Safe Multisig Proposals](#safe-multisig-proposals)
- [Troubleshooting](#troubleshooting)

## Prerequisites

- Rust and Cargo installed
- Docker and Docker Compose installed
- Foundry (for verification commands)
- Access to an Ethereum RPC endpoint

## Assumptions

- the config in .env file is valid, if not, change it
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
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
unset ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-fee --rpc-url=$RPC_URL
```

## Multisig Owner

### Deploying with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS
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
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --timelock-owner --rpc-url=$RPC_URL
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
  deploy --deploy-ops-timelock --deploy-fee --timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
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
RUST_LOG=info cargo run --bin deploy -- --deploy-safe-exit-timelock --deploy-esp-token --timelock-owner --rpc-url=$RPC_URL
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
  deploy --deploy-safe-exit-timelock --deploy-esp-token --timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
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
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
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
--timelock-target-contract FeeContract \
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
--timelock-target-contract FeeContract \
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
  --timelock-target-contract FeeContract \
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
  --timelock-target-contract FeeContract \
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
RUST_LOG=info cargo run --bin deploy -- --deploy-ops-timelock --deploy-fee --timelock-owner --rpc-url=$RPC_URL --out .env.mydemo
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
--timelock-target-contract FeeContract \
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
--timelock-target-contract FeeContract \
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
  --timelock-target-contract FeeContract \
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
  --timelock-target-contract FeeContract \
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
export ESPRESSO_SEQUENCER_ETH_MNEMONIC="your mnemonic here"
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
export ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS=YOUR_MULTISIG_ADDRESS
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
  # if doing a real run then add --dry-run
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
  # if doing a real run then add --dry-run
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
