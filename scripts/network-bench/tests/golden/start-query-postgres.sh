#!/usr/bin/env bash
set -eEu -o pipefail

echo '{}' > /opt/bench/containers.json

id=$(docker create --name postgres --network host --restart no -v /data/pg:/var/lib/postgresql -v /data/pg-tls:/tls:ro --shm-size=8g -e POSTGRES_USER=root -e POSTGRES_PASSWORD=password -e POSTGRES_DB=espresso docker.io/library/postgres:18@sha256:0000000000000000000000000000000000000000000000000000000000000000 -c shared_buffers=8GB -c effective_cache_size=24GB -c huge_pages=off -c work_mem=64MB -c maintenance_work_mem=2GB -c max_wal_size=16GB -c min_wal_size=4GB -c checkpoint_timeout=15min -c checkpoint_completion_target=0.9 -c wal_compression=lz4 -c default_toast_compression=lz4 -c random_page_cost=1.1 -c effective_io_concurrency=200 -c autovacuum_vacuum_cost_limit=2000 -c autovacuum_max_workers=4 -c autovacuum_work_mem=512MB -c autovacuum_naptime=1min -c autovacuum_vacuum_scale_factor=0.2 -c autovacuum_analyze_scale_factor=0.1 -c track_io_timing=off -c wal_buffers=256MB -c max_connections=100 -c shared_preload_libraries=pg_stat_statements -c pg_stat_statements.track=all -c log_min_duration_statement=200 -c log_parameter_max_length=0 -c log_checkpoints=on -c log_lock_waits=on -c ssl=on -c ssl_cert_file=/tls/server.crt -c ssl_key_file=/tls/server.key)
jq --arg n postgres --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

mkdir -p /data/pg/payload

id=$(docker create --name espresso-node --network host --restart no --env-file /opt/bench/node.env -v /data/journal:/store -v /opt/bench/genesis.toml:/opt/bench/genesis.toml:ro -v /data/pg/payload:/payload ghcr.io/x/espresso-node:t@sha256:0000000000000000000000000000000000000000000000000000000000000000 /bin/espresso-node -- storage-sql -- http -- query -- submit -- catchup -- config -- light-client)
jq --arg n espresso-node --arg i "$id" '. + {($n): $i}' /opt/bench/containers.json > /opt/bench/containers.json.tmp
mv /opt/bench/containers.json.tmp /opt/bench/containers.json

