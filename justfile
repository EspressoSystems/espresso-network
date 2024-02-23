default:
    just --list

demo *args:
    docker compose up {{args}}

demo-native:
    cargo build --release
    scripts/demo-native

down *args:
    docker compose down {{args}}

docker-cli *cmd:
    docker exec -it espresso-sequencer-example-rollup-1 bin/cli {{cmd}}

cli *cmd:
    target/release/cli {{cmd}}

pull:
    docker compose pull

docker-stop-rm:
    docker stop $(docker ps -aq); docker rm $(docker ps -aq)

anvil *args:
    docker run -p 127.0.0.1:8545:8545 ghcr.io/foundry-rs/foundry:latest "anvil {{args}}"

test:
    cargo build --bin diff-test --release
    cargo test --release --all-features

# Helpful shortcuts for local development
dev-orchestrator:
    target/release/orchestrator -p 8080 -n 1 

dev-da-server:
    target/release/web-server -p 8081

dev-consensus-server:
    target/release/web-server -p 8082

dev-state-relay-server:
    target/release/state-relay-server -p 8083

dev-sequencer:
    target/release/sequencer \
    --orchestrator-url http://localhost:8080 \
    --da-server-url http://localhost:8081 \
    --consensus-server-url http://localhost:8082 \
    --state-relay-server-url http://localhost:8083 \
    -- http --port 8083  -- query --storage-path storage

dev-commitment:
     target/release/commitment-task --sequencer-url http://localhost:50000 \
     --l1-provider http://localhost:8545 \
     --eth-mnemonic "test test test test test test test test test test test junk" \
     --deploy

build-docker-images:
    scripts/build-docker-images

# generate rust bindings for contracts
gen-bindings:
    forge bind --contracts ./contracts/src/ --crate-name contract-bindings --bindings-path contract-bindings --overwrite --force
    cargo fmt --all
    cargo sort -g -w

# Lint solidity files
sol-lint:
    forge fmt
    solhint --fix 'contracts/{script,src,test}/**/*.sol'

# Build diff-test binary and forge test
# Note: we use an invalid etherscan api key in order to avoid annoying warnings. See https://github.com/EspressoSystems/espresso-sequencer/issues/979
sol-test:
    cargo build --bin diff-test --release
    forge test

# Deploy contracts to local blockchain for development and testing
dev-deploy url="http://localhost:8545" mnemonics="test test test test test test test test test test test junk" num_blocks_per_epoch="10" num_init_validators="5":
    forge build
    MNEMONICS="{{mnemonics}}" forge script 'contracts/test/LightClientTest.s.sol' \
    --sig "run(uint32 numBlocksPerEpoch, uint32 numInitValidators)" {{num_blocks_per_epoch}} {{num_init_validators}} \
    --fork-url {{url}} --broadcast
