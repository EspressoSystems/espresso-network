use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../serialization/api/proto");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // Generate gRPC service stubs
    tonic_prost_build::configure()
        .out_dir(&src_dir)
        .extern_path(".espresso.api.v2", "::serialization_api::v2")
        .file_descriptor_set_path(out_dir.join("reflection_descriptor.bin"))
        .compile_protos(
            &[
                "v2/common.proto",
                "v2/rewards.proto",
                "v2/data.proto",
                "v2/consensus.proto",
            ],
            &[proto_root.to_str().unwrap()],
        )?;

    println!("cargo:rerun-if-changed=../../serialization/api/proto/v2/common.proto");
    println!("cargo:rerun-if-changed=../../serialization/api/proto/v2/rewards.proto");
    println!("cargo:rerun-if-changed=../../serialization/api/proto/v2/data.proto");
    println!("cargo:rerun-if-changed=../../serialization/api/proto/v2/consensus.proto");

    Ok(())
}
