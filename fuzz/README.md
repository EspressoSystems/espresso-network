# Fuzz targets

Coverage-guided fuzzing with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) for code that decodes or verifies
bytes from peers and query servers. This crate has its own workspace and lockfile, so normal builds never compile
libFuzzer.

| Target             | Input                                                      | Checks                                                                                    |
| ------------------ | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `vid_ns_shares`    | edited bincode of an honest AvidmGf2 common and its shares | no panic in verify or recover; verified shards covering the threshold recover the payload |
| `ns_table`         | namespace table and payload bytes                          | no panic walking the payload; both namespace table parsers agree on every valid table     |
| `inclusion_proofs` | edited bincode or JSON of honest tx and namespace proofs   | no panic decoding or verifying; no proof of a transaction or namespace not in the block   |

## Running

Sanitizers need nightly, which the `nightly` dev shell provides along with `cargo-fuzz`:

```bash
nix develop .#nightly
cargo fuzz run vid_ns_shares -- -max_total_time=300 -rss_limit_mb=2048
```

A crash is saved under `fuzz/artifacts/<target>/`. Replay it with `cargo fuzz run <target> <file>`, and shrink it with
`cargo fuzz tmin <target> <file>`. Turn each one into a regular unit test next to the code before fixing it.
