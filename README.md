# Espresso Network

[![Build](https://github.com/EspressoSystems/espresso-network/actions/workflows/build.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/build.yml)
[![Test](https://github.com/EspressoSystems/espresso-network/actions/workflows/test.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/test.yml)
[![Docs](https://github.com/EspressoSystems/espresso-network/actions/workflows/docs.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/docs.yml)
[![Contracts](https://github.com/EspressoSystems/espresso-network/actions/workflows/contracts.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/contracts.yml)
[![Lint](https://github.com/EspressoSystems/espresso-network/actions/workflows/lint.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/lint.yml)
[![Audit](https://github.com/EspressoSystems/espresso-network/actions/workflows/audit.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/audit.yml)
[![Ubuntu](https://github.com/EspressoSystems/espresso-network/actions/workflows/ubuntu-install-without-nix.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/ubuntu-install-without-nix.yml)
[![Build without lockfile](https://github.com/EspressoSystems/espresso-network/actions/workflows/build-without-lockfile.yml/badge.svg)](https://github.com/EspressoSystems/espresso-network/actions/workflows/build-without-lockfile.yml)
[![Coverage Status](https://coveralls.io/repos/github/EspressoSystems/espresso-network/badge.svg?branch=main)](https://coveralls.io/github/EspressoSystems/espresso-network?branch=main)

Espresso is a high-performance blockchain that provides fast, final settlement for other chains and applications. Each
integrated system keeps its own execution environment and sequencer. Espresso finalizes their transaction data through
its own decentralized proof-of-stake BFT consensus (HotShot). Other applications or chains can verify that finality
through zero-knowledge proofs, without trusted intermediaries.

As of October 2026, [benchmarks](https://docs.espressosys.com/network/network/benchmarks) of the mainnet consensus code
on 100 validators (dedicated AWS servers with emulated global latency) showed a median time to finality of 334 ms. At
peak load, the network finalized 181 MB/s of transaction data (about 1 million simple token transfers per second) with
median settlement under one second.

This repository contains the Espresso node, HotShot consensus, the data availability layer, the light client and its
Ethereum contracts, and staking tooling.

- [Official Documentation](https://docs.espressosys.com/network/)
- [Rust Documentation](https://espresso-network.docs.espressosys.com/espresso_node/)
- [Smart Contract Documentation](https://espresso-network.docs.espressosys.com/contracts/)

### Architecture

See [How Espresso Fits](https://docs.espressosys.com/network/learn/espresso-in-the-modular-stack) for an overview, and
[Architecture](doc/architecture.md) for implementation details.

#### ZK rollups integration

In order for ZK rollups to rely on blocks produced by Espresso as a source of transactions, it is required to adjust the
circuit that encodes the state update logic. See [zk-rollups integration](doc/zk-integration.md) for more details.

# Development

- Obtain code: `git clone git@github.com:EspressoSystems/espresso-network`.
- Make sure [nix](https://nixos.org/download.html) is installed.
- Activate the environment with `nix-shell`, or `nix develop`. If using [direnv](https://direnv.net/), copy
  `.envrc.example` to `.envrc.local` (or create your own `.envrc.local` file) and run `direnv allow`.
- For installation without nix please see [ubuntu.md](./doc/ubuntu.md).
- The rust code documentation can be found at
  [espresso-network.docs.espressosys.com](https://espresso-network.docs.espressosys.com). Please note the disclaimer
  about API stability at the end of the readme.

## Development commands

```sh
just # see available commands
just doc --open
just build
just test --package espresso-types # gate by package to avoid long runtime
```

## Running a local network

A full local network is run two ways:

```sh
just demo         # Docker Compose, images from ghcr (updated on every push to main)
just demo-native  # process-compose, building and running the binaries locally
```

- `just demo-native` builds the binaries first, so it picks up uncommitted changes.
- Genesis and process variants are available as additional just recipes.

See [process-compose.yaml](process-compose.yaml) and [docker-compose.yaml](docker-compose.yaml) for more information.

# Contracts

## Development

A foundry project for the contracts specific to HotShot can be found in the directory `contracts`.

To compile

```shell
forge build
just contracts-test-forge
just gen-bindings # update rust contract bindings
forge doc # build docs
```

## Deployment

The deploy binary is used for contract deployment.

```bash
cargo run --bin deploy -- --help
ghcr.io/espressosystems/espresso-network/deploy:$DOCKER_TAG deploy --help
```

See [process-compose.yaml](process-compose.yaml) and [docker-compose.yaml](docker-compose.yaml) for example invocations.

### Dry run upgrades via Docker

You can only run a dry run for multisig upgrades but you need to stand up all services via docker compose Example:

```bash
just pull
just demo
docker compose run --rm upgrade-prover-contracts-v2 /bin/deploy --upgrade-light-client-v2 --dry-run --use-multisig
```

If making dev changes locally run, `./scripts/build-docker-images-native` instead of `just pull`.

For AWS ECS, ensure all required environment variables and secrets are set in your task definition.

### Logging

You can control the log level using the `RUST_LOG` environment variable. For example:

```bash
RUST_LOG=info cargo run --bin deploy -- [FLAGS]
RUST_LOG=debug cargo run --bin deploy -- [FLAGS]
```

### Benchmarking and profiling

The gas consumption for verifying a plonk proof as well as updating the state of the light client contract can be seen
by running:

```sh
just gas-benchmarks
cat gas-benchmarks.txt
# [PASS] test_verify_succeeds() (gas: 507774)
# [PASS] testCorrectUpdateBench() (gas: 594533)
```

In order to profile the gas consumption of the light client contract do the following:

1. Set the environment variables `SEPOLIA_RPC_URL`, `MNEMONIC` and `ETHERSCAN_API_KEY`.
2. `just lc-contract-profiling-sepolia`
3. Create an account on [sentio.xyz](https://app.sentio.xyz/).
4. Use the hash of the transaction generated in step two when calling the function `newFinalizedState` in order to
   obtain the gas profile.

# License

## Copyright

**(c) 2022 Espresso Systems** `espresso-network` was developed by Espresso Systems. While we plan to adopt an open
source license, we have not yet selected one. As such, all rights are reserved for the time being. Please reach out to
us if you have thoughts on licensing.

# Disclaimer

**DISCLAIMER:** This software is provided "as is", without warranty of any kind.

**DISCLAIMER:** The Rust library crates provided in this repository are intended primarily for use by the binary targets
in this repository. We make no guarantees of public API stability. If you are building on these crates, reach out by
opening an issue to discuss the APIs you need.

# Security

The smart contracts in [`contracts/`](contracts/) have been externally
[audited](https://github.com/EspressoSystems/espresso-audits). The node software, including HotShot consensus, has been
internally reviewed (see [`audits/`](audits/)) but not externally audited.

To report a vulnerability, see our
[security policy](https://github.com/EspressoSystems/espresso-network/security/policy).
