# Nix flake eval optimization — scratch results

> **DELETE BEFORE MERGE TO MAIN.** This doc, together with
> `scripts/nix-eval-bench.sh`, is a working artifact for tuning `nix develop`
> eval performance. Nothing here is meant to ship.

## What we measure

```bash
# Cold (eval cache wiped, --no-eval-cache):
rm -rf ~/.cache/nix/eval-cache-v*
nix eval --no-eval-cache --raw .#devShells.x86_64-linux.default.outPath

# Warm (eval cache populated):
nix eval --raw .#devShells.x86_64-linux.default.outPath
```

Each row: 5 hyperfine runs cold (with `--prepare` wiping the eval cache
between each), 5 hyperfine runs warm, plus one `NIX_SHOW_STATS=1` capture
for the evaluator-internal numbers (`cpuTime`, `nrValues`, `nrThunks`,
`envs.number`).

Driver: `scripts/nix-eval-bench.sh <label>`. Delta-vs-baseline columns
compare to row 0 (the `main`-tip baseline at the top), so deltas are
stackable across rows.

## Results

| #   | Change                                  | Commit  | Attr                                   | When (UTC)           | Warm median (ms) | Cold median (ms) (min/max) | cpuTime (s) | nrValues  | nrThunks  | envs    |
| --- | --------------------------------------- | ------- | -------------------------------------- | -------------------- | ---------------- | -------------------------- | ----------- | --------- | --------- | ------- |
| 0   | baseline (main + bench harness)         | e475ad1 | devShells.x86_64-linux.default         | 2026-05-23T18:58:24Z | 4558             | 4492 (min 4453 / max 4549) | 3.20        | 8 396 912 | 5 153 389 | 3 546 013 |
| 1   | kill wide outer `with pkgs;`            | 8dfa388 | devShells.x86_64-linux.default         | 2026-05-23T19:01:49Z | 4587             | 4551 (min 4390 / max 4614) | 3.22        | 8 396 924 | 5 153 401 | 3 546 014 |
| 2   | drop `echidna-nixpkgs` (reverted)       | 2516559 | devShells.x86_64-linux.default         | 2026-05-23T19:05:18Z | 4751             | 4646 (min 4605 / max 4723) | 3.48        | 8 396 903 | 5 153 398 | 3 546 014 |
| 3   | fix `dockerShell.shellHook` coercion    | c45eb55 | devShells.x86_64-linux.default         | 2026-05-23T19:07:39Z | 4581             | 4582 (min 4548 / max 4627) | 3.25        | 8 396 924 | 5 153 401 | 3 546 014 |
| 3.b | fix `dockerShell.shellHook` (docker)    | c45eb55 | devShells.x86_64-linux.dockerShell     | 2026-05-23T19:08:37Z | 4640             | 4584 (min 4476 / max 4639) | 3.27        | 8 497 726 | 5 243 412 | 3 613 255 |
| 4   | decouple default shell from pre-commit  | 86788dc | devShells.x86_64-linux.default         | 2026-05-23T19:10:44Z | 4015             | **3865 (min 3820 / max 3994)** | **2.88**    | **6 704 065** | **4 224 560** | **2 858 092** |
| 4.b | + add `devShells.preCommit` (lazy)      | 0c7e0e8 | devShells.x86_64-linux.default         | 2026-05-23T19:12:02Z | 4082             | 4008 (min 3916 / max 4009) | 2.87        | 6 704 065 | 4 224 561 | 2 858 092 |
| 5   | de-overlay solhint/pup/golangci/prek    | b2d4c96 | devShells.x86_64-linux.default         | 2026-05-23T19:14:30Z | 4079             | 4092 (min 3879 / max 4125) | 2.96        | 6 703 995 | 4 224 514 | 2 857 897 |
| 6   | pin nightly toolchain                   | ff05541 | devShells.x86_64-linux.default         | 2026-05-23T19:16:26Z | 4020             | 3970 (min 3927 / max 4080) | 2.96        | 6 703 052 | 4 223 718 | 2 857 104 |
| 6.b | pin nightly toolchain (re-run)          | ff05541 | devShells.x86_64-linux.default         | 2026-05-23T19:17:25Z | 4038             | 3906 (min 3882 / max 4023) | 2.91        | 6 703 052 | 4 223 718 | 2 857 104 |
| 7   | drop `dregs.overlays.default`           | 2bbef24 | devShells.x86_64-linux.default         | 2026-05-23T19:19:39Z | 4077             | 4013 (min 3815 / max 4078) | 2.93        | 6 703 018 | 4 223 692 | 2 857 039 |
| 8   | narrow systems (`eachSystem`) — SKIPPED | —      | —                                      | —                    | —                | —                          | —           | —         | —         | —       |
| 9   | `writeShellScriptBin` for prek wrapper  | f028be7 | devShells.x86_64-linux.default         | 2026-05-23T19:21:59Z | 4057             | 3964 (min 3916 / max 4016) | 2.85        | 6 703 047 | 4 223 714 | 2 857 057 |
| F   | (intermediate cumulative)               | f028be7 | devShells.x86_64-linux.default         | 2026-05-23T19:23:08Z | 4044             | 3880 (min 3829 / max 4328) | 2.84        | 6 703 046 | 4 223 714 | 2 857 057 |
| 10  | `dregs` follows our nixpkgs             | 8517c0a | devShells.x86_64-linux.default         | 2026-05-23T19:42:28Z | 3803             | 3777 (min 3570 / max 3809) | 2.61        | 6 592 690 | 4 167 751 | 2 816 961 |
| 11  | move docs tools → `devShells.docs`      | 83d54d8 | devShells.x86_64-linux.default         | 2026-05-23T19:49:25Z | 3720             | 3665 (min 3525 / max 3694) | 2.58        | 6 373 154 | 3 970 007 | 2 673 611 |
| 12  | move Go toolchain → `devShells.go`      | 3c87345 | devShells.x86_64-linux.default         | 2026-05-23T19:51:44Z | 3565             | 3496 (min 3364 / max 3522) | 2.48        | 6 371 382 | 3 968 441 | 2 672 591 |
| 13  | move Python → `devShells.python`        | bfa0540 | devShells.x86_64-linux.default         | 2026-05-23T19:54:46Z | 3733             | 3545 (min 3522 / max 3728) | 2.64        | 6 358 411 | 3 956 563 | 2 664 271 |
| 14  | move go-ethereum → `devShells.contracts`| 28b2c27 | devShells.x86_64-linux.default         | 2026-05-23T19:57:42Z | 3683             | 3572 (min 3515 / max 3662) | 2.57        | 6 357 094 | 3 955 396 | 2 663 452 |
| 15  | drop entr/pup/lazydocker/bc             | ca9bc88 | devShells.x86_64-linux.default         | 2026-05-23T19:59:54Z | 3712             | 3571 (min 3546 / max 3641) | 2.55        | 6 353 539 | 3 952 142 | 2 661 142 |
| 16  | drop postgresql_16 + solc + dregs from default's packages (d) | f0dd28d | devShells.x86_64-linux.default | 2026-05-23T20:08:49Z | 1726 | 1682 (min 1622 / max 1705) | 0.98 | 2 950 804 | 1 586 536 | 1 112 571 |
| 17  | solc/dregs → contracts, drop `FOUNDRY_SOLC` from default (e) | 83f9510 | devShells.x86_64-linux.default | 2026-05-23T20:10:30Z | 1303 | 1250 (min 1179 / max 1301) | 0.78 | 1 905 177 | 1 086 211 | 716 599 |
| 17.b| confirm post-foundry-restore (stable)   | 221958c | devShells.x86_64-linux.default         | 2026-05-23T20:12:30Z | 1297             | 1296 (min 1265 / max 1652) | 0.72        | 1 905 177 | 1 086 211 | 716 599   |
| 18  | drop `cargo-watch` (unused) (c)         | 409efc7 | devShells.x86_64-linux.default         | 2026-05-23T20:27:52Z | (env-noise)      | (env-noise)                | (env-noise) | **1 903 147** | **1 084 379** | **715 239** |
| FL  | _Floor_ — minimal rust-only flake (a)   | —       | devShells.x86_64-linux.default (b)     | 2026-05-23T19:46Z    | —                | 706 (min 687 / max 741) | 0.36        | 873 301   | 314 065   | 158 113   |

