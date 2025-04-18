use crate::{v0_1, v0_3, BlockSize, ChainId, FeeAccount, FeeAmount};
use alloy::primitives::{Address, U256};
use alloy_compat::ethers_serde;
use committable::{Commitment, Committable};
use itertools::Either;
use serde::{Deserialize, Serialize};

/// Global variables for an Espresso blockchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
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

    /// `StakeTable `(proxy) contract address on L1.
    ///
    /// This is optional so that stake can easily be toggled on/off, with no need to deploy a
    /// contract when they are off. In a future release, after PoS is switched on and thoroughly
    /// tested, this may be made mandatory.
    #[serde(with = "ethers_serde::option_address")]
    pub stake_table_contract: Option<Address>,

    /// Account that receives sequencing bids.
    pub bid_recipient: Option<FeeAccount>,
}

#[derive(Clone, Debug, Copy, PartialEq, Deserialize, Serialize, Eq, Hash)]
/// A commitment to a ChainConfig or a full ChainConfig.
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

        let comm = if let Some(addr) = self.stake_table_contract {
            comm.u64_field("stake_table_contract", 1)
                .fixed_size_bytes(&addr.0)
        } else {
            comm
        };

        // With `ChainConfig` upgrades we want commitments w/out
        // fields added >= v0_99 to have the same commitment as <= v0_99
        // commitment. Therefore `None` values are simply ignored.
        let comm = if let Some(bid_recipient) = self.bid_recipient {
            comm.fixed_size_field("bid_recipient", &bid_recipient.to_fixed_bytes())
        } else {
            comm
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

impl From<&v0_1::ResolvableChainConfig> for ResolvableChainConfig {
    fn from(
        &v0_1::ResolvableChainConfig { chain_config }: &v0_1::ResolvableChainConfig,
    ) -> ResolvableChainConfig {
        match chain_config {
            Either::Left(chain_config) => ResolvableChainConfig {
                chain_config: Either::Left(ChainConfig::from(chain_config)),
            },
            Either::Right(c) => ResolvableChainConfig {
                chain_config: Either::Right(Commitment::from_raw(*c.as_ref())),
            },
        }
    }
}

impl From<&v0_3::ResolvableChainConfig> for ResolvableChainConfig {
    fn from(
        &v0_3::ResolvableChainConfig { chain_config }: &v0_3::ResolvableChainConfig,
    ) -> ResolvableChainConfig {
        match chain_config {
            Either::Left(chain_config) => ResolvableChainConfig {
                chain_config: Either::Left(ChainConfig::from(chain_config)),
            },
            Either::Right(c) => ResolvableChainConfig {
                chain_config: Either::Right(Commitment::from_raw(*c.as_ref())),
            },
        }
    }
}

impl From<v0_1::ChainConfig> for ChainConfig {
    fn from(chain_config: v0_1::ChainConfig) -> ChainConfig {
        let v0_1::ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
            ..
        } = chain_config;

        ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
            stake_table_contract: None,
            bid_recipient: None,
        }
    }
}

impl From<v0_3::ChainConfig> for ChainConfig {
    fn from(chain_config: v0_3::ChainConfig) -> ChainConfig {
        let v0_3::ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
            stake_table_contract,
            ..
        } = chain_config;

        ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
            stake_table_contract,
            bid_recipient: None,
        }
    }
}

impl From<ChainConfig> for v0_1::ChainConfig {
    fn from(chain_config: ChainConfig) -> v0_1::ChainConfig {
        let ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
            ..
        } = chain_config;

        v0_1::ChainConfig {
            chain_id,
            max_block_size,
            base_fee,
            fee_contract,
            fee_recipient,
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
            stake_table_contract: None,
            bid_recipient: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_upgrade_chain_config_v99_resolvable_chain_config_from_v1() {
        let expectation: ResolvableChainConfig = ChainConfig::default().into();
        let v1_resolvable: v0_1::ResolvableChainConfig = v0_1::ChainConfig::default().into();
        let v99_resolvable: ResolvableChainConfig = ResolvableChainConfig::from(&v1_resolvable);
        assert_eq!(expectation, v99_resolvable);
        let expectation: ResolvableChainConfig = ChainConfig::default().commit().into();
        let v1_resolvable: v0_1::ResolvableChainConfig =
            v0_1::ChainConfig::default().commit().into();
        let v99_resolvable: ResolvableChainConfig = ResolvableChainConfig::from(&v1_resolvable);
        assert_eq!(expectation, v99_resolvable);
    }

    #[test]
    fn test_upgrade_chain_config_v99_resolvable_chain_config_from_v3() {
        let expectation: ResolvableChainConfig = ChainConfig::default().into();
        let v3_resolvable: v0_3::ResolvableChainConfig = v0_3::ChainConfig::default().into();
        let v99_resolvable: ResolvableChainConfig = ResolvableChainConfig::from(&v3_resolvable);
        assert_eq!(expectation, v99_resolvable);
        let expectation: ResolvableChainConfig = ChainConfig::default().commit().into();
        let v3_resolvable: v0_3::ResolvableChainConfig =
            v0_3::ChainConfig::default().commit().into();
        let v99_resolvable: ResolvableChainConfig = ResolvableChainConfig::from(&v3_resolvable);
        assert_eq!(expectation, v99_resolvable);
    }

    #[test]
    fn test_upgrade_chain_config_v1_chain_config_from_v99() {
        let expectation = v0_1::ChainConfig::default();
        let v99_chain_config = ChainConfig::default();
        let v1_chain_config = v0_1::ChainConfig::from(v99_chain_config);
        assert_eq!(expectation, v1_chain_config);
    }

    #[test]
    fn test_upgrade_chain_config_v3_chain_config_from_v99() {
        let expectation = v0_3::ChainConfig::default();
        let v99_chain_config = ChainConfig::default();
        let v3_chain_config = v0_3::ChainConfig::from(v99_chain_config);
        assert_eq!(expectation, v3_chain_config);
    }
}
