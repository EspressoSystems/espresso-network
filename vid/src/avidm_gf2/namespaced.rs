//! This file implements the namespaced AvidmGf2 scheme.

use std::ops::Range;

use jf_merkle_tree::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

use super::{AvidmGf2Commit, AvidmGf2Share};
use crate::{
    avidm_gf2::{AvidmGf2Scheme, MerkleTree},
    VidError, VidResult, VidScheme,
};

/// Dummy struct for namespaced AvidmGf2 scheme
pub struct NsAvidmGf2Scheme;

/// Namespaced commitment type
pub type NsAvidmGf2Commit = super::AvidmGf2Commit;
/// Namespaced parameter type
pub type NsAvidmGf2Param = super::AvidmGf2Param;

/// VID Common data that needs to be broadcasted to all storage nodes
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct NsAvidmGf2Common {
    /// The AvidmGf2 parameters
    pub param: NsAvidmGf2Param,
    /// The list of all namespace commitments
    pub ns_commits: Vec<AvidmGf2Commit>,
    /// The size of each namespace
    pub ns_lens: Vec<usize>,
}

impl NsAvidmGf2Common {
    /// Return the total payload byte length
    pub fn payload_byte_len(&self) -> usize {
        self.ns_lens.iter().sum()
    }
}

/// Namespaced share for each storage node, contains one [`AvidmGf2Share`] for each namespace.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct NsAvidmGf2Share(pub(crate) Vec<AvidmGf2Share>);

impl NsAvidmGf2Share {
    /// Return the number of namespaces in this share
    pub fn num_nss(&self) -> usize {
        self.0.len()
    }

    /// Return the weight of this share
    pub fn weight(&self) -> usize {
        self.0.first().map_or(0, |share| share.weight())
    }

    /// Validate the share structure
    pub fn validate(&self) -> bool {
        let weight = self.weight();
        self.0
            .iter()
            .all(|share| share.validate() && share.weight() == weight)
    }

    /// Check whether this share contains a given namespace
    pub fn contains_ns(&self, ns_index: usize) -> bool {
        ns_index < self.num_nss()
    }

    /// Return the inner share for a given namespace if there exists one.
    pub fn inner_ns_share(&self, ns_index: usize) -> Option<AvidmGf2Share> {
        self.0.get(ns_index).cloned()
    }
}

