//! This file implements the namespaced AvidMGF2 scheme.

use std::ops::Range;

use jf_merkle_tree::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

use super::{AvidMGF2Commit, AvidMGF2Share, RawAvidMGF2Share};
use crate::{
    avidm_gf2::{AvidMGF2Scheme, MerkleTree},
    VidError, VidResult, VidScheme,
};

/// Dummy struct for namespaced AvidMGF2 scheme
pub struct NsAvidMGF2Scheme;

/// Namespaced commitment type
pub type NsAvidMGF2Commit = super::AvidMGF2Commit;
/// Namespaced parameter type
pub type NsAvidMGF2Param = super::AvidMGF2Param;

/// VID Common data that needs to be broadcasted to all storage nodes
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct NsAvidMGF2Common {
    /// The AvidMGF2 parameters
    pub param: NsAvidMGF2Param,
    /// The list of all namespace commitments
    pub ns_commits: Vec<AvidMGF2Commit>,
    /// The size of each namespace
    pub ns_lens: Vec<usize>,
}

/// Namespaced share for each storage node
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct NsAvidMGF2Share {
    /// Index number of the given share.
    pub(crate) index: u32,
    /// Actual share content
    pub(crate) content: Vec<RawAvidMGF2Share>,
}

impl NsAvidMGF2Share {
    /// Return the inner share for a given namespace if there exists one.
    pub fn inner_ns_share(
        &self,
        common: &NsAvidMGF2Common,
        ns_index: usize,
    ) -> Option<AvidMGF2Share> {
        if ns_index >= common.ns_lens.len() || ns_index >= self.content.len() {
            return None;
        }
        Some(AvidMGF2Share {
            index: self.index,
            content: self.content[ns_index].clone(),
        })
    }
}

