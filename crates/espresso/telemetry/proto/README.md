# Vendored Prometheus remote-write protobuf

Source: [`prometheus/prometheus`](https://github.com/prometheus/prometheus) tag
[`v2.55.1`](https://github.com/prometheus/prometheus/tree/v2.55.1/prompb)

- `types.proto` — from `prompb/types.proto`
- `remote.proto` — from `prompb/remote.proto`

## Why vendored

The remote-write 1.0 protocol is frozen (Prometheus 2.x baseline; 2.0 is opt-in on receivers that explicitly enable it).
Vector's `prometheus_remote_write` source speaks 1.0. The previously-used `prometheus_remote_write` Rust crate was
unmaintained, so we generate types directly from upstream protos via `prost-build` (see `../build.rs`) and own the
dependency graph.

## Modifications from upstream

`gogoproto`-specific bits removed because they are Go-codegen hints with no effect on the wire format and `prost-build`
does not accept them:

- Removed `import "gogoproto/gogo.proto";` from both files.
- Removed all `[(gogoproto.nullable) = false]` field options.

That is the entire diff. The wire format produced by `prost`-generated code is byte-for-byte identical to the upstream
Go implementation.

## Updating

When refreshing from upstream:

1. Pull the latest `prompb/{types,remote}.proto` from the desired tag.
2. Update the tag reference at the top of this file.
3. Re-strip the `gogoproto` import + annotations (see "Modifications" above).
4. Run `cargo build -p espresso-telemetry` to re-generate.
5. `cargo nextest run -p espresso-telemetry` to confirm the wire format and tests still match.
