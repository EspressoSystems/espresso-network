use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // Generate message types with serde support for JSON serialization and OpenAPI schema
    // Output directly to src/ so generated types are committed to git for visibility
    prost_build::Config::new()
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]",
        )
        // Flatten oneof fields to remove wrapper in JSON serialization
        .field_attribute("RewardMerkleProofV2.proof_type", "#[serde(flatten)]")
        .field_attribute("MerkleNode.node_type", "#[serde(flatten)]")
        // Skip Empty's dummy field so it serializes as {}
        .field_attribute("Empty.dummy", "#[serde(skip)]")
        .out_dir(&out_dir)
        .compile_protos(&["v2/common.proto", "v2/rewards.proto"], &[proto_root])?;

    println!("cargo:rerun-if-changed=proto/v2/common.proto");
    println!("cargo:rerun-if-changed=proto/v2/rewards.proto");

    Ok(())
}
