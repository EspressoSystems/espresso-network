#!/usr/bin/env bash
set -eEu -o pipefail

# instance_initiated_shutdown_behavior=terminate: this bounds cost even if the run dies.
shutdown -P +$TTL_MIN

export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y docker.io chrony rsync gzip jq curl

cat >> /etc/chrony/chrony.conf <<'CHRONY'
server 169.254.169.123 prefer iburst minpoll 4 maxpoll 4
CHRONY
systemctl restart chrony

mkdir -p /etc/docker
cat > /etc/docker/daemon.json <<'DAEMON'
{"log-driver":"json-file","log-opts":{"max-size":"2g","max-file":"4"}}
DAEMON
systemctl restart docker

mkdir -p /data/journal /opt/bench
$EXTRA_DIRS

echo '{}' > /opt/bench/digests.json
$DOCKER_PULLS

chronyc tracking > /opt/bench/chrony.txt
jq -n --argjson digests "$$(cat /opt/bench/digests.json)" --rawfile chrony /opt/bench/chrony.txt \
  '{digests: $$digests, chronyc_tracking: $$chrony}' > /opt/bench/ready.json
