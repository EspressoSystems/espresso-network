# Tuning

`main`: works with any image. `bench`: needs an experimental bench branch image. Env vars go in `--node-env`.

## Node

| Setting                                                                                      | Needs                                                                                                                           |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `journal` experiment (consensus on the journal)                                              | bench, on by default through `ESPRESSO_NODE_EXPERIMENTS=all`                                                                    |
| `blake3-tx-hash` experiment                                                                  | bench, on by default through `ESPRESSO_NODE_EXPERIMENTS=all`                                                                    |
| `blob-dir` experiment: `ESPRESSO_NODE_BLOB_DIR` keeps payloads and VID shares outside SQLite | bench, on by default through `ESPRESSO_NODE_EXPERIMENTS=all`; the harness sets `ESPRESSO_NODE_BLOB_DIR=/payload` on query nodes |
| `--query-engine sqlite`                                                                      | main                                                                                                                            |
| `EXPERIMENTAL_SQLITE_PRAGMAS=synchronous=normal`                                             | bench                                                                                                                           |
| `GLIBC_TUNABLES=glibc.malloc.hugetlb=1`                                                      | main                                                                                                                            |
| `--allocator NAME`                                                                           | `build-allocators.yml` image                                                                                                    |
| `--prune [RETENTION]`                                                                        | main                                                                                                                            |

With `--query-engine sqlite`, `--query-db volume` keeps SQLite and payloads off the root disk the journal uses; `tmpfs`
drops SQLite disk I/O entirely (needs the `blob-dir` experiment). `EXPERIMENTAL_SQLITE_PRAGMAS` takes comma-separated
`key=value` pragmas applied after the defaults.

With `--prune` and `--query-engine sqlite`, each prune batch holds the write lock, which shows as query lag.

## Host (with `--latency`)

- `--mtu 9001` is needed for very high throughput, but only traffic inside one VPC gets it; real inter-region paths
  carry 1500, so results overstate a WAN deployment.
- `--tcp-cc bbr` (default) is needed for good throughput. `--tcp-cc bbr_hold` (experimental) keeps throughput stable
  after an overload. `--tcp-cc bbr3` runs BBR v3 on the XanMod kernel (x86_64 only).
