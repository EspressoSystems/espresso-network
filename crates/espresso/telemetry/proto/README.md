# Vendored Prometheus remote-write protobuf

Source: [`prometheus/prometheus`](https://github.com/prometheus/prometheus) tag
[`v2.55.1`](https://github.com/prometheus/prometheus/tree/v2.55.1/prompb)

- `types.proto` — from `prompb/types.proto`
- `remote.proto` — from `prompb/remote.proto`

## Why vendored

The remote-write 1.0 protocol is frozen (Prometheus 2.x baseline; 2.0 is opt-in on receivers that explicitly enable it).
Vector's `prometheus_remote_write` source speaks 1.0. We generate types directly from upstream protos via `prost-build`
(see `../build.rs`) rather than depend on a third-party crate, so we own the dependency graph.

## Modifications from upstream

`gogoproto`-specific bits removed because they are Go-codegen hints with no effect on the wire format and `prost-build`
does not accept them:

- Removed `import "gogoproto/gogo.proto";` from both files.
- Removed all `[(gogoproto.nullable) = false]` field options.

That is the entire diff. The wire format produced by `prost`-generated code is byte-for-byte identical to the upstream
Go implementation.

## Updating

Run `scripts/update-prometheus-protos.sh [--tag vX.Y.Z]`. The script fetches the protos, strips the `gogoproto` bits,
bumps the tag references, and runs `cargo check -p espresso-telemetry`. Follow up with
`cargo nextest run -p espresso-telemetry` to confirm tests still pass.
