<!--
 ~ Copyright (c) 2022 Espresso Systems (espressosys.com)
 ~ This file is part of the HotShot Query Service library.
 ~
 ~ This program is free software: you can redistribute it and/or modify it under the terms of the GNU
 ~ General Public License as published by the Free Software Foundation, either version 3 of the
 ~ License, or (at your option) any later version.
 ~ This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
 ~ even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 ~ General Public License for more details.
 ~ You should have received a copy of the GNU General Public License along with this program. If not,
 ~ see <https://www.gnu.org/licenses/>.
 -->

# HotShot Query Service

The HotShot Query Service is a minimal, generic query service that can be integrated into any decentralized application
running on the [HotShot](https://github.com/EspressoSystems/HotShot) consensus layer. It provides all the features that
HotShot itself expects of a query service (such as providing consensus-related data for catchup and synchronization) as
well as some application-level features that deal only with consensus-related or application-agnostic data. In addition,
the query service is provided as an extensible library, which makes it easy to add additional, application-specific
features (such as a JSON-RPC service in the case of an EVM application).

## Features

- Provides a query interface to a persistent store containing the history of a blockchain (represented as a chain of
  HotShot [leaves](https://hotshot.docs.espressosys.com/hotshot_types/data/struct.Leaf.html))
- Provides a query interface for validator status and metrics
- Integrates with HotShot to populate the persistent database
- Generic over application types (within a leaf, the block and block commitment can be any type the application
  specifieds)
- Transparently handles asynchronous data availability &mdash; block commitments are provided as soon as they are
  committed by consensus; block data is provided once it has disseminated from the data availability committee; missing
  data is fetched asynchronously from other nodes
- Serves as a source of data for HotShot catchup
- Easily extensible with application-specific functionality

## Development

```bash
git clone git@github.com:EspressoSystems/hotshot-query-service
cd hotshot-query-service
```

### Environment

We recommend installing dependencies with the [nix](https://nixos.org) package manager, which will ensure you use the
same Rust version and related tools as everyone else using a Nix environment (including the CI system). If you prefer
not to use Nix, you should still be able to build the project with the normal [rustup](https://rustup.rs)-installed
toolchain, and you can skip this section.

#### Install nix

Installation instructions can be found [here](https://nixos.org/download.html). If in a rush, running the following
command and following the on-screen instructions should work in most cases

```bash
curl -L https://nixos.org/nix/install | sh
```

#### Activate the nix environment

To activate a shell with the development environment run

```bash
nix-shell
```

from within the top-level directory of the repo.

Note: for the remainder of this README it is necessary that this environment is active.

#### direnv (optional, but recommended for development)

To avoid manually activating the nix shell each time, the [direnv](https://direnv.net/) shell extension can be used to
activate the environment when entering the local directory of this repo. Note that direnv needs to be
[hooked](https://direnv.net/docs/hook.html) into the shell to function.

To enable `direnv` run

```bash
direnv allow
```

from the root directory of this repo.

### Building and testing

Once you have Cargo installed (either via Nix or Rustup) you can build the project with `cargo build` or
`cargo build --release`. To run unit tests, we recommend using

```bash
cargo nextest run --release --all-features
```

You can also run unit tests using the development profile, but without the optimizations of the release profile, some
stack sizes exceed the default Rust stack. Therefore, you will need to configure a default stack size to run the
development build. 3MB seems to work well:

```bash
RUST_MIN_STACK=3145728 cargo nextest run --all-features
```

#### Doctest

Doctests require the `testing` feature. Due to a [limitation of doctest](https://stackoverflow.com/a/55727482) it is
recommended to run doctests via

```bash
cargo nextest run --doc --features testing
```
