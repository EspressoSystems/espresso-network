#!/bin/bash
set -eEu -o pipefail

if [[ -v ESPRESSO_SEQUENCER_GENESIS_SECRET ]]; then
  echo "Loading genesis file from AWS secrets manager"
  aws secretsmanager  get-secret-value --secret-id ${ESPRESSO_SEQUENCER_GENESIS_SECRET} --query SecretString --output text | tee /genesis/injected.toml >/dev/null
fi

/bin/espresso-node "$@"
