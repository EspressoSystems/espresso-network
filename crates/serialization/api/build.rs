use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // Generate message types
    // Output directly to src/ so generated types are committed to git for visibility
    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&["v1/common.proto", "v1/rewards.proto"], &[proto_root])?;

    println!("cargo:rerun-if-changed=proto/v1/common.proto");
    println!("cargo:rerun-if-changed=proto/v1/rewards.proto");

    Ok(())
}
