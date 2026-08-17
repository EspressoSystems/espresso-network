use anyhow::{Context, Result};
use committable::Commitment;
use espresso_types::{
    Certificate2, FeeAccount, FeeMerkleTree, Leaf2, StakeTableState,
    v0_3::{ChainConfig, RewardAccountV1, RewardMerkleTreeV1},
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
    /// A request for the full stake table state committed to at the given epoch
    StakeTableState { epoch: u64 },
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
    /// A response with the full stake table state committed to at the requested epoch
    StakeTableState(StakeTableState),
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
impl RequestTrait for Request {
    type Response = Response;

    fn validate(&self) -> Result<()> {
        match self {
            Self::Accounts(..) => Ok(()),
            Self::Leaf(..) => Ok(()),
            Self::ChainConfig(..) => Ok(()),
            Self::BlocksFrontier(..) => Ok(()),
            Self::RewardAccountsV2(..) => Ok(()),
            Self::RewardAccountsV1(..) => Ok(()),
            Self::VidShare(..) => Ok(()),
            Self::StateCert(..) => Ok(()),
            Self::RewardMerkleTreeV2(..) => Ok(()),
            Self::Cert2(..) => Ok(()),
            Self::StakeTableState { epoch } => {
                // Below epoch 2, `stake_table_snapshot_root_height` has no epoch root to
                // point at (it subtracts 2 from `epoch`).
                anyhow::ensure!(
                    *epoch >= 2,
                    "stake table state is only available from epoch 2 onwards, got {epoch}"
                );
                Ok(())
            },
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
    use committable::Committable as _;
    use espresso_types::{
        stake_table_state_from_l1_events, testing::TestValidator, v0_3::StakeTableEvent,
    };
    use hotshot_contract_adapter::sol_types::StakeTableV3::{
        ValidatorExitV2, ValidatorRegisteredV2, ValidatorRegisteredV3,
    };

    use super::*;

    fn deregister_event(account: Address) -> StakeTableEvent {
        StakeTableEvent::DeregisterV2(ValidatorExitV2 {
            validator: account,
            unlocksAt: U256::ZERO,
        })
    }

    /// A fully populated `StakeTableState` (all five fields non-empty) must survive the
    /// request-response wire format with its commitment intact; equality alone would not
    /// catch a field dropped symmetrically on both sides.
    #[test]
    fn stake_table_state_roundtrip() {
        let registered = TestValidator::random();
        let exited = TestValidator::random();

        let events = vec![
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&registered)),
            StakeTableEvent::RegisterV2(ValidatorRegisteredV2::from(&exited)),
            deregister_event(exited.account),
        ];
        let state = stake_table_state_from_l1_events(events).unwrap();
        assert!(!state.validators().is_empty());
        assert!(!state.validator_exits().is_empty());
        assert!(!state.used_bls_keys().is_empty());
        assert!(!state.used_schnorr_keys().is_empty());
        assert!(!state.used_x25519_keys().is_empty());

        let response = Response::StakeTableState(state.clone());
        let decoded = Response::from_bytes(&response.to_bytes().unwrap()).unwrap();
        let Response::StakeTableState(decoded_state) = decoded else {
            panic!("response did not roundtrip");
        };
        assert_eq!(decoded_state, state);
        assert_eq!(decoded_state.commit(), state.commit());
    }

    /// Map and set ordering must not affect the commitment: `commit()` sorts every field, so
    /// the same set of events applied in a different order must produce the same hash after a
    /// serde round trip.
    #[test]
    fn stake_table_state_commit_independent_of_insertion_order() {
        let val_a = TestValidator::random();
        let val_b = TestValidator::random();
        let val_c = TestValidator::random();

        let forward = vec![
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_a)),
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_b)),
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_c)),
            deregister_event(val_b.account),
        ];
        let backward = vec![
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_c)),
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_b)),
            StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&val_a)),
            deregister_event(val_b.account),
        ];

        let state_forward = stake_table_state_from_l1_events(forward).unwrap();
        let state_backward = stake_table_state_from_l1_events(backward).unwrap();

        let roundtrip = |state: StakeTableState| {
            let response = Response::StakeTableState(state);
            let decoded = Response::from_bytes(&response.to_bytes().unwrap()).unwrap();
            let Response::StakeTableState(decoded_state) = decoded else {
                panic!("response did not roundtrip");
            };
            decoded_state.commit()
        };

        assert_eq!(roundtrip(state_forward), roundtrip(state_backward));
    }

    /// Epochs below 2 have no snapshot root to derive a stake table from.
    #[test]
    fn stake_table_state_epoch_validation() {
        use request_response::request::Request as _;

        assert!(Request::StakeTableState { epoch: 0 }.validate().is_err());
        assert!(Request::StakeTableState { epoch: 1 }.validate().is_err());
        assert!(Request::StakeTableState { epoch: 2 }.validate().is_ok());
    }
}