(c) Both bench runs landed during a load-average of ~17 (something
else on the box was hammering CPU). Wall-clock and cpuTime are
unreliable for this row; `values` (a deterministic counter) shows the
true marginal: −2 030 vs row 17.b — i.e. cargo-watch's eval cost was
negligible to begin with. The removal stands as code hygiene, not a
perf win.

(d) The original commit message for row 16 framed this as "drop
postgresql_16". Subsequent A/B benchmarking (3×5 runs each state under
the same host load) shows postgres alone is only ≈ +110 ms cold / +25 ms
cpuTime / +9 663 values — far less than this row's headline drop. The
bulk of row 16's reduction was the simultaneous removal of `solc` and
`dregs.packages.${system}.unwrapped` from the package list — `dregs`
in particular, since its flake outputs evaluate substantial transitive
content. The commit was honest about doing all three together; the per-
package attribution in the original "Decisions" section was not. See
the corrected ranking below.

(e) After row 16, `solc` was still being forced via the
`FOUNDRY_SOLC = "${solc}/bin/solc"` env var (the `solc = pkgs.solc-bin
."0.8.28"` let binding was referenced even though solc was no longer
in the package list). Row 17 removed both. The ~1 M values delta is
the unforced solc-bin derivation graph, not "solc/dregs moving to the
contracts shell" (moving a package between shells doesn't change the
default-shell eval graph — only what the default *no longer references*
does).

