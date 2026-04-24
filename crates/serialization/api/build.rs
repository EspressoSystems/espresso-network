use std::path::PathBuf;

// Example values for OpenAPI documentation
const EXAMPLE_ETH_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const EXAMPLE_HEIGHT: u64 = 1000000;
const EXAMPLE_OFFSET: u64 = 0;
const EXAMPLE_LIMIT: u64 = 100;
const EXAMPLE_NAMESPACE_ID: u32 = 10001;
const EXAMPLE_EPOCH: u64 = 100;
const EXAMPLE_BLOCK_RANGE_LAST: u64 = 1000100;

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
        // Data API examples
        .field_attribute(
            "GetNamespaceProofRequest.namespace_id",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_NAMESPACE_ID),
        )
        .field_attribute(
            "GetNamespaceProofRequest.block",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_HEIGHT),
        )
        .field_attribute(
            "GetNamespaceProofRequest.first",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_HEIGHT),
        )
        .field_attribute(
            "GetNamespaceProofRequest.last",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_BLOCK_RANGE_LAST),
        )
        .field_attribute(
            "GetIncorrectEncodingProofRequest.namespace_id",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_NAMESPACE_ID),
        )
        .field_attribute(
            "GetIncorrectEncodingProofRequest.block_height",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_HEIGHT),
        )
        // Consensus API examples
        .field_attribute(
            "GetStateCertificateRequest.epoch",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_EPOCH),
        )
        .field_attribute(
            "GetStakeTableRequest.epoch",
            format!(r#"#[schemars(example = "{}")]"#, EXAMPLE_EPOCH),
        )
        .out_dir(&out_dir)
        .compile_protos(
            &[
                "v2/common.proto",
                "v2/rewards.proto",
                "v2/data.proto",
                "v2/consensus.proto",
            ],
            &[proto_root],
        )?;

    println!("cargo:rerun-if-changed=proto/v2/common.proto");
    println!("cargo:rerun-if-changed=proto/v2/rewards.proto");
    println!("cargo:rerun-if-changed=proto/v2/data.proto");
    println!("cargo:rerun-if-changed=proto/v2/consensus.proto");

    Ok(())
}
