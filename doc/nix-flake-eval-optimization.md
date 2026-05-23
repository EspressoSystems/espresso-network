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
| 10  | **`dregs` follows our nixpkgs**         | 8517c0a | devShells.x86_64-linux.default         | 2026-05-23T19:42:28Z | 3803             | **3777 (min 3570 / max 3809)** | **2.61**    | **6 592 690** | **4 167 751** | **2 816 961** |
| FL  | _Floor_ — minimal rust-only flake (a)   | —       | devShells.x86_64-linux.default (b)     | 2026-05-23T19:46Z    | —                | **706 (min 687 / max 741)** | **0.36**    | **873 301** | **314 065**   | **158 113** |

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

**Net change vs baseline (row 0 → row 10):**

| Metric    | Baseline   | Current    | Floor      | Δ vs baseline   | % of optimizable gap closed |
| --------- | ---------- | ---------- | ---------- | --------------- | --------------------------- |
| Cold (ms) | 4 492      | **3 777**  | 706        | **−715 (−15.9 %)** | 19 %                     |
| Warm (ms) | 4 558      | 3 803      | —          | −755 (−16.6 %)  | —                            |
| cpuTime   | 3.20 s     | 2.61 s     | 0.36 s     | −0.59 s (−18.4 %) | 21 %                       |
| values    | 8 396 912  | 6 592 690  | 873 301    | **−1 804 222 (−21.5 %)** | 24 %                |
| thunks    | 5 153 389  | 4 167 751  | 314 065    | −985 638 (−19.1 %) | 20 %                      |
| envs      | 3 546 013  | 2 816 961  | 158 113    | −729 052 (−20.6 %) | 22 %                      |

Floor reference: a minimal flake with `nixpkgs` + `rust-overlay` + a single
devShell containing `pkg-config`, `openssl`, and the full stable Rust
toolchain. "Optimizable gap" = `current − floor` vs `baseline − floor`.

**Two contributors do almost all the work:**

1. Row 4 — pre-commit decoupling (≈−14 % cold, ≈−20 % values on its own).
2. Row 10 — `dregs` follows our nixpkgs (≈−5 % cold, eliminates a whole
   second nixpkgs source tree). Surfaced by `--trace-function-calls`
   profiling.

Everything else is rounding noise.

**Keep:**

- Row 1 — explicit `pkgs.lib`/`pkgs.stdenv` instead of wide outer `with pkgs;`
  (no perf, kept for explicitness).
- Row 3 — `dockerShell.shellHook` fix (correctness bug — was concatenating
  the default-shell *derivation* into the hook string).
- Row 4 — pre-commit decoupling + new `devShells.preCommit` shell.
- Row 5 — local packages (`solhint`, `pup`, `golangci-lint`,
  `prek-as-pre-commit`) moved from overlays to `let` (hygiene, not perf).
- Row 6 — pinned nightly toolchain instead of `selectLatestNightlyWith`.
- Row 7 — `dregs.overlays.default` replaced with direct
  `dregs.packages.${system}.unwrapped` reference.
- Row 9 — `writeShellScriptBin` for the `prek-as-pre-commit` wrapper
  instead of `runCommand` + symlink.
- Row 10 — `inputs.dregs.inputs.nixpkgs.follows = "nixpkgs";` — kills a
  duplicate nixpkgs tree found via profiling.
- (Also includes the CI fix in `.github/workflows/contracts.yml` to use
  `.#preCommit` for `pre-commit run` invocations, since the default shell
  no longer auto-installs hooks.)

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

**Cumulative `nix develop` cold-eval improvement: ~16 %, ~21 % fewer
allocated values.** We've closed roughly a fifth of the gap to a
minimal-rust-shell floor (`3 777 − 706 = 3 071 ms` still in espresso-
specific tooling). Further reductions require shrinking the package list
in the default shell — see "Next steps" below.

## Next steps (not yet attempted)

The profile (~85K calls into `make-derivation.nix`) and the floor
benchmark (706 ms with a small package list) both point to the same
remaining lever: **the package list in `devShells.default`**. Concrete
candidates for a follow-up round:

- Figures / docs: `graphviz`, `plantuml`, `mdbook` — used only when
  editing diagrams or building docs locally.
- Optional UX: `lazydocker`, `entr`, `coreutils`, `bc`, `libusb1` — small
  individually, but they each pull a derivation graph.
- Python tooling: `python3`, `ruff`, `ty` — used by some scripts; could
  move to a `python` devShell or rely on the user's profile.
- Go tooling beyond `abigen`: `go`, `golangci-lint` — both can move into
  a `go` devShell since most contributors aren't touching Go daily.

Each one is a measurable experiment that didn't fit in this pass.
