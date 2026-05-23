#!/usr/bin/env bash
# Scratch benchmarking helper for `nix develop` eval optimization.
# DELETE BEFORE MERGE TO MAIN — paired with doc/nix-flake-eval-optimization.md.
#
# Usage:
#   scripts/nix-eval-bench.sh <label>            # default shell
#   scripts/nix-eval-bench.sh <label> <attr>     # custom devShell attr
#
# Examples:
#   scripts/nix-eval-bench.sh baseline
#   scripts/nix-eval-bench.sh dockerShellFix devShells.x86_64-linux.dockerShell
#
# Output: a Markdown table row on stdout + a one-line summary on stderr.

set -euo pipefail

label="${1:-untitled}"
attr="${2:-devShells.x86_64-linux.default}"

runs="${RUNS:-5}"
stats_path="${STATS_PATH:-/tmp/nix-bench-stats.json}"
hyperfine_export="${HYPERFINE_EXPORT:-/tmp/nix-bench-hyperfine.json}"

NIX_FLAGS=(--extra-experimental-features 'nix-command flakes')

cache_wipe='rm -rf "$HOME/.cache/nix/eval-cache-v"* 2>/dev/null || true'
eval_cmd="nix ${NIX_FLAGS[*]} eval --no-eval-cache --raw .#${attr}.outPath >/dev/null"

# 1) Cold hyperfine run.
nix "${NIX_FLAGS[@]}" shell nixpkgs#hyperfine nixpkgs#jq --command bash -c "
  set -euo pipefail
  hyperfine \
    --warmup 0 \
    --runs ${runs} \
    --prepare '${cache_wipe}' \
    --export-json '${hyperfine_export}' \
    '${eval_cmd}' >&2
" || { echo "hyperfine failed" >&2; exit 1; }

cold_median_ms=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '(.results[0].median * 1000) | floor' "${hyperfine_export}")
cold_min_ms=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '(.results[0].min * 1000) | floor' "${hyperfine_export}")
cold_max_ms=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '(.results[0].max * 1000) | floor' "${hyperfine_export}")

# 2) One stats capture (cache cleared first).
bash -c "${cache_wipe}"
NIX_SHOW_STATS=1 NIX_SHOW_STATS_PATH="${stats_path}" \
  nix "${NIX_FLAGS[@]}" eval --no-eval-cache --raw ".#${attr}.outPath" \
  >/dev/null 2>&1 || { echo "stats run failed" >&2; exit 1; }

cpu_time=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '.cpuTime' "${stats_path}")
values=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '.nrValues // .values // 0' "${stats_path}")
thunks=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '.nrThunks // .thunks // 0' "${stats_path}")
envs=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '.envs.number // 0' "${stats_path}")

# 3) Warm hyperfine run (no cache wipe, eval cache enabled).
nix "${NIX_FLAGS[@]}" shell nixpkgs#hyperfine --command bash -c "
  set -euo pipefail
  hyperfine \
    --warmup 1 \
    --runs ${runs} \
    --export-json '${hyperfine_export}.warm' \
    'nix ${NIX_FLAGS[*]} eval --raw .#${attr}.outPath >/dev/null' >&2
" || { echo "warm hyperfine failed" >&2; exit 1; }

warm_median_ms=$(nix "${NIX_FLAGS[@]}" shell nixpkgs#jq --command jq -r \
  '(.results[0].median * 1000) | floor' "${hyperfine_export}.warm")

# 4) Output a markdown row.
printf '| %s | %s | %s | %s | %s (min %s / max %s) | %s | %s | %s | %s |\n' \
  "${label}" "${attr}" "$(date -u +%FT%TZ)" \
  "${warm_median_ms}" "${cold_median_ms}" "${cold_min_ms}" "${cold_max_ms}" \
  "${cpu_time}" "${values}" "${thunks}" "${envs}"

echo "→ ${label} (${attr}): cold ${cold_median_ms} ms / warm ${warm_median_ms} ms / cpuTime ${cpu_time}s / values ${values}" >&2
