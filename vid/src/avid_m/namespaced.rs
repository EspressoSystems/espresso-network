//! This file implements the namespaced AvidM scheme.

use std::ops::Range;

use jf_merkle_tree::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

use super::{AvidMCommit, AvidMShare, RawAvidMShare};
use crate::{
    avid_m::{AvidMScheme, MerkleTree},
    VidError, VidResult, VidScheme,
};

/// Dummy struct for namespaced AvidM scheme
pub struct NsAvidMScheme;

/// Namespaced commitment type
pub type NsAvidMCommit = super::AvidMCommit;
/// Namespaced parameter type
pub type NsAvidMParam = super::AvidMParam;

/// Namespaced share for each storage node
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct NsAvidMShare {
    /// Index number of the given share.
    index: u32,
    /// The list of all namespace commitments
    ns_commits: Vec<AvidMCommit>,
    /// The size of each namespace
    ns_lens: Vec<usize>,
    /// Actual share content
    content: Vec<RawAvidMShare>,
}

impl NsAvidMShare {
    fn inner_ns_share(&self, ns_id: usize) -> AvidMShare {
        AvidMShare {
            index: self.index,
            payload_byte_len: self.ns_lens[ns_id],
            content: self.content[ns_id].clone(),
        }
    }

    /// Return the length of underlying payload in bytes
    pub fn payload_byte_len(&self) -> usize {
        self.ns_lens.iter().sum()
    }
}

