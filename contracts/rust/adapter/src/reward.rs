use alloy::{
    primitives::{Bytes, B256, U256},
    sol_types::SolValue,
};
use derive_more::From;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Eq, PartialEq, From)]
pub struct RewardProofSiblings([B256; 160]);

#[derive(Clone, Debug, Eq, PartialEq, From, Default)]
pub struct RewardAuthRootInputs([B256; 7]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardAuthData {
    siblings: RewardProofSiblings,
    auth_root_inputs: RewardAuthRootInputs,
}

impl RewardAuthData {
    pub fn new(siblings: RewardProofSiblings, auth_root_inputs: RewardAuthRootInputs) -> Self {
        RewardAuthData {
            siblings,
            auth_root_inputs,
        }
    }

    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> alloy::sol_types::Result<Self> {
        let (siblings, auth_root_inputs): ([B256; 160], [B256; 7]) =
            SolValue::abi_decode(bytes.as_ref())?;
        Ok(RewardAuthData {
            siblings: siblings.into(),
            auth_root_inputs: auth_root_inputs.into(),
        })
    }

    pub fn to_bytes(&self) -> Bytes {
        (self.siblings.0, self.auth_root_inputs.0)
            .abi_encode()
            .into()
    }
}

impl Serialize for RewardAuthData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        alloy::hex::serde::serialize(self.to_bytes(), serializer)
    }
}

impl<'de> Deserialize<'de> for RewardAuthData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Bytes = alloy::hex::serde::deserialize(deserializer)?;
        RewardAuthData::from_bytes(bytes).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RewardClaimInput {
    pub lifetime_rewards: U256,
    pub auth_data: RewardAuthData,
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    fn test_input() -> RewardClaimInput {
        let siblings = RewardProofSiblings([B256::random(); 160]);
        let auth_root_inputs = RewardAuthRootInputs([B256::random(); 7]);
        RewardClaimInput {
            lifetime_rewards: B256::random().into(),
            auth_data: RewardAuthData::new(siblings, auth_root_inputs),
        }
    }

    #[test]
    fn test_reward_claim_input_roundtrip_json() {
        let original = test_input();
        let json = serde_json::to_string(&original).unwrap();
        let decoded: RewardClaimInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_abi_auth_data() {
        let original = test_input();
        let json = serde_json::to_string(&original).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let auth_str = value.get("auth_data").unwrap().as_str().unwrap();
        assert!(auth_str.starts_with("0x"));

        // The auth data is sent "as-is" to the reward claim contract and need
        // to ABI decode to the inner types in order for the contract to process
        // them.
        let (siblings, auth_root_inputs): ([B256; 160], [B256; 7]) =
            SolValue::abi_decode(&alloy::hex::decode(&auth_str[2..]).unwrap()).unwrap();
        assert_eq!(siblings, original.auth_data.siblings.0);
        assert_eq!(auth_root_inputs, original.auth_data.auth_root_inputs.0);
    }
}
