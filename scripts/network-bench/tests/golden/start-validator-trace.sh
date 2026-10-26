#!/usr/bin/env bash
set -eEu -o pipefail

echo '{}' > /opt/bench/containers.json

mkdir -p /opt/bench/trace

id=$(docker create --name espresso-node --network host --restart no --env-file /opt/bench/node.env -v /data/journal:/store -v /opt/bench/genesis.toml:/opt/bench/genesis.toml:ro -v /opt/bench/trace:/trace ghcr.io/x/espresso-node:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/espresso-node -- storage-fs -- http -- status -- submit -- catchup -- config)
jq --arg n espresso-node --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

