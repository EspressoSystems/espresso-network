use std::sync::Arc;

use bitvec::vec::BitVec;
use espresso_types::{v0_3::Validator, SeqTypes};
use hotshot::types::BLSPubKey;
use hotshot_query_service::explorer::{BlockDetail, ExplorerHistograms};
use hotshot_types::PeerConfig;
use serde::{Deserialize, Serialize};

use super::{client_id::ClientId, data_state::NodeIdentity};

/// [ServerMessage] represents the messages that the server can send to the
/// client for a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// This allows the client to know what client_id they have been assigned
    YouAre(ClientId),

    /// LatestBlock is a message that is meant to show the most recent block
    /// that has arrived.
    LatestBlock(Arc<BlockDetail<SeqTypes>>),

    /// LatestNodeIdentity is a message that is meant to show the most recent
    /// node identity that has arrived.
    LatestNodeIdentity(Arc<NodeIdentity>),

    /// LatestVoters is a message that is meant to show the most recent
    /// voters that have arrived.
    LatestVoters(BitVec<u16>),

    /// BlocksSnapshot is a message that is sent in response to a request for
    /// the snapshot of block information that is available.
    BlocksSnapshot(Arc<Vec<BlockDetail<SeqTypes>>>),

    /// NodeIdentitySnapshot is a message that is sent in response to a request
    /// for the snapshot of the current node identity information.
    NodeIdentitySnapshot(Arc<Vec<NodeIdentity>>),

    /// HistogramSnapshot is a message that is sent in response to a request
    /// for the snapshot of the current histogram information.
    HistogramSnapshot(Arc<ExplorerHistograms>),

    /// VotersSnapshot is a message that is sent in response to a request for
    /// the snapshot of the current voters information.
    VotersSnapshot(Arc<Vec<BitVec<u16>>>),

    // New Messages are added to the end of the numeration in order to
    // preserve existing enumeration values. This is done explicitly for
    // backwards compatibility.
    //
    /// LatestValidator is a message that is meant to show the most recent
    /// validator that has arrived.
    LatestValidator(Arc<Validator<BLSPubKey>>),

    /// LatestStakeTable is a message that is meant to show the most recent
    /// stake table that has arrived.
    LatestStakeTable(Arc<Vec<PeerConfig<SeqTypes>>>),

    /// ValidatorSnapshot is a message that is sent in response to a request
    /// for the snapshot of the current validators information.
    ValidatorsSnapshot(Arc<Vec<Validator<BLSPubKey>>>),

    /// StakeTableSnapshot is a message that is sent in response to a request
    /// for the snapshot of the current stake table information.
    StakeTableSnapshot(Arc<Vec<PeerConfig<SeqTypes>>>),

    // UnrecognizedRequest is a message that is sent when the server receives
    // a request that it does not recognize. This is useful for debugging and
    // for ensuring that the client is sending valid requests.
    UnrecognizedRequest(serde_json::Value),
}

impl PartialEq for ServerMessage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::YouAre(lhs), Self::YouAre(rhg)) => lhs == rhg,
            (Self::LatestBlock(lhs), Self::LatestBlock(rhs)) => lhs == rhs,
            (Self::LatestNodeIdentity(lhs), Self::LatestNodeIdentity(rhs)) => lhs == rhs,
            (Self::LatestVoters(lhs), Self::LatestVoters(rhs)) => lhs == rhs,
            (Self::LatestValidator(lhs), Self::LatestValidator(rhs)) => lhs == rhs,
            (Self::LatestStakeTable(lhs), Self::LatestStakeTable(rhs)) => lhs == rhs,
            (Self::BlocksSnapshot(lhs), Self::BlocksSnapshot(rhs)) => lhs == rhs,
            (Self::NodeIdentitySnapshot(lhs), Self::NodeIdentitySnapshot(rhs)) => lhs == rhs,
            (Self::HistogramSnapshot(_), Self::HistogramSnapshot(_)) => false,
            (Self::VotersSnapshot(lhs), Self::VotersSnapshot(rhs)) => lhs == rhs,
            (Self::ValidatorsSnapshot(lhs), Self::ValidatorsSnapshot(rhs)) => lhs == rhs,
            (Self::StakeTableSnapshot(lhs), Self::StakeTableSnapshot(rhs)) => lhs == rhs,
            (Self::UnrecognizedRequest(lhs), Self::UnrecognizedRequest(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {}
