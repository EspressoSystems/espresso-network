//! Generated API schema types from protobuf schemas.
//!
//! **DO NOT MODIFY FILES IN THIS CRATE MANUALLY**
//!
//! Types are generated from .proto files in the `proto/` directory.
//! To change API schemas, edit the .proto files and run: cargo build -p serialization-api
//!
//! Generated Rust types are committed to git for visibility in code review.

// Generated code - committed to git for visibility in code review
pub mod v2 {
    include!("espresso.api.v2.rs");
}

pub use v2::*;

/// Trait for converting between implementation types and proto serialization types
///
/// Implementations define their own internal types for addresses and rewards data,
/// then provide conversions to/from the proto types for API serialization.
pub trait EspressoSerializations {
    // Request types (implementation-defined)

    /// Address type used by the implementation
    type Address;

    // Response types (implementation-defined)

    /// Reward claim input type
    type RewardClaimInput;

    /// Reward balance type
    type RewardBalance;

    /// Reward account query data type (balance + proof)
    type RewardAccountQueryData;

    /// Paginated reward amounts type
    type RewardAmounts;

    /// Reward merkle tree snapshot data type
    type RewardMerkleTreeData;

    // Deserialize proto/string types → internal types

    /// Deserialize an address string from a proto request into the implementation's Address type
    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address>;

    // Serialize internal types → proto types

    /// Serialize implementation's RewardClaimInput to proto RewardClaimInput
    ///
    /// Takes the original address string since the internal type may not contain it
    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput>;

    /// Serialize implementation's RewardBalance to proto RewardBalance
    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance>;

    /// Serialize implementation's RewardAccountQueryData to proto RewardAccountQueryDataV2
    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

    /// Serialize implementation's RewardAmounts to proto RewardAmounts
    fn serialize_reward_amounts(
        &self,
        value: &Self::RewardAmounts,
    ) -> anyhow::Result<RewardAmounts>;

    /// Serialize implementation's RewardMerkleTreeData to proto RewardMerkleTreeV2Data
    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data>;
}

#[cfg(test)]
mod serde_tests {
    use super::v2::*;

    #[test]
    fn test_empty_node_serialization() {
        let empty_node = MerkleNode {
            node_type: Some(merkle_node::NodeType::Empty(Empty { dummy: None })),
        };

        let json = serde_json::to_string(&empty_node).unwrap();
        println!("Empty serialization: {}", json);

        // Empty now serializes as {"Empty":{}} with proto-generated serde
        assert_eq!(json, r#"{"Empty":{}}"#);
    }

    #[test]
    fn test_leaf_node_serialization() {
        let leaf_node = MerkleNode {
            node_type: Some(merkle_node::NodeType::Leaf(Leaf {
                pos: "FIELD~test_pos".to_string(),
                elem: "FIELD~test_elem".to_string(),
                value: "FIELD~test_value".to_string(),
            })),
        };

        let json = serde_json::to_string(&leaf_node).unwrap();
        println!("Leaf serialization: {}", json);

        // Proto-generated field order: pos, elem, value (as defined in .proto file)
        assert_eq!(
            json,
            r#"{"Leaf":{"pos":"FIELD~test_pos","elem":"FIELD~test_elem","value":"FIELD~test_value"}}"#
        );
    }

    #[test]
    fn test_branch_node_serialization() {
        let branch_node = MerkleNode {
            node_type: Some(merkle_node::NodeType::Branch(Branch {
                value: "FIELD~branch_value".to_string(),
                children: vec![
                    MerkleNode {
                        node_type: Some(merkle_node::NodeType::Empty(Empty { dummy: None })),
                    },
                ],
            })),
        };

        let json = serde_json::to_string(&branch_node).unwrap();
        println!("Branch serialization: {}", json);

        // Verify Branch contains Empty as string
        assert!(json.contains(r#""Branch""#));
        assert!(json.contains(r#""Empty""#));
    }

    #[test]
    fn test_forgotten_subtree_serialization() {
        let forgotten_node = MerkleNode {
            node_type: Some(merkle_node::NodeType::ForgottenSubtree(ForgottenSubtree {
                value: "FIELD~forgotten_value".to_string(),
            })),
        };

        let json = serde_json::to_string(&forgotten_node).unwrap();
        println!("ForgottenSubtree serialization: {}", json);

        assert_eq!(
            json,
            r#"{"ForgottenSubtree":{"value":"FIELD~forgotten_value"}}"#
        );
    }

    #[test]
    fn test_merkle_node_schema() {
        let schema = schemars::schema_for!(MerkleNode);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        println!("MerkleNode schema:\n{}", schema_json);

        // Verify schema contains oneOf with 4 variants
        assert!(schema_json.contains("\"oneOf\""), "Schema should have oneOf");
        assert!(schema_json.contains("\"Empty\""), "Schema should contain Empty variant");
        assert!(schema_json.contains("\"Leaf\""), "Schema should contain Leaf variant");
        assert!(schema_json.contains("\"Branch\""), "Schema should contain Branch variant");
        assert!(schema_json.contains("\"ForgottenSubtree\""), "Schema should contain ForgottenSubtree variant");
    }
}
