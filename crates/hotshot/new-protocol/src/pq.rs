//! Post-quantum (SLH-DSA) signatures on upgrade votes, carried in a trailer after the message.
//!
//! Proof of concept. Nodes on 0.7.x decode messages with `bincode::deserialize`, which ignores
//! trailing bytes, so they read and count the vote as today and never see the trailer. New nodes
//! read the trailer from the end of the bytes, so they need not encode the message again.
//!
//! Trailer layout: `signature (SIGNATURE_LEN bytes) | length (u32, little endian) | TRAILER_TAG`.

use std::{collections::BTreeMap, sync::Arc};

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::UpgradeLock,
    simple_vote::{UpgradeProposalData2, UpgradeVote2, VersionedVoteData},
    slh_dsa,
    traits::node_implementation::NodeType,
};
use hotshot_utils::anytrace::{self, Result};
use parking_lot::Mutex;

/// FIPS 205 context string of an SLH-DSA signature on an upgrade vote.
pub const UPGRADE_VOTE_CONTEXT: &[u8] = b"espresso upgrade vote";

/// The last bytes of a trailer.
pub const TRAILER_TAG: [u8; 4] = *b"PQS1";

/// Append `signature` as a trailer after an encoded message.
pub fn append_trailer(message: &mut Vec<u8>, signature: &slh_dsa::Signature) {
    message.reserve(TRAILER_LEN);
    message.extend_from_slice(signature.as_bytes());
    message.extend_from_slice(&SIGNATURE_LEN_FIELD);
    message.extend_from_slice(&TRAILER_TAG);
}

/// The signature in the trailer at the end of `bytes`.
///
/// `None` when the bytes do not end with a well-formed trailer: too short, another tag, or a
/// length that is not `SIGNATURE_LEN`.
pub fn read_trailer(bytes: &[u8]) -> Option<slh_dsa::Signature> {
    let trailer = &bytes[bytes.len().checked_sub(TRAILER_LEN)?..];
    let (signature, footer) = trailer.split_at(slh_dsa::SIGNATURE_LEN);
    let (len, tag) = footer.split_at(SIGNATURE_LEN_FIELD.len());
    if tag != TRAILER_TAG || len != SIGNATURE_LEN_FIELD {
        return None;
    }
    slh_dsa::Signature::try_from(signature).ok()
}

/// Sign an upgrade vote with SLH-DSA, over the same commitment its BLS signature signs.
pub fn sign_upgrade_vote<T: NodeType>(
    keypair: &slh_dsa::Keypair,
    vote: &UpgradeVote2<T>,
    upgrade_lock: &UpgradeLock<T>,
) -> Result<slh_dsa::Signature> {
    let commit = vote_commitment(vote, upgrade_lock)?;
    Ok(keypair.sign(UPGRADE_VOTE_CONTEXT, commit.as_ref()))
}

/// Check an SLH-DSA signature made by [`sign_upgrade_vote`].
pub fn verify_upgrade_vote<T: NodeType>(
    key: &slh_dsa::PublicKey,
    vote: &UpgradeVote2<T>,
    signature: &slh_dsa::Signature,
    upgrade_lock: &UpgradeLock<T>,
) -> Result<()> {
    let commit = vote_commitment(vote, upgrade_lock)?;
    key.verify(UPGRADE_VOTE_CONTEXT, commit.as_ref(), signature)
        .map_err(|err| anytrace::warn!("SLH-DSA upgrade vote signature: {err}"))
}

/// The SLH-DSA key a voter registered, for an epoch.
pub trait PqKeys<T: NodeType>: Send + Sync {
    fn slh_dsa_key(
        &self,
        epoch: EpochNumber,
        voter: &T::SignatureKey,
    ) -> Option<slh_dsa::PublicKey>;
}

/// The SLH-DSA settings of a node.
#[derive(Clone)]
pub struct PqVoting<T: NodeType> {
    /// Signs this node's upgrade votes. Without one, the node sends votes without a trailer, as
    /// 0.7.x does.
    pub keypair: Option<Arc<slh_dsa::Keypair>>,
    /// The keys other nodes' trailers are checked against.
    pub keys: Arc<dyn PqKeys<T>>,
    /// The upgrade votes whose trailer verified.
    pub tally: PqTally<T>,
}

