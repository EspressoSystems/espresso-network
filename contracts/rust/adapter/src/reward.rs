use alloy::{
    primitives::{Bytes, B256, U256},
    sol_types::SolValue,
};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

// Defined in espresso-types, but we don't want to depend on the crate just for this constant
pub const REWARD_MERKLE_TREE_V2_HEIGHT: usize = 160;
pub const OTHER_AUTH_ROOT_INPUTS_LEN: usize = 7;

type RewardAuthDataRaw = (
    [B256; REWARD_MERKLE_TREE_V2_HEIGHT],
    [B256; OTHER_AUTH_ROOT_INPUTS_LEN],
);

#[derive(Clone, Debug, Eq, PartialEq, From)]
pub struct RewardProofSiblings([B256; REWARD_MERKLE_TREE_V2_HEIGHT]);

#[derive(Clone, Debug, Eq, PartialEq, From, Default)]
pub struct RewardAuthRootInputs([B256; OTHER_AUTH_ROOT_INPUTS_LEN]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardAuthData {
    siblings: RewardProofSiblings,
    auth_root_inputs: RewardAuthRootInputs,
}

impl RewardAuthData {
    /// Create reward auth data from proof siblings.
    ///
    /// Auth root inputs (other than the reward merkle tree root) are currently
    /// all zero placeholder values.
    pub fn new(siblings: RewardProofSiblings) -> Self {
        RewardAuthData {
            siblings,
            auth_root_inputs: Default::default(),
        }
    }
}

impl From<RewardAuthDataRaw> for RewardAuthData {
    fn from((siblings, auth_root_inputs): RewardAuthDataRaw) -> Self {
        RewardAuthData {
            siblings: siblings.into(),
            auth_root_inputs: auth_root_inputs.into(),
        }
    }
}

impl TryFrom<RewardAuthDataEncoded> for RewardAuthData {
    type Error = alloy::sol_types::Error;

    fn try_from(value: RewardAuthDataEncoded) -> Result<Self, Self::Error> {
        let decoded: RewardAuthDataRaw = SolValue::abi_decode(&value.0)?;
        Ok(decoded.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, From, Into)]
pub struct RewardAuthDataEncoded(Bytes);

impl From<RewardAuthData> for RewardAuthDataEncoded {
    fn from(value: RewardAuthData) -> Self {
        Self(
            (value.siblings.0, value.auth_root_inputs.0)
                .abi_encode()
                .into(),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RewardClaimInput {
    pub lifetime_rewards: U256,
    pub auth_data: RewardAuthDataEncoded,
}

#[cfg(test)]
mod tests {
    use std::array::from_fn;

    use serde_json;

    use super::*;

    impl RewardAuthData {
        fn random() -> Self {
            (from_fn(|_| B256::random()), from_fn(|_| B256::random())).into()
        }
    }

    impl RewardClaimInput {
        fn random() -> Self {
            Self {
                lifetime_rewards: B256::random().into(),
                auth_data: RewardAuthData::random().into(),
            }
        }
    }

    #[test]
    fn test_reward_claim_input_roundtrip_json() {
        let original = RewardClaimInput::random();
        let json = serde_json::to_string(&original).unwrap();
        let decoded: RewardClaimInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_abi_auth_data() {
        let original = RewardClaimInput::random();
        let auth_data = RewardAuthData::try_from(original.auth_data.clone()).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let auth_str = value.get("auth_data").unwrap().as_str().unwrap();
        assert!(auth_str.starts_with("0x"));

        // The auth data is sent "as-is" to the reward claim contract and needs to ABI decode to the
        // inner types in order for the contract to process thenm.
        let (siblings, auth_root_inputs): RewardAuthDataRaw =
            SolValue::abi_decode(&alloy::hex::decode(&auth_str[2..]).unwrap()).unwrap();
        assert_eq!(siblings, auth_data.siblings.0);
        assert_eq!(auth_root_inputs, auth_data.auth_root_inputs.0);
    }
}
