use std::fmt;

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::traits::node_implementation::NodeType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use versions::{TX_DIGEST_VERSION, Version};

/// Identifies a transaction inside the block builder: pooling, forwarding and dedup.
///
/// Holds either a [`keccak`] or a [`blake3`] digest. Blocks of a view running
/// [`TX_DIGEST_VERSION`] or later are identified by [`blake3`], earlier ones by [`keccak`].
/// Neither appears in a header, a signature or an API, so changing the scheme only touches
/// consensus.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TxDigest(pub [u8; 32]);

impl fmt::Display for TxDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for TxDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TxDigest({self})")
    }
}

impl<T> From<Commitment<T>> for TxDigest
where
    T: Committable,
{
    fn from(commitment: Commitment<T>) -> TxDigest {
        TxDigest(commitment.into())
    }
}

/// Whether blocks of a view running `version` are identified by [`blake3`].
pub fn uses_blake3(version: Version) -> bool {
    version >= TX_DIGEST_VERSION
}

/// The transaction's [`Committable`] commitment, the hash users and rollups see.
pub fn keccak<Tx>(tx: &Tx) -> TxDigest
where
    Tx: Committable,
{
    tx.commit().into()
}

/// BLAKE3 over the transaction's bincode encoding.
pub fn blake3<Tx>(tx: &Tx) -> TxDigest
where
    Tx: Serialize,
{
    let mut hasher = blake3::Hasher::new_derive_key("hotshot new-protocol tx digest v1");
    // Every node must hash the same bytes, so this is plain bincode and never the
    // version-dependent encoding the network layer uses.
    bincode::serialize_into(&mut hasher, tx).expect("transactions serialize");
    TxDigest(*hasher.finalize().as_bytes())
}

/// The digest of each transaction in a block of a view running `version`, in
/// [`BlockPayload::transactions`] order.
pub fn block_digests<T>(
    payload: &T::BlockPayload,
    metadata: &<T::BlockPayload as BlockPayload<T>>::Metadata,
    version: Version,
) -> Vec<TxDigest>
where
    T: NodeType,
{
    if uses_blake3(version) {
        let transactions = payload.transactions(metadata).collect::<Vec<_>>();
        transactions.par_iter().map(blake3).collect()
    } else {
        payload
            .transaction_commitments(metadata)
            .into_iter()
            .map(TxDigest::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestMetadata, TestTransaction},
        node_types::TestTypes,
    };
    use versions::NEW_PROTOCOL_VERSION;

    use super::*;

    #[test]
    fn keccak_is_the_commitment() {
        let tx = TestTransaction::new(Vec::from([1, 2, 3]));
        assert_eq!(keccak(&tx).0, <[u8; 32]>::from(tx.commit()));
    }

    #[test]
    fn blake3_hashes_the_bincode_encoding() {
        let tx = TestTransaction::new(Vec::from([1, 2, 3]));
        let mut hasher = blake3::Hasher::new_derive_key("hotshot new-protocol tx digest v1");
        hasher.update(&bincode::serialize(&tx).unwrap());
        assert_eq!(blake3(&tx).0, *hasher.finalize().as_bytes());
        assert_ne!(blake3(&tx), keccak(&tx));
    }

    #[test]
    fn block_digests_follow_the_version_in_transaction_order() {
        let txs = (0..64u8)
            .map(|i| TestTransaction::new(Vec::from([i; 3])))
            .collect::<Vec<_>>();
        let metadata = TestMetadata {
            num_transactions: txs.len() as u64,
        };
        let payload = TestBlockPayload {
            transactions: txs.clone(),
        };

        let old = block_digests::<TestTypes>(&payload, &metadata, NEW_PROTOCOL_VERSION);
        let new = block_digests::<TestTypes>(&payload, &metadata, TX_DIGEST_VERSION);

        assert_eq!(old, txs.iter().map(keccak).collect::<Vec<_>>());
        assert_eq!(new, txs.iter().map(blake3).collect::<Vec<_>>());
    }
}
