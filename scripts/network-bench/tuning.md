# Tuning

Settings for comparable `aws-bench` runs. "Branches" lists where a setting works; `main` means any image.

## Node

| Setting                                             | Effect                                                                | Branches                                                                                                       |
| --------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `--consensus-storage journal`                       | validators and node0 consensus on `storage-journal`; fast query path  | `bench-optimize-block-build`, `bench-payload-on-disk`, `ab/vid-encode-once-journal`, `ab/journal-query-replay` |
| `--consensus-storage fs` (default)                  | validators `storage-fs`, node0 `storage-sql`                          | `main`                                                                                                         |
| `--node-env ESPRESSO_QUERY_PAYLOAD_DIR=/payload`    | query service stores payloads as files, not in Postgres               | `bench-optimize-block-build`, `bench-payload-on-disk`, `ab/vid-encode-once-journal`                            |
| `--node-env ESPRESSO_BENCH_BLAKE3_TX_HASH=1`        | blake3 tx hash; all nodes must set it                                 | `bench-optimize-block-build`, `ab/vid-encode-once-journal`                                                     |
| `--node-env GLIBC_TUNABLES=glibc.malloc.hugetlb=1`  | fewer page faults in block build (33 MB: 17 ms to 5 ms)               | `main`                                                                                                         |
| `--node-env ESPRESSO_NODE_EMPTY_BLOCK_DELAY=50ms`   | avoids the ~450 MB/s collapse after a submission gap (default 500 ms) | `main`                                                                                                         |
| `--allocator {jemalloc,mimalloc,snmalloc,tcmalloc}` | `espresso-node-alloc:<tag>-<allocator>` image                         | release tags; branches after `build-allocators.yml`                                                            |

## Host (with `--latency`, undone on reset)

| Setting                     | Value                                                                                                                                 |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `tcp_rmem` / `tcp_wmem` max | 256 MB (~128 MB window; Ubuntu caps one flow near 25 MB/s at 158 ms)                                                                  |
| `tcp_notsent_lowat`         | 128 KB                                                                                                                                |
| `tcp_slow_start_after_idle` | 0                                                                                                                                     |
| `--tcp-cc`                  | `cubic` + `fq_codel` (default); `bbr` paces ~30% slower on cross-region sockets after the first overload; `bbr_hold` is an experiment |
| `--mtu`                     | 1500 (inter-region path)                                                                                                              |

## Comparing runs

- A/B `--tcp-cc` and `EMPTY_BLOCK_DELAY` before attributing a capacity difference to node code.
- Check the `storage` / `nodes` / `node-env` lines at the `[y/N]` prompt.
