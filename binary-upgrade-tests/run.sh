#!/usr/bin/env bash
# Binary upgrade test driver.
#
# Boots docker-compose using the docker-compose.yaml and .env from the BASE_TAG
# git revision (the era-appropriate files for the old binary), then rolls each
# espresso-node-N from BASE_TAG to UPGRADE_TAG one-by-one, then bulk-upgrades
# the rest. Runs scripts/smoke-test-demo before and after.
#
# The new binary uses crates/espresso/utils/src/env_compat.rs to migrate the
# old ESPRESSO_SEQUENCER_* env names to current ESPRESSO_* / ESPRESSO_NODE_*
# names, so a single compose file works for both old and new images.

set -euo pipefail
IFS=$'\n\t'

usage() {
  cat <<EOF
Usage: ${0##*/} [-h|--help]

Environment:
  BASE_TAG         Base docker tag (and git ref for compose+env)  (default: 20260505)
  UPGRADE_TAG      Target docker tag                              (default: main)
  KEEP_RUNNING     If 1, do not docker compose down on exit       (default: 0)
  UPGRADE_PULL     If 1, docker compose pull for upgrade tag      (default: 0)
  SETTLE_SECONDS   Sleep after bulk upgrade before final smoke    (default: 30)
EOF
}

case "${1:-}" in
  -h | --help)
    usage
    exit 0
    ;;
  "") ;;
  *)
    printf 'Unknown argument: %s\n' "$1" >&2
    usage >&2
    exit 1
    ;;
esac

# Run from repo root.
cd "$(git rev-parse --show-toplevel)"

# shellcheck source=binary-upgrade-tests/lib.sh
source binary-upgrade-tests/lib.sh

if [[ ! -f .env ]]; then
  err ".env not found. Copy .env.docker.example to .env (or use the dev shell) first."
  exit 1
fi

BASE_TAG="${BASE_TAG:-20260505}"
UPGRADE_TAG="${UPGRADE_TAG:-main}"
KEEP_RUNNING="${KEEP_RUNNING:-0}"
SETTLE_SECONDS="${SETTLE_SECONDS:-30}"
UPGRADE_PULL="${UPGRADE_PULL:-0}"

export BASE_TAG UPGRADE_TAG KEEP_RUNNING SETTLE_SECONDS UPGRADE_PULL

# Genesis ships inside the base image at /genesis; demo-drb-header.toml is
# already there at 20260505 (V0.4, no protocol upgrade configured).
export ESPRESSO_SEQUENCER_GENESIS_FILE="${ESPRESSO_SEQUENCER_GENESIS_FILE:-genesis/demo-drb-header.toml}"
export ESPRESSO_NODE_GENESIS_FILE="${ESPRESSO_NODE_GENESIS_FILE:-genesis/demo-drb-header.toml}"

mkdir -p tmp
BASE_DIR="$(mktemp -d tmp/binary-upgrade.XXXXXX)"
export BASE_DIR

trap cleanup EXIT

# Source project .env so smoke-test-demo and bridge-related env are available
# to the host shell (host port mappings line up across both eras).
set -a
# shellcheck disable=SC1091
source .env
set +a

log "Extracting docker-compose.yaml and .env from git ref ${BASE_TAG} into ${BASE_DIR}"
extract_base_files "${BASE_TAG}"

# Preflight: clean any stale stack.
compose down -v >/dev/null 2>&1 || true

log "Pulling base images (DOCKER_TAG=${BASE_TAG})"
DOCKER_TAG="${BASE_TAG}" compose pull --policy missing

if [[ "${UPGRADE_PULL}" == "1" ]]; then
  log "Pulling upgrade images (DOCKER_TAG=${UPGRADE_TAG})"
  DOCKER_TAG="${UPGRADE_TAG}" compose pull --policy missing
fi

log "Starting network on ${BASE_TAG}"
# Don't use --wait: prover-one-shot has a broken baked-in healthcheck that
# never reaches healthy, and deploy-lcv3-upgrade in the 20260505 compose
# retries forever (its hotshot-config fetch defaults to localhost:24000).
# Run in background so the script can proceed; the smoke test polls for actual
# readiness, and the trap handles cleanup.
DOCKER_TAG="${BASE_TAG}" compose up -d >>tmp/compose-up.log 2>&1 &

log "Initial smoke test"
DOCKER_TAG="${BASE_TAG}" timeout 600 scripts/smoke-test-demo

for N in 0 1 2 3 4; do
  log "Rolling espresso-node-${N} to ${UPGRADE_TAG}"
  roll_node "${N}" "${UPGRADE_TAG}"
done

log "Bulk-upgrading remaining services to ${UPGRADE_TAG}"
bulk_upgrade_remaining "${UPGRADE_TAG}"

log "Settling for ${SETTLE_SECONDS} seconds"
sleep "${SETTLE_SECONDS}"

log "Asserting all espresso-node-N containers run image tag ${UPGRADE_TAG}"
for N in 0 1 2 3 4; do
  assert_service_image "espresso-node-${N}" "${UPGRADE_TAG}"
done

log "Final smoke test"
DOCKER_TAG="${UPGRADE_TAG}" timeout 600 scripts/smoke-test-demo

log "Binary upgrade test complete"
