#!/usr/bin/env bash
set -e

HEALTHCHECK_TIMEOUT=300
L1_TIMEOUT=60

# Wait for staking UI service
healthy=false
for i in $(seq $HEALTHCHECK_TIMEOUT); do
    if curl -s --fail http://localhost:8080/healthcheck; then
        healthy=true
        break
    fi
    echo "Waiting for service to become healthy"
    sleep 1
done
if ! $healthy; then
    echo "Timed out waiting for service to become healthy"
    exit 1
fi

initial_l1_block=$(curl -s http://localhost:8080/v0/staking/l1/block/latest | jq .number)
echo "Service healthy, initial L1 block: ${initial_l1_block}"

# Wait for L1 block height to increase
increased=false
for i in $(seq $L1_TIMEOUT); do
    l1_block=$(curl -s http://localhost:8080/v0/staking/l1/block/latest | jq .number)
    if [[ $l1_block > $initial_l1_block ]]; then
        echo "L1 block increased from ${initial_l1_block} to ${l1_block}"
        increased=true
        break
    fi
    echo "Waiting for L1 block to increase"
    sleep 1
done
if ! $increased; then
    echo "Timed out waiting for L1 block to increase"
    exit 1
fi
