[![Coverage Status](https://coveralls.io/repos/github/EspressoSystems/staking-ui-service/badge.svg?branch=main&t=JypAHg)](https://coveralls.io/github/EspressoSystems/staking-ui-service?branch=main)

# Staking UI Service

A scalable backend service for the Espresso delegation UI, designed to support tens of thousands of simultaneous clients with real time data updates.

## Overview

This service provides a polling based REST API that streams blockchain state to UI clients using a **snapshot + updates** pattern. Instead of Web socket, clients poll cache optimized endpoints to receive incremental updates. The state is organized into independent modules sourced from either Ethereum L1 (validator registrations, delegations, withdrawals) or Espresso (validator statistics, rewards). Each module maintains a finalized snapshot in SQL database and unfinalized updates in memory, with automatic handling of L1 reorgs. Clients bootstrap by fetching a complete snapshot indexed by block hash, then poll for sequential updates to maintain realtime state. This design by using the block hashes make most endpoints pure functions that can be cached. This design ensures handling of L1 reorgs, automatic failover between RPC providers, and strong data consistency guarantee across all clients.

## Dependencies

It is recommended to use [Nix](https://nixos.org/download/) for managing build-time dependencies.
Once you have installed Nix, you can enter a development shell with all required dependencies[^1]
installed by running `nix develop`.

Nix is not required, and it is possible to install all dependencies manually. The main thing you
will need is a stable Rust installation (>= 1.90), which you can install using
[rustup](https://rustup.rs/). It is also recommended to install the [`just` command runner](https://github.com/casey/just).

[^1]:
    This provides all required dependencies, but not the optional `cargo-llvm-cov` dependency for
    generating coverage reports. To install this, first enter the nix shell and then run
    `cargo install cargo-llvm-cov`.

## Development

Most common development tasks are invocable through `just`. For example:

- Run unit tests: `just test`
- Lint: `just lint`
- Build release binaries: `just build release`
- Generate an HTML code coverage report: `just coverage`

Run `just` by itself to see a list of all possible commands.

## Running

The service can be run either as a native executable or as a Docker container. To build and run
natively, use `just run`. You can optionally pass in a build profile as well as arguments to the
service itself, e.g. `just run release --port 3000`.

To run via Docker, first [build or obtain a Docker image](#docker), then run
`docker run ghcr.io/espressosystems/staking-ui-service:main`.

No matter how you run the service, you will need to configure it properly to connect to an Espresso
and Ethereum blockchain. Service configuration is outlined in the next section.

## Configuration

The following environment variables are _required_ for the service to run:

| Variable                               | Description                                                                                         |
| -------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `ESPRESSO_STAKING_SERVICE_L1_HTTP`     | One or more (comma-separated) HTTP RPC endpoints for reading from Ethereum (or a different layer 1) |
| `ESPRESSO_STAKING_SERVICE_STAKE_TABLE` | Address of stake table contract on layer 1                                                          |
| `ESPRESSO_STAKING_SERVICE_STORAGE`     | Path to local storage (a SQLite database will be created or opened here)                            |

In additional, the following _optional_ variables are available for customization:

| Variable                         | Description                                                                                                                                                   |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ESPRESSO_STAKING_SERVICE_L1_WS` | Comma-separated list of WebSockets RPC endpoints for reading from Ethereum. Providing these in addition to HTTP can reduce the cost of streaming data from L1 |
| `ESPRESSO_STAKING_SERVICE_PORT`  | Port for the HTTP server to run on. **Default: 8080**.                                                                                                        |

## Docker

The ultimate build target for this service is a Docker image, enabling easy deployment in a variety
of settings. The image is defined in the [Dockerfile](./Dockerfile). The CI pipeline automatically
builds a cross-platform (for ARM and AMD on Linux) image from this Dockerfile and publishes it as
`ghcr.io/espressosystems/staking-ui-service:main`.

If you are running Linux, you can build your own version of this image using `just build-docker`.

Unfortunately, this is not so easy on MacOS, since Rust cross-compilation for Linux is...
complicated. Mac users will have to make do with the published images:
`docker pull ghcr.io/espressosystems/staking-ui-service:main`.
