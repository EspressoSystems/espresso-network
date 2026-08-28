# Espresso node compile times: measurements and where to cut

Status: in progress (2026-08-28). Owner: compile-time investigation, branch `ma/compilation-times`. All numbers are
measured, source is named for each. Do not trust unlabeled estimates in this file; there should be none.

## TL;DR

- CI wall time for a release binary is set by one serial chain that ends in a single crate: `espresso-node` lib (264-368
  s) followed by a wrapper bin unit (205-284 s). Together 65-78 % of the job's wall clock. Nothing else on the graph
  matters until this is fixed.
- The wrapper bin units (`espresso-node` bin from a 55-line `main.rs`, `espresso-node-sqlite` bin, `espresso-dev-node`
  bin) are expensive only in release. In the `test` profile the same bin units cost 15-30 s. Hypothesis under test:
  `-Cshare-generics` is on by default at opt-level<=1 and off at opt-level 3, so release re-monomorphizes the whole node
  in every wrapper bin.
- `espresso-node` lib costs the same at opt-level 1 (test profile, 266-280 s) as at opt-level 3 (release, 264-303 s).
  The lib is therefore not LLVM-optimization bound; it is bound by frontend work plus codegen volume.
- The 4 parallel release build jobs share 24 % redundant CPU (1180 s of 4918 s) even after accounting for feature
  differences; the `espresso-node` lib alone is built 4x with 4 different feature sets (`[]`, `testing+hotshot-testing`,
  `embedded-db`, `embedded-db+testing`).
- The release job `other` builds the node lib with the `testing` feature (368 s, the single most expensive unit in CI)
  because feature unification pulls test-only code into a release build.

## Method

- CI data: `cargo --timings` HTML artifacts already produced by `build.yml` and `test.yml`
  (`scripts/cargo-timing-summary`), parsed with a local script (unit durations, per-crate sums, critical path,
  redundancy across jobs). Run analyzed: build.yml 33198385363 and test.yml 33198385391, PR branch
  `ma/stake-table-events-tests`, both green, warm `Swatinem/rust-cache` from main (so third-party deps are cached;
  workspace crates are not).
- Runner: `ubuntu-latest`, 4 vCPU. Average parallelism in these jobs is 1.4-2.9, i.e. the jobs are serialization-bound,
  not core-bound.
- Local data: 32-core / 62 GB machine, rustc 1.97.1 (pinned in `rust-toolchain.toml`), `CARGO_INCREMENTAL=0`,
  `RUSTC_WRAPPER` (kache) disabled, dedicated target dir.

## CI measurements (build.yml run 33198385363, release profile)

| job                            | wall  | cpu-s | avg par | top units                                                 |
| ------------------------------ | ----- | ----- | ------- | --------------------------------------------------------- |
| Build espresso-node AMD        | 748 s | 1050  | 1.4     | node lib 303 s, node bin 284 s                            |
| Build other AMD                | 683 s | 1948  | 2.9     | node lib (testing) 368 s, lcqs bin 110 s, deploy bin 78 s |
| Build espresso-node-sqlite AMD | 671 s | 960   | 1.4     | node lib 264 s, sqlite bin 258 s                          |
| Build espresso-dev-node AMD    | 646 s | 960   | 1.5     | node lib 280 s, dev-node bin 206 s                        |

Critical path of the `espresso-node` job (durations of the units on the path):

```
hotshot-types build-script 7 s -> hotshot-types 25 s -> hotshot-contract-adapter 45 s ->
hotshot-task-impls 14 s -> hotshot 8 s -> hotshot-example-types 3 s -> hotshot-testing 30 s ->
hotshot-new-protocol 15 s -> espresso-types 60 s -> espresso-contract-deployer 25 s ->
hotshot-state-prover 42 s -> staking-cli 35 s -> espresso-node 303 s -> espresso-node bin 284 s
```

Notable: `hotshot-testing` (release!), `espresso-contract-deployer`, `hotshot-state-prover` and `staking-cli` are all on
the path to the node binary, i.e. the node lib depends on the deployer, the ZK prover and the staking CLI as regular
dependencies.

## CI measurements (test.yml run 33198385391, test profile)

| job                             | wall  | cpu-s | avg par | top units                                           |
| ------------------------------- | ----- | ----- | ------- | --------------------------------------------------- |
| Build test artifacts (sqlite)   | 833 s | 1858  | 2.2     | node lib+tests 431 s, node lib 280 s, hqs test 91 s |
| Build test artifacts (postgres) | 819 s | 1815  | 2.2     | node lib+tests 418 s, node lib 277 s, hqs test 88 s |
| Build test binaries             | 398 s | 722   | 1.8     | node lib 266 s, all bins 15-30 s each               |

The `Build test binaries` job is the cheapest evidence for the share-generics hypothesis: same crate graph, opt-level 1,
and every wrapper bin costs 15-30 s instead of 205-284 s.

## Redundancy across the 4 release build jobs

Total CPU 4918 s. Deduplicated by (crate, target, feature set): 3737 s. Redundant: 1180 s (24 %). Redundant per crate
(sum of all but one build): contract-adapter 128 s, state-prover 125 s, staking-cli 103 s, hotshot-testing 90 s,
hotshot-types 71 s, contract-deployer 70 s, espresso-types 54-62 s, task-impls 43 s, espresso-api 42 s, vid 41 s.

Splitting into more jobs cannot go below the serial chain (~640-750 s), so the redundancy costs runner minutes, not wall
clock.

## Local measurements (32 cores, rustc 1.97.1, no incremental, no rustc wrapper)