/// Per upgrade proposal view, the voters whose upgrade vote carried a valid SLH-DSA signature.
pub struct PqTally<T: NodeType>(Arc<Mutex<BTreeMap<ViewNumber, Vec<T::SignatureKey>>>>);

impl<T: NodeType> PqTally<T> {
    /// The voters with a valid SLH-DSA signature for the upgrade proposed at `view`, in the order
    /// their votes arrived.
    pub fn signers(&self, view: ViewNumber) -> Vec<T::SignatureKey> {
        self.0.lock().get(&view).cloned().unwrap_or_default()
    }

    /// Record a verified signer once per view: the network may deliver a vote more than once.
    /// Returns the number of signers recorded for `view`.
    pub(crate) fn record(&self, view: ViewNumber, voter: T::SignatureKey) -> usize {
        let mut tally = self.0.lock();
        let signers = tally.entry(view).or_default();
        if !signers.contains(&voter) {
            signers.push(voter);
        }
        signers.len()
    }
}

/// The whole trailer: signature, length field and tag.
const TRAILER_LEN: usize = slh_dsa::SIGNATURE_LEN + 8;

const SIGNATURE_LEN_FIELD: [u8; 4] = (slh_dsa::SIGNATURE_LEN as u32).to_le_bytes();

/// The commitment the vote's BLS signature signs.
fn vote_commitment<T: NodeType>(
    vote: &UpgradeVote2<T>,
    upgrade_lock: &UpgradeLock<T>,
) -> Result<Commitment<VersionedVoteData<T, UpgradeProposalData2>>> {
    Ok(VersionedVoteData::new(vote.data.clone(), vote.view_number, upgrade_lock)?.commit())
}

impl<T: NodeType> Clone for PqTally<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: NodeType> Default for PqTally<T> {
    fn default() -> Self {
        Self(Arc::default())
    }
}

