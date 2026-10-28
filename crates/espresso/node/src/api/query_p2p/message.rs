use std::ops::Range;

use ::light_client::consensus::{leaf::LeafProof, payload::PayloadProof};
use espresso_types::SeqTypes;
use hotshot_query_service::availability::LeafQueryData;
use serde::{Deserialize, Serialize};

/// A message on the query lane.
///
/// A requester picks `id`, and the reply carries it back. Replies are matched by sender and id,
/// so only the peer that was asked can answer.
#[derive(Debug, Serialize, Deserialize)]
pub(super) enum QueryMessage {
    Request { id: u64, request: QueryRequest },
    Reply { id: u64, reply: QueryReply },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum QueryRequest {
    /// A finality proof for the leaf at `height`, as the light client API serves it.
    LeafProof { height: u64, finalized: Option<u64> },
    /// The leaves at the heights in `ranges`, unproven.
    Leaves { ranges: Vec<Range<u64>> },
    /// Payload proofs for the heights in `ranges`, served a page at a time.
    PayloadProofs { ranges: Vec<Range<u64>> },
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum QueryReply {
    LeafProof(LeafProof),
    Leaves(Vec<LeafQueryData<SeqTypes>>),
    /// Proofs for the first requested heights. `next` is the first height left out, if any.
    PayloadProofs {
        proofs: Vec<PayloadProof>,
        next: Option<u64>,
    },
    /// The peer cannot serve the request: it is busy, or lacks the data.
    Unavailable,
}

impl QueryMessage {
    pub(super) fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<QueryMessage> {
        Ok(bincode::deserialize(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Peers on different releases must agree on the encoding, so variant order is fixed.
    #[test]
    #[expect(clippy::single_range_in_vec_init, reason = "a list holding one range")]
    fn encoding_is_stable() {
        let request = QueryMessage::Request {
            id: 7,
            request: QueryRequest::Leaves {
                ranges: Vec::from([3..5]),
            },
        };
        let reply = QueryMessage::Reply {
            id: 7,
            reply: QueryReply::Unavailable,
        };

        assert_eq!(
            request.encode().unwrap(),
            Vec::from([
                0, 0, 0, 0, // Request
                7, 0, 0, 0, 0, 0, 0, 0, // id
                1, 0, 0, 0, // Leaves
                1, 0, 0, 0, 0, 0, 0, 0, // one range
                3, 0, 0, 0, 0, 0, 0, 0, // start
                5, 0, 0, 0, 0, 0, 0, 0, // end
            ])
        );
        assert_eq!(
            reply.encode().unwrap(),
            Vec::from([
                1, 0, 0, 0, // Reply
                7, 0, 0, 0, 0, 0, 0, 0, // id
                3, 0, 0, 0, // Unavailable
            ])
        );
    }
}
