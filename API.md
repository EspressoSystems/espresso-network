Espresso nodes provide both a HTTP/JSON and gRPC API.

The protobuf message format for the API is defined in `crates/serialization/api`. Rust structs are generated from these files via `prost`, with a JSON representation derived via `serde`. The `serialization-api` crate provides an `ApiSerializations` trait for users (e.g. `espresso-node`) to define conversions to and from internal types to the `prost`-generated structs.

The `axum` (HTTP/JSON) and `tonic` (gRPC) APIs are defined in `crates/espresso/api`. The `espresso-api` crate defines traits for various types of data served at these APIs. Users must provide a representation of the node's state that implements these traits, using their own internal types defined in the `ApiSerializations` trait from the `serialization-api` crate. These implementations are wrapped in handlers that consume and produce gRPC request and response types, which are in turn wrapped in handlers that extract parameters from a URL path and return a JSON response.

# Adding a new endpoint

To add a new endpoint to the API, you must:

- Define new protobuf serialization types in the `serialization-api` crate at `crates/serialization/api`, and ensure that the `prost`-generated rust types are built by `cargo build -p serialization-api`.
- Extend the `ApiSerialization` trait in the `serialization-api` crate to allow the implementation to declare the corresponding internal types, as well as any necessary conversions to or from these types.
- Declare any desired gRPC endpoints in the protobuf files.
- Define a trait interface (or extend one of the current API traits) in `espresso-api` for the application to implement, which performs the desired logic with our internal types (e.g. queries the database, reads consensus state).
- Create Tonic (gRPC) and/or Axum (HTTP/JSON) handlers wrapping the trait logic.
- Serve the handlers at the desired routes.
- Implement the new methods/trait for the application's `NodeApiState` struct as needed.
