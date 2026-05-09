#!/usr/bin/env bash
# Helpers for the binary upgrade test driver. Sourced by run.sh.

# BASE_DIR is set by run.sh to a freshly created mktemp -d directory.
# It holds the docker-compose.yaml and .env extracted from BASE_TAG.

log() {
  printf '[%s] %s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$*" >&2
}

err() {
  printf '[%s] ERROR: %s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$*" >&2
}

cleanup() {
  if [[ "${KEEP_RUNNING:-0}" == "1" ]]; then
    log "KEEP_RUNNING=1, leaving compose stack up at ${BASE_DIR:-?}"
    return 0
  fi
  log "Tearing down compose stack"
  compose down -v >/dev/null 2>&1 || true
  if [[ -n "${BASE_DIR:-}" && -d "${BASE_DIR}" ]]; then
    rm -rf "${BASE_DIR}"
  fi
}

# Extract BASE_TAG-era docker-compose.yaml and .env into $BASE_DIR.
# These files use the old ESPRESSO_SEQUENCER_* env names, which the old binary
# reads natively and the new binary maps to current names via env_compat.rs.
extract_base_files() {
  local base_tag="$1"
  git show "${base_tag}:docker-compose.yaml" >"${BASE_DIR}/docker-compose.yaml"
  git show "${base_tag}:.env" >"${BASE_DIR}/.env"
}

# Wrapper that targets the extracted base compose file plus a persistent-
# storage override for nodes 2/3/4 (otherwise they cannot rejoin after a roll).
# --project-directory pins the project root so relative volume mounts in the
# extracted compose (e.g. ./geth-config) resolve against the actual repo.
compose() {
  docker compose \
    --project-directory . \
    --env-file "${BASE_DIR}/.env" \
    -f "${BASE_DIR}/docker-compose.yaml" \
    -f binary-upgrade-tests/compose.persist-storage.yaml \
    "$@"
}

# get_block_height <api_url> -> integer on stdout (empty on non-numeric/failure)
get_block_height() {
  local api_url="$1"
  local out
  out="$(curl -sL --max-time 5 "${api_url}/node/block-height" 2>/dev/null || true)"
  if [[ "${out}" =~ ^[0-9]+$ ]]; then
    printf '%s' "${out}"
  fi
}

# wait_for_height_advance <api_url> <delta> <timeout_seconds>
wait_for_height_advance() {
  local api_url="$1"
  local delta="$2"
  local timeout_seconds="$3"

  local initial=""
  local current
  local start elapsed
  start="${SECONDS}"

  while [[ -z "${initial}" ]]; do
    initial="$(get_block_height "${api_url}")"
    elapsed=$((SECONDS - start))
    if ((elapsed > timeout_seconds)); then
      err "Could not read initial height from ${api_url} within ${timeout_seconds}s"
      return 1
    fi
    [[ -z "${initial}" ]] && sleep 2
  done

  log "Initial height at ${api_url}: ${initial}; waiting for advance by ${delta}"
  while true; do
    current="$(get_block_height "${api_url}")"
    if [[ -n "${current}" ]] && ((current >= initial + delta)); then
      log "Height advanced from ${initial} to ${current} at ${api_url}"
      return 0
    fi
    elapsed=$((SECONDS - start))
    if ((elapsed > timeout_seconds)); then
      err "Timed out after ${timeout_seconds}s waiting for height to advance from ${initial} (last: ${current:-unknown}) at ${api_url}"
      return 1
    fi
    sleep 2
  done
}

# Look up host API port for espresso-node-N from the project .env (current names).
node_api_port() {
  local n="$1"
  local var="ESPRESSO_NODE_${n}_API_PORT"
  printf '%s' "${!var:-}"
}

# assert_service_image <service> <expected_tag>
# Asserts the given compose service's running container is on
# `<image>:<expected_tag>` (matched as a suffix on .Config.Image).
assert_service_image() {
  local service="$1"
  local expected_tag="$2"
  local cid actual
  cid="$(compose ps -q "${service}")"
  if [[ -z "${cid}" ]]; then
    err "service ${service} has no running container"
    return 1
  fi
  actual="$(docker inspect "${cid}" --format='{{.Config.Image}}')"
  if [[ "${actual}" != *":${expected_tag}" ]]; then
    err "service ${service} image is ${actual}, expected tag ${expected_tag}"
    return 1
  fi
  log "service ${service} image: ${actual}"
}

# Pick a stable monitor node to poll for liveness during a roll.
# Nodes 0, 1, 3, 4 have the `query` module; node 2 does not. Use node-1 by
# default; when rolling node-1, use node-0 instead.
monitor_port_for_roll() {
  local rolling_n="$1"
  local monitor_n=1
  if [[ "${rolling_n}" == "1" ]]; then monitor_n=0; fi
  node_api_port "${monitor_n}"
}

# roll_node <n> <upgrade_tag>
# Recreate just espresso-node-N at the upgrade tag, leaving other services
# untouched. Wait for liveness via block-height advance from a stable monitor
# node (not the one being rolled).
roll_node() {
  local n="$1"
  local upgrade_tag="$2"
  local port
  port="$(monitor_port_for_roll "${n}")"
  if [[ -z "${port}" ]]; then
    err "Could not resolve monitor API port while rolling espresso-node-${n}"
    return 1
  fi

  log "Recreating espresso-node-${n} with tag=${upgrade_tag}"
  # No --wait: the new image's baked-in healthcheck reads ESPRESSO_NODE_API_PORT
  # but the old compose only sets ESPRESSO_SEQUENCER_API_PORT (the binary maps
  # via env_compat.rs but the healthcheck shell can't). wait_for_height_advance
  # against the API verifies consensus liveness directly.
  DOCKER_TAG="${upgrade_tag}" compose up -d --no-deps --force-recreate "espresso-node-${n}"

  wait_for_height_advance "http://localhost:${port}" 2 120
}

# bulk_upgrade_remaining <upgrade_tag>
# Recreate the remaining long-running services pinned to DOCKER_TAG at the
# upgrade tag. Excludes:
#   - the 5 espresso-nodes (already rolled)
#   - postgres dbs, keydb, demo-l1-network, block-explorer, wait-for-v4
#   - one-shot deploy / staking services (deploy-*, stake-for-demo,
#     fund-builder, claim-rewards). They already ran in phase 1; re-running
#     them is redundant and some (e.g. deploy-lcv3-upgrade in 20260505) retry
#     forever and would block the bulk up.
bulk_upgrade_remaining() {
  local upgrade_tag="$1"
  local -a service_list

  mapfile -t service_list < <(compose config --services |
    grep -Ev '^espresso-node-[0-4]$' |
    grep -Ev '^espresso-node-db-' |
    grep -Ev '^(keydb|demo-l1-network|block-explorer|wait-for-v4)$' |
    grep -Ev '^(deploy-|claim-rewards|fund-builder|stake-for-demo)')

  if [[ ${#service_list[@]} -eq 0 ]]; then
    err "No remaining services found to upgrade"
    return 1
  fi

  local joined
  joined="$(IFS=' '; printf '%s' "${service_list[*]}")"
  log "Bulk-upgrading ${#service_list[@]} services to ${upgrade_tag}: ${joined}"
  DOCKER_TAG="${upgrade_tag}" compose up -d --no-deps "${service_list[@]}"
}
