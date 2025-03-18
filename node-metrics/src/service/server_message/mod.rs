use std::sync::Arc;

use bitvec::vec::BitVec;
use espresso_types::SeqTypes;
use hotshot_query_service::explorer::{BlockDetail, ExplorerHistograms};
use serde::{Deserialize, Serialize};

use super::{client_id::ClientId, data_state::NodeIdentity};

/// [ServerMessage] represents the messages that the server can send to the
/// client for a response.
#[derive(Debug, Serialize, Deserialize)]
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
}

impl PartialEq for ServerMessage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::YouAre(lhs), Self::YouAre(rhg)) => lhs == rhg,
            (Self::LatestBlock(lhs), Self::LatestBlock(rhs)) => lhs == rhs,
            (Self::LatestNodeIdentity(lhs), Self::LatestNodeIdentity(rhs)) => lhs == rhs,
            (Self::LatestVoters(lhs), Self::LatestVoters(rhs)) => lhs == rhs,
            (Self::BlocksSnapshot(lhs), Self::BlocksSnapshot(rhs)) => lhs == rhs,
            (Self::NodeIdentitySnapshot(lhs), Self::NodeIdentitySnapshot(rhs)) => lhs == rhs,
            (Self::HistogramSnapshot(_), Self::HistogramSnapshot(_)) => false,
            (Self::VotersSnapshot(lhs), Self::VotersSnapshot(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {}
