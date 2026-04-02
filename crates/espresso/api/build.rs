use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../serialization/api/proto");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Generate gRPC service stubs
    tonic_prost_build::configure()
        .extern_path(".espresso.api.v1", "::serialization_api::v1")
        .file_descriptor_set_path(out_dir.join("reflection_descriptor.bin"))
        .compile_protos(&["v1/common.proto"], &[proto_root.to_str().unwrap()])?;

    println!("cargo:rerun-if-changed=../../serialization/api/proto/v1/common.proto");

    Ok(())
}
