use alloy::primitives::U256;
use sequencer_utils::{
    impl_serde_from_string_or_integer, impl_to_fixed_bytes, ser::FromStringOrInteger,
};

use super::parse_size;
use crate::{BlockSize, ChainId};

impl_serde_from_string_or_integer!(ChainId);
impl_to_fixed_bytes!(ChainId, U256);

impl FromStringOrInteger for ChainId {
    type Binary = ethers_core::types::U256;
    type Integer = u64;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self> {
        Ok(Self(U256::from_limbs(b.0)))
    }

    fn from_integer(i: Self::Integer) -> anyhow::Result<Self> {
        Ok(i.into())
    }

    fn from_string(s: String) -> anyhow::Result<Self> {
        if let Some(stripped) = s.strip_prefix("0x") {
            Ok(Self(U256::from_str_radix(stripped, 16)?))
        } else {
            Ok(Self(U256::from_str_radix(&s, 10)?))
        }
    }

    fn to_binary(&self) -> anyhow::Result<Self::Binary> {
        Ok(ethers_core::types::U256(self.0.into_limbs()))
    }

    fn to_string(&self) -> anyhow::Result<String> {
        Ok(format!("{self}"))
    }
}

impl From<u64> for ChainId {
    fn from(id: u64) -> Self {
        Self(U256::from(id))
    }
}

impl_serde_from_string_or_integer!(BlockSize);

impl FromStringOrInteger for BlockSize {
    type Binary = u64;
    type Integer = u64;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self> {
        Ok(Self(b))
    }

    fn from_integer(i: Self::Integer) -> anyhow::Result<Self> {
        Ok(Self(i))
    }

    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(parse_size(&s)?.into())
    }

    fn to_binary(&self) -> anyhow::Result<Self::Binary> {
        Ok(self.0)
    }

    fn to_string(&self) -> anyhow::Result<String> {
        Ok(format!("{self}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v0_99::{ChainConfig, ResolvableChainConfig};

    #[test]
    fn test_chainid_serde_json_as_decimal() {
        let id = ChainId::from(123);
        let serialized = serde_json::to_string(&id).unwrap();

        // The value is serialized as a decimal string.
        assert_eq!(serialized, "\"123\"");

        // Deserialization produces the original value
        let deserialized: ChainId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn test_chainid_serde_json_from_hex() {
        // For backwards compatibility, chain ID can also be deserialized from a 0x-prefixed hex
        // string.
        let id: ChainId = serde_json::from_str("\"0x123\"").unwrap();
        assert_eq!(id, ChainId::from(0x123));
    }

    #[test]
    fn test_chainid_serde_json_from_number() {
        // For convenience, chain ID can also be deserialized from a decimal number.
        let id: ChainId = serde_json::from_str("123").unwrap();
        assert_eq!(id, ChainId::from(123));
    }

    #[test]
    fn test_chainid_serde_bincode_unchanged() {
        // For non-human-readable formats, ChainId just serializes as the underlying U256.
        // note: for backward compat, it has to be the same as ethers' U256 instead of alloy's
        let n = ethers_core::types::U256::from(123);
        let id = ChainId(U256::from(123));
        assert_eq!(
            bincode::serialize(&n).unwrap(),
            bincode::serialize(&id).unwrap(),
        );
    }

    #[test]
    fn test_block_size_serde_json_as_decimal() {
        let size = BlockSize::from(123);
        let serialized = serde_json::to_string(&size).unwrap();

        // The value is serialized as a decimal string.
        assert_eq!(serialized, "\"123\"");

        // Deserialization produces the original value
        let deserialized: BlockSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, size);
    }

    #[test]
    fn test_block_size_serde_json_from_number() {
        // For backwards compatibility, block size can also be deserialized from a decimal number.
        let size: BlockSize = serde_json::from_str("123").unwrap();
        assert_eq!(size, BlockSize::from(123));
    }

    #[test]
    fn test_block_size_serde_bincode_unchanged() {
        // For non-human-readable formats, BlockSize just serializes as the underlying u64.
        let n = 123u64;
        let size = BlockSize(n);
        assert_eq!(
            bincode::serialize(&n).unwrap(),
            bincode::serialize(&size).unwrap(),
        );
    }

    #[test]
    fn test_chain_config_equality() {
        let chain_config = ChainConfig::default();
        assert_eq!(chain_config, chain_config.clone());
        let ChainConfig {
            chain_id,
            max_block_size,
            ..
        } = chain_config;
        let other_config = ChainConfig {
            chain_id,
            max_block_size,
            base_fee: 1.into(),
            ..Default::default()
        };
        assert!(chain_config != other_config);
    }

    #[test]
    fn test_resolve_chain_config() {
        let chain_config = ChainConfig::default();
        let resolvable: ResolvableChainConfig = chain_config.into();
        assert_eq!(chain_config, resolvable.resolve().unwrap());
    }
}
