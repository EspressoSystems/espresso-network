default:
    just --list

doc *args:
    cargo doc --no-deps --document-private-items {{args}}

demo *args:
    docker compose up {{args}}

demo-native *args: build
    scripts/demo-native {{args}}

lint:
    #!/usr/bin/env bash
    set -euxo pipefail
    # Use the same target dir for both `clippy` invocations
    export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target}
    cargo clippy --workspace --features testing --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --manifest-path sequencer-sqlite/Cargo.toml -- -D warnings

build profile="test":
    #!/usr/bin/env bash
    set -euxo pipefail
    # Use the same target dir for both `build` invocations
    export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target}
    cargo build --profile {{profile}}
    cargo build --profile {{profile}} --manifest-path ./sequencer-sqlite/Cargo.toml

demo-native-mp *args: build
    scripts/demo-native -f process-compose.yaml -f process-compose-mp.yml {{args}}

demo-native-benchmark:
    cargo build --release --features benchmarking
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

test *args:
    @echo 'Omitting slow tests. Use `test-slow` for those. Or `test-all` for all tests.'
    @echo 'features: "embedded-db"'
    cargo nextest run --locked --workspace --features embedded-db --verbose {{args}}
    cargo nextest run --locked --workspace --verbose {{args}}

test-slow:
    @echo 'Only slow tests are included. Use `test` for those deemed not slow. Or `test-all` for all tests.'
    @echo 'features: "embedded-db"'
    cargo nextest run --locked --release --workspace --features embedded-db --verbose --profile slow
    cargo nextest run --locked --release --workspace --verbose --profile slow

test-all:
    @echo 'features: "embedded-db"'
    cargo nextest run --locked --release --workspace --features embedded-db --verbose --profile all
    cargo nextest run --locked --release --workspace --verbose --profile all

test-integration:
	@echo 'NOTE that demo-native must be running for this test to succeed.'
	INTEGRATION_TEST_SEQUENCER_VERSION=2 cargo nextest run --all-features --nocapture --profile integration smoke
test-integration-mp:
    @echo 'NOTE that demo-native-mp must be running for this test to succeed.'
    INTEGRATION_TEST_SEQUENCER_VERSION=99 cargo nextest run --all-features --nocapture --profile integration

clippy:
    @echo 'features: "embedded-db"'
    cargo clippy --workspace --features embedded-db --all-targets -- -D warnings
    cargo clippy --workspace -- -D warnings

check-features *args:
    cargo hack check --each-feature {{args}}

# Helpful shortcuts for local development
dev-orchestrator:
    target/release/orchestrator -p 8080 -n 1

dev-cdn *args:
    RUST_LOG=info cargo run --release --bin dev-cdn -- {{args}}

dev-state-relay-server:
    target/release/state-relay-server -p 8083

dev-sequencer:
    target/release/sequencer \
    --orchestrator-url http://localhost:8080 \
    --cdn-endpoint "127.0.0.1:1738" \
    --state-relay-server-url http://localhost:8083 \
    -- http --port 8083  -- query --storage-path storage

build-docker-images:
    scripts/build-docker-images-native

# generate rust bindings for contracts
REGEXP := "^LightClient$|^LightClientArbitrum$|^LightClientStateUpdateVK$|^FeeContract$|PlonkVerifier$|^ERC1967Proxy$|^LightClientMock$|^LightClientStateUpdateVKMock$|^PlonkVerifier2$|^PermissionedStakeTable$"
gen-bindings:
    forge bind --contracts ./contracts/src/ --crate-name contract-bindings --bindings-path contract-bindings --select "{{REGEXP}}" --overwrite --force

    # Foundry doesn't include bytecode in the bindings for LightClient.sol, since it links with
    # libraries. However, this bytecode is still needed to link and deploy the contract. Copy it to
    # the source tree so that the deploy script can be compiled whenever the bindings are up to
    # date, without needed to recompile the contracts.
    mkdir -p contract-bindings/artifacts
    jq '.bytecode.object' < contracts/out/LightClient.sol/LightClient.json > contract-bindings/artifacts/LightClient_bytecode.json
    jq '.bytecode.object' < contracts/out/LightClientArbitrum.sol/LightClientArbitrum.json > contract-bindings/artifacts/LightClientArbitrum_bytecode.json
    jq '.bytecode.object' < contracts/out/LightClientMock.sol/LightClientMock.json > contract-bindings/artifacts/LightClientMock_bytecode.json

    cargo fmt --all
    cargo sort -g -w

# Lint solidity files
sol-lint:
    forge fmt
    solhint --fix 'contracts/{script,src,test}/**/*.sol'

# Build diff-test binary and forge test
# Note: we use an invalid etherscan api key in order to avoid annoying warnings. See https://github.com/EspressoSystems/espresso-sequencer/issues/979
sol-test:
    cargo build --profile test --bin diff-test
    forge test

# Deploys the light client contract on Sepolia and call it for profiling purposes.
NUM_INIT_VALIDATORS := "5"
MAX_HISTORY_SECONDS := "864000" # 10 days
lc-contract-profiling-sepolia:
    @sh -c 'source ./.env.contracts'
    #!/usr/bin/env bash
    set -euxo pipefail
    forge script contracts/test/script/LightClientTestScript.s.sol --sig "runBench(uint64 numInitValidators, uint32 stateHistoryRetentionPeriod)" {{NUM_INIT_VALIDATORS}} {{MAX_HISTORY_SECONDS}} --fork-url ${SEPOLIA_RPC_URL} --broadcast --verify --etherscan-api-key ${ETHERSCAN_API_KEY} --chain-id sepolia
    LC_CONTRACT_ADDRESS=`cat contracts/broadcast/LightClientTestScript.s.sol/11155111/runBench-latest.json | jq -r .receipts[-1].contractAddress`

    echo $LC_CONTRACT_ADDRESS
    forge script contracts/script/LightClientCallNewFinalizedState.s.sol --sig "run(uint32 numInitValidators, address lcContractAddress)" {{NUM_INIT_VALIDATORS}} $LC_CONTRACT_ADDRESS --fork-url ${SEPOLIA_RPC_URL}  --broadcast  --chain-id sepolia

gas-benchmarks:
    cargo build --profile test --bin diff-test
    forge snapshot --mt "test_verify_succeeds|testCorrectUpdateBench"
    @[ -n "$(git diff --name-only .gas-snapshot)" ] && echo "⚠️ Uncommitted gas benchmarks, please stage them before committing." && exit 1 || exit 0

# This is meant for local development and produces HTML output. In CI
# the lcov output is pushed to coveralls.
code-coverage:
  @echo "Running code coverage"
  nix develop .#coverage -c cargo test --all-features --no-fail-fast --release --workspace -- --skip service::test::test_
  grcov . -s . --binary-path $CARGO_TARGET_DIR/debug/ -t html --branch --ignore-not-existing -o $CARGO_TARGET_DIR/coverage/ \
      --ignore 'contract-bindings/*' --ignore 'contracts/*'
  @echo "HTML report available at: $CARGO_TARGET_DIR/coverage/index.html"

# Download Aztec's SRS for production
download-srs:
    @echo "Check existence or download SRS for production"
    @./scripts/download_srs_aztec.sh

# Download Aztec's SRS for test (smaller degree usually)
dev-download-srs:
    @echo "Check existence or download SRS for dev/test"
    @AZTEC_SRS_PATH="$PWD/data/aztec20/kzg10-aztec20-srs-65544.bin" ./scripts/download_srs_aztec.sh
