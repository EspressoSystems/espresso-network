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
| F   | **final / cumulative**                  | f028be7 | devShells.x86_64-linux.default         | 2026-05-23T19:23:08Z | 4044             | **3880 (min 3829 / max 4328)** | **2.84**    | **6 703 046** | **4 223 714** | **2 857 057** |

### Why row 8 is skipped

`flake-utils.lib.eachDefaultSystem` wraps the outputs as
`{ devShells.x86_64-linux = …; devShells.aarch64-linux = …; … }` lazily.
Asking for `.#devShells.x86_64-linux.default` only forces the matching
system attribute, so the other three systems contribute zero to the eval
graph for that query. Narrowing the input list to `eachSystem [ … ]` only
helps `nix flake show` / `nix flake check`, which is out of scope for the
`nix develop` cold-eval target we're optimizing.

## Decisions

**Net change vs baseline (row 0 → row F):**

| Metric    | Baseline   | Final      | Δ           |
| --------- | ---------- | ---------- | ----------- |
| Cold (ms) | 4 492      | 3 880      | **−612 (−13.6 %)** |
| Warm (ms) | 4 558      | 4 044      | −514 (−11.3 %)   |
| cpuTime   | 3.20 s     | 2.84 s     | −0.36 s (−11.3 %) |
| values    | 8 396 912  | 6 703 046  | **−1 693 866 (−20.2 %)** |
| thunks    | 5 153 389  | 4 223 714  | −929 675 (−18.0 %)     |
| envs      | 3 546 013  | 2 857 057  | −688 956 (−19.4 %)     |

**Single biggest contributor:** decoupling the default devShell from
`self.checks.${system}.pre-commit-check` (row 4). Hooks now opt-in via
`nix develop .#preCommit`. Everything else is sub-noise.

**Keep:**

- Row 1 — explicit `pkgs.lib`/`pkgs.stdenv` instead of wide outer `with pkgs;`
  (no perf, kept for explicitness).
- Row 3 — `dockerShell.shellHook` fix (correctness bug — was concatenating
  the default-shell *derivation* into the hook string).
- Row 4 — pre-commit decoupling + new `devShells.preCommit` shell. **The
  win.**
- Row 5 — local packages (`solhint`, `pup`, `golangci-lint`,
  `prek-as-pre-commit`) moved from overlays to `let` (hygiene, not perf).
- Row 6 — pinned nightly toolchain instead of `selectLatestNightlyWith`.
- Row 7 — `dregs.overlays.default` replaced with direct
  `dregs.packages.${system}.unwrapped` reference.
- Row 9 — `writeShellScriptBin` for the `prek-as-pre-commit` wrapper
  instead of `runCommand` + symlink.

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

**Cumulative `nix develop` cold-eval improvement: ~13–14 %, ~20 % fewer
allocated values.** Almost all of that comes from a single architectural
change (row 4). The other kept changes are hygiene / correctness wins.
