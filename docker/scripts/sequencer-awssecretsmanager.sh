#!/bin/bash
set -eEu -o pipefail

if [[ -n "${ESPRESSO_SEQUENCER_GENESIS_SECRET:-}" ]]; then
  echo "Loading genesis file from AWS Secrets Manager..."
  aws secretsmanager get-secret-value --secret-id "${ESPRESSO_SEQUENCER_GENESIS_SECRET}" \
       --query SecretString --output text | tee /genesis/injected.toml > /dev/null
fi

exec /bin/sequencer "$@"