Cold build of everything needed for the release binary
(`cargo build --locked --release -p espresso-node --bin espresso-node --timings`): wall 354 s, 2001 cpu-s, avg
parallelism 5.7, 1429 units. Top units: `espresso-node` lib 123 s, `espresso-node` bin 96 s, `aws-lc-sys` build-script
59 s, `libsqlite3-sys` build-script 34 s, `hotshot-contract-adapter` 22 s.

The lib:bin ratio is the same locally (123:96) as in CI (303:284), so the wrapper-bin cost is not a CI artifact. With 32
cores the whole dependency graph disappears into the parallel region and the node crate is what remains.

## CI cost of a PR, all workflows (measured, median of 3 recent green PR runs)

| workflow                | wall   | runner-min | longest job                                                  |
| ----------------------- | ------ | ---------- | ------------------------------------------------------------ |
| slowtest.yaml           | 56 min | 506-520    | `slow-tests-postgres-2` 3297 s                               |
| test.yml                | 48 min | 501-526    | `test-integration` cell 1936 s (after a 890 s build)         |
| cargo-features.yml      | 43 min | 81-91      | `just check-features-ci` 2603 s, `--tests` 2512 s (uncached) |
| build.yml               | 24 min | 99-109     | `Build espresso-node AMD` 783 s                              |
| hotshot.yml             | 21 min | 324-330    | `test-new-protocol (standard)` 1239 s                        |
| contracts.yml           | 21 min | 64-66      | diff-test builds, uncached                                   |
| build-crypto-helper.yml | 15 min | 46         | 5 uncached builds of `sdks/crypto-helper` per PR             |
| docs.yml                | 9 min  | 10         | `cargo doc` 505 s, uncached                                  |
| lint.yml                | 5 min  | 10         | clippy                                                       |

Per PR: ~1650-1710 runner-minutes, ~56 min wall, of which Rust compilation is ~1050-1090 runner-minutes (~64 %).

Cache notes (all in `.github/workflows/`):

- Workspace crates are recompiled in every job of every run by design: `Swatinem/rust-cache` caches dependency artifacts
  only and deletes workspace-member artifacts before saving, and `CARGO_INCREMENTAL=0` is set (`build.yml:38`). So the
  workspace-crate rebuild is expected, not a cache bug. The lever is the _cost_ of those crates and the _number of jobs_
  that pay it, not caching them.
- `save-if` restricts _dependency_ cache saving to main/release-\* everywhere except `verify-proposals.yml:36-39`, which
  is the right policy; it only means a PR that changes `Cargo.lock` pays for third-party deps too.
- `prefix-key` is fragmented across `v3-rust`, `v1-rust`, `v2-hotshot`, `v1-bench` and unset, so jobs that compile
  identical dependency graphs cannot share even the dependency cache.
- The same `test`-profile/default-features graph is compiled 5x per PR in 5 jobs (test.yml archive 890 s, test.yml
  test-bins 604 s, contracts.yml diff-test x3 uncached at `contracts.yml:112`, contracts-mutation `build-diff-test` 162
  s, verify-proposals `deploy` 1360 s), and the `test`-profile/`embedded-db` graph 4x (964 s, 549 s, 653 s, slowtest
  dev-node 1473 s); two of those run on 4 vCPU while the archive job gets 8.
- `hotshot.yml`: 34 jobs share `shared-key: hotshot-tests`, only the `test-ci tests_1` cell saves (`hotshot.yml:61`).
- `build-allocators.yml` fails in all 8 matrix cells at `git apply` (`build-allocators.yml:69`) on every recent run, so
  its build step never executes. Verified on runs 33182961257, 33138064931, 33105369833. Dead CI weight, unrelated to
  compile time but worth deleting or fixing.
- `coverage.yml` and `unused-deps.yml` are `disabled_manually`.

## Dependency edges that are pure waste (verified in-tree)

Measured on the release critical path for `espresso-node`:

| edge                                               | cost on path | status                                                                                                                                                                                                                                      |
| -------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `hotshot-state-prover`                             | 42 s         | **zero uses** in `crates/espresso/node/src` (grep: no `hotshot_state_prover` anywhere). Required dep at `crates/espresso/node/Cargo.toml:74`                                                                                                |
| `staking-cli`                                      | 35 s         | all 5 uses are in test modules: `genesis.rs:1575` (`mod test`), `lib.rs:1054` (`pub mod testing`), `persistence.rs:257` (`mod tests`), `api.rs:1931` (`pub mod test_helpers`), `api.rs:3624` (`mod test`). Required dep at `Cargo.toml:109` |
| `hotshot-testing`                                  | 30 s         | reaches release builds through `hotshot-builder-shared/src/lib.rs:5` (`pub mod testing;` with no `cfg`, crate has no `[features]`), plus `hotshot-new-protocol` and `hotshot-query-service` normal deps                                     |
| `generic-tests`                                    | -            | **zero occurrences** in the crate (`Cargo.toml:65`)                                                                                                                                                                                         |
| `rstest`, `rstest_reuse`, `test-utils`, `tempfile` | -            | test-only, declared as normal deps (`Cargo.toml:101,102,114,113`)                                                                                                                                                                           |

`hotshot-contract-adapter` (45 s on the path) is expensive because of checked-in `sol!` output:
`contracts/rust/adapter/src/` is 302 640 lines, of which `src/bindings/` is 300 721; 6 383 `impl` blocks, 5 942
`#[derive]`. `src/lib.rs:8` compiles all 31 binding modules unconditionally, and the 3 mock modules alone are 53 123
lines. Its two consumers on the consensus path (`hotshot-task-impls`, `hotshot-new-protocol`) use exactly two functions
from it, `derive_signed_state_digest` and `validate_light_client_state_update_certificate`
(`contracts/rust/adapter/src/light_client.rs:143,166`).

