# Installing on ubuntu (without nix)

<!-- Note that all lines that start with four spaces will be executed in the CI -->

## Install system dependencies

The `postgresql` package provides the server binaries (`initdb`, `postgres`, `pg_ctl`, `pg_isready`) the SQL tests use;
docker is not required.

    sudo apt-get update
    sudo apt-get install -y curl cmake pkg-config libssl-dev protobuf-compiler git postgresql lsb-release gpg nodejs npm
    sudo npm install -g yarn

The postgres server binaries are not on `PATH` on Debian/Ubuntu; add them.

    export "PATH=$(ls -d /usr/lib/postgresql/*/bin | sort -V | tail -1):$PATH"

## Install just

Just is outdated in the official ubuntu repos.

    curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | sudo bash -s -- --to /usr/local/bin/

## Install rust dependencies

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source $HOME/.cargo/env

## Install nextest test runner

    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    cargo binstall cargo-nextest --secure --no-confirm

## Install foundry

    curl --proto '=https' --tlsv1.2 -sSf -L https://foundry.paradigm.xyz | bash
    export "PATH=$HOME/.foundry/bin:$PATH"
    foundryup

## Clone the repository

    git clone --recursive https://github.com/espressosystems/espresso-network
    cd espresso-network

## Install npm dependencies

    yarn install

## Build the contracts

    forge build

## Build and smoke-test the rust tests

Compiling the test binaries verifies the toolchain and system dependencies. The SQL tests run against a native postgres
server (installed above), so a single migration test smoke-tests that path without docker.

    just nextest --no-run
    just nextest --no-fail-fast test_migrations

To run the full suite, use `just test` (slow) or `just test-all`.

## Run the foundry tests

Here a single fuzz run is used just to check that things are working.

    env FOUNDRY_FUZZ_RUNS=1 forge test -v