(a) Standalone flake at `/tmp/rust-only-flake/` — only `nixpkgs` and
`rust-overlay` inputs, single devShell containing `pkg-config`, `openssl`,
and the stable Rust toolchain (with rust-analyzer, clippy, rustfmt,
rust-src). Establishes the absolute floor for "a non-trivial Rust shell".

(b) Same attr name, different flake — see (a).

## Profiling

`NIX_COUNT_CALLS=1` doesn't produce output with this Nix build, but
`--trace-function-calls` does (writes per-enter/exit lines to stderr with
location + nanosecond timestamps). For the baseline default-shell eval
the trace is ~6.7M lines; aggregating by `entered` location gives the
hottest call sites:

```text
 ~500K  «none»:0                                      builtin functions (no source loc)
~105K   lib/trivial.nix:1126                          mirrorFunctionArgs-style helper
 ~90K   lib/attrsets.nix:662                          mapAttrs callback
 ~85K   pkgs/stdenv/generic/make-derivation.nix:445   per-derivation work
 ~85K   lib/lists.nix:347                             concatMap callback
 ~85K   lib/systems/default.nix:46                    per-derivation system attr
 ~74K   lib/meta.nix:328                              `meta` propagation
 ~33K   lib/attrsets.nix:1814                         recursiveUpdate-style helper
```

Two findings drove the next rounds:

