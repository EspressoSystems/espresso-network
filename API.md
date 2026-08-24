Espresso nodes provide both a HTTP/JSON and gRPC API, served by the `espresso-api` crate at `crates/espresso/api`.

The v1 API is a set of hand-written Axum routes over traits defined in `crates/espresso/api/src/v1/`.

The node implements the API traits in `crates/espresso/node/src/api/state.rs`.

## The v2 API

The v2 API is defined entirely in protobuf. Each rpc in `crates/espresso/api/proto/v2/*.proto` is the single definition
site for an endpoint: the rpc signature gives the request and response types, the `google.api.http` option gives the
HTTP route, and the comments become the generated documentation.

From those files, `crates/espresso/api/build.rs` generates everything else into `crates/espresso/api/src/generated/`
(committed to git for review visibility, never edited by hand):

- `espresso.api.v2.rs`: message types and the tonic server traits (clients are not generated)
- `espresso.api.v2.serde.rs`: canonical protoJSON Serialize/Deserialize impls for the message types (pbjson)
- `espresso.api.v2.rest.rs`: Axum handlers that transcode HTTP/JSON onto the tonic service traits
- `espresso.api.v2.openapi.json`: OpenAPI 3.0 document for the REST routes, served at `/v2/docs/openapi.json` with
  Swagger UI at `/v2` and Scalar at `/v2/scalar` (`generated/openapi.rs` is the hand-written build-script module that
  produces it)

One implementation of a tonic service trait therefore serves both transports: `serve_axum` mounts the generated
`*_rest_router` functions under `/v2/...`, and `serve_tonic` registers the tonic servers plus gRPC reflection (the
descriptor set is exported as `espresso_api::FILE_DESCRIPTOR_SET`).

### What is served today

`StatusService` and `TokenService`, nine endpoints under `/v2/status/...` and `/v2/token/...`. Everything else a client
needs is still on v1. Every route in the OpenAPI document is a route `serve_axum` mounts; the two are asserted equal in
`crates/espresso/api/src/axum.rs`.

### Adding an endpoint to an existing service

1. Define the rpc in the service's proto file, for example `crates/espresso/api/proto/v2/status.proto`:

   ```proto
   message GetUptimeRequest {}

   message UptimeResponse {
     // Seconds since the process started, not since it last decided a block
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

   Request message fields become HTTP query parameters.

   Proto comments are published API surface: an rpc comment becomes the operation summary, and a field comment becomes
   the parameter or property description. So comment an rpc, and comment a field whose units, encoding, or zero value a
   caller cannot guess (`in ESP (decimal string)`, `TaggedBase64`, `zero before the node has seen a view`). Do not
   comment a message to restate its own name: `// Latest block height response` above `message BlockHeightResponse` only
   adds a line to keep in sync.

2. Regenerate:

   ```sh
   nix develop --command cargo check -p espresso-api
   ```

   Commit the changes to `src/generated/` together with the proto change.

3. Implement the new trait method in `crates/espresso/node/src/api/state.rs`. The build fails there until you do, which
   is the complete to-do list. Follow the local pattern: a `fetch_*` method returning internal types (`anyhow::Result`),
   a `serialize_*` conversion to the proto type, and a thin tonic method composing them. Map errors with `to_status` so
   `AvailabilityError::NotFound` becomes gRPC `not_found` / HTTP 404.

4. Verify:

   ```sh
   nix develop --command cargo test -p espresso-api
   ```

   For a request against a running node, start one with SQL storage (v2 is only mounted by `serve_axum`) and curl the
   route. gRPC can be exercised with `grpcurl` against the tonic port; reflection is enabled.

### Adding a new service

1. Create `crates/espresso/api/proto/v2/<name>.proto` (the build globs the directory, so no build script change) with
   the service, its rpcs, and their `google.api.http` options.
