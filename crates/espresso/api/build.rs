use std::path::PathBuf;

/// All .proto files under `<proto_root>/v2`, sorted for deterministic codegen.
fn v2_proto_files(proto_root: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<_> = std::fs::read_dir(proto_root.join("v2"))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "proto").then_some(path)
        })
        .collect();
    files.sort();
    Ok(files)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    std::fs::create_dir_all(&src_dir)?;

    // Generate message types plus tonic client and server stubs in one pass.
    tonic_prost_build::configure()
        .out_dir(&src_dir)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute("Empty.dummy", "#[serde(skip)]")
        // google.api types back the HTTP annotations consumed at build time by
        // tonic-rest-build; nothing references them at runtime, so skip generating them.
        .extern_path(".google.api", "::google_api_unused")
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile_protos(&v2_proto_files(&proto_root)?, &[proto_root])?;

    // Generate Axum REST handlers for every service with google.api.http annotations.
    // The proto file is the single definition site: path and method come from the
    // annotation, request/response types from the rpc signature, and the handlers call
    // through the tonic service traits generated above.
    let descriptor_bytes = std::fs::read(out_dir.join("descriptor.bin"))?;
    let rest_config =
        tonic_rest_build::RestCodegenConfig::new().package("espresso.api.v2", "proto");
    let rest_code = tonic_rest_build::generate(&descriptor_bytes, &rest_config)?;
    // Repo convention: no em dashes in committed text.
    let rest_code = rest_code.replace('\u{2014}', "-");
    std::fs::write(src_dir.join("espresso.api.v2.rest.rs"), rest_code)?;

    println!("cargo:rerun-if-changed=proto");

    Ok(())
}
