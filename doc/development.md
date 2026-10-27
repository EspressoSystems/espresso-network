# Development

## Getting started

- Clone: `git clone git@github.com:EspressoSystems/espresso-network`
- Install [nix](https://nixos.org/download.html)
- Activate the dev shell: `nix develop` (or `nix-shell` for legacy)
- Optional: copy `.envrc.example` to `.envrc.local` and run `direnv allow` so any shell entering the repo gets the env
  automatically
- Without nix: see [ubuntu.md](./ubuntu.md)

## Dev shells

The default shell is the small, fast daily driver. Add-on shells contain everything in default plus extras.

- `nix develop` (default): rust toolchain, postgres, anvil, solhint, forge fmt, prettier, process-compose
- `nix develop .#contracts`: `solc` + `go-ethereum` (abigen) + `FOUNDRY_SOLC` set
- `nix develop .#mutation`: contracts + `dregs` for mutation testing
- `nix develop .#docs`: `plantuml`, `graphviz`, `mdbook` for `just doc-figures`, `just doc-contracts`
- `nix develop .#go`: Go toolchain + `golangci-lint` for `sdks/go/`
- `nix develop .#python`: `python3`, `uv`, `ruff`, `ty` for repo scripts
- `nix develop .#aws`: python + `opentofu`, `awscli2` for `scripts/network-bench/aws`
- `nix develop .#dockerShell`: `docker`

Standalone shells:

- `nix develop .#echidna`: `slither`, `echidna`, `crytic-compile`
- `nix develop .#crossShell`, `.#armCrossShell`: rust cross-compile (musl)
- `nix develop .#nightly`, `.#coverage`, `.#rustShell`: minimal rust variants

Recipes that need an add-on shell declare it with `[script("scripts/with-tools", "<devshell>", "<command>"...)]`. The
body runs directly if all commands are on PATH, otherwise inside `nix develop .#<devshell>`; without nix it fails
listing the missing commands. Module justfiles pass the path relative to their own directory. Requires just >= 1.44.

If a daily workflow forces you to leave the default shell often, it probably belongs in default. File an issue.

## Pre-commit hooks

- Source of truth: `.pre-commit-config.yaml` (committed)
- Runner: [`prek`](https://github.com/j178/prek), a rust reimplementation of pre-commit
- Entering `nix develop` runs `prek install`; the hook lives in the shared git hooks dir, so it covers all worktrees
- Checkouts that used the old git-hooks.nix shell have an untracked `.pre-commit-config.yaml` symlink; delete it once
  before pulling (`rm .pre-commit-config.yaml`), otherwise git refuses to overwrite it
- Commit from inside the dev shell: hooks run `rustfmt`, `nixpkgs-fmt`, `prettier` etc. from PATH
- Contract or figure work: use `nix develop .#contracts` / `.#docs`; from the default shell the `contract-bindings` and
  `doc` hooks enter that shell via nix on each matching commit
- To add or change a hook: edit `.pre-commit-config.yaml`
- To run a hook manually: `pre-commit run <id>` from inside the default shell

## Tests

- Per-package, fast: `cargo nextest run -p <package>` (package layout: `doc/agents/rust.md`)
- Full workspace: `just test` (slow, mostly used by CI)
- Long-running: `just test-slow`
- Process-compose integration: `just test-demo base`, `just test-demo pos-base`, etc.
- Contracts: `just sol-test` (foundry unit + fuzz + invariant)
- SQL storage tests start a native postgres (`initdb`, `postgres` on PATH; in the default shell)

## Running a local network

- `just demo`: Docker Compose, images from ghcr (updated on every push to main)
- `just demo-native`: process-compose, builds and runs the binaries locally (native postgres via
  `scripts/run-postgres`), so it picks up uncommitted changes
- Variants: `just demo-native-da-committees`, `just demo-native-large-block-upgrade`, `just demo-native-ff`,
  `just demo-native-benchmark`
- Stop with `Ctrl-C` then `scripts/cleanup-process-compose` if anything was left behind
- See `process-compose.yaml` and `docker-compose.yaml` for more information

## Contracts

Enter the contracts shell: `nix develop .#contracts`.

- Compile: `forge build`
- Test: `just sol-test`
- Lint: `solhint 'contracts/{script,src,test}/**/*.sol'` (also a pre-commit hook)
- Format: `forge fmt`
- Rust bindings: `just gen-bindings` (regenerates rust + ABI exports under `contracts/rust/adapter/src/bindings`,
  `contracts/artifacts/`, `sdks/go/`)
- Solidity docs: `just doc-contracts`

Conventions:

- `contracts/src` is production code
- `contracts/demo` is demo-only code
- V2 contracts inherit V1 storage. Never modify V1 storage layout.
- Use the latest version's rust bindings (e.g. `StakeTableV3`) for runtime code; V1/V2 bindings exist only for
  deploy/upgrade code

## Deployment

The `deploy` binary deploys and upgrades contracts.

- Help: `cargo run --bin deploy -- --help`, or
  `ghcr.io/espressosystems/espresso-network/deploy:$DOCKER_TAG deploy --help`
- Example invocations: `process-compose.yaml`, `docker-compose.yaml`
- Log level: `RUST_LOG=debug cargo run --bin deploy -- ...`

## Documentation

- Rust docs (rendered): [espresso-network.docs.espressosys.com](https://espresso-network.docs.espressosys.com)
- Build locally: `just doc --open`
- Architecture / figures: `just doc-figures`

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

See `CLAUDE.md` and `doc/agents/` for more detail and pitfalls.
