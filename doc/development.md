# Development

## Getting started

- Clone: `git clone git@github.com:EspressoSystems/espresso-network`
- Install [nix](https://nixos.org/download.html)
- Activate the dev shell: `nix develop` (or `nix-shell` for legacy)
- Optional: copy `.envrc.example` to `.envrc.local` and run `direnv allow` so any shell entering the repo gets the env
  automatically
- Without nix: see [ubuntu.md](./ubuntu.md)

## Dev shells

The default shell is the small, fast daily driver. Other shells are either add-ons (everything in default plus extras)
or standalone (smaller, focused on one toolchain).

Add-ons (default + extras, suitable for daily use):

- `nix develop` (default): rust toolchain, anvil, solhint, forge fmt, prettier, process-compose
- `nix develop .#contracts`: default + `solc` + `go-ethereum` (abigen) + `FOUNDRY_SOLC` set. Use when working on
  contracts and rust tests at the same time (e.g. running `cargo test` against forge-built contracts)
- `nix develop .#mutation`: contracts + `dregs` for mutation testing
- `nix develop .#dockerShell`: default + `docker`

Standalone (smaller, separate toolchain):

- `nix develop .#docs`: `plantuml`, `graphviz`, `mdbook` for `make doc`
- `nix develop .#go`: full Go toolchain + `golangci-lint` for `sdks/go/`
- `nix develop .#python`: `python3`, `ruff`, `ty` for repo scripts
- `nix develop .#echidna`: `slither`, `echidna`, `crytic-compile`
- `nix develop .#crossShell`, `.#armCrossShell`: rust cross-compile (musl)
- `nix develop .#nightly`, `.#coverage`, `.#rustShell`: minimal rust variants

If a daily workflow forces you to leave the default shell often, it probably belongs in default. File an issue.

## Pre-commit hooks

- Source of truth: `.pre-commit-config.yaml` (committed)
- Runner: [`prek`](https://github.com/j178/prek), a rust reimplementation of pre-commit
- Entering `nix develop` installs `.git/hooks/pre-commit` automatically. The hook is nix-aware: it runs `prek` directly
  when the dev shell is active, and re-enters `nix develop` first when committing from outside (IDE, plain terminal)
- To add or change a hook: edit `.pre-commit-config.yaml`
- To run a hook manually: `pre-commit run <id>` from inside the default shell

## Tests

- Per-package, fast: `cargo nextest run -p <package>` (see `CLAUDE.md` for the package layout)
- Full workspace: `just test` (slow, mostly used by CI)
- Long-running: `just test-slow`
- Process-compose integration: `just test-demo base`, `just test-demo pos-base`, etc.
- Contracts: `just sol-test` (foundry unit + fuzz + invariant)

## Running a local network

- `just demo-native` brings up a full local network via `process-compose` (postgres in docker, espresso nodes, prover,
  etc.)
- Variants: `just demo-native-pos`, `just demo-native-pos-base`, `just demo-native-drb-header`,
  `just demo-native-epoch-reward`, etc.
- Stop with `Ctrl-C` then `just cleanup-process-compose` if anything was left behind

## Contracts

Enter the contracts shell: `nix develop .#contracts`.

- Compile: `forge build`
- Test: `just sol-test`
- Lint: `solhint 'contracts/{script,src,test}/**/*.sol'` (also a pre-commit hook)
- Format: `forge fmt`
- Rust bindings: `just gen-bindings` (regenerates rust + ABI exports under `contracts/rust/adapter/src/bindings`,
  `contracts/artifacts/`, `sdks/go/`)
- Solidity docs: `forge doc`

Conventions:

- `contracts/src` is production code
- `contracts/demo` is demo-only code
- V2 contracts inherit V1 storage. Never modify V1 storage layout.
- Use the latest version's rust bindings (e.g. `StakeTableV3`) for runtime code; V1/V2 bindings exist only for
  deploy/upgrade code

## Deployment

Three paths exist.

### Via the `deploy` binary (preferred)

- Build and run: `cargo run --bin deploy -- [FLAGS/OPTIONS]`
- Help: `cargo run --bin deploy -- --help`
- Source: `crates/espresso/node/src/bin/deploy.rs`

Common env vars (full list in the source):

- `ESPRESSO_L1_PROVIDER`: L1 JSON-RPC endpoint
- `ESPRESSO_ETH_MNEMONIC`: deployer wallet mnemonic
- `ESPRESSO_ETH_MULTISIG_ADDRESS`: multisig admin address
- `ESPRESSO_DEPLOYER_ACCOUNT_INDEX`: wallet account index
- `ESPRESSO_API_NODE_URL`: espresso node URL for HotShot config

Use a `.env` file with:

```bash
set -a; source .env; set +a
```

### Via Docker compose

- Pull images: `just pull`
- Deploy: `just demo deploy-prover-contracts`
- Local-changes rebuild: `./scripts/build-docker-images-native --image $IMAGE` instead of `just pull`

### Dry-run multisig upgrades

```bash
just pull
just demo
docker compose run --rm upgrade-prover-contracts-v2 \
    /bin/deploy --upgrade-light-client-v2 --dry-run --use-multisig
```

For AWS ECS, set all required env vars and secrets on the task definition.

## Logging

- Per-binary level: `RUST_LOG=debug cargo run --bin deploy -- ...`
- In docker: `docker run --env-file .env.docker -e RUST_LOG=debug ...`
- See `.env.docker.example` for the docker env-file format

## Documentation

- Rust docs (rendered): [espresso-network.docs.espressosys.com](https://espresso-network.docs.espressosys.com)
- Build locally: `just doc --open`
- Architecture / figures (requires `.#docs` shell): `make doc`

## Benchmarking and profiling

Gas consumption:

```sh
just gas-benchmarks
cat gas-benchmarks.txt
```

Light-client contract gas profile via sentio:

1. Set `SEPOLIA_RPC_URL`, `MNEMONIC`, `ETHERSCAN_API_KEY`
2. `just lc-contract-profiling-sepolia`
3. Sign up at [sentio.xyz](https://app.sentio.xyz/)
4. Use the resulting `newFinalizedState` transaction hash to pull the gas profile in sentio

## Daily commands cheat sheet

- `cargo fmt`
- `cargo check -p <package> --tests`
- `cargo clippy -p <package> --tests`
- `cargo nextest run -p <package>`
- `just check` and `just lint` before pushing (full workspace, slower)
- `forge fmt`, `forge test`, `just sol-test`
- `just gen-bindings` after touching contracts

See `CLAUDE.md` for more detail and pitfalls.