`hotshot-state-prover` additionally builds `jf-plonk`, `jf-relation`, `jf-rescue` and `jf-signature` **twice** per
build, because the `*-compat` workspace deps are the same packages from a different branch (`Cargo.toml:282,291,299,309`
vs the non-compat entries).

Correction to an earlier hypothesis: `espresso-node` has no `Versions` type parameter at all
(`grep -rn "Versions" crates/espresso/node/src` is empty), so there is no protocol-version monomorphization matrix in
this crate. The generic surface that is instantiated twice is `<N: ConnectedNetwork, P: SequencerPersistence>` with
`network::Production` fixed and two `DataSourceOptions` impls (`api/data_source.rs:59,67`: sql and fs).

## Release artifact size (local build)

`target/release/espresso-node` is 161 MB unstripped: `.text` 58.4 MB, `.rodata` 36.9 MB, `.eh_frame` 6.2 MB,
`.gcc_except_table` 3.8 MB, 236 996 symbols. The compile cost of the node crate is proportional to this volume;
`.gcc_except_table` + `.eh_frame` (10 MB) is unwinding metadata, i.e. what `panic = "abort"` would remove.

## Structure of `espresso-node` (48 447 LOC), and what actually costs

LOC accounting: production 24 606, `feature = "testing"`-gated 2 191, `#[cfg(test)]` 16 986, bins 4 664. `src/api.rs` is
11 054 lines of which 1 893 are production; `src/persistence.rs` is 2 991 of which 197 are production. The 17 k lines of
test code do not affect release builds but they are exactly why the test-profile lib unit costs 431 s versus 280 s for
the release lib.

### The monomorphization multiplier (verified)

`run.rs` calls `run_with_storage<S>` for both `persistence::sql::Options` and `persistence::fs::Options`
(`run.rs:95-112`), giving two `P: SequencerPersistence` instantiations. Each one reaches
`api/options.rs:159 serve<N, P, F>`, which branches **at runtime** on the configured storage and therefore instantiates
_both_ `init_with_query_module_fs<N, P>` (`api/options.rs:274`) and `init_with_query_module_sql<N, P>`
(`api/options.rs:343`).

Result: the full `hotshot-query-service` data-source stack, the 22 `#[async_trait] impl<D>` blocks in
`api/state.rs:119-2672`, and the 4 generic axum routers in `crates/espresso/api/src/lib.rs:59, 150,200,234` are
codegened 4x (2 persistence x 2 query modules) inside `espresso-node` instead of once. `src/bin/reset-storage.rs:66,74`
re-emits the storage half again.

This is the single largest lever on the 280-303 s lib unit that does not require splitting the crate: make the storage
choice a `dyn` boundary (one `Box<dyn SequencerPersistence>` / `Box<dyn DataSource>`) instead of a type parameter, or
drop the runtime branch so that each `S` instantiates only its own query module.

### Why the crate cannot be split today

Extractable with zero intra-crate dependencies: `genesis.rs` (390 prod lines), `state_signature/relay_server*` (1 102),
`consensus_handle.rs` (749), `api/light_client.rs` (559), `network/cdn.rs`, `startup_catchup.rs`, `state_cert.rs`,
`util.rs`.

Everything else is blocked by three items in `src/api.rs` - `BlocksFrontier` (`api.rs:106`), `RewardMerkleTreeV2Data`
(`api.rs:1322`), `RewardMerkleTreeDataSource` (`api.rs:1349`) - imported by `catchup.rs:60`, `state.rs:32`,
`api/sql.rs:56`, `request_response/data_source.rs:34`. The resulting cycles (`api <-> catchup`,
`api <-> request_response`, `api -> context -> request_response -> api`) all originate in the import block
`api.rs:78-92`. Moving those three items into `espresso-types` or a small `espresso-node-traits` crate unblocks
extracting `api/state.rs` (2 672 production lines, only two intra-crate imports), then `persistence/`.

### Why the lib is compiled in 4 feature combinations

- `slow-tests/Cargo.toml:20` declares `espresso-node = { features = ["testing"] }` in `[dependencies]` (not
  dev-dependencies) and `slow-tests` is a default member (`Cargo.toml:108`). Any workspace-wide `cargo build` therefore
  unifies `testing` into the release node lib. This is why `build.yml`'s `other` job pays 368 s for the node lib instead
  of 303 s and drags `hotshot-testing` into a release build.
- `crates/espresso/node/Cargo.toml:139` is a self dev-dependency with `testing`.
- `embedded-db` is a compile-time SQL dialect switch (21 `cfg` sites, all in the SQL path), which forces the whole lib
  to be rebuilt for `espresso-node-sqlite` (`node-sqlite/src/main.rs` is 8 lines) and again for `espresso-dev-node`.
- `dev-node` depends on the `testing`-gated `api::test_helpers`, `api::data_source::testing`,
  `testing::TestConfigBuilder` (`dev-node/src/main.rs:37-48`), which is why `testing` cannot simply be moved to
  dev-dependencies without moving that code into a separate test-support crate.

## Local unit-level measurements (32 cores, release profile, warm deps)

| what                                                                   | wall          | note                                                       |
| ---------------------------------------------------------------------- | ------------- | ---------------------------------------------------------- |
| `espresso-node` lib + `espresso-node` bin, both touched                | 216 s / 213 s | two runs, +-1.5 % noise                                    |
| `espresso-node` lib alone                                              | 123 s         | `--lib`                                                    |
| `espresso-node` bin alone, lib already fresh                           | 90 s          | compiles a 55-line `main.rs` and links                     |
| `cargo check` of the whole graph incl. node lib (cold check artifacts) | 92 s          | not a per-crate number                                     |
| `cargo check` of the node lib alone (warm)                             | 33 s          | frontend + rmeta only; matches the 36 s of frontend passes |

