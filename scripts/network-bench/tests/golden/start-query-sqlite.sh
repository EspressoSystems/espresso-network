#!/usr/bin/env bash
set -eEu -o pipefail

echo '{}' > /opt/bench/containers.json

mkdir -p /data/pg/payload

id=$(docker create --name espresso-node --network host --restart no --env-file /opt/bench/node.env -v /data/journal:/store -v /opt/bench/genesis.toml:/opt/bench/genesis.toml:ro -v /data/pg/payload:/payload ghcr.io/x/espresso-node:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/espresso-node -- http -- query -- submit -- catchup -- config -- light-client)
jq --arg n espresso-node --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