#[cfg(test)]
mod tests {
    use committable::Committable;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        message::UpgradeLock,
        signature_key::BLSPubKey,
        simple_vote::{UpgradeVote2, VersionedVoteData},
        slh_dsa::{self, SIGNATURE_LEN},
        traits::signature_key::SignatureKey,
    };
    use versions::{TIMEOUT_EPOCH_VERSION, Upgrade, version};

    use super::{
        PqTally, TRAILER_TAG, UPGRADE_VOTE_CONTEXT, append_trailer, read_trailer,
        sign_upgrade_vote, verify_upgrade_vote,
    };
    use crate::{
        message::{ConsensusMessage, Message, MessageType, Unchecked, Validated},
        upgrade::expected_upgrade_data,
    };

    /// The trailer adds `SIGNATURE_LEN + 8` bytes and leaves the message bytes as they were:
    /// the signature, its length as a little-endian u32, then `TRAILER_TAG`.
    #[test]
    fn trailer_round_trips_with_pinned_layout() {
        let signature = filler_signature();
        let length = u32::try_from(SIGNATURE_LEN).unwrap().to_le_bytes();

        for message in [Vec::new(), b"an encoded message".to_vec()] {
            let mut bytes = message.clone();
            append_trailer(&mut bytes, &signature);

            assert_eq!(bytes.len(), message.len() + SIGNATURE_LEN + 8);
            let (head, trailer) = bytes.split_at(message.len());
            assert_eq!(head, message.as_slice());
            let (signature_bytes, rest) = trailer.split_at(SIGNATURE_LEN);
            assert_eq!(signature_bytes, signature.as_bytes());
            assert_eq!(rest[..4], length);
            assert_eq!(rest[4..], TRAILER_TAG);

            assert_eq!(read_trailer(&bytes), Some(signature.clone()));
        }
    }

    /// Bytes that do not end with a well-formed trailer have none: too short, another tag,
    /// another length, or a message encoded as 0.7.x encodes it.
    #[test]
    fn read_trailer_rejects_malformed_bytes() {
        let mut valid = b"an encoded message".to_vec();
        append_trailer(&mut valid, &filler_signature());
        assert!(read_trailer(&valid).is_some());
        let trailer_at = valid.len() - (SIGNATURE_LEN + 8);
        let length_at = valid.len() - 8;
        let tag_at = valid.len() - 4;

        assert_eq!(read_trailer(&[]), None);
        assert_eq!(read_trailer(&TRAILER_TAG), None);
        assert_eq!(read_trailer(&valid[length_at..]), None);
        assert_eq!(read_trailer(&valid[trailer_at + 1..]), None);

        for tag in [*b"PQS2", *b"pqs1", [0; 4]] {
            let mut bytes = valid.clone();
            bytes[tag_at..].copy_from_slice(&tag);
            assert_eq!(read_trailer(&bytes), None, "tag {tag:?}");
        }

        let signature_len = u32::try_from(SIGNATURE_LEN).unwrap();
        for length in [0, signature_len - 1, signature_len + 1, u32::MAX] {
            let mut bytes = valid.clone();
            bytes[length_at..tag_at].copy_from_slice(&length.to_le_bytes());
            assert_eq!(read_trailer(&bytes), None, "length {length}");
        }

        let lock = lock();
        let encoded = lock
            .serialize(&vote_message(upgrade_vote(7, 1, &lock)))
            .unwrap();
        assert_eq!(read_trailer(&encoded), None);
    }

    /// A 0.7.x node decodes with the unchanged `UpgradeLock::deserialize`. It reads an upgrade
    /// vote with a trailer exactly as the vote without one, and the BLS signature still verifies.
    #[test]
    fn old_decoder_reads_upgrade_vote_and_ignores_trailer() {
        let lock = lock();
        let message = vote_message(upgrade_vote(7, 1, &lock));
        let plain = lock.serialize(&message).unwrap();
        let mut with_trailer = plain.clone();
        append_trailer(&mut with_trailer, &filler_signature());

        let (expected, expected_version) = lock
            .deserialize::<Message<TestTypes, Unchecked>>(&plain)
            .unwrap();
        let (decoded, version) = lock
            .deserialize::<Message<TestTypes, Unchecked>>(&with_trailer)
            .unwrap();

        assert_eq!(decoded, expected);
        assert_eq!(version, expected_version);
        assert_eq!(decoded, message.into_unchecked());
        assert!(bls_signature_verifies(&decoded_vote(decoded), &lock));
    }

    /// The SLH-DSA signature signs, under `UPGRADE_VOTE_CONTEXT`, the commitment the vote's BLS
    /// signature signs. It verifies only with the signer's key, for that vote, unchanged.
    #[test]
    fn slh_dsa_signature_binds_key_and_vote() {
        let lock = lock();
        let vote = upgrade_vote(7, 1, &lock);
        let keypair = slh_dsa::Keypair::generate();
        let key = keypair.public_key();
        let signature = sign_upgrade_vote(&keypair, &vote, &lock).unwrap();

        let result = verify_upgrade_vote(&key, &vote, &signature, &lock);
        assert!(result.is_ok(), "{result:?}");
        let commit = VersionedVoteData::new(vote.data.clone(), vote.view_number, &lock)
            .unwrap()
            .commit();
        let result = key.verify(UPGRADE_VOTE_CONTEXT, commit.as_ref(), &signature);
        assert!(result.is_ok(), "{result:?}");

        let other_key = slh_dsa::Keypair::generate().public_key();
        assert!(verify_upgrade_vote(&other_key, &vote, &signature, &lock).is_err());

        let other_view = upgrade_vote(8, 1, &lock);
        assert!(verify_upgrade_vote(&key, &other_view, &signature, &lock).is_err());

        let other_data = upgrade_vote(7, 2, &lock);
        assert_ne!(other_data.data, vote.data);
        assert!(verify_upgrade_vote(&key, &other_data, &signature, &lock).is_err());

        let changed = flip_byte(&signature, SIGNATURE_LEN / 2);
        assert!(verify_upgrade_vote(&key, &vote, &changed, &lock).is_err());
    }

    /// A trailer whose signature has one flipped byte: the message still decodes with the
    /// unchanged decoder and its BLS signature still verifies; only the SLH-DSA check fails.
    #[test]
    fn broken_trailer_fails_only_the_slh_dsa_check() {
        let lock = lock();
        let vote = upgrade_vote(7, 1, &lock);
        let keypair = slh_dsa::Keypair::generate();
        let signature = sign_upgrade_vote(&keypair, &vote, &lock).unwrap();
        let broken = flip_byte(&signature, SIGNATURE_LEN / 2);
        let message = vote_message(vote);
        let mut bytes = lock.serialize(&message).unwrap();
        append_trailer(&mut bytes, &broken);

        let (decoded, _) = lock
            .deserialize::<Message<TestTypes, Unchecked>>(&bytes)
            .unwrap();
        assert_eq!(decoded, message.into_unchecked());
        let vote = decoded_vote(decoded);
        assert!(bls_signature_verifies(&vote, &lock));

        let trailer = read_trailer(&bytes);
        assert_eq!(trailer.as_ref(), Some(&broken));
        let key = keypair.public_key();
        assert!(verify_upgrade_vote(&key, &vote, &broken, &lock).is_err());
        let result = verify_upgrade_vote(&key, &vote, &signature, &lock);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn default_tally_has_no_signers() {
        let tally = PqTally::<TestTypes>::default();
        for view in [0, 1, 7, u64::MAX] {
            assert!(
                tally.signers(ViewNumber::new(view)).is_empty(),
                "view {view}"
            );
        }
    }

    /// The network runs 0.7 and votes on the next version.
    fn lock() -> UpgradeLock<TestTypes> {
        UpgradeLock::new(Upgrade::new(TIMEOUT_EPOCH_VERSION, version(0, 8)))
    }

    /// A BLS-signed upgrade vote of node 0 for the upgrade proposed at `view` in `epoch`.
    fn upgrade_vote(
        view: u64,
        epoch: u64,
        lock: &UpgradeLock<TestTypes>,
    ) -> UpgradeVote2<TestTypes> {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
        let view = ViewNumber::new(view);
        let data = expected_upgrade_data(&lock.upgrade(), view, EpochNumber::new(epoch)).unwrap();
        UpgradeVote2::create_signed_vote(data, view, &public_key, &private_key, lock).unwrap()
    }

    fn vote_message(vote: UpgradeVote2<TestTypes>) -> Message<TestTypes, Validated> {
        Message {
            sender: vote.signature.0,
            message_type: MessageType::Consensus(ConsensusMessage::UpgradeVote(vote)),
        }
    }

    fn decoded_vote(message: Message<TestTypes, Unchecked>) -> UpgradeVote2<TestTypes> {
        match message.message_type {
            MessageType::Consensus(ConsensusMessage::UpgradeVote(vote)) => vote,
            other => panic!("expected an upgrade vote, got {other:?}"),
        }
    }

    fn bls_signature_verifies(
        vote: &UpgradeVote2<TestTypes>,
        lock: &UpgradeLock<TestTypes>,
    ) -> bool {
        let commit = VersionedVoteData::new(vote.data.clone(), vote.view_number, lock)
            .unwrap()
            .commit();
        vote.signature
            .0
            .validate(&vote.signature.1, commit.as_ref())
    }

    /// A well-formed signature that no key made: the trailer format does not check it.
    fn filler_signature() -> slh_dsa::Signature {
        let bytes: Vec<u8> = (0..SIGNATURE_LEN).map(|i| (i % 251) as u8).collect();
        slh_dsa::Signature::try_from(bytes.as_slice()).unwrap()
    }

    fn flip_byte(signature: &slh_dsa::Signature, index: usize) -> slh_dsa::Signature {
        let mut bytes = signature.as_bytes().to_vec();
        bytes[index] ^= 0x01;
        slh_dsa::Signature::try_from(bytes.as_slice()).unwrap()
    }
}
