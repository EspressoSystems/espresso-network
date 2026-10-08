#!/usr/bin/env bash
# Builds and loads tcp_bbr_hold: mainline BBR v1 of the running kernel's version, held in
# STARTUP (pacing gain 2.89) and without PROBE_RTT (min RTT kept for an hour). Bursty validator
# traffic otherwise makes cross-region sockets leave STARTUP for good at the first overload.
# Experiment only: not a stock congestion control. Run as root on a bench node; a second run
# does nothing.
set -euo pipefail
if grep -qw bbr_hold /proc/sys/net/ipv4/tcp_available_congestion_control; then
  exit 0
fi
kver=$(uname -r)
tag=v$(echo "$kver" | cut -d. -f1-2)
DEBIAN_FRONTEND=noninteractive apt-get install -y -q "linux-headers-$kver" build-essential >/dev/null
dir=$(mktemp -d)
curl -sfL "https://raw.githubusercontent.com/torvalds/linux/$tag/net/ipv4/tcp_bbr.c" -o "$dir/tcp_bbr_hold.c"
sed -i \
  -e 's/= "bbr",/= "bbr_hold",/' \
  -e 's/bbr_probe_rtt_mode_ms = 200;/bbr_probe_rtt_mode_ms = 0;/' \
  -e 's/bbr_min_rtt_win_sec = 10;/bbr_min_rtt_win_sec = 3600;/' \
  -e 's/if (bbr_full_bw_reached(sk) || !bbr->round_start || rs->is_app_limited)/if (true)/' \
  -e 's/ret = register_btf_kfunc_id_set(BPF_PROG_TYPE_STRUCT_OPS, &tcp_bbr_kfunc_set);/ret = 0;/' \
  -e 's/^static const struct btf_kfunc_id_set tcp_bbr_kfunc_set/static const struct btf_kfunc_id_set __maybe_unused tcp_bbr_kfunc_set/' \
  "$dir/tcp_bbr_hold.c"
# Every substitution must have hit: a source change upstream fails here, not silently.
for pattern in '"bbr_hold"' 'bbr_probe_rtt_mode_ms = 0;' 'bbr_min_rtt_win_sec = 3600;' 'if (true)' 'ret = 0;' '__maybe_unused tcp_bbr_kfunc_set'; do
  grep -qF "$pattern" "$dir/tcp_bbr_hold.c" || { echo "bbr-hold: patch missed: $pattern" >&2; exit 1; }
done
echo 'obj-m += tcp_bbr_hold.o' > "$dir/Makefile"
make -s -C "/lib/modules/$kver/build" M="$dir" modules
insmod "$dir/tcp_bbr_hold.ko"
grep -qw bbr_hold /proc/sys/net/ipv4/tcp_available_congestion_control
