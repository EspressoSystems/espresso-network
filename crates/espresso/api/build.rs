use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Generates the OpenAPI document from the compiled descriptor set. Lives under
/// `src/generated/` next to its output, but it is this build script's module, not a
/// generated file.
#[path = "src/generated/openapi.rs"]
mod openapi;

/// Proto package the v2 API is defined in, shared with [`openapi`].
pub const PACKAGE: &str = "espresso.api.v2";

/// All .proto files under `<proto_root>/v2`, sorted for deterministic codegen.
fn v2_proto_files(proto_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<_> = fs::read_dir(proto_root.join("v2"))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "proto").then_some(path)
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Writes only when the content changes, so a rerun does not bump mtimes on artifacts that are
/// `include!`d and force a rebuild of the whole crate.
fn write_if_changed(path: PathBuf, content: &str) -> std::io::Result<()> {
    if fs::read_to_string(&path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    fs::write(path, content)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("proto");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let src_dir = manifest_dir.join("src/generated");
    fs::create_dir_all(&src_dir)?;

    // Generate message types plus the tonic server traits in one pass. The types get no serde
    // derives: JSON is canonical protoJSON, implemented by the pbjson pass below. Clients are
    // skipped because nothing in the workspace consumes this API over gRPC; the REST handlers and
    // the node's service impls only need the server side.
    tonic_prost_build::configure()
        .out_dir(&src_dir)
        .build_client(false)
        // google.api types back the HTTP annotations consumed at build time by tonic-rest-build.
        // Nothing references them at runtime, so point them at a crate that does not exist: if a
        // future proto ever puts one in a message field, the result is an unresolved-crate error
        // naming `google_api_unused` rather than silently generating the types.
        .extern_path(".google.api", "::google_api_unused")
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile_protos(&v2_proto_files(&proto_root)?, &[proto_root])?;

    let descriptor_bytes = fs::read(out_dir.join("descriptor.bin"))?;

    // Generate canonical protoJSON Serialize/Deserialize impls for the message types:
    // lowerCamelCase names, 64-bit integers as decimal strings, bytes as base64, oneofs
    // flattened, defaults omitted. Unknown fields are rejected, so a misspelled query
    // parameter is a 400 rather than a silently wrong response.
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_bytes)?
        .out_dir(&src_dir)
        .build(&[&format!(".{PACKAGE}")])?;

    // Generate Axum REST handlers for every service with google.api.http annotations.
    // The proto file is the single definition site: path and method come from the
    // annotation, request/response types from the rpc signature, and the handlers call
    // through the tonic service traits generated above.
    let rest_config = tonic_rest_build::RestCodegenConfig::new().package(PACKAGE, "proto");
    let rest_code = tonic_rest_build::generate(&descriptor_bytes, &rest_config)?;
    // Repo convention: no em dashes in committed text.
    let rest_code = rest_code.replace('\u{2014}', "-");
    write_if_changed(src_dir.join("espresso.api.v2.rest.rs"), &rest_code)?;

    // Document the same routes for HTTP clients, from the same descriptor set.
    let spec = openapi::generate(&descriptor_bytes)?;
    write_if_changed(
        src_dir.join("espresso.api.v2.openapi.json"),
        &serde_json::to_string_pretty(&spec)?,
    )?;

    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=src/generated/openapi.rs");

    Ok(())
}