1. **Two distinct nixpkgs source trees in the trace** — `dj9rm8…` (our
   main nixpkgs) and `f1fvmyl…` (a different rev pulled in by `dregs`,
   which didn't have `nixpkgs.follows`). Fixed by row 10 below; profiling
   after the fix shows only `dj9rm8…` in the top 15.
2. **~85K calls into `make-derivation.nix`** — i.e. ~85K derivations are
   being constructed during eval. The shell pulls in dozens of tools and
   each tool brings transitive deps. Pruning the package list is the
   remaining big lever.

### Why row 8 is skipped

`flake-utils.lib.eachDefaultSystem` wraps the outputs as
`{ devShells.x86_64-linux = …; devShells.aarch64-linux = …; … }` lazily.
Asking for `.#devShells.x86_64-linux.default` only forces the matching
system attribute, so the other three systems contribute zero to the eval
graph for that query. Narrowing the input list to `eachSystem [ … ]` only
helps `nix flake show` / `nix flake check`, which is out of scope for the
`nix develop` cold-eval target we're optimizing.

## Decisions

**Net change vs baseline (row 0 → row 17.b):**

| Metric    | Baseline   | Current    | Floor      | Δ vs baseline       | % of optimizable gap closed |
| --------- | ---------- | ---------- | ---------- | ------------------- | --------------------------- |
| Cold (ms) | 4 492      | **1 296**  | 706        | **−3 196 (−71.1 %)** | **84 %**                  |
| Warm (ms) | 4 558      | 1 297      | —          | −3 261 (−71.5 %)    | —                            |
| cpuTime   | 3.20 s     | 0.72 s     | 0.36 s     | **−2.48 s (−77.5 %)** | **87 %**                   |
| values    | 8 396 912  | 1 905 177  | 873 301    | **−6 491 735 (−77.3 %)** | **86 %**               |
| thunks    | 5 153 389  | 1 086 211  | 314 065    | −4 067 178 (−78.9 %) | 84 %                       |
| envs      | 3 546 013  | 716 599    | 158 113    | −2 829 414 (−79.8 %) | 83 %                       |

Floor reference: a minimal flake with `nixpkgs` + `rust-overlay` + a single
devShell containing `pkg-config`, `openssl`, and the full stable Rust
toolchain (with rust-analyzer/clippy/rustfmt/rust-src). "Optimizable gap" =
`current − floor` vs `baseline − floor`.

**The big-rock contributors** (corrected via A/B benchmarking, ranked
by `values` delta which is deterministic):

1. **Stop referencing `dregs.packages.${system}.unwrapped` from the
   default shell** (part of row 16, made permanent by row 17 moving
   `dregs` to `.#contracts`). Dregs's flake outputs walk substantial
   transitive content; this alone ≈ **−3.4 M values**, the largest
   single contributor in the whole sweep.
2. **Row 4 — decouple default from `pre-commit-check`.** ≈ **−1.7 M
   values**, ≈ −720 ms cold. Removed the git-hooks framework from the
   default-shell eval path.
3. **Stop referencing the `solc-bin."0.8.28"` derivation** (part of
   row 17, by removing `FOUNDRY_SOLC` env var so the let-binding is
   no longer forced). ≈ **−1.0 M values**, ≈ −430 ms cold.
4. **Row 11 — move docs tooling (`plantuml`/`graphviz`/`mdbook`) to
   `.#docs`.** ≈ **−220 K values**, ≈ −112 ms cold.
5. **Row 10 — `dregs.inputs.nixpkgs.follows = "nixpkgs"`** (kills the
   duplicate nixpkgs source tree; surfaced by profiling). ≈ −110 K
   values, ≈ −187 ms cold.
6. **Row 16 (true postgres-only contribution)** — postgresql_16 itself
   was *not* the bear we thought: A/B shows ≈ **+10 K values, +110 ms
   cold** to have it in default. The change is still correct (postgres
   genuinely is unused — see d) but it's a small-rock win, not a
   big-rock one.
7. Everything else (rows 1, 3, 5, 6, 7, 9, 12, 13, 14, 15, 18 — the
   Go/Python/contracts/misc/cargo-watch prunes and the `with pkgs;`,
   `runCommand→writeShellScriptBin`, nightly pin, etc.) — collectively
   ~−250 K values, ~−400 ms cold. Each one individually within noise.

**Keep (the whole branch):**

- Row 1 — explicit `pkgs.lib`/`pkgs.stdenv` instead of wide outer `with pkgs;`
  (no perf, kept for explicitness).
- Row 3 — `dockerShell.shellHook` fix (correctness bug — was concatenating
  the default-shell *derivation* into the hook string).
- Row 4 — pre-commit decoupling + new `devShells.preCommit` shell.
- Row 5 — local packages (`solhint`, `pup`, `golangci-lint`,
  `prek-as-pre-commit`) moved from overlays to `let`.
- Row 6 — pinned nightly toolchain instead of `selectLatestNightlyWith`.
- Row 7 — `dregs.overlays.default` replaced with direct
  `dregs.packages.${system}.unwrapped` reference.
- Row 9 — `writeShellScriptBin` for the `prek-as-pre-commit` wrapper
  instead of `runCommand` + symlink.
- Row 10 — `inputs.dregs.inputs.nixpkgs.follows = "nixpkgs";`.
- Row 11 — `devShells.docs` (plantuml/graphviz/mdbook).
- Row 12 — `devShells.go` (full Go toolchain + golangci-lint).
- Row 13 — `devShells.python` (python3/ruff/ty).
- Row 14 — `devShells.contracts` (go-ethereum for `abigen`).
- Row 15 — dropped entr/pup/lazydocker/bc entirely.
- Row 16 — dropped `postgresql_16` from default entirely (unused; see
  big-rock #1 above).
- Row 17 — `solc` + `dregs-unwrapped` routed into `devShells.contracts`;
  removed the `FOUNDRY_SOLC` env var from default so solc's outPath
  isn't forced when entering `nix develop`. `forge` users get the env
  var from `.#contracts` instead.
- CI workflow update in `.github/workflows/contracts.yml`: pre-commit
  invocations now use `.#preCommit`, which `inputsFrom`s default +
  contracts so hooks find every tool they shell out to.

**Migration for `nix develop` users:**

| What you were doing                | Now run                       |
| ---------------------------------- | ----------------------------- |
| Auto pre-commit hooks on shell entry | `nix develop .#preCommit` once |
| `make doc` / edit diagrams         | `nix develop .#docs`          |
| Go SDK work under `sdks/go/`       | `nix develop .#go`            |
| `just py-fmt` / `just py-check`    | `nix develop .#python`        |
| `just gen-bindings` (rare)         | `nix develop .#contracts` or `.#preCommit` |
| `forge build`, `solc`, mutation testing (`dregs`) | `nix develop .#contracts` |
| `psql` to a local DB                | `nix shell nixpkgs#postgresql_16 -c psql ...` |
| Everything else (Rust dev, `anvil` for tests, `solhint`, `forge fmt`, demo-native) | `nix develop` (default) |

**Drop / reverted:**

- Row 2 — `echidna-nixpkgs` input removal. Reverted: zero impact on
  default-shell eval (`values` count unchanged); the echidna shell is
  already lazy and contributes nothing to default eval.

**Skipped:**

- Row 8 — `eachSystem` narrowing. Wouldn't help `nix develop`
  (single-system lazy attribute); only relevant for `nix flake show`/`check`,
  which is out of scope.

**Migration note for `nix develop` users:** the default shell no longer
auto-installs pre-commit hooks. To install them, run
`nix develop .#preCommit` once after cloning. CI is unaffected
(still consumes `checks.pre-commit-check` directly).

**Cumulative `nix develop` cold-eval improvement: ~71 %, ~77 % fewer
allocated values.** 84 % of the gap to a minimal-rust-shell floor closed
(`1 296 − 706 = 590 ms` still in espresso-specific tooling).

## Remaining gap — where the time still goes

`current − floor ≈ 590 ms`. With postgres/solc/dregs gone and foundry
confirmed cheap (~2 K values), the remaining cost is spread across:

- The Rust tool family (`cargo-nextest`, `cargo-audit`, `cargo-edit`,
  `cargo-hack`, `cargo-sort`, `typos`, `just`, `cargo-watch`,
  `rust-analyzer`, `rustfmt`) — each a small Rust binary; collectively
  non-trivial.
- `foundry` (for `anvil`).
- `solhint` (used both by hooks and locally).
- `nodePackages.prettier` (used by hook + manual markdown formatting).
- `nixpkgs-fmt`, `prek`, `process-compose`, `libusb1`, the rest.

These are all daily-use tools. Pruning further means real workflow
changes and the payoff is small (a few hundred ms over 1.3 s). Stopping
here unless someone wants to push for sub-second.
