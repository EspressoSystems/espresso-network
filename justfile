mod hotshot

default:
    just --list

doc *args:
    cargo doc --no-deps --document-private-items {{args}}

demo *args:
    #!/usr/bin/env bash
    # The TUI wouldn't work on the CI
    CI=${CI:-false}
    if [ "$CI" = "true" ]; then
        docker compose up {{args}}
    else
        trap "exit" INT TERM
        trap cleanup EXIT
        cleanup(){
            docker compose down -v
        }
        >/dev/null 2>&1 docker compose up {{args}} &
        lazydocker
    fi


demo-native *args: (build "test")
    scripts/demo-native {{args}}

fmt:
    cargo fmt --all

fix *args:
    just clippy --fix {{args}}

lint *args:
    just clippy -- -D warnings

clippy *args:
    # check all targets in default workspace members
    cargo clippy --features testing --all-targets {{args}}
    # check entire workspace (including sequencer-sqlite crate) with embedded-db feature
    cargo clippy --workspace --features "embedded-db testing" --all-targets {{args}}

build profile="dev" features="":
    cargo build --profile {{profile}} {{features}}
    cargo build --profile {{profile}} -p sequencer-sqlite {{features}}

demo-native-pos *args: (build "test" "--no-default-features --features fee,pos")
    ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE=data/genesis/demo-pos.toml scripts/demo-native -f process-compose.yaml {{args}}

demo-native-pos-base *args: (build "test" "--no-default-features --features pos")
    ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE=data/genesis/demo-pos-base.toml scripts/demo-native -f process-compose.yaml {{args}}

demo-native-drb-header-upgrade *args: (build "test" "--no-default-features --features pos,drb-and-header")
    ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE=data/genesis/demo-drb-header-upgrade.toml scripts/demo-native -f process-compose.yaml {{args}}

demo-native-fee-to-drb-header-upgrade *args: (build "test" "--no-default-features --features fee,drb-and-header")
    ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE=data/genesis/demo-fee-to-drb-header-upgrade.toml scripts/demo-native -f process-compose.yaml {{args}}

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

nextest *args:
    # exclude hotshot-testing because it takes ages to compile and has its own hotshot.just file
    cargo nextest run --locked --workspace --exclude sequencer-sqlite --exclude hotshot-testing --verbose {{args}}

test *args:
    @echo 'Omitting slow tests. Use `test-slow` for those. Or `test-all` for all tests.'
    @echo 'features: "embedded-db"'
    just nextest --features embedded-db  {{args}}
    just nextest {{args}}

test-slow *args:
    @echo 'Only slow tests are included. Use `test` for those deemed not slow. Or `test-all` for all tests.'
    @echo 'features: "embedded-db"'
    just nextest --features embedded-db --profile slow {{args}}
    just nextest --profile slow {{args}}

test-all:
    @echo 'features: "embedded-db"'
    just nextest --features embedded-db --profile all
    just nextest --profile all

test-integration: (build "test" "--features fee")
	INTEGRATION_TEST_SEQUENCER_VERSION=2 cargo nextest run -p tests --nocapture --profile integration test_native_demo_basic

check-features *args:
    cargo hack check --each-feature {{args}}

check-features-ci *args:
    # check each pair of features plus `default` and `--no-default-features`
    cargo hack check --feature-powerset \
        --depth 2 \
        --exclude-all-features \
        --exclude hotshot \
        --exclude hotshot-builder-api \
        --exclude hotshot-contract-adapter \
        --exclude hotshot-events-service \
        --exclude hotshot-example-types \
        --exclude hotshot-libp2p-networking \
        --exclude hotshot-macros \
        --exclude hotshot-orchestrator \
        --exclude hotshot-query-service \
        --exclude hotshot-state-prover \
        --exclude hotshot-task \
        --exclude hotshot-task-impls \
        --exclude hotshot-testing \
        --exclude hotshot-types \
        --exclude hotshot-utils \
        --exclude vid \
        {{args}}

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
REGEXP := "^LightClient(V\\d+)?$|^LightClientArbitrum(V\\d+)?$|^FeeContract$|PlonkVerifier(V\\d+)?$|^ERC1967Proxy$|^LightClient(V\\d+)?Mock$|^StakeTable$|^StakeTableV2$|^EspToken$|^EspTokenV2$|^OpsTimelock$|^SafeExitTimelock$|^OwnableUpgradeable$|^RewardClaimPrototypeMock$|RewardClaim$"
gen-bindings:
    # Update the git submodules
    git submodule update --init --recursive

    # Generate the alloy bindings
    # TODO: `forge bind --alloy ...` fails if there's an unliked library so we pass pass it an address for the PlonkVerifier contract.
    forge bind --skip test --skip script --use "0.8.28" --alloy --alloy-version "0.13.0" --contracts ./contracts/src/ \
      --module --bindings-path contracts/rust/adapter/src/bindings --select "{{REGEXP}}" --overwrite --force \
      --libraries contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier:0xffffffffffffffffffffffffffffffffffffffff \
      --libraries contracts/src/libraries/PlonkVerifierV2.sol:PlonkVerifierV2:0xffffffffffffffffffffffffffffffffffffffff \
      --libraries contracts/src/libraries/PlonkVerifierV3.sol:PlonkVerifierV3:0xffffffffffffffffffffffffffffffffffffffff

    just export-contract-abis
    just gen-go-bindings

# export select ABIs, to let downstream projects can use them without solc compilation
export-contract-abis:
    rm -rv contracts/artifacts/abi
    mkdir -p contracts/artifacts/abi
    for contract in LightClient{,Mock,V2{,Mock}} StakeTable EspToken; do \
        cat "contracts/out/${contract}.sol/${contract}.json" | jq .abi > "contracts/artifacts/abi/${contract}.json"; \
    done

# Lint solidity files
sol-lint:
    forge fmt
    solhint --fix 'contracts/{script,src,test}/**/*.sol'

# Build diff-test binary and forge test
# Note: we use an invalid etherscan api key in order to avoid annoying warnings. See https://github.com/EspressoSystems/espresso-sequencer/issues/979
sol-test *args:
    export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target} &&\
    cargo build --release --bin diff-test &&\
    env PATH="${CARGO_TARGET_DIR}/release:$PATH" forge test {{ args }}

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
    2>&1 | tee log.txt

gen-go-bindings:
	abigen --abi contracts/artifacts/abi/LightClient.json --pkg lightclient --out sdks/go/light-client/lightclient.go
	abigen --abi contracts/artifacts/abi/LightClientMock.json --pkg lightclientmock --out sdks/go/light-client-mock/lightclient.go

build-go-crypto-helper *args:
    ./scripts/build-go-crypto-helper {{args}}

test-go:
    #!/usr/bin/env bash
    export LD_LIBRARY_PATH=$PWD/sdks/go/verification/target/lib:$LD_LIBRARY_PATH
    cd sdks/go && go test -v ./...

contracts-test-echidna *args:
    nix develop .#echidna -c echidna contracts/test/StakeTableV2.echidna.sol --contract StakeTableV2EchidnaTest --config contracts/echidna.yaml {{args}}

contracts-test-forge *args='-vv':
    forge test --no-match-test "testFuzz_|invariant_" {{args}}

contracts-test-fuzz *args='-vv':
    forge test --match-test testFuzz {{args}}

contracts-test-invariant *args='-vv':
    forge test --match-test invariant_ {{args}}
