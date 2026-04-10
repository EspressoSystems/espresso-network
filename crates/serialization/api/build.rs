use std::path::PathBuf;

// Example values for OpenAPI documentation
const EXAMPLE_ETH_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const EXAMPLE_HEIGHT: u64 = 1000000;
const EXAMPLE_OFFSET: u64 = 0;
const EXAMPLE_LIMIT: u64 = 100;

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
        // Add OpenAPI examples for path parameters
        .field_attribute(
            "GetRewardClaimInputRequest.address",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_ETH_ADDRESS),
        )
        .field_attribute(
            "GetRewardBalanceRequest.address",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_ETH_ADDRESS),
        )
        .field_attribute(
            "GetRewardAccountProofRequest.address",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_ETH_ADDRESS),
        )
        .field_attribute(
            "GetRewardBalancesRequest.height",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_HEIGHT),
        )
        .field_attribute(
            "GetRewardBalancesRequest.offset",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_OFFSET),
        )
        .field_attribute(
            "GetRewardBalancesRequest.limit",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_LIMIT),
        )
        .field_attribute(
            "GetRewardMerkleTreeRequest.height",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_HEIGHT),
        )
        .out_dir(&out_dir)
        .compile_protos(&["v2/common.proto", "v2/rewards.proto"], &[proto_root])?;

    println!("cargo:rerun-if-changed=proto/v2/common.proto");
    println!("cargo:rerun-if-changed=proto/v2/rewards.proto");

    Ok(())
}
