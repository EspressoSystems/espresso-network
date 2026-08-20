Espresso nodes provide both a HTTP/JSON and gRPC API, served by the `espresso-api` crate at `crates/espresso/api`.

The v1 API is a set of hand-written Axum routes over traits defined in `crates/espresso/api/src/v1/`.

The v2 API is defined entirely in protobuf under `crates/espresso/api/proto/v2/`; message types, tonic services, and
Axum REST handlers are all generated from the proto files into the `espresso_api::proto` and `espresso_api::rest`
modules. See `doc/api-v2.md` for the full workflow, including how to add endpoints and services.

The node implements the API traits in `crates/espresso/node/src/api/state.rs`.
