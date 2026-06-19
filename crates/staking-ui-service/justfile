mod demo

default:
    just --list

doc *args:
    cargo doc --no-deps --document-private-items {{args}}

fmt:
    cargo fmt --all

fix *args:
    cargo clippy --fix {{args}}

lint *args:
    cargo clippy --all-targets {{args}} -- -D warnings

build profile="dev" *args:
    cargo build --profile {{profile}} {{args}}

run-profile profile *args:
    #!/usr/bin/env bash
    set -e
    storage=$(mktemp /tmp/staking-ui.XXXXXX)
    echo "temporary database: $storage"
    function cleanup {
        rm -r "$storage"
    }
    trap cleanup EXIT
    cargo run --profile {{profile}} -- --path "${storage}" {{args}}

run-decaf:
    just run \
        --stake-table-address 0x40304fbe94d5e7d1492dd90c53a2d63e8506a037 \
        --reward-contract-address 0x0000000000000000000000000000000000000000 \
        --http-providers "${SEPOLIA_RPC_URL:-https://ethereum-sepolia.publicnode.com}" \
        --espresso-url https://cache.decaf.testnet.espresso.network/v1/

run-local: build
    #!/usr/bin/env bash
    just demo::up l1 espresso-dev-node
    function demo_down {
        just demo::down
    }
    trap demo_down EXIT
    just run \
        --stake-table-address 0xefdc2a236dba7a8f60726b49abc79ee6b22ed445 \
        --reward-contract-address 0x0000000000000000000000000000000000000000 \
        --http-providers http://localhost:8545 \
        --l1-ws-provider ws://localhost:8546 \
        --espresso-url http://localhost:24000/v1

run *args: (run-profile "dev" args)

test *args:
    cargo nextest run --locked --workspace --verbose {{args}}

coverage:
    cargo llvm-cov nextest --ignore-filename-regex testing.rs
    cargo llvm-cov report --html --ignore-filename-regex testing.rs
    @echo "HTML report available at file://${CARGO_TARGET_DIR:-target}/llvm-cov/html/index.html"

build-docker: (build "release" "--features" "rand")
    docker build \
        --build-arg CARGO_TARGET_DIR=$(realpath -s --relative-to="$PWD" "${CARGO_TARGET_DIR:-target}") \
        -t ghcr.io/espressosystems/staking-ui-service:main \
        -f docker/staking-ui-service.Dockerfile \
        .
    docker build \
        --build-arg CARGO_TARGET_DIR=$(realpath -s --relative-to="$PWD" "${CARGO_TARGET_DIR:-target}") \
        -t ghcr.io/espressosystems/staking-client-swarm:main \
        -f docker/client-swarm.Dockerfile \
        .
