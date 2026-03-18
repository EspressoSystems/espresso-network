#!/bin/bash
set -eEu -o pipefail

if [[ -v LIGHT_CLIENT_GENESIS_SECRET ]]; then
  echo "Loading genesis file from AWS secrets manager"
  aws secretsmanager  get-secret-value --secret-id ${LIGHT_CLIENT_GENESIS_SECRET} --query SecretString --output text | tee /genesis/injected.toml >/dev/null
fi

/bin/light-client-query-service "$@"
