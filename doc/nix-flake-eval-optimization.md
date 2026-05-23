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
| 4   | decouple default shell from pre-commit  | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 5   | de-overlay solhint/pup/golangci         | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 6   | pin nightly toolchain                   | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 7   | coalesce remaining overlays             | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 8   | narrow systems (`eachSystem`)           | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 9   | replace `prek-as-pre-commit` runCommand | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |

## Decisions

_(filled in at the end)_

- **Keep:**
- **Drop:**
- **Net change vs baseline:**
