# Tuning

`main`: works with any image. `bench`: needs an experimental bench branch image. Env vars go in `--node-env`.

## Node

| Setting                                          | Needs                        |
| ------------------------------------------------ | ---------------------------- |
| `--consensus-storage journal`                    | bench                        |
| `ESPRESSO_NODE_BLOB_DIR=/payload`                | harness default, `blob-dir`  |
| `ESPRESSO_BENCH_BLAKE3_TX_HASH=1`                | bench                        |
| `--query-engine sqlite`                          | main                         |
| `EXPERIMENTAL_SQLITE_PRAGMAS=synchronous=normal` | bench                        |
| `GLIBC_TUNABLES=glibc.malloc.hugetlb=1`          | harness default              |
| `--allocator NAME`                               | `build-allocators.yml` image |
| `--prune [RETENTION]`                            | main                         |

With `--query-engine sqlite`, `--query-db volume` keeps SQLite and payloads off the root disk the journal uses; `tmpfs`
drops SQLite disk I/O entirely (needs an image with the `blob-dir` experiment). `EXPERIMENTAL_SQLITE_PRAGMAS` takes
comma-separated `key=value` pragmas applied after the defaults.

With `--prune` and `--query-engine sqlite`, each prune batch holds the write lock, which shows as query lag.

## Host (shaped runs, the default)

- `--mtu 9001` (default) is needed for very high throughput, but only traffic inside one VPC gets it; real inter-region
  paths carry 1500, so results overstate a WAN deployment. `--mtu 1500` gives internet-sized packets.
- `--tcp-cc bbr_hold` (default) keeps throughput stable after an overload; `--tcp-cc bbr` is stock BBR. `--tcp-cc bbr3`
  runs BBR v3 on the XanMod kernel (x86_64 only).
