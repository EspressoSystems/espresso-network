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

# wait_for_height_at_or_above <api_url> <target_height> <timeout_seconds>
wait_for_height_at_or_above() {
  local api_url="$1"
  local target="$2"
  local timeout_seconds="$3"
  local current start=$SECONDS

  while true; do
    current="$(get_block_height "${api_url}")"
    if [[ -n "${current}" ]] && ((current >= target)); then
      log "Height ${current} >= ${target} at ${api_url}"
      return 0
    fi
    if ((SECONDS - start > timeout_seconds)); then
      err "Timed out after ${timeout_seconds}s waiting for height >= ${target} at ${api_url} (last: ${current:-unknown})"
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

# upgraded_services
# Lists every compose service that should be on UPGRADE_TAG after the test:
# the 5 espresso-nodes plus everything bulk_upgrade_remaining recreates.
# Excludes:
#   - databases and infrastructure that aren't espresso images
#     (espresso-node-db-*, keydb, demo-l1-network, block-explorer)
#   - one-shots that already ran in phase 1 and shouldn't be re-run on the
#     new tag (deploy-*, claim-rewards, fund-builder, stake-for-demo,
#     cdn-whitelist, wait-for-v4). Some of these stay in "running" state
#     while retrying, so we can't rely on container state to filter them.
upgraded_services() {
  local exclude='^(espresso-node-db-.*|keydb|demo-l1-network|block-explorer|wait-for-v4|cdn-whitelist|claim-rewards|fund-builder|stake-for-demo|deploy-.*)$'
  compose config --services | grep -Ev "${exclude}"
}

# assert_all_espresso_images <expected_tag>
# Asserts every upgraded service whose image is published under
# ghcr.io/espressosystems/espresso-network/ is on the expected tag.
assert_all_espresso_images() {
  local expected_tag="$1"
  local prefix="ghcr.io/espressosystems/espresso-network/"
  local service cid image rc=0

  while read -r service; do
    cid="$(compose ps -q "${service}")"
    [[ -z "${cid}" ]] && continue
    image="$(docker inspect "${cid}" --format='{{.Config.Image}}')"
    [[ "${image}" != "${prefix}"* ]] && continue
    if [[ "${image}" != *":${expected_tag}" ]]; then
      err "service ${service} image is ${image}, expected tag ${expected_tag}"
      rc=1
    else
      log "service ${service} image: ${image}"
    fi
  done < <(upgraded_services)

  return "${rc}"
}

# roll_node <n> <upgrade_tag>
# Recreate just espresso-node-N at the upgrade tag, leaving other services
# untouched. Sample a reference height before the recreate, then wait for
# every query-enabled node (0, 1, 3, 4) to reach reference + 2. This verifies
# the rolled node rejoined consensus and the rest didn't stall. Node 2 has no
# `query` module so it can't be polled.
roll_node() {
  local n="$1"
  local upgrade_tag="$2"

  # Pick a non-rolled node to sample reference height from.
  local ref_n=0
  [[ "${n}" == "0" ]] && ref_n=1
  local ref_port
  ref_port="$(node_api_port "${ref_n}")"
  local initial
  initial="$(get_block_height "http://localhost:${ref_port}")"
  if [[ -z "${initial}" ]]; then
    err "Could not read reference height from espresso-node-${ref_n}"
    return 1
  fi
  local target=$((initial + 2))

  log "Recreating espresso-node-${n} with tag=${upgrade_tag}; waiting for all query-enabled nodes to reach height ${target}"
  # No --wait: the new image's baked-in healthcheck reads ESPRESSO_NODE_API_PORT
  # but the old compose only sets ESPRESSO_SEQUENCER_API_PORT (the binary maps
  # via env_compat.rs but the healthcheck shell can't). The height polling
  # below verifies consensus liveness directly.
  DOCKER_TAG="${upgrade_tag}" compose up -d --no-deps --force-recreate "espresso-node-${n}"

  local monitor_n port
  for monitor_n in 0 1 3 4; do
    port="$(node_api_port "${monitor_n}")"
    if [[ -z "${port}" ]]; then
      err "Could not resolve API port for espresso-node-${monitor_n}"
      return 1
    fi
    wait_for_height_at_or_above "http://localhost:${port}" "${target}" 120 || return 1
  done
}

# bulk_upgrade_remaining <upgrade_tag>
# Recreate every service in upgraded_services except the 5 espresso-nodes
# (already rolled individually) at the upgrade tag.
bulk_upgrade_remaining() {
  local upgrade_tag="$1"
  local -a service_list

  mapfile -t service_list < <(upgraded_services | grep -Ev '^espresso-node-[0-4]$')

  if [[ ${#service_list[@]} -eq 0 ]]; then
    err "No remaining services found to upgrade"
    return 1
  fi

  local joined
  joined="$(IFS=' '; printf '%s' "${service_list[*]}")"
  log "Bulk-upgrading ${#service_list[@]} services to ${upgrade_tag}: ${joined}"
  DOCKER_TAG="${upgrade_tag}" compose up -d --no-deps "${service_list[@]}"
}
