# The v2 API

The v2 API is defined entirely in protobuf. Each rpc in `crates/espresso/api/proto/v2/*.proto` is the single definition
site for an endpoint: the rpc signature gives the request and response types, the `google.api.http` option gives the
HTTP route, and the comments become the generated documentation.

From those files, `crates/espresso/api/build.rs` generates everything else into `crates/espresso/api/src/generated/`
(committed to git for review visibility, never edited by hand):

- `espresso.api.v2.rs`: message types (with serde), tonic clients, and tonic servers
- `espresso.api.v2.rest.rs`: Axum handlers that transcode HTTP/JSON onto the tonic service traits

One implementation of a tonic service trait therefore serves both transports: `serve_axum` mounts the generated
`*_rest_router` functions under `/v2/...`, and `serve_tonic` registers the tonic servers plus gRPC reflection (the
descriptor set is exported as `espresso_api::FILE_DESCRIPTOR_SET`).

## Adding an endpoint to an existing service

1. Define the rpc in the service's proto file, for example `crates/espresso/api/proto/v2/status.proto`:

   ```proto
   // Request for the node's uptime (no parameters)
   message GetUptimeRequest {}

   // Node uptime response
   message UptimeResponse {
     // Seconds since the node started
     uint64 seconds = 1;
   }

   service StatusService {
     // ...existing rpcs...

     // Get the seconds since the node started
     rpc GetUptime(GetUptimeRequest) returns (UptimeResponse) {
       option (google.api.http) = {get: "/v2/status/uptime"};
     }
   }
   ```

   Request message fields become HTTP query parameters. Give every message and rpc a comment; they flow into the
   generated docs.

2. Regenerate:

   ```sh
   nix develop --command cargo check -p espresso-api
   ```

   Commit the changes to `src/generated/` together with the proto change.

3. Implement the new trait method. The build now fails everywhere the trait is implemented, which is the complete to-do
   list:
   - `crates/espresso/node/src/api/state.rs`: the real implementation. Follow the local pattern: a `fetch_*` method
     returning internal types (`anyhow::Result`), a `serialize_*` conversion to the proto type, and a thin tonic method
     composing them. Map errors with `to_status` so `AvailabilityError::NotFound` becomes gRPC `not_found` / HTTP 404.
   - `crates/espresso/api/examples/test_api.rs`: a mock returning fixed data.

4. Verify:

   ```sh
   nix develop --command cargo test -p espresso-api
   nix develop --command cargo run -p espresso-api --example test_api
   curl http://localhost:5001/v2/status/uptime
   ```

   gRPC can be exercised with `grpcurl` against the tonic port; reflection is enabled.

## Adding a new service

1. Create `crates/espresso/api/proto/v2/<name>.proto` (the build globs the directory, so no build script change) with
   the service, its rpcs, and their `google.api.http` options.
2. Regenerate as above.
3. Implement the generated `<name>_service_server::<Name>Service` trait on `NodeApiStateImpl` and `TestApi`.
4. Wire the transports in `crates/espresso/api/src/lib.rs`: add the trait bound to `serve_axum` and `serve_tonic`, merge
   `rest::<name>_service_rest_router(...)` in `serve_axum`, and `add_service` the tonic server in `serve_tonic`.

## Rules and caveats

- Field and rpc numbers are frozen once merged. Only make additive changes: new fields, new rpcs, new messages. Never
  renumber, reuse, or change the type of an existing field.
- Never edit `src/generated/` by hand; change the protos and rebuild.
- Only GET bindings are used so far. The generator (`tonic-rest-build`) supports other methods, but decide the
  request-body mapping deliberately before introducing the first one.
- JSON is the serde encoding of the prost types (snake_case fields, oneofs as externally-tagged enums), not canonical
  protoJSON. Switching to protoJSON (pbjson) is a known open decision; it must happen before external clients depend on
  the current shape.
- HTTP errors follow the Google API error model:
  `{"error": {"code": 400, "message": "...", "status": "INVALID_ARGUMENT"}}`, derived from the tonic `Status` returned
  by the trait implementation.
- The vendored `crates/espresso/api/proto/google/api/` protos exist only so protoc can resolve the `google.api.http`
  annotation; they are build-time inputs, trimmed to the definitions.
