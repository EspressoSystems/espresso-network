# Tuning

`main`: works with any image. `bench`: needs an experimental bench branch image. Env vars go in `--node-env`.

## Node

| Setting                                          | Needs                        |
| ------------------------------------------------ | ---------------------------- |
| `--consensus-storage journal`                    | bench                        |
| `ESPRESSO_QUERY_PAYLOAD_DIR=/payload`            | bench                        |
| `ESPRESSO_BENCH_BLAKE3_TX_HASH=1`                | bench                        |
| `--query-engine sqlite`                          | main                         |
| `EXPERIMENTAL_SQLITE_PRAGMAS=synchronous=normal` | bench                        |
| `GLIBC_TUNABLES=glibc.malloc.hugetlb=1`          | main                         |
| `--allocator NAME`                               | `build-allocators.yml` image |

With `--query-engine sqlite`, `--query-db volume` keeps SQLite and payloads off the root disk the journal uses; `tmpfs`
drops SQLite disk I/O entirely (needs `ESPRESSO_QUERY_PAYLOAD_DIR`). `EXPERIMENTAL_SQLITE_PRAGMAS` takes comma-separated
`key=value` pragmas applied after the defaults.

## Host (with `--latency`)

- `--mtu 9001` is needed for very high throughput, but only traffic inside one VPC gets it; real inter-region paths
  carry 1500, so results overstate a WAN deployment.
- `--tcp-cc bbr` (default) is needed for good throughput. `--tcp-cc bbr_hold` (experimental) keeps throughput stable
  after an overload. `--tcp-cc bbr3` runs BBR v3 on the XanMod kernel (x86_64 only).
