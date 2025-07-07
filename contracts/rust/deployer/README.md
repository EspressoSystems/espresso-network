# Deploying Contracts

## Assumptions

- the config in .env file is valid, if not, change it
- if using multisigs, the eth network is supported by Sepolia

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

### Deploying with Docker compose

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
  -e ESPRESSO_OPS_TIMELOCK_ADMIN \
  -e ESPRESSO_OPS_TIMELOCK_PROPOSERS \
  -e ESPRESSO_OPS_TIMELOCK_EXECUTORS \
  -e ESPRESSO_OPS_TIMELOCK_DELAY \
  -e RUST_LOG \
  -e RPC_URL \
  -e ESPRESSO_SEQUENCER_ETH_MNEMONIC \
  -e ESPRESSO_DEPLOYER_ACCOUNT_INDEX \
  -e ESPRESSO_SEQUENCER_L1_PROVIDER \
  -e ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL \
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

## Multisig Owner

### Deploying with Cargo

```bash
set -a
source .env
set +a
unset ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS
RUST_LOG=info cargo run --bin deploy -- --deploy-esp-token --rpc-url=$RPC_URL
```

### Deploying with Docker Compose

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

# Timelock Proposals

These are demonstration commands and should not be used in production environments

## Transfer Ownership

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

## Upgrade To And Call

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
