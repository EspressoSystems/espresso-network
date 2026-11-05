# Tuning

`main`: works with any image. `bench`: needs an experimental bench branch image. Env vars go in `--node-env`.

## Node

| Setting                                          | Needs                        |
| ------------------------------------------------ | ---------------------------- |
| `--consensus-storage journal`                    | bench                        |
| `ESPRESSO_NODE_BLOB_DIR=/payload`                | bench                        |
| `ESPRESSO_BENCH_BLAKE3_TX_HASH=1`                | bench                        |
| `--query-engine sqlite`                          | main                         |
| `EXPERIMENTAL_SQLITE_PRAGMAS=synchronous=normal` | bench                        |
| `GLIBC_TUNABLES=glibc.malloc.hugetlb=1`          | main                         |
| `--allocator NAME`                               | `build-allocators.yml` image |
| `--prune [RETENTION]`                            | main                         |

With `--query-engine sqlite`, `--query-db volume` keeps SQLite and payloads off the root disk the journal uses; `tmpfs`
drops SQLite disk I/O entirely (needs `ESPRESSO_NODE_BLOB_DIR`). `EXPERIMENTAL_SQLITE_PRAGMAS` takes comma-separated
`key=value` pragmas applied after the defaults.

With `--prune` and `--query-engine sqlite`, each prune batch holds the write lock, which shows as query lag.

## Experimental VID leader traffic

These flags are enabled by presence, including a value of `0`; leave them unset to disable them.

- `ESPRESSO_BENCH_SKIP_NEXT_LEADER_REBROADCAST=1`: the leader of V+1 does not rebroadcast its share of V.
- `ESPRESSO_BENCH_SKIP_NEXT_LEADER_RECONSTRUCTION=1`: all nodes omit rebroadcasts of V to the leader of V+1,
  which retains its assigned share for proposal validation but skips reconstruction and recovery fetching of V.
  It can still propose V+1, while abstaining from votes that require V's payload.
- `ESPRESSO_BENCH_SKIP_PRODUCER_REBROADCAST=1`: omit rebroadcasts back to the leader that produced the block;
  that leader already has the complete payload.

Original leader-to-recipient dispersal is retained. Epoch boundaries select the next leader from the next epoch.
The next-leader flags spend fault tolerance: a deliberately absent honest vote or share can prevent progress when
other nodes also withhold their contributions. The producer omission alone does not require abstaining.

A query-serving validator that skips reconstruction can lack the payloads needed to serve blocks and track benchmark
transactions. Fleet experiments must provide query recovery or exempt that validator from the receive omission;
these flags do not add query recovery. The existing decided-byte counter undercount also needs to be accounted for
when comparing offered load. Neither the query path nor that counter is changed by these flags.

## Host (with `--latency`)

- `--mtu 9001` is needed for very high throughput, but only traffic inside one VPC gets it; real inter-region paths
  carry 1500, so results overstate a WAN deployment.
- `--tcp-cc bbr` (default) is needed for good throughput. `--tcp-cc bbr_hold` (experimental) keeps throughput stable
  after an overload. `--tcp-cc bbr3` runs BBR v3 on the XanMod kernel (x86_64 only).
