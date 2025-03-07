# Installing on ubuntu (without nix)

<!-- Note that all lines that start with four spaces will be executed in the CI -->

## Install system dependencies

    sudo apt-get update
    sudo apt-get install -y curl cmake pkg-config libssl-dev protobuf-compiler git postgresql-client lsb-release gpg

## Install docker

Refer to https://docs.docker.com/engine/install/ubuntu

## Install just

Just is not available in the official ubuntu repos.

    curl --proto '=https' --tlsv1.2 -sSf 'https://proget.makedeb.org/debian-feeds/prebuilt-mpr.pub' | gpg --dearmor | sudo tee /usr/share/keyrings/prebuilt-mpr-archive-keyring.gpg 1> /dev/null
    echo "deb [arch=all,$(dpkg --print-architecture) signed-by=/usr/share/keyrings/prebuilt-mpr-archive-keyring.gpg] https://proget.makedeb.org prebuilt-mpr $(lsb_release -cs)" | sudo tee /etc/apt/sources.list.d/prebuilt-mpr.list
    sudo apt-get update
    sudo apt-get install -y just

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

    git clone --recursive https://github.com/espressosystems/espresso-sequencer
    cd espresso-sequencer

## Build the contracts

    forge build

## Run the rust tests

To run the SQL tests docker needs to be installed and running.

    export "PATH=$PWD/target/release:$PATH"
    cargo build --release --bin diff-test
    just test --no-fail-fast

## Run the foundry tests

Here a single fuzz run is used just to check that things are working.

    env FOUNDRY_FUZZ_RUNS=1 forge test -v
