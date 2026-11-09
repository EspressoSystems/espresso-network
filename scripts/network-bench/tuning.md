# Tuning

`main`: works with any image. `bench`: needs an experimental bench branch image. Env vars go in `--node-env`.

## Node

| Setting                                          | Needs                        |
| ------------------------------------------------ | ---------------------------- |
| `--consensus-storage journal`                    | bench                        |
| `ESPRESSO_QUERY_PAYLOAD_DIR=/payload`            | bench                        |
| `ESPRESSO_BENCH_BLAKE3_TX_HASH=1`                | bench                        |
| `--node-stake N,N,...`                           | main                         |
| `--query-engine sqlite`                          | main                         |
| `EXPERIMENTAL_SQLITE_PRAGMAS=synchronous=normal` | bench                        |
| `GLIBC_TUNABLES=glibc.malloc.hugetlb=1`          | main                         |
| `--allocator NAME`                               | `build-allocators.yml` image |
| `--prune [RETENTION]`                            | main                         |
| `ESPRESSO_NODE_CLIQUENET_CONTROL_LANE=true`      | main                         |

With `--query-engine sqlite`, `--query-db volume` keeps SQLite and payloads off the root disk the journal uses; `tmpfs`
drops SQLite disk I/O entirely (needs `ESPRESSO_QUERY_PAYLOAD_DIR`). `EXPERIMENTAL_SQLITE_PRAGMAS` takes comma-separated
`key=value` pragmas applied after the defaults.

With `--prune` and `--query-engine sqlite`, each prune batch holds the write lock, which shows as query lag.

`--node-stake` writes `ESPRESSO_NODE_STAKE_VALUE`, the stake a node registers with the orchestrator, into each node's
environment. Share sizes, vote thresholds and leader frequency follow it, and VID uses the raw stakes as shard counts
while the total stays at or below 1000.

`ESPRESSO_NODE_CLIQUENET_CONTROL_LANE=true` sends proposals, votes and certificates on a second cliquenet network, the
control lane, on the port above the cliquenet port (9001), so they do not queue behind VID shares. Set it on every node
or none. The fleet's security group passes all traffic between its hosts; the socket samples (`ss` on port 9000) leave
out the lane's connections.

## Host (with `--latency`)

- `--mtu 9001` is needed for very high throughput, but only traffic inside one VPC gets it; real inter-region paths
  carry 1500, so results overstate a WAN deployment.
- `--tcp-cc bbr` (default) is needed for good throughput. `--tcp-cc bbr_hold` (experimental) keeps throughput stable
  after an overload. `--tcp-cc bbr3` runs BBR v3 on the XanMod kernel (x86_64 only).
