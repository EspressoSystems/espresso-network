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
`docker run ghcr.io/espressosystems/staking-ui-service:latest`.

No matter how you run the service, you will need to configure it properly to connect to an Espresso
and Ethereum blockchain. Service configuration is outlined in the next section.

## Configuration

The following environment variables are _required_ for the service to run:

| Variable                                           | Description                                                                                         |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `ESPRESSO_STAKING_SERVICE_L1_HTTP`                 | One or more (comma-separated) HTTP RPC endpoints for reading from Ethereum (or a different layer 1) |
| `ESPRESSO_STAKING_SERVICE_STAKE_TABLE_ADDRESS`     | Address of stake table contract on layer 1                                                          |
| `ESPRESSO_STAKING_SERVICE_REWARD_CONTRACT_ADDRESS` | Address of the reward claim contract on layer 1                                                     |
| `ESPRESSO_STAKING_SERVICE_ESPRESSO_URL`            | URL for an Espresso query service                                                                   |
| `ESPRESSO_STAKING_SERVICE_DB_PATH`                 | Path to local storage (a SQLite database will be created or opened here)                            |

In additional, the following _optional_ variables are available for customization:

| Variable                                                    | Description                                                                                                                                                   |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ESPRESSO_STAKING_SERVICE_L1_WS`                            | Comma-separated list of WebSockets RPC endpoints for reading from Ethereum. Providing these in addition to HTTP can reduce the cost of streaming data from L1 |
| `ESPRESSO_STAKING_SERVICE_L1_RETRY_DELAY`                   | Delay before retrying failed L1 RPC requests (default: `1s`)                                                                                                  |
| `ESPRESSO_STAKING_SERVICE_L1_POLLING_INTERVAL`              | Interval for polling HTTP RPCs for new blocks (if not using WebSockets) (default: `7s`)                                                                       |
| `ESPRESSO_STAKING_SERVICE_L1_SUBSCRIPTION_TIMEOUT`          | Maximum time to wait for L1 new heads before considering a stream invalid and reconnecting (default: `2m`)                                                    |
| `ESPRESSO_STAKING_SERVICE_L1_FREQUENT_FAILURE_TOLERANCE`    | Fail over to another provider if the current provider fails twice within this window (default: `1m`)                                                          |
| `ESPRESSO_STAKING_SERVICE_L1_CONSECUTIVE_FAILURE_TOLERANCE` | Fail over to another provider if the current provider fails many times in a row (default: `10`)                                                               |
| `ESPRESSO_STAKING_SERVICE_L1_FAILOVER_REVERT`               | Revert back to the first provider this duration after failing over (default: `30m`)                                                                           |
| `ESPRESSO_STAKING_SERVICE_L1_RATE_LIMIT_DELAY`              | Amount of time to wait after receiving a 429 response before making more L1 RPC requests (default: same as L1 retry delay)                                    |
| `ESPRESSO_STAKING_SERVICE_L1_EVENTS_MAX_BLOCK_RANGE`        | Maximum number of blocks allowed in a ranged `eth_getLogs` query (default: `10000`)                                                                           |
| `ESPRESSO_STAKING_SERVICE_STREAM_TIMEOUT`                   | Time after which an idle Espresso block stream is considered dead and restarted (default: `1m`)                                                               |
| `ESPRESSO_STAKING_SERVICE_PORT`                             | Port for the HTTP server to run on. (default: `8080`)                                                                                                         |
| `ESPRESSO_STAKING_SERVICE_DB_MAX_CONNECTIONS`               | Maximum number of simultaneous open SQL connections (default: `5`)                                                                                            |
| `RUST_LOG`                                                  | Log level (e.g. `debug`, `info`, `warn`)                                                                                                                      |
| `RUST_LOG_FORMAT`                                           | Log formatting (`full`, `compact`, or `json`)                                                                                                                 |

## Docker

The ultimate build target for this service is a Docker image, enabling easy deployment in a variety
of settings. The image is defined in the [Dockerfile](./docker/staking-ui-service.Dockerfile). The
CI pipeline automatically builds a cross-platform (for ARM and AMD on Linux) image from this
Dockerfile and publishes it as `ghcr.io/espressosystems/staking-ui-service:latest`.

If you are running Linux, you can build your own version of this image using `just build-docker`.

Unfortunately, this is not so easy on MacOS, since Rust cross-compilation for Linux is...
complicated. Mac users will have to make do with the published images:
`docker pull ghcr.io/espressosystems/staking-ui-service:latest`.

## Demo

A full-system demo is available via [demo/docker-compose.yaml](demo/docker-compose.yaml). This
system includes:

- A geth node running a local L1 devnet
- An `espresso-dev-node` simulating an Espresso network
- An instance of the staking UI service

The demo can be controlled via the `demo` Just module:

- `just demo::up`: start all services
- `just demo::logs <service>`: print logs for a service
- `just demo::down`: shut down all services and clean up

The demo runs the staking service via the Docker tag
`docker pull ghcr.io/espressosystems/staking-ui-service:latest`. By default this will pull the
image from the GHCR registry. You can override this tag locally to run the demo with some changes
that haven't been pushed yet, using the [Docker build instructions](#docker). You can replace your
local image by pulling from the registry using `just demo::pull`.

As mentioned above, it is only possible to build a local version of the staking service Docker if
you are running on Linux. Otherwise, you can test local changes against the demo by starting all
Docker services _except_ the staking UI service, and then running the staking service as a native
executable, connecting to the Docker services via TCP. This is possible by running `just run-local`.

### Limitations

The current demo has some known limitations:

- The Espresso dev node does not deploy a reward contract, so the reward contract address is set to
  the zero address
- Startup is slow because we deploy new contracts every time. This could potentially be improved by
  creating an L1 image that has contracts already deployed

### Testing the UI

The primary purpose of this service is to be a backend for the staking UI. You can test the latest
version of the UI against the backend service using the [online demo](https://bookish-doodle-kq5le1n.pages.github.io/?path=/story/sites-delegation-ui-page--local-dev-net&args=l1ValidatorServiceURL:;l1ValidatorServiceURLEncoded:aHR0cDovL2xvY2FsaG9zdDo4MDgwL3YwL3N0YWtpbmcv)
of the UI. This allows you to test the UI against various deployments of the staking service.

Due to browser restrictions, if you want to test the UI against the [local demo](#demo) of the
staking service, you will also need to run the UI demo locally:

- Check out the UI with `git clone git@github.com:EspressoSystems/espresso-block-explorer.git`
- If you have already cloned the UI, make sure you have the latest version with `git pull`
- Run the demo with `npm run --workspace packages/espresso-block-explorer-components storybook`

Once the UI is running, you should also start (in a separate shell) the local demo of the staking UI
service, using either `just demo::up` or `just run-local`. Then you can open the UI using
[your browser](http://localhost:6006/?path=/story/sites-delegation-ui-page--local-dev-net&args=l1ValidatorServiceURL:;l1ValidatorServiceURLEncoded:aHR0cDovL2xvY2FsaG9zdDo4MDgwL3YwL3N0YWtpbmcv).