`-Ztime-passes` for the node lib (frontend passes only; codegen-phase lines were not captured, to be redone with
`-Zself-profile`):

```
type_check_crate        15.6 s
MIR_borrow_checking     13.9 s
coherence_checking       5.3 s
macro_expand_crate       0.9 s
everything else         <0.2 s each
peak rss               1418 MB
```

So ~36 s of the ~123 s lib build is the frontend (type check + borrowck + coherence) and the remaining ~85 s is
monomorphization + LLVM. Macro expansion is negligible (0.9 s), which rules out "too many derives/async_trait" as the
primary cost: the cost is the _volume of code generated from_ those items, not their expansion.

### The wrapper bin: 90 s to compile 55 lines, measured

`-Ztime-passes` on the `espresso-node` bin unit (lib already built, release):

```
total                                   91.2 s
  codegen_crate                         56.1 s
    monomorphization_collector_graph_walk 42.0 s
    codegen_to_LLVM_IR                  11.8 s
  finish_ongoing_codegen                32.6 s
    LLVM_passes                         23.7 s
    LLVM_thinlto                        19.9 s
  link_binary / run_linker               1.5 s
  frontend (parse+resolve+typeck+borrowck) ~1 s
```

Linking is 1.5 s, i.e. irrelevant. The bin spends 42 s walking the monomorphization graph and 56 s generating code for a
file that contains one call to `espresso_node::main`. The bin crate is re-instantiating the node's generic code out of
the rlib, because at `opt-level = 3` rustc turns `-Cshare-generics` off, so generic instances in an upstream rlib cannot
be reused and every downstream crate codegens its own copies. In the `test` profile (opt-level 1, share-generics on) the
same bin unit costs 15-30 s in CI.

Every wrapper pays this: `espresso-node` bin 284 s, `espresso-node-sqlite` bin 258 s, `espresso-dev-node` bin 206 s in
CI, on top of the 264-368 s lib unit they each depend on.

### Where the node lib's time goes (rustc `-Zself-profile`, release, 16 CGUs)

Self time, summed over all rustc threads (455 s CPU for a 123 s wall build):

| item                                          | self time | count                       |
| --------------------------------------------- | --------- | --------------------------- |
| LLVM_module_optimize                          | 184.4 s   | 16 CGUs                     |
| LLVM_lto_optimize                             | 42.2 s    | 16                          |
| evaluate_obligation                           | 38.1 s    | 659 215 (30.2 M cache hits) |
| LLVM_passes                                   | 25.2 s    | 1                           |
| finish_ongoing_codegen                        | 23.6 s    | 1                           |
| LLVM_module_codegen_emit_obj                  | 23.2 s    | 16                          |
| normalize_canonicalized_projection            | 19.4 s    | 58 447                      |
| codegen_select_candidate                      | 17.4 s    | 124 715                     |
| LLVM_thin_lto_import                          | 15.0 s    | 16                          |
| codegen_module                                | 13.2 s    | 16                          |
| LLVM_thinlto                                  | 10.8 s    | 1                           |
| items_of_instance (total, not self)           | 46.2 s    | 294 029                     |
| monomorphization_collector_graph_walk (total) | 46.6 s    | 1                           |

Two conclusions:

1. **LLVM is ~71 % of the CPU time** (~325 s of 455 s). The input to LLVM is 286 030 monomorphized items
   (`check_mono_item` count) - that is the number to reduce.
2. **`codegen_module_perform_lto` totals 88 s** (`LLVM_lto_optimize` 42 s + `LLVM_thin_lto_import` 15 s + `LLVM_thinlto`
   11 s + overhead). The default release profile (`lto = false`) means _thin-local LTO across the crate's 16 CGUs_,
   which is not free. `lto = "off"` removes that step entirely; measured separately below.
3. Trait solving is ~74 s CPU (`evaluate_obligation` + `normalize_canonicalized_projection` +
   `codegen_select_candidate`), i.e. ~16 %. That is the cost of the deep generic/`async_trait` layering, and it is
   second-order compared to codegen volume.

### Codegen volume, measured

Unoptimized LLVM IR emitted by the node lib (`--emit=llvm-ir -Cno-prepopulate-passes -Ccodegen-units=1`): **22.6 M IR
lines across 279 859 functions, 1.71 GB of .ll**. `check_mono_item` in the self-profile counts 286 030 monomorphized
items, so the two agree.

For scale: the resulting stripped-of-nothing binary is 161 MB with a 58 MB `.text`. LLVM spends 184 s of CPU optimizing
16 CGUs built from that IR.

The wrapper bin re-emits almost all of it. Same flags, `--bin espresso-node` (whose `main.rs` is 55 lines): **20.2 M IR
lines across 245 505 functions, 1.62 GB of .ll** - i.e. 89 % of the lib's IR volume is generated a second time in the
bin crate. `-Zprint-mono-items=y` agrees: the bin instantiates 243 807 mono items versus the lib's 268 140. This is the
mechanism behind the 90 s (local) / 284 s (CI) bin unit, and it repeats for `espresso-node-sqlite` and
`espresso-dev-node`.

### Parallel frontend (`-Zthreads`, nightly-only flag, measured via `RUSTC_BOOTSTRAP=1`)

| variant                      | node lib wall |
| ---------------------------- | ------------- |
| baseline (1 thread frontend) | 123 s         |
| `-Zthreads=8`                | 104 s         |
| `-Zthreads=16`               | 101 s         |