impl NsAvidmGf2Scheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<NsAvidmGf2Param> {
        NsAvidmGf2Param::new(recovery_threshold, total_weights)
    }

    /// Commit to a payload given namespace table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn commit(
        param: &NsAvidmGf2Param,
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<(NsAvidmGf2Commit, NsAvidmGf2Common)> {
        let ns_table = ns_table.into_iter().collect::<Vec<_>>();
        let ns_lens = ns_table.iter().map(|r| r.len()).collect::<Vec<_>>();
        let ns_commits = ns_table
            .into_iter()
            .map(|ns_range| AvidmGf2Scheme::commit(param, &payload[ns_range]))
            .collect::<Result<Vec<_>, _>>()?;
        let common = NsAvidmGf2Common {
            param: param.clone(),
            ns_commits,
            ns_lens,
        };
        let commit = MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))
            .map_err(|err| VidError::Internal(err.into()))?
            .commitment();
        Ok((NsAvidmGf2Commit { commit }, common))
    }

    /// Disperse a payload according to a distribution table and a namespace
    /// table.
    /// WARN: it assumes that the namespace table is well formed, i.e. ranges
    /// are non-overlapping and cover the whole payload.
    pub fn ns_disperse(
        param: &NsAvidmGf2Param,
        distribution: &[u32],
        payload: &[u8],
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<(NsAvidmGf2Commit, NsAvidmGf2Common, Vec<NsAvidmGf2Share>)> {
        let num_storage_nodes = distribution.len();
        let mut ns_commits = vec![];
        let mut disperses = vec![];
        let mut ns_lens = vec![];
        for ns_range in ns_table {
            ns_lens.push(ns_range.len());
            let (commit, shares) =
                AvidmGf2Scheme::disperse(param, distribution, &payload[ns_range])?;
            ns_commits.push(commit);
            disperses.push(shares);
        }
        let common = NsAvidmGf2Common {
            param: param.clone(),
            ns_commits,
            ns_lens,
        };
        let commit = NsAvidmGf2Commit {
            commit: MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))
                .map_err(|err| VidError::Internal(err.into()))?
                .commitment(),
        };
        let mut shares = vec![NsAvidmGf2Share::default(); num_storage_nodes];
        disperses.into_iter().for_each(|ns_disperse| {
            shares
                .iter_mut()
                .zip(ns_disperse)
                .for_each(|(share, ns_share)| share.0.push(ns_share))
        });
        Ok((commit, common, shares))
    }

    /// Verify a namespaced share
    pub fn verify_share(
        commit: &NsAvidmGf2Commit,
        common: &NsAvidmGf2Common,
        share: &NsAvidmGf2Share,
    ) -> VidResult<crate::VerificationResult> {
        if !(common.ns_commits.len() == common.ns_lens.len()
            && common.ns_commits.len() == share.num_nss()
            && share.validate())
        {
            return Err(VidError::InvalidShare);
        }
        // Verify the share for each namespace
        for (commit, content) in common.ns_commits.iter().zip(share.0.iter()) {
            if AvidmGf2Scheme::verify_share(&common.param, commit, content)?.is_err() {
                return Ok(Err(()));
            }
        }
        // Verify the namespace MT commitment
        let expected_commit = NsAvidmGf2Commit {
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
    pub fn recover(common: &NsAvidmGf2Common, shares: &[NsAvidmGf2Share]) -> VidResult<Vec<u8>> {
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
        common: &NsAvidmGf2Common,
        ns_index: usize,
        shares: &[NsAvidmGf2Share],
    ) -> VidResult<Vec<u8>> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        if ns_index >= common.ns_lens.len()
            || !shares.iter().all(|share| share.contains_ns(ns_index))
        {
            return Err(VidError::IndexOutOfBound);
        }
        let ns_commit = &common.ns_commits[ns_index];
        let shares: Vec<_> = shares
            .iter()
            .filter_map(|share| share.inner_ns_share(ns_index))
            .collect();
        AvidmGf2Scheme::recover(&common.param, ns_commit, &shares)
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use crate::avidm_gf2::namespaced::NsAvidmGf2Scheme;

    #[test]
    fn round_trip() {
        // play with these items
        let num_storage_nodes = 9;
        let ns_lens = [15, 33];
        let ns_table = [(0usize..15), (15..48)];
        let payload_byte_len = ns_lens.iter().sum();

        let mut rng = jf_utils::test_rng();

        // more items as a function of the above
        let weights: Vec<u32> = (0..num_storage_nodes)
            .map(|_| rng.next_u32() % 5 + 1)
            .collect();
        let total_weights: u32 = weights.iter().sum();
        let recovery_threshold = total_weights.div_ceil(3) as usize;
        let params = NsAvidmGf2Scheme::setup(recovery_threshold, total_weights as usize).unwrap();

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
            NsAvidmGf2Scheme::ns_disperse(&params, &weights, &payload, ns_table.iter().cloned())
                .unwrap();

        assert_eq!(shares.len(), num_storage_nodes);

        assert_eq!(
            commit,
            NsAvidmGf2Scheme::commit(&params, &payload, ns_table.iter().cloned())
                .unwrap()
                .0
        );

        // verify shares
        shares.iter().for_each(|share| {
            assert!(NsAvidmGf2Scheme::verify_share(&commit, &common, share).is_ok_and(|r| r.is_ok()))
        });

        // test payload recovery on a random subset of shares
        shares.shuffle(&mut rng);
        let mut cumulated_weights = 0;
        let mut cut_index = 0;
        while cumulated_weights <= recovery_threshold {
            cumulated_weights += shares[cut_index].weight();
            cut_index += 1;
        }
        let ns0_payload_recovered =
            NsAvidmGf2Scheme::ns_recover(&common, 0, &shares[..cut_index]).unwrap();
        assert_eq!(ns0_payload_recovered[..], payload[ns_table[0].clone()]);
        let ns1_payload_recovered =
            NsAvidmGf2Scheme::ns_recover(&common, 1, &shares[..cut_index]).unwrap();
        assert_eq!(ns1_payload_recovered[..], payload[ns_table[1].clone()]);
        let payload_recovered = NsAvidmGf2Scheme::recover(&common, &shares[..cut_index]).unwrap();
        assert_eq!(payload_recovered, payload);
    }
}
