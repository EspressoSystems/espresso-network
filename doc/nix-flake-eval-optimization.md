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

| #   | Change                                  | Commit | Attr                                   | When (UTC) | Warm median (ms) | Cold median (ms) (min/max) | cpuTime (s) | nrValues  | nrThunks  | envs |
| --- | --------------------------------------- | ------ | -------------------------------------- | ---------- | ---------------- | -------------------------- | ----------- | --------- | --------- | ---- |
| 0   | baseline (main)                         | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 1   | kill `with pkgs;`                       | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 2   | drop `echidna-nixpkgs`                  | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 3   | fix `dockerShell.shellHook` coercion    | TBD    | devShells.x86_64-linux.default         |            |                  |                            |             |           |           |      |
| 3.b | fix `dockerShell.shellHook` (docker)    | TBD    | devShells.x86_64-linux.dockerShell     |            |                  |                            |             |           |           |      |
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