-15 % to -18 %, bounded by the frontend share (~36 s of 123 s). Not a fix, and it is unstable, but it caps what any
frontend-side work can win on this crate.

## Refuted: collapsing the fs/sql storage matrix in `run.rs`

Patching `run.rs` so that only the sql branch of `run_with_storage` is instantiated (the fs fallback replaced by
`bail!`) changed nothing: node lib 122.7 s vs 123 s baseline, node bin 89.8 s vs 90 s.

The two `P: SequencerPersistence` instantiations reachable from `main` are not what generates the 286 k mono items. The
volume comes from the generic machinery itself (`api::state`, the query-service data sources, the router builders) being
instantiated per type parameter combination wherever it is used, so removing one call site does not remove the
instantiations. Any real fix has to erase the type parameters (dyn dispatch) or reduce the amount of generic code, not
remove one caller.

### codegen-units

`[profile.release.package.espresso-node] codegen-units = 64` (default 16), 32-core machine: lib 119.7 s (baseline 123
s), bin 88.0 s (baseline 90 s). No material change; the crate is not CGU-parallelism starved on a 32-core box. The
4-core CI case is measured separately below.

## What the 281 946 mono items in the node lib actually are

`-Zprint-mono-items=y` on the release lib, categorised by substring (categories overlap because one symbol can mention
several):

| category                                           | items  | share  |
| -------------------------------------------------- | ------ | ------ |
| mentions `hotshot_query_service`                   | 86 082 | 30.5 % |
| `tokio::runtime::task` plumbing                    | 75 117 | 26.6 % |
| mentions `axum`                                    | 46 491 | 16.5 % |
| mentions `api::state::NodeApiStateImpl`            | 43 788 | 15.5 % |
| mentions `espresso_api` routers                    | 43 576 | 15.5 % |
| `ark_*` / `jf_*` crypto                            | 35 559 | 12.6 % |
| `tracing::instrument::Instrumented`                | 20 780 | 7.4 %  |
| `serde_json` ser/de                                | 17 355 | 6.2 %  |
| `futures`                                          | 15 869 | 5.6 %  |
| `std::panicking::catch_unwind::do_call`/`do_catch` | 14 034 | 5.0 %  |
| BTreeMap internals                                 | 13 372 | 4.7 %  |
| `alloy`                                            | 12 593 | 4.5 %  |
| `tower`                                            | 6 974  | 2.5 %  |
| `sqlx`                                             | 6 735  | 2.4 %  |

By root path: `std` 114 744, `tokio` 45 919, `alloc` 10 534, `axum` 9 504, `serde_json` 6 620, `hashbrown` 4 748,
`hotshot_query_service` 4 329, `tower` 3 525.

The most-duplicated single identities (type arguments erased):

```
x6820  std::panicking::catch_unwind::do_call::<AssertUnwindSafe<{closure@tokio::runtime::task::harness...
x6820  std::panicking::catch_unwind::do_catch::<...same...>
x2536  <tower::util::map_future::MapFuture<axum::util::MapIntoResponse<axum::handler::HandlerService<{closure@espresso_api...
x2226  alloc::collections::btree::node::Handle::<...>
x1404  <api::state::NodeApiStateImpl<Arc<hotshot_query_service::data_source::ExtensibleDataSource<...>>>
x1324  tokio::runtime::task::harness::Harness::<Pin<Box<tracing::instrument::Instrumented<{async block...
x 840  {closure@espresso_api::axum::router_availability<api::state::NodeApiStateImpl<Arc<hotshot_query_service...
```

Two mechanisms produce almost all of it:

1. **The API state type is a type parameter.** `crates/espresso/api/src/axum.rs` (190 KB) exposes 16
   `router_*<S>(state: S)` generic builders (`axum.rs:292,563,1360,1429,1505,...`), and
   `crates/espresso/api/src/lib.rs:91,123,174` merges them. They get instantiated for each concrete state, and the
   concrete states are `NodeApiStateImpl<Arc<ExtensibleDataSource<FetchingDataSource<..>>>>` (32 738 items),
   `NodeApiStateImpl<Arc<ExtensibleDataSource<MetricsDataSource<..>>>>` (2 670) and
   `NodeApiStateImpl<Arc<ApiState<CombinedNetworks<SeqTypes>, persistence::..>>>` (2 238). Every handler in every router
   drags its own axum `HandlerService` -> tower `MapFuture` -> `Instrumented` -> tokio task chain.
2. **Every distinct spawned future type instantiates the whole tokio task machinery.** There are 474 distinct
   `tokio::runtime::task::harness::Harness<F>` instantiations, and they account for 75 117 mono items (26.6 %) including
   14 034 `catch_unwind` shims. Many are already `Pin<Box<Instrumented<{async block}>>>` - boxed, but still a _distinct
   type per async block_, so boxing without erasing to `Pin<Box<dyn Future<Output = ()> + Send>>` buys nothing.

### Wall-clock model of the node lib unit (full `-Ztime-passes`, 32 cores, 127.5 s total)

```
frontend                            ~35 s   type_check 15.1 + borrowck 13.8 + coherence 5.0 + expand 0.8
monomorphization_collector_graph_walk 45.0 s  single-threaded, proportional to the 280 k mono items
generate_crate_metadata              59.8 s  span *contains* the collector walk + partitioning,
                                             so metadata encoding itself is ~12 s
codegen_to_LLVM_IR                   12.5 s
LLVM_passes / LLVM_thinlto / finish  ~35 s   parallel over 16 CGUs (184 s CPU)
link_rlib                             0.1 s
peak RSS                            8.1 GB
```

The single largest _wall-clock_ item is the monomorphization collector: 45 s in the lib, and the same walk runs again
for 42 s in the wrapper bin. Both are single-threaded and both scale with the number of mono items, which is why cutting
mono items is worth more than any flag.

