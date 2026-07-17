#!/bin/bash
set -eEu -o pipefail

# Backward compat: ESPRESSO_SEQUENCER_GENESIS_SECRET -> ESPRESSO_NODE_GENESIS_SECRET
if [[ ! -v ESPRESSO_NODE_GENESIS_SECRET ]] && [[ -v ESPRESSO_SEQUENCER_GENESIS_SECRET ]]; then
  echo "ERROR: ESPRESSO_SEQUENCER_GENESIS_SECRET is deprecated, use ESPRESSO_NODE_GENESIS_SECRET instead" >&2
  ESPRESSO_NODE_GENESIS_SECRET="$ESPRESSO_SEQUENCER_GENESIS_SECRET"
fi

if [[ -v ESPRESSO_NODE_GENESIS_SECRET ]]; then
  echo "Loading genesis file from AWS secrets manager"
  aws secretsmanager  get-secret-value --secret-id ${ESPRESSO_NODE_GENESIS_SECRET} --query SecretString --output text | tee /genesis/injected.toml >/dev/null
fi

/bin/espresso-node "$@"
