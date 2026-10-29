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

- See [doc/development.md](./doc/development.md).

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