Peak RSS of 8.1 GB for one rustc process is also worth noting: `rust-toolchain.toml` pins 1.97.1 because "on 1.98.0 the
slow-test and test-artifact jobs die with SIGTERM/143 while building". A 16 GB runner building with `-j4` cannot fit two
such units at once.

Metadata sizes (release, this workspace): `libespresso_node.rmeta` 17.2 MB, rlib 61 MB;
`libhotshot_contract_adapter.rmeta` **91.9 MB** (the checked-in `sol!` bindings, which every dependent decodes);
`libhotshot_task_impls.rmeta` 21.6 MB.

### `lto = "off"` (32 cores)

`[profile.release] lto = "off"` (default `false` means thin-local LTO across the crate's CGUs): node lib 124.7 s, node
bin 89.6 s - identical to baseline. A second run of the same configuration measured the lib at 147.5 s, so single-unit
measurements on this machine carry roughly +-20 % variance; only the pinned 4-core runs below are used for A/B
conclusions. The 88 s of LTO CPU is fully absorbed by parallelism on a 32-core box. Whether it helps on a 4-core runner
is measured in the CI-shaped runs below.

`cargo llvm-lines` for the **bin** target confirms it is the same work again: 12 259 366 IR lines over 245 508 copies
(lib: 13 755 485 / 279 862), with an identical per-crate distribution - hotshot-task-impls 1.05 M in both, espresso-api
0.50 M in both, aide 0.36 M in both, hotshot-query-service 0.64 M vs 0.90 M. The wrapper bin regenerates 89 % of the
lib's codegen.

## CI-shaped local runs (4 pinned cores, `-j4`, third-party deps warm, all workspace crates cold)

This is the A/B harness: `taskset -c 0-3 cargo build -j4 --release -p espresso-node --bin espresso-node` after
`cargo clean --release -p <each workspace crate>`, which reproduces the shape of the `Build espresso-node AMD` job
(restore-only cache from main).

| variant                                           | wall      | node lib    | node bin    | cpu-s   |
| ------------------------------------------------- | --------- | ----------- | ----------- | ------- |
| baseline (invalid: third-party deps also rebuilt) | 548 s     | 169.1 s     | 156.7 s     | 1221    |
| **baseline-2 (reference, clean, idle machine)**   | **385 s** | **160.5 s** | **145.8 s** | **560** |
| `lto = "off"`                                     | 393 s     | 189.6 s     | 121.9 s     | 519     |
| dep-cut (state-prover removed from the node only) | 385 s     | 159.4 s     | 146.9 s     | 560     |
| single storage call site in `run.rs`              | 489 s     | 194.7 s     | 193.9 s     | 719     |

Those three rows are **contaminated**: an implementation agent was compiling on cores 8-31 during part of the window,
and LLVM workloads share memory bandwidth regardless of core pinning (cpu-s per run drifted 519 -> 560 -> 719 for
near-identical work). A serialized clean A/B block (`baseline`, non-async entry point, full dep-cut, single
(persistence, query-module) combination, and the combination of the first two) is queued to run with an idle machine;
only those numbers will be used for conclusions.

Clean run, machine idle: **non-async entry point = 287 s wall / 460 cpu-s**, versus the 385 s / 560 cpu-s baseline-2 -
**-25 % of the whole job's wall clock on 4 cores**, and the bin unit disappears from the profile entirely (lib 205.6 s,
then espresso-types 28.8 s, contract-adapter 25.1 s). Scaled to the real job (748 s, bin 284 s) that is roughly 470 s,
about -37 %.

Also note the dep-cut row still built `hotshot-state-prover` (20.0 s): `cargo tree -i` shows it enters the release graph
twice, from `espresso-node` _and_ from `staking-cli`, so both edges have to be cut.

The first baseline row is not comparable: switching the profile back invalidated the third-party dependency artifacts,
so that run rebuilt them too (1221 cpu-s vs 519). A clean baseline with warm deps is queued as `baseline-2`; all A/B
conclusions use that one.

Real CI for the same job is 748 s / 303 s / 284 s, i.e. this machine's cores are ~1.8x faster than the runner's; the
_shape_ is the same (lib+bin = 60 % of wall here, 78 % in CI), so relative deltas carry over.

## `cargo llvm-lines` attribution for the node lib

Total 13 755 485 IR lines over 279 862 copies (llvm-lines counts function bodies, hence less than the 22.6 M raw `.ll`
lines). By defining crate:

| crate                 | IR lines |
| --------------------- | -------- |
| tokio                 | 1.87 M   |
| alloc                 | 1.57 M   |
| core                  | 1.45 M   |
| hotshot-task-impls    | 1.05 M   |
| espresso-node         | 0.96 M   |
| hotshot-query-service | 0.90 M   |
| espresso-api          | 0.50 M   |
| aide                  | 0.36 M   |
| axum                  | 0.36 M   |
| serde_json            | 0.28 M   |
| hotshot-types         | 0.26 M   |
| hotshot-new-protocol  | 0.26 M   |

Largest generic families (type arguments erased; copies in the second column):

```
0.52 M  x5969  alloc::collections::btree::node::{Handle,NodeRef} methods
0.25 M  x730   hotshot_query_service::data_source::fetching::Fetcher<SeqTypes, ..>
0.16 M  x1422  espresso_node::api::state::NodeApiStateImpl<Arc<hotshot_query_service..>>
0.16 M  x1036  hotshot_query_service::data_source::extension::ExtensibleDataSource<..>
0.40 M  ~2500  espresso_api::axum::router_{availability,node,catchup,..}<NodeApiStateImpl<..>>
               plus aide::axum::routing::get_with::<..> per route
0.40 M  x~800  hotshot_task_impls task states (DaTaskState, QuorumVoteTaskState,
               VoteDependencyHandle, UpgradeTaskState, ..), each instantiated exactly twice
0.11 M  x4220  tokio Harness<Pin<Box<Instrumented<..>>>> + tower MapFuture<MapIntoResponse<..>>
```

