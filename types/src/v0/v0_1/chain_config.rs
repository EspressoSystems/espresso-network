use alloy::primitives::{Address, U256};
use alloy_compat::ethers_serde;
use committable::{Commitment, Committable};
use derive_more::{Deref, Display, From, Into};
use itertools::Either;

use serde::{Deserialize, Serialize};

use crate::{FeeAccount, FeeAmount};

#[derive(Default, Hash, Copy, Clone, Debug, Display, PartialEq, Eq, From, Into)]
#[display("{_0}")]
pub struct ChainId(pub U256);

#[derive(Hash, Copy, Clone, Debug, Default, Display, PartialEq, Eq, From, Into, Deref)]
#[display("{_0}")]
pub struct BlockSize(pub(crate) u64);

/// Global variables for an Espresso blockchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Espresso chain ID
    pub chain_id: ChainId,

    /// Maximum size in bytes of a block
    pub max_block_size: BlockSize,

    /// Minimum fee in WEI per byte of payload
    pub base_fee: FeeAmount,

    /// Fee contract address on L1.
    ///
    /// This is optional so that fees can easily be toggled on/off, with no need to deploy a
    /// contract when they are off. In a future release, after fees are switched on and thoroughly
    /// tested, this may be made mandatory.
    #[serde(with = "ethers_serde::option_address")]
    pub fee_contract: Option<Address>,

    /// Account that receives sequencing fees.
    ///
    /// This account in the Espresso fee ledger will always receive every fee paid in Espresso,
    /// regardless of whether or not their is a `fee_contract` deployed. Once deployed, the fee
    /// contract can decide what to do with tokens locked in this account in Espresso.
    pub fee_recipient: FeeAccount,
}

#[derive(Clone, Debug, Copy, PartialEq, Deserialize, Serialize, Eq, Hash)]
pub struct ResolvableChainConfig {
    pub(crate) chain_config: Either<ChainConfig, Commitment<ChainConfig>>,
}

impl Committable for ChainConfig {
    fn tag() -> String {
        "CHAIN_CONFIG".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("chain_id", &self.chain_id.to_fixed_bytes())
            .u64_field("max_block_size", *self.max_block_size)
            .fixed_size_field("base_fee", &self.base_fee.to_fixed_bytes())
            .fixed_size_field("fee_recipient", &self.fee_recipient.to_fixed_bytes());
        let comm = if let Some(addr) = self.fee_contract {
            comm.u64_field("fee_contract", 1).fixed_size_bytes(&addr.0)
        } else {
            comm.u64_field("fee_contract", 0)
        };
        comm.finalize()
    }
}

impl ResolvableChainConfig {
    pub fn commit(&self) -> Commitment<ChainConfig> {
        match self.chain_config {
            Either::Left(config) => config.commit(),
            Either::Right(commitment) => commitment,
        }
    }
    pub fn resolve(self) -> Option<ChainConfig> {
        match self.chain_config {
            Either::Left(config) => Some(config),
            Either::Right(_) => None,
        }
    }
}

impl From<Commitment<ChainConfig>> for ResolvableChainConfig {
    fn from(value: Commitment<ChainConfig>) -> Self {
        Self {
            chain_config: Either::Right(value),
        }
    }
}

impl From<ChainConfig> for ResolvableChainConfig {
    fn from(value: ChainConfig) -> Self {
        Self {
            chain_config: Either::Left(value),
        }
    }
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            chain_id: U256::from(35353).into(), // arbitrarily chosen chain ID
            max_block_size: 30720.into(),
            base_fee: 0.into(),
            fee_contract: None,
            fee_recipient: Default::default(),
        }
    }
}
