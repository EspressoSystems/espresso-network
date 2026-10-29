#!/usr/bin/env bash
set -eEu -o pipefail

echo '{}' > /opt/bench/containers.json

id=$(docker create --name anvil --network host --restart no --entrypoint anvil ghcr.io/foundry-rs/foundry:latest@sha256:0000000000000000000000000000000000000000000000000000000000000000 --host 0.0.0.0 --port 8545 --chain-id 31337 --accounts 20 --balance 1000000000 --block-time 1)
jq --arg n anvil --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name deploy --network host --restart no --env-file /opt/bench/ctl.env ghcr.io/x/deploy:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/deploy --deploy-ops-timelock --deploy-safe-exit-timelock --deploy-fee-v1 --deploy-esp-token-v1 --deploy-stake-table-v1 --upgrade-stake-table-v2 --upgrade-stake-table-v3)
jq --arg n deploy --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name orchestrator --network host --restart no --env-file /opt/bench/ctl.env ghcr.io/x/orchestrator:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/orchestrator)
jq --arg n orchestrator --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name state-relay-server --network host --restart no --env-file /opt/bench/ctl.env ghcr.io/x/state-relay-server:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/state-relay-server)
jq --arg n state-relay-server --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name stake --network host --restart no --env-file /opt/bench/stake.env ghcr.io/x/staking-cli:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/staking-cli --no-config stake-for-demo --num-validators 5)
jq --arg n stake --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name deploy-lc --network host --restart no --entrypoint sh --env-file /opt/bench/ctl.env ghcr.io/x/deploy:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 -c 'unset ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS && exec /bin/deploy --deploy-light-client-v1 --upgrade-light-client-v2 --upgrade-light-client-v3')
jq --arg n deploy-lc --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

id=$(docker create --name state-prover --network host --restart no --env-file /opt/bench/ctl.env --cpu-shares 2 -v /data/ark-srs:/root/.local/share/ark-srs -e ESPRESSO_STATE_RELAY_SERVER_URL=http://localhost:30011 -e ESPRESSO_L1_PROVIDER=http://localhost:8545 -e ESPRESSO_STATE_PROVER_UPDATE_INTERVAL=0s ghcr.io/x/prover-service:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /usr/local/bin/state-prover -d)
jq --arg n state-prover --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