impl NsAvidMGF2Scheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<NsAvidMGF2Param> {
        NsAvidMGF2Param::new(recovery_threshold, total_weights)
    }

    /// Commit to a payload given namespace table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn commit(
        param: &NsAvidMGF2Param,
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<(NsAvidMGF2Commit, NsAvidMGF2Common)> {
        let ns_table = ns_table.into_iter().collect::<Vec<_>>();
        let ns_lens = ns_table.iter().map(|r| r.len()).collect::<Vec<_>>();
        let ns_commits = ns_table
            .into_iter()
            .map(|ns_range| AvidMGF2Scheme::commit(param, &payload[ns_range]))
            .collect::<Result<Vec<_>, _>>()?;
        let common = NsAvidMGF2Common {
            param: param.clone(),
            ns_commits,
            ns_lens,
        };
        let commit = MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))
            .map_err(|err| VidError::Internal(err.into()))?
            .commitment();
        Ok((NsAvidMGF2Commit { commit }, common))
    }

    /// Disperse a payload according to a distribution table and a namespace
    /// table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn ns_disperse(
        param: &NsAvidMGF2Param,
        distribution: &[u32],
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<(NsAvidMGF2Commit, NsAvidMGF2Common, Vec<NsAvidMGF2Share>)> {
        let mut ns_commits = vec![];
        let mut disperses = vec![];
        let mut ns_lens = vec![];
        for ns_range in ns_table {
            ns_lens.push(ns_range.len());
            let (commit, shares) =
                AvidMGF2Scheme::disperse(param, distribution, &payload[ns_range])?;
            ns_commits.push(commit);
            disperses.push(shares);
        }
        let common = NsAvidMGF2Common {
            param: param.clone(),
            ns_commits,
            ns_lens,
        };
        let commit = NsAvidMGF2Commit {
            commit: MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))
                .map_err(|err| VidError::Internal(err.into()))?
                .commitment(),
        };
        // let ns_commits: Vec<_> = ns_commits
        //     .into_iter()
        //     .map(|comm| AvidMGF2Commit { commit: comm })
        //     .collect();
        let mut shares = vec![NsAvidMGF2Share::default(); disperses[0].len()];
        shares.iter_mut().for_each(|share| {
            share.index = disperses[0][0].index;
        });
        disperses.into_iter().for_each(|ns_disperse| {
            shares
                .iter_mut()
                .zip(ns_disperse)
                .for_each(|(share, ns_share)| share.content.push(ns_share.content))
        });
        Ok((commit, common, shares))
    }

    /// Verify a namespaced share
    pub fn verify_share(
        commit: &NsAvidMGF2Commit,
        common: &NsAvidMGF2Common,
        share: &NsAvidMGF2Share,
    ) -> VidResult<crate::VerificationResult> {
        if !(common.ns_commits.len() == common.ns_lens.len()
            && common.ns_commits.len() == share.content.len())
        {
            return Err(VidError::InvalidShare);
        }
        // Verify the share for each namespace
        for (commit, content) in common.ns_commits.iter().zip(share.content.iter()) {
            if AvidMGF2Scheme::verify_internal(&common.param, commit, content)?.is_err() {
                return Ok(Err(()));
            }
        }
        // Verify the namespace MT commitment
        let expected_commit = NsAvidMGF2Commit {
            commit: MerkleTree::from_elems(
                None,
                common.ns_commits.iter().map(|commit| commit.commit),
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
    pub fn recover(common: &NsAvidMGF2Common, shares: &[NsAvidMGF2Share]) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        let mut result = vec![];
        for ns_index in 0..common.ns_lens.len() {
            result.append(&mut Self::ns_recover(common, ns_index, shares)?)
        }
        Ok(result)
    }

    /// Recover the payload for a given namespace.
    /// Given namespace ID should be valid for all shares, i.e. `ns_commits` and `content` have
    /// at least `ns_index` elements for all shares.
    pub fn ns_recover(
        common: &NsAvidMGF2Common,
        ns_index: usize,
        shares: &[NsAvidMGF2Share],
    ) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        if shares
            .iter()
            .any(|share| ns_index >= common.ns_lens.len() || ns_index >= share.content.len())
        {
            return Err(VidError::IndexOutOfBound);
        }
        let ns_commit = &common.ns_commits[ns_index];
        let shares: Vec<_> = shares
            .iter()
            .filter_map(|share| share.inner_ns_share(common, ns_index))
            .collect();
        AvidMGF2Scheme::recover(&common.param, ns_commit, &shares)
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use crate::avidm_gf2::namespaced::NsAvidMGF2Scheme;

    #[test]
    fn round_trip() {
        // play with these items
        let num_storage_nodes = 9;
        let ns_lens = [15, 33];
        let ns_table = [(0usize..15), (15..48)];
        let payload_byte_len = ns_lens.iter().sum();

        // more items as a function of the above

        let mut rng = jf_utils::test_rng();

        let weights: Vec<u32> = (0..num_storage_nodes)
            .map(|_| rng.next_u32() % 5 + 1)
            .collect();
        let total_weights: u32 = weights.iter().sum();
        let recovery_threshold = total_weights.div_ceil(3) as usize;
        let params = NsAvidMGF2Scheme::setup(recovery_threshold, total_weights as usize).unwrap();

        println!(
            "recovery_threshold:: {recovery_threshold} num_storage_nodes: {num_storage_nodes} \
             payload_byte_len: {payload_byte_len}"
        );
        println!("weights: {weights:?}");

        let payload = {
            let mut bytes_random = vec![0u8; payload_byte_len];
            rng.fill_bytes(&mut bytes_random);
            bytes_random
        };

        let (commit, common, mut shares) =
            NsAvidMGF2Scheme::ns_disperse(&params, &weights, &payload, ns_table.iter().cloned())
                .unwrap();

        assert_eq!(shares.len(), num_storage_nodes);

        assert_eq!(
            commit,
            NsAvidMGF2Scheme::commit(&params, &payload, ns_table.iter().cloned())
                .unwrap()
                .0
        );

        // verify shares
        shares.iter().for_each(|share| {
            assert!(NsAvidMGF2Scheme::verify_share(&commit, &common, share).is_ok_and(|r| r.is_ok()))
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
            NsAvidMGF2Scheme::ns_recover(&common, 0, &shares[..cut_index]).unwrap();
        assert_eq!(ns0_payload_recovered[..], payload[ns_table[0].clone()]);
        let ns1_payload_recovered =
            NsAvidMGF2Scheme::ns_recover(&common, 1, &shares[..cut_index]).unwrap();
        assert_eq!(ns1_payload_recovered[..], payload[ns_table[1].clone()]);
        let payload_recovered = NsAvidMGF2Scheme::recover(&common, &shares[..cut_index]).unwrap();
        assert_eq!(payload_recovered, payload);
    }
}
