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
    pub(crate) index: u32,
    /// The list of all namespace commitments
    pub(crate) ns_commits: Vec<AvidMCommit>,
    /// The size of each namespace
    pub(crate) ns_lens: Vec<usize>,
    /// Actual share content
    pub(crate) content: Vec<RawAvidMShare>,
}

impl NsAvidMShare {
    /// Return the number of namespaces.
    /// WARN: it assume that the share is well formed, i.e. `ns_commits`, `ns_lens`, and `content` have the same length.
    pub fn num_nss(&self) -> usize {
        self.ns_commits.len()
    }

    /// Return the inner share for a given namespace if there exists one.
    pub fn inner_ns_share(&self, ns_index: usize) -> Option<AvidMShare> {
        if ns_index >= self.ns_lens.len() || ns_index >= self.content.len() {
            return None;
        }
        Some(AvidMShare {
            index: self.index,
            payload_byte_len: self.ns_lens[ns_index],
            content: self.content[ns_index].clone(),
        })
    }

    /// Return the length of underlying payload in bytes
    pub fn payload_byte_len(&self) -> usize {
        self.ns_lens.iter().sum()
    }

    /// Peek if the share contains a given namespace.
    pub fn contains_ns(&self, ns_index: usize) -> bool {
        self.ns_commits.len() > ns_index
            && self.ns_lens.len() > ns_index
            && self.content.len() > ns_index
    }

    /// Return the list of namespace commitments.
    pub fn ns_commits(&self) -> &[AvidMCommit] {
        &self.ns_commits
    }

    /// Return the list of namespace byte lengths.
    pub fn ns_lens(&self) -> &[usize] {
        &self.ns_lens
    }

    /// Return the commitment for a given namespace.
    /// WARN: will panic if `ns_index` is out of bound.
    pub fn ns_commit(&self, ns_index: usize) -> &AvidMCommit {
        &self.ns_commits[ns_index]
    }

    /// Return the byte length of a given namespace.
    /// WARN: will panic if `ns_index` is out of bound.
    pub fn ns_len(&self, ns_index: usize) -> usize {
        self.ns_lens[ns_index]
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
        for ns_index in 0..shares[0].ns_lens.len() {
            result.append(&mut Self::ns_recover(param, ns_index, shares)?)
        }
        Ok(result)
    }

    /// Recover the payload for a given namespace.
    /// Given namespace ID should be valid for all shares, i.e. `ns_commits` and `content` have
    /// at least `ns_index` elements for all shares.
    pub fn ns_recover(
        param: &NsAvidMParam,
        ns_index: usize,
        shares: &[NsAvidMShare],
    ) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        if shares
            .iter()
            .any(|share| ns_index >= share.ns_lens.len() || ns_index >= share.content.len())
        {
            return Err(VidError::IndexOutOfBound);
        }
        let ns_commit = shares[0].ns_commits[ns_index];
        let shares: Vec<_> = shares
            .iter()
            .filter_map(|share| share.inner_ns_share(ns_index))
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