impl NsAvidMScheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<NsAvidMParam> {
        NsAvidMParam::new(recovery_threshold, total_weights)
    }

    /// Commit to a payload given namespace table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn commit(
        param: &NsAvidMParam,
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<NsAvidMCommit> {
        let ns_commits = ns_table
            .into_iter()
            .map(|ns_range| {
                AvidMScheme::commit(param, &payload[ns_range]).map(|commit| commit.commit)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(NsAvidMCommit {
            commit: MerkleTree::from_elems(None, &ns_commits)
                .map_err(|err| VidError::Internal(err.into()))?
                .commitment(),
        })
    }

    /// Disperse a payload according to a distribution table and a namespace
    /// table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn ns_disperse(
        param: &NsAvidMParam,
        distribution: &[u32],
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<(NsAvidMCommit, Vec<NsAvidMShare>)> {
        let mut ns_commits = vec![];
        let mut disperses = vec![];
        let mut ns_lens = vec![];
        for ns_range in ns_table {
            ns_lens.push(ns_range.len());
            let (commit, shares) = AvidMScheme::disperse(param, distribution, &payload[ns_range])?;
            ns_commits.push(commit.commit);
            disperses.push(shares);
        }
        let commit = NsAvidMCommit {
            commit: MerkleTree::from_elems(None, &ns_commits)
                .map_err(|err| VidError::Internal(err.into()))?
                .commitment(),
        };
        let ns_commits: Vec<_> = ns_commits
            .into_iter()
            .map(|comm| AvidMCommit { commit: comm })
            .collect();
        let mut shares = vec![NsAvidMShare::default(); disperses[0].len()];
        shares.iter_mut().for_each(|share| {
            share.index = disperses[0][0].index;
            share.ns_commits = ns_commits.clone();
            share.ns_lens = ns_lens.clone();
        });
        disperses.into_iter().for_each(|ns_disperse| {
            shares
                .iter_mut()
                .zip(ns_disperse)
                .for_each(|(share, ns_share)| share.content.push(ns_share.content))
        });
        Ok((commit, shares))
    }

    /// Verify a namespaced share
    pub fn verify_share(
        param: &NsAvidMParam,
        commit: &NsAvidMCommit,
        share: &NsAvidMShare,
    ) -> VidResult<crate::VerificationResult> {
        if !(share.ns_commits.len() == share.ns_lens.len()
            && share.ns_commits.len() == share.content.len())
        {
            return Err(VidError::InvalidShare);
        }
        // Verify the share for each namespace
        for (commit, content) in share.ns_commits.iter().zip(share.content.iter()) {
            if AvidMScheme::verify_internal(param, commit, content)?.is_err() {
                return Ok(Err(()));
            }
        }
        // Verify the namespace MT commitment
        let expected_commit = NsAvidMCommit {
            commit: MerkleTree::from_elems(
                None,
                share.ns_commits.iter().map(|commit| commit.commit),
            )
            .map_err(|err| VidError::Internal(err.into()))?
            .commitment(),
        };
        Ok(if &expected_commit == commit {
            Ok(())
        } else {
            Err(())
        })
    }

    /// Recover the entire payload from enough share
    pub fn recover(param: &NsAvidMParam, shares: &[NsAvidMShare]) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        let mut result = vec![];
        for ns_id in 0..shares[0].ns_lens.len() {
            result.append(&mut Self::ns_recover(param, ns_id, shares)?)
        }
        Ok(result)
    }

    /// Recover the payload for a given namespace.
    /// Given namespace ID should be valid for all shares, i.e. `ns_commits` and `content` have
    /// at least `ns_id` elements for all shares.
    pub fn ns_recover(
        param: &NsAvidMParam,
        ns_id: usize,
        shares: &[NsAvidMShare],
    ) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        if shares
            .iter()
            .any(|share| ns_id >= share.ns_lens.len() || ns_id >= share.content.len())
        {
            return Err(VidError::IndexOutOfBound);
        }
        let ns_commit = shares[0].ns_commits[ns_id];
        let shares: Vec<_> = shares
            .iter()
            .map(|share| share.inner_ns_share(ns_id))
            .collect();
        AvidMScheme::recover(param, &ns_commit, &shares)
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use crate::avid_m::namespaced::NsAvidMScheme;

    #[test]
    fn round_trip() {
        // play with these items
        let num_storage_nodes = 9;
        let recovery_threshold = 3;
        let ns_lens = [15, 33];
        let ns_table = [(0usize..15), (15..48)];
        let payload_byte_len = ns_lens.iter().sum();

        // more items as a function of the above

        let mut rng = jf_utils::test_rng();

        let weights: Vec<u32> = (0..num_storage_nodes)
            .map(|_| rng.next_u32() % 5 + 1)
            .collect();
        let total_weights: u32 = weights.iter().sum();
        let params = NsAvidMScheme::setup(recovery_threshold, total_weights as usize).unwrap();

        println!(
            "recovery_threshold:: {} num_storage_nodes: {} payload_byte_len: {}",
            recovery_threshold, num_storage_nodes, payload_byte_len
        );
        println!("weights: {:?}", weights);

        let payload = {
            let mut bytes_random = vec![0u8; payload_byte_len];
            rng.fill_bytes(&mut bytes_random);
            bytes_random
        };

        let (commit, mut shares) =
            NsAvidMScheme::ns_disperse(&params, &weights, &payload, ns_table.iter().cloned())
                .unwrap();

        assert_eq!(shares.len(), num_storage_nodes);

        assert_eq!(
            commit,
            NsAvidMScheme::commit(&params, &payload, ns_table.iter().cloned()).unwrap()
        );

        // verify shares
        shares.iter().for_each(|share| {
            assert!(NsAvidMScheme::verify_share(&params, &commit, share).is_ok_and(|r| r.is_ok()))
        });

        // test payload recovery on a random subset of shares
        shares.shuffle(&mut rng);
        let mut cumulated_weights = 0;
        let mut cut_index = 0;
        while cumulated_weights <= recovery_threshold {
            cumulated_weights += shares[cut_index].content[0].range.len();
            cut_index += 1;
        }
        let ns0_payload_recovered =
            NsAvidMScheme::ns_recover(&params, 0, &shares[..cut_index]).unwrap();
        assert_eq!(ns0_payload_recovered[..], payload[ns_table[0].clone()]);
        let ns1_payload_recovered =
            NsAvidMScheme::ns_recover(&params, 1, &shares[..cut_index]).unwrap();
        assert_eq!(ns1_payload_recovered[..], payload[ns_table[1].clone()]);
        let payload_recovered = NsAvidMScheme::recover(&params, &shares[..cut_index]).unwrap();
        assert_eq!(payload_recovered, payload);
    }
}