Two things stand out:

- Every `hotshot-task-impls` consensus task state appears **exactly twice**, once per `espresso_node::Node<N, P>`
  combination, i.e. the whole consensus stack is monomorphized once per persistence backend inside `espresso-node`.
- The router families exist once per `(state, query-module)` combination, and `aide`'s `get_with` doc-generation code is
  instantiated per route per state type.

`serve<N, P, F>` (`api/options.rs:159`) calls **both** `init_with_query_module_sql` (`:189`) and
`init_with_query_module_fs` (`:192`) behind a runtime `if`, so the 2 persistence backends x 2 query modules give 4
copies of the entire query/API stack. Deleting only the `run.rs` fs call site (2 -> 1 persistence) changed nothing
because `serve` still instantiates both query modules; the experiment that deletes both branches is queued.

## Is `SeqTypes` the problem? No.

`SeqTypes` is the type most code is generic over, but it has exactly one impl in a release build and therefore
multiplies nothing:

- 150 986 of 281 946 mono items (54 %) mention `SeqTypes`; 161 515 llvm-lines rows totalling 8.42 M of 13.76 M IR lines
  (61 %).
- `TestTypes` (the other `Versions`/`NodeType` impl) appears **0 times** in the release lib, so no second instantiation
  of the hotshot generic code exists there.
- The actual multipliers are the _other_ parameters: `espresso_node::Node<N, P>` (every hotshot-task-impls task state
  appears exactly twice, one copy per persistence backend) and the API state/data-source parameters (4 copies of the
  query/API stack).

What `TYPES` genericity does cost is trait solving, not codegen: `evaluate_obligation` 38.1 s +
`normalize_canonicalized_projection` 19.4 s + `codegen_select_candidate` 17.4 s ~= 74 s CPU (16 % of the lib), plus
metadata size. Removing the parameter would buy that 16 %, not the 71 % that is LLVM, so it is a much worse trade than
removing the N/P/D multiplication.

Open: in the `test` profile `TestTypes` is a second impl, so the same generic code may be instantiated twice across the
test graph. Measurement queued (test-profile mono-item dump).

## Coordination

This investigation touches Rust and `Cargo.toml` only. `.github/workflows/` is owned by the `ma/reduce-ci-test-time`
branch; no file under it is modified here.

## Fix 1, implemented: test-only deps of `espresso-node` made optional

Branch `ma/compile-times-depcut` (commit 63737de9757), 4 files, +10/-23:

- `crates/espresso/node/Cargo.toml`: `generic-tests` and `hotshot-state-prover` deleted (both unused); `rstest`,
  `rstest_reuse` moved to `[dev-dependencies]`; `hotshot-builder-refactored`, `staking-cli`, `test-utils` made
  `optional = true` and added to the `testing` feature.
- `staking-cli/Cargo.toml`: `hotshot-state-prover` made optional, enabled by staking-cli's own `testing` feature (its
  single use is `src/deploy.rs:36`, in a module gated at `src/lib.rs:48`).
- Workspace root `Cargo.toml`: unreferenced `generic-tests` entry removed. `Cargo.lock` shrinks.

No `cfg` gate needed changing: the affected code is all under `#[cfg(any(test, feature = "testing"))]`, and dev targets
still get it because `crates/espresso/node/Cargo.toml` has a self dev-dependency with `features = ["testing"]`.

Verified green: `cargo check` for `-p espresso-node --release --lib`, `-p espresso-node --lib --features testing`,
`-p espresso-node --tests`, `-p staking-cli --lib`, `-p staking-cli --lib --features testing`.
`cargo tree -p espresso-node -e normal -i hotshot-state-prover` now reports no match, i.e. the prover is out of the
release graph (it was reachable twice: directly and via staking-cli).

Unrelated crates that still depend on `hotshot-state-prover` directly and were left alone:
`crates/espresso/dev-node/Cargo.toml:23`, `crates/builder/Cargo.toml:27`, `tests/Cargo.toml:23`.

## Fix 2, measured: give the lib a non-async entry point (the big one)

Hypothesis: the wrapper bin re-codegens the whole async call graph because _it_ polls the lib's future. `Future::poll`
for an `async fn` body is a shim, and shims are always instantiated in the crate that needs them, so polling one future
from `main.rs` cascades into every future it awaits, transitively - which is the entire node.

Test (10 lines): move the runtime creation and `block_on` from `main.rs` into the lib.

```rust
// crates/espresso/node/src/run.rs
pub fn main_blocking(migrated_envs: Vec<(&str, &str)>) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(main(migrated_envs));
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    result
}
// crates/espresso/node/src/main.rs
espresso_node::main_blocking(migrated_envs)   // replaces Runtime::new + block_on
```

Result, 32-core local, warm deps:

|                       | node lib | node bin  | lib+bin wall |
| --------------------- | -------- | --------- | ------------ |
| baseline              | 123 s    | 90 s      | 216 s        |
| non-async entry point | 136.5 s  | **1.2 s** | **139 s**    |

The bin unit collapses from 90 s to 1.2 s. (The lib's 136.5 s vs 123 s is within this machine's +-20 % single-unit
noise.) In CI that unit is 284 s.

