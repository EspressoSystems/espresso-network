// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Vote, Accumulator, and Certificate Types

use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    marker::PhantomData,
    num::NonZeroU64,
};

use bitvec::{bitvec, vec::BitVec};
use committable::{Commitment, Committable};
use hotshot_utils::anytrace::*;
use primitive_types::U256;
use tracing::error;

use crate::{
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_certificate::Threshold,
    simple_vote::{VersionedVoteData, Voteable},
    traits::{
        node_implementation::{NodeType, Versions},
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig, StakeTableEntries,
};

/// A simple vote that has a signer and commitment to the data voted on.
pub trait Vote<TYPES: NodeType>: HasViewNumber<TYPES> {
    /// Type of data commitment this vote uses.
    type Commitment: Voteable<TYPES>;

    /// Get the signature of the vote sender
    fn signature(&self) -> <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType;
    /// Gets the data which was voted on by this vote
    fn date(&self) -> &Self::Commitment;
    /// Gets the Data commitment of the vote
    fn data_commitment(&self) -> Commitment<Self::Commitment>;

    /// Gets the public signature key of the votes creator/sender
    fn signing_key(&self) -> TYPES::SignatureKey;
}

/// Any type that is associated with a view
pub trait HasViewNumber<TYPES: NodeType> {
    /// Returns the view number the type refers to.
    fn view_number(&self) -> TYPES::View;
}

/**
The certificate formed from the collection of signatures a committee.
The committee is defined by the `Membership` associated type.
The votes all must be over the `Commitment` associated type.
*/
pub trait Certificate<TYPES: NodeType, T>: HasViewNumber<TYPES> {
    /// The data commitment this certificate certifies.
    type Voteable: Voteable<TYPES>;

    /// Threshold Functions
    type Threshold: Threshold<TYPES>;

    /// Build a certificate from the data commitment and the quorum of signers
    fn create_signed_certificate<V: Versions>(
        vote_commitment: Commitment<VersionedVoteData<TYPES, Self::Voteable, V>>,
        data: Self::Voteable,
        sig: <TYPES::SignatureKey as SignatureKey>::QcType,
        view: TYPES::View,
    ) -> Self;

    /// Checks if the cert is valid in the given epoch
    fn is_valid_cert<V: Versions>(
        &self,
        stake_table: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry>,
        threshold: NonZeroU64,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> impl std::future::Future<Output = Result<()>>;
    /// Returns the amount of stake needed to create this certificate
    // TODO: Make this a static ratio of the total stake of `Membership`
    fn threshold(membership: &EpochMembership<TYPES>) -> impl Future<Output = u64> + Send;

    /// Get  Stake Table from Membership implementation.
    fn stake_table(
        membership: &EpochMembership<TYPES>,
    ) -> impl Future<Output = Vec<PeerConfig<TYPES::SignatureKey>>> + Send;

    /// Get Total Nodes from Membership implementation.
    fn total_nodes(membership: &EpochMembership<TYPES>) -> impl Future<Output = usize> + Send;

    /// Get  `StakeTableEntry` from Membership implementation.
    fn stake_table_entry(
        membership: &EpochMembership<TYPES>,
        pub_key: &TYPES::SignatureKey,
    ) -> impl Future<Output = Option<PeerConfig<TYPES::SignatureKey>>> + Send;

    /// Get the commitment which was voted on
    fn data(&self) -> &Self::Voteable;

    /// Get the vote commitment which the votes commit to
    fn data_commitment<V: Versions>(
        &self,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> impl std::future::Future<Output = Result<Commitment<VersionedVoteData<TYPES, Self::Voteable, V>>>>;
}
/// Mapping of vote commitment to signatures and bitvec
type SignersMap<COMMITMENT, KEY> = HashMap<
    COMMITMENT,
    (
        BitVec,
        Vec<<KEY as SignatureKey>::PureAssembledSignatureType>,
    ),
>;

#[allow(clippy::type_complexity)]
/// Accumulates votes until a certificate is formed.  This implementation works for all simple vote and certificate pairs
pub struct VoteAccumulator<
    TYPES: NodeType,
    VOTE: Vote<TYPES>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
    V: Versions,
> {
    /// Map of all signatures accumulated so far
    pub vote_outcomes: VoteMap2<
        Commitment<VersionedVoteData<TYPES, <VOTE as Vote<TYPES>>::Commitment, V>>,
        TYPES::SignatureKey,
        <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    >,
    /// A bitvec to indicate which node is active and send out a valid signature for certificate aggregation, this automatically do uniqueness check
    /// And a list of valid signatures for certificate aggregation
    pub signers: SignersMap<
        Commitment<VersionedVoteData<TYPES, <VOTE as Vote<TYPES>>::Commitment, V>>,
        TYPES::SignatureKey,
    >,
    /// Phantom data to specify the types this accumulator is for
    pub phantom: PhantomData<(TYPES, VOTE, CERT)>,
    /// version information
    pub upgrade_lock: UpgradeLock<TYPES, V>,
}

impl<
        TYPES: NodeType,
        VOTE: Vote<TYPES>,
        CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
        V: Versions,
    > VoteAccumulator<TYPES, VOTE, CERT, V>
{
    /// Add a vote to the total accumulated votes for the given epoch.
    /// Returns the accumulator or the certificate if we
    /// have accumulated enough votes to exceed the threshold for creating a certificate.
    pub async fn accumulate(
        &mut self,
        vote: &VOTE,
        membership: EpochMembership<TYPES>,
    ) -> Option<CERT> {
        let key = vote.signing_key();

        let vote_commitment = match VersionedVoteData::new(
            vote.date().clone(),
            vote.view_number(),
            &self.upgrade_lock,
        )
        .await
        {
            Ok(data) => data.commit(),
            Err(e) => {
                tracing::warn!("Failed to generate versioned vote data: {e}");
                return None;
            },
        };

        if !key.validate(&vote.signature(), vote_commitment.as_ref()) {
            error!("Invalid vote! Vote Data {:?}", vote.date());
            return None;
        }

        let stake_table_entry = CERT::stake_table_entry(&membership, &key).await?;
        let stake_table = CERT::stake_table(&membership).await;
        let total_nodes = CERT::total_nodes(&membership).await;
        let threshold = CERT::threshold(&membership).await;

        let vote_node_id = stake_table
            .iter()
            .position(|x| *x == stake_table_entry.clone())?;

        let original_signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType =
            vote.signature();

        let (total_stake_casted, total_vote_map) = self
            .vote_outcomes
            .entry(vote_commitment)
            .or_insert_with(|| (U256::from(0), BTreeMap::new()));

        // Check for duplicate vote
        if total_vote_map.contains_key(&key) {
            return None;
        }
        let (signers, sig_list) = self
            .signers
            .entry(vote_commitment)
            .or_insert((bitvec![0; total_nodes], Vec::new()));
        if signers.get(vote_node_id).as_deref() == Some(&true) {
            error!("Node id is already in signers list");
            return None;
        }
        signers.set(vote_node_id, true);
        sig_list.push(original_signature);

        *total_stake_casted += stake_table_entry.stake_table_entry.stake();
        total_vote_map.insert(key, (vote.signature(), vote_commitment));

        if *total_stake_casted >= threshold.into() {
            // Assemble QC
            let real_qc_pp: <<TYPES as NodeType>::SignatureKey as SignatureKey>::QcParams =
                <TYPES::SignatureKey as SignatureKey>::public_parameter(
                    StakeTableEntries::<TYPES>::from(stake_table).0,
                    U256::from(threshold),
                );

            let real_qc_sig = <TYPES::SignatureKey as SignatureKey>::assemble(
                &real_qc_pp,
                signers.as_bitslice(),
                &sig_list[..],
            );

            let cert = CERT::create_signed_certificate::<V>(
                vote_commitment,
                vote.date().clone(),
                real_qc_sig,
                vote.view_number(),
            );
            return Some(cert);
        }
        None
    }
}

/// Mapping of commitments to vote tokens by key.
type VoteMap2<COMMITMENT, PK, SIG> = HashMap<COMMITMENT, (U256, BTreeMap<PK, (SIG, COMMITMENT)>)>;
