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

The Espresso Network is the global confirmation layer for rollups in the Ethereum ecosystem. Espresso's
[global confirmation layer(GCL)](https://docs.espressosys.com/network) provides agreement on inputs to a collection of
composable blockchains, providing a high trust, fast, and verifiable way to process inputs on any chain, providing fast
confirmations in return.

- [Official Documentation](https://docs.espressosys.com/network/)
- [Rust Documentation](https://espresso-network.docs.espressosys.com/espresso_node/)
- [Smart Contract Documentation](https://espresso-network.docs.espressosys.com/contracts/)

### Architecture

The diagram below shows how the Espresso Confirmation Layer fits into the rollup centric Ethereum ecosystem. See
[Architecture](./doc/architecture.md) for details.

![Architecture](./doc/espresso-overview.svg)

#### ZK rollups integration

In order for ZK rollups to rely on blocks produced by Espresso as a source of transactions, it is required to adjust the
circuit that encodes the state update logic. See [zk-rollups integration](doc/zk-integration.md) for more details.

# Running the demo

Refer to [espresso-example-l2](https://github.com/EspressoSystems/espresso-example-l2) for instructions on how to run
a dockerized Espresso Network with an example Layer 2 rollup application.

# Development

- See [doc/development.md](./doc/development.md).

# License

## Copyright

**(c) 2022 Espresso Systems** `espresso-network` was developed by Espresso Systems. While we plan to adopt an open
source license, we have not yet selected one. As such, all rights are reserved for the time being. Please reach out to
us if you have thoughts on licensing.

# Disclaimer

**DISCLAIMER:** This software is provided "as is" and its security has not been externally audited. Use at your own
risk.

**DISCLAIMER:** The Rust library crates provided in this repository are intended primarily for use by the binary targets
in this repository. We make no guarantees of public API stability. If you are building on these crates, reach out by
opening an issue to discuss the APIs you need.
