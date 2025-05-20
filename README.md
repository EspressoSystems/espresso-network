# Espresso Network

[![Build](https://github.com/EspressoSystems/espresso-network/actions/workflows/build.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/build.yml)
[![Test](https://github.com/EspressoSystems/espresso-network/actions/workflows/test.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/test.yml)
[![Docs rust](https://github.com/EspressoSystems/espresso-network/actions/workflows/doc-rust.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/doc-rust.yml)
[![Contracts](https://github.com/EspressoSystems/espresso-network/actions/workflows/contracts.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/contracts.yml)
[![Lint](https://github.com/EspressoSystems/espresso-network/actions/workflows/lint.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/lint.yml)
[![Audit](https://github.com/EspressoSystems/espresso-network/actions/workflows/audit.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/audit.yml)
[![Ubuntu](https://github.com/EspressoSystems/espresso-network/actions/workflows/ubuntu-install-without-nix.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/ubuntu-install-without-nix.yml)
[![Build without lockfile](https://github.com/EspressoSystems/espresso-network/actions/workflows/build-without-lockfile.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/build-without-lockfile.yml)
[![Coverage Status](https://coveralls.io/repos/github/EspressoSystems/espresso-network/badge.svg?branch=main)](https://coveralls.io/github/EspressoSystems/espresso-network?branch=main)

The Espresso Network is the global confirmation layer for rollups in the Ethereum ecosystem. Espresso's
[global confirmation layer(GCL)](https://docs.espressosys.com/network) provides agreement on inputs to a collection of
composable blockchains, providing a high trust, fast, and verifiable way to process inputs on any chain, providing fast
confirmations in return.

[Official Documentation](https://docs.espressosys.com/network/)

### Architecture

The diagram below shows how the Espresso Confirmation Layer fits into the rollup centric Ethereum ecosystem. See
[Architecture](./doc/architecture.md) for details.

![Architecture](./doc/espresso-overview.svg)

#### ZK rollups integration

In order for ZK rollups to rely on blocks produced by Espresso as a source of transactions, it is required to adjust the
circuit that encodes the state update logic. See [zk-rollups integration](doc/zk-integration.md) for more details.

# Running the demo

Refer to [sequencer-example-l2](https://github.com/EspressoSystems/sequencer-example-l2) for instructions on how to run
a dockerized Espresso Sequencer network with an example Layer 2 rollup application.

# Development

- Obtain code: `git clone git@github.com:EspressoSystems/espresso-network`.
- Make sure [nix](https://nixos.org/download.html) is installed.
- Activate the environment with `nix-shell`, or `nix develop`, or `direnv allow` if using [direnv](https://direnv.net/).
- For installation without nix please see [ubuntu.md](./doc/ubuntu.md).

## Documentation

The rust code documentation can be found at [sequencer.docs.espressosys.com](https://sequencer.docs.espressosys.com).
Please note the disclaimer about API stability at the end of the readme.

To generate the documentation locally and view it in the browser, run

    just doc --open

## Run the tests

    just pull # to pull docker images
    just test

## Building figures

    make doc

## Building and running

Docker images and the [docker-compose-demo.yaml](docker-compose-demo.yaml) file are provided for convenience. The
Docker-based demo fetches the images from the `ghcr` repository, where they are updated with every push to `main` on
GitHub. For testing uncommitted changes, you can also run the binaries by manually building and running the services.

Build all executables with `cargo build --release`. You may then start a sequencer network. First, start an
orchestrator. Choose a port `$PORT` to run it on and decide how many sequencer nodes `$N` you will use, then run
`target/release/orchestrator -p $PORT -n $N`.

The sequencer will distribute a HotShot configuration to all the nodes which connect to it, which specifies consensus
parameters like view timers. There is a default config, but you can override any parameters you want by passing
additional options to the `orchestrator` executable. Run `target/release/orchestrator --help` to see a list of available
options.

Next, you must launch a `cdn` instance, which is necessary to facilitate consensus.

```bash
just dev-cdn -- -p 1738
```

In this case, we run it on port 1738.

Once you have started the orchestrator and the CDN, you must connect `$N` sequencer nodes to them, after which the
network will start up automatically. To start one node, run

```bash
target/release/sequencer \
    --orchestrator-url http://localhost:$PORT \
    --cdn-endpoint "127.0.0.1:1738"  \
    -- http --port 8083 -- query --storage-path storage -- submit
```

A useful Bash snippet for running `$N` nodes simultaneously in the background of your shell is:

```bash
for i in `seq $N`; do
    target/release/sequencer \
        --orchestrator-url http://localhost:$PORT \
        --cdn-endpoint "127.0.0.1:1738"  \
done
```

For running a full demo natively run `just demo-native`.

### Contracts

#### Development

A foundry project for the contracts specific to HotShot can be found in the directory `contracts`.

To compile

```shell
forge build
```

To run the tests

```shell
just sol-test
```

In order to avoid constant warnings about checksum mismatches with [svm-rs](https://github.com/roynalnaruto/svm-rs)
managed `solc` we set `FOUNDRY_SRC` to solc installed via flake.nix.

- To use the contracts from rust generate the rust contracts bindings: `just gen-bindings`.
- Bindings are only generated for contracts in the `contracts/src` folder

To generate documentation in `./docs` for solidity code run

```shell
forge doc
```

#### Deployment

To deploy the contracts to a local testnet, first run a dev chain (e.g. `anvil`), then run

    forge script DeployHotShot --broadcast --rpc-url local

To deploy to sepolia set `SEPOLIA_RPC_URL` and `MNEMONIC` env vars and run

    forge script DeployHotShot --broadcast --rpc-url sepolia

To additionally verify the contract on etherscan set the `ETHERSCAN_API_KEY` env var and run

    forge script DeployHotShot --broadcast --rpc-url sepolia --verify

Running the script will save a file with details about the deployment in `contracts/broadcast/$CHAIN_ID`.

#### Folder Structure Rationale

- code for demo purposes goes into the `contracts/demo` folder
- code that eventually ends up in production goes into the `contracts/src` folder

#### Benchmarking and profiling

The gas consumption for verifying a plonk proof as well as updating the state of the light client contract can be seen
by running:

```
> just gas-benchmarks
> cat gas-benchmarks.txt
[PASS] test_verify_succeeds() (gas: 507774)
[PASS] testCorrectUpdateBench() (gas: 594533)
```

In order to profile the gas consumption of the light client contract do the following:

1. Set the environment variables `SEPOLIA_RPC_URL`, `MNEMONIC` and `ETHERSCAN_API_KEY`.
2. `just lc-contract-profiling-sepolia`
3. Create an account on [sentio.xyz](https://app.sentio.xyz/).
4. Use the hash of the transaction generated in step two when calling the function `newFinalizedState` in order to
   obtain the gas profile.

# License

## Copyright

**(c) 2022 Espresso Systems** `espresso-sequencer` was developed by Espresso Systems. While we plan to adopt an open
source license, we have not yet selected one. As such, all rights are reserved for the time being. Please reach out to
us if you have thoughts on licensing.

# Disclaimer

**DISCLAIMER:** This software is provided "as is" and its security has not been externally audited. Use at your own
risk.

**DISCLAIMER:** The Rust library crates provided in this repository are intended primarily for use by the binary targets
in this repository. We make no guarantees of public API stability. If you are building on these crates, reach out by
opening an issue to discuss the APIs you need.
