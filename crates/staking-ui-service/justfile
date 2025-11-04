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
    cargo run --profile {{profile}} -- {{args}}

run-decaf:
    #!/usr/bin/env bash
    set -e
    storage=$(mktemp /tmp/staking-ui.XXXXXX)
    echo "temporary database: $storage"
    function cleanup {
        rm -r "$storage"
    }
    trap cleanup EXIT
    just run \
        --stake-table 0x40304fbe94d5e7d1492dd90c53a2d63e8506a037 \
        --storage "${storage}" \
        --http-providers https://ethereum-sepolia.publicnode.com

run *args: (run-profile "dev" args)

test *args:
    cargo nextest run --locked --workspace --verbose {{args}}

coverage:
    cargo llvm-cov nextest
    cargo llvm-cov report --html

build-docker: (build "release")
    docker build \
        --build-arg CARGO_TARGET_DIR=$(realpath -s --relative-to="$PWD" "${CARGO_TARGET_DIR:-target}") \
        -t ghcr.io/espressosystems/staking-ui-service:main \
        .
