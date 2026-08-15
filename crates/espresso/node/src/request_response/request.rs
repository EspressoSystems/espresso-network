use anyhow::{Context, Result};
use committable::Commitment;
use espresso_types::{
    Certificate2, FeeAccount, FeeMerkleTree, Leaf2,
    v0_3::{ChainConfig, EventKey, RewardAccountV1, RewardMerkleTreeV1, StakeTableEvent},
    v0_4::{RewardAccountV2, RewardMerkleTreeV2},
};
use hotshot_types::{data::VidShare, simple_certificate::LightClientStateUpdateCertificateV2};
use request_response::{Serializable, request::Request as RequestTrait};
use serde::{Deserialize, Serialize};

use crate::{SeqTypes, api::BlocksFrontier};

// Some type aliases for readability
type Height = u64;
type ViewNumber = u64;
type RequestId = u64;

/// The outermost request type. This an enum that contains all the possible requests that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// A request for the accounts at a given height and view
    Accounts(Height, ViewNumber, Vec<FeeAccount>),
    /// A request for the leaf chain at a given height
    Leaf(Height),
    /// A request for a chain config with a particular commitment
    ChainConfig(Commitment<ChainConfig>),
    /// A request for the blocks frontier
    BlocksFrontier(Height, ViewNumber),
    /// A request for the reward accounts at a given height and view
    RewardAccountsV2(Height, ViewNumber, Vec<RewardAccountV2>),
    /// A request for the v1 reward accounts at a given height and view
    RewardAccountsV1(Height, ViewNumber, Vec<RewardAccountV1>),
    /// A request for the VID share at the given block height
    VidShare(Height, RequestId),
    /// A request for the state certificate at a given epoch
    StateCert(u64),
    /// A request for data to reconstruct the reward merkle tree at a given height
    RewardMerkleTreeV2(u64, ViewNumber),
    /// A request for the cert2 at or above the given height
    Cert2(Height),
    /// A request for the stake table events with L1 block numbers in the given
    /// (inclusive) range
    StakeTableEvents {
        from_l1_block: u64,
        to_l1_block: u64,
    },
}

/// The outermost response type. This an enum that contains all the possible responses that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// A response for the accounts at a given height and view
    Accounts(FeeMerkleTree),
    /// A request for the leaf chain at a given height
    Leaf(Vec<Leaf2>),
    /// A response for a chain config with a particular commitment
    ChainConfig(ChainConfig),
    /// A response for the blocks frontier
    BlocksFrontier(BlocksFrontier),
    /// A response for the reward accounts at a given height and view
    RewardAccountsV2(RewardMerkleTreeV2),
    /// A response for the v1 reward accounts at a given height and view
    RewardAccountsV1(RewardMerkleTreeV1),
    /// A response for a VID share at the given block height
    VidShare(VidShare),
    /// A response for a state certificate at a given epoch
    StateCert(LightClientStateUpdateCertificateV2<SeqTypes>),
    /// A response with data to reconstruct the reward merkle tree at a given height
    RewardMerkleTreeV2(#[serde(with = "serde_bytes")] Vec<u8>),
    /// A response with the earliest cert2 (fast finality protocol)
    Cert2(Certificate2<SeqTypes>),
    /// A response with the stake table events in the requested L1 block range
    StakeTableEvents(Vec<(EventKey, StakeTableEvent)>),
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
impl RequestTrait for Request {
    type Response = Response;

    fn validate(&self) -> Result<()> {
        match self {
            Self::StakeTableEvents {
                from_l1_block,
                to_l1_block,
            } => {
                anyhow::ensure!(
                    from_l1_block <= to_l1_block,
                    "invalid stake table events range [{from_l1_block}, {to_l1_block}]"
                );
                Ok(())
            },
            // All other requests are valid
            _ => Ok(()),
        }
    }
}

/// Implement the `Serializable` trait for the `Request` type. This tells the request response
/// protocol how to serialize and deserialize the request
impl Serializable for Request {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}

/// Implement the `Serializable` trait for the `Response` type. This tells the request response
/// protocol how to serialize and deserialize the response.
impl Serializable for Response {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use espresso_types::testing::TestValidator;
    use hotshot_contract_adapter::sol_types::StakeTableV3::{Delegated, ValidatorRegisteredV2};

    use super::*;

    /// The new stake table event types must survive the request-response wire format
    /// (bincode), which is stricter than the serde_json used for their persistence.
    #[test]
    fn stake_table_events_roundtrip() {
        let val = TestValidator::random();
        let events: Vec<(EventKey, StakeTableEvent)> = vec![
            ((10, 0), ValidatorRegisteredV2::from(&val).into()),
            (
                (20, 1),
                Delegated {
                    delegator: Address::random(),
                    validator: val.account,
                    amount: U256::from(123),
                }
                .into(),
            ),
        ];

        let request = Request::StakeTableEvents {
            from_l1_block: 1,
            to_l1_block: 100,
        };
        let decoded = Request::from_bytes(&request.to_bytes().unwrap()).unwrap();
        let Request::StakeTableEvents {
            from_l1_block: 1,
            to_l1_block: 100,
        } = decoded
        else {
            panic!("request did not roundtrip: {decoded:?}");
        };

        let response = Response::StakeTableEvents(events.clone());
        let decoded = Response::from_bytes(&response.to_bytes().unwrap()).unwrap();
        let Response::StakeTableEvents(decoded_events) = decoded else {
            panic!("response did not roundtrip");
        };
        assert_eq!(decoded_events, events);
    }

    /// An inverted block range must fail request validation.
    #[test]
    fn stake_table_events_range_validation() {
        use request_response::request::Request as _;

        assert!(
            Request::StakeTableEvents {
                from_l1_block: 5,
                to_l1_block: 4,
            }
            .validate()
            .is_err()
        );
        assert!(
            Request::StakeTableEvents {
                from_l1_block: 4,
                to_l1_block: 4,
            }
            .validate()
            .is_ok()
        );
    }
}