2. Regenerate as above.
3. Implement the generated `<name>_service_server::<Name>Service` trait on `NodeApiStateImpl`.
4. Wire the transports in `crates/espresso/api/src/lib.rs`: add the trait bound to `serve_axum` and `serve_tonic`, merge
   `rest::<name>_service_rest_router(...)` in `serve_axum`, and `add_service` the tonic server in `serve_tonic`.

### Rules and caveats

- Field and rpc numbers are frozen once merged. Only make additive changes: new fields, new rpcs, new messages. Never
  renumber, reuse, or change the type of an existing field.
- Never edit `src/generated/` by hand; change the protos and rebuild.
- Only GET bindings are used so far. The generator (`tonic-rest-build`) supports other methods, but decide the
  request-body mapping deliberately before introducing the first one.
- v2 addresses resources with flat query parameters, not v1-style path parameters: one static route per rpc, with every
  field of the request message as a query parameter, so a future block-height lookup is `/v2/...?height=5` rather than
  `/v2/.../5`. This is deliberate. The route lives in the proto annotation and stays a constant, so adding a parameter
  is an additive proto change rather than a new URL shape, and a single binding serves both gRPC and REST. Every
  endpoint served today is parameterless, so this rule binds future endpoints rather than existing ones.
- Consequently request messages must stay flat: scalars and `optional` scalars only. The generated handlers extract with
  `axum::extract::Query`, and `serde_urlencoded` cannot decode repeated or nested message fields, so the first request
  message with a `repeated` or message-typed field silently fails to deserialize. Structured input needs the POST body
  mapping decided above, not a nested request message on a GET.
- Unknown fields are rejected rather than ignored, in both JSON bodies and query strings: any query parameter on a
  parameterless endpoint is a 400. This is pbjson's default and is worth keeping, since a typo'd parameter would
  otherwise return a confidently wrong response. Those rejections come from `axum::extract::Query`, not from the
  handler, so they arrive as plain text; `axum::v2_error_envelope` rewrites them into the envelope below, which is why
  the v2 routers are layered with it in `serve_axum`. The layer covers the mounted routes only: a request to an unknown
  path under `/v2/`, or a known path with the wrong method, is answered by the router before the layer runs and keeps
  axum's plain-text body.
- Regenerate inside the nix shell. `prost-build` shells out to `protoc`, so a different local version can produce a
  different descriptor and leave `src/generated/` dirty after a plain `cargo build`. CI enforces the committed artifacts
  match the protos (`Check generated API code is up to date` in `lint.yml`).
- `tonic-rest` is pinned with `=` because its runtime half is on the public HTTP path: it renders every v2 error body
  and copies request headers into tonic metadata. Read the diff before bumping it.
- JSON is canonical protoJSON (generated by pbjson): lowerCamelCase field names, 64-bit integers as decimal strings,
  bytes as base64, oneofs flattened into the parent object, defaults omitted (so a zero-valued field is absent, not
  `0`). Standard protobuf tooling can generate compatible clients. Deserialization accepts both camelCase and the
  original proto field names, so query parameters keep their snake_case proto names. Absent request fields take their
  proto3 defaults instead of erroring. The shape is pinned by `crates/espresso/api/tests/proto_json.rs`.
- Only `serve_axum` (the SQL storage mode) mounts the v2 routes and their docs. `serve_axum_fs`, `serve_axum_status`,
  and `serve_axum_bare` serve v1 only, so v2 requests 404 there. `TestNetwork` defaults to filesystem storage when a
  test does not configure storage, which is why v2 endpoints need a SQL-backed network to exercise.
- HTTP errors follow the Google API error model:
  `{"error": {"code": 400, "message": "...", "status": "INVALID_ARGUMENT"}}`, derived from the tonic `Status` returned
  by the trait implementation.
- The vendored `crates/espresso/api/proto/google/api/` protos exist only so protoc can resolve the `google.api.http`
  annotation; they are build-time inputs, trimmed to the definitions.