`crates/espresso/node-sqlite/src/main.rs` is the identical 8-line pattern (`rt.block_on(espresso_node::main(...))`), so
its 258 s bin unit should collapse the same way. `crates/espresso/dev-node/src/main.rs:255` blocks on a _local_
`async_main`, so it needs its own treatment: whatever lib futures it awaits still get their poll shims instantiated
there.

This is the cheapest large win found: ~280 s off the `Build espresso-node AMD` job and ~250 s off
`Build espresso-node-sqlite AMD`, for a 10-line change with no behavioural difference (the runtime is created and shut
down exactly as before, one stack frame deeper).

## Profile knobs on `espresso-node` (32-core local, lib / bin unit times)

| variant | node lib | node bin |
|---|---|---|
| baseline (release, opt-level 3, codegen-units 16) | 123 s | 90 s |
| `codegen-units = 64` | 119.7 s | 88.0 s |
| `codegen-units = 256` | 118.4 s | 98.3 s |
| `opt-level = 2` | 137.0 s | 97.2 s |
| `opt-level = 1` | 171.4 s | **16.1 s** |
| `lto = "off"` | 124.7 s | 89.6 s |
| `panic = "abort"` (whole build) | 118.3 s | 85.2 s |
| non-async entry point (fix 2) | 136.5 s | **1.2 s** |

No profile knob helps the lib. The `opt-level = 1` row confirms the share-generics mechanism from
the other side: at opt-level <= 1 rustc turns `-Cshare-generics` on, the bin reuses the lib's generic
instances and drops to 16 s - at the cost of an unoptimized node and a slower lib. Fix 2 gets a
better result (1.2 s) with no optimization change.

## Fix 3, implemented: erase spawned-future types

Branch `ma/compile-times-boxed-spawns` (commit 63ae02a). Rationale from the mono-item dump: 75 117 of
281 946 items (26.6 %) are `tokio::runtime::task` plumbing over 474 distinct `Harness<F>` types, and
each distinct spawned future type costs ~150 items. Origin of the async block in those items:

```
13 860  crates/espresso/node itself      5 060  hotshot_task
12 760  hotshot_query_service            4 620  hotshot_new_protocol
 9 020  axum                             4 180  hotshot_task_impls
 8 800  request_response                 3 080  hotshot
```

Boxing alone does not help - `Pin<Box<Instrumented<{async block}>>>` is still one type per block. The
change erases to a single trait object before `tokio::spawn` sees it, via three helpers
(`crates/espresso/node/src/util.rs:17,24`, `request-response/src/util.rs:12`,
`hotshot-query-service/src/task.rs:23`), each generic only over its argument:

```rust
fn spawn<T>(fut: impl Future<Output = T> + Send + 'static) -> JoinHandle<T> {
    tokio::spawn(Box::pin(fut) as Pin<Box<dyn Future<Output = T> + Send>>)
}
```

18 direct sites changed plus two funnels that cover ~20 more: `context.rs:825`
(`spawn_with_log_level!`, which all `TaskList::spawn` callers go through, boxed *after*
`.instrument(span)` so spans are preserved) and `hotshot-query-service/src/task.rs:95`
(`Task::spawn`, behind `BackgroundTask::spawn` and the fetching machinery). `cargo check` green for
`espresso-node --lib --release`, `request-response`, `hotshot-query-service`. Measurement queued
(wall time plus a like-for-like mono-item count against the 281 946 baseline).

## Fix 4, implemented: erase the API state generic in the routers

Branch `ma/compile-times-dyn-api` (commit de15c2c4218). All **15** `router_*<S>(state: S)` builders in
`crates/espresso/api/src/axum.rs` are now non-generic.

- New `crates/espresso/api/src/dyn_api.rs` (1029 lines): one object-safe mirror trait per `v1` trait
  (`DynRewardApi:52`, `DynAvailability:178`, `DynNodeApi`, `DynCatchupApi`, ... 14 in total), each
  with a blanket forwarding impl over the typed trait, plus state aliases
  (`dyn_api.rs:34-49`) that are all `Arc<dyn Dyn*Api>`.
- Response bodies are erased with `type Erased = Box<dyn erased_serde::Serialize + Send + Sync>`
  (`dyn_api.rs:27`), so the JSON and VBS bytes are unchanged; streams become
  `BoxStream<'static, Erased>`.
- `create_router_v1` (`axum.rs:3272`) keeps its generic bounds and wraps the state in an `Arc` once;
  `serve_axum*` in `crates/espresso/api/src/lib.rs:89,172,219,252` do the same, so **no caller in
  `crates/espresso/node` had to change**.
- Three methods could not be erased because their associated types are *inputs*
  (`v1::CatchupApi::FeeAccount`/`RewardAccountV1` at `v1/catchup.rs:8-10`, `v1::SubmitApi::Transaction`
  at `v1/submit.rs:8`); they now take `(&HeaderMap, &[u8])` and decode behind the object
  (`dyn_api.rs:711,783,833`) with the same 400/500 mapping. 11 associated types gained `+ 'static`.
- New dependency `erased-serde 0.4`.
- Green: `cargo check -p espresso-api`, `-p espresso-node --lib`, `-p espresso-node --lib --features
  testing`, plus `cargo check -p espresso-api --tests` and `cargo clippy -p espresso-api
  --all-targets`.

Measurement queued.

## Open items

- Local `-Ztime-passes` split (frontend vs LLVM vs link) for the node lib and the node bin.
- `cargo llvm-lines` for the node lib and bin: which generic instantiations dominate codegen.
- Measure `-Zshare-generics=y` in release.
- Measure codegen-units variations for the node lib.
- Measure removing test-only deps from `espresso-node` normal deps.
- Measure moving the 13 bins out of the `espresso-node` crate.
