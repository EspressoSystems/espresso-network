//! This module contains the original namespace proof implementation for ADVZ scheme.

use hotshot_types::{
    data::VidCommitment,
    traits::EncodeBytes,
    vid::advz::{ADVZCommon, ADVZScheme, advz_scheme},
};
use jf_vid::{
    VidScheme,
    payload_prover::{PayloadProver, Statement},
};

use crate::{
    NamespaceId, NsIndex, NsTable, Payload, PayloadByteLen, Transaction, v0_1::ADVZNsProof,
};

impl ADVZNsProof {
    /// Returns the payload bytes for the `index`th namespace, along with a
    /// proof of correctness for those bytes. Returns `None` on error.
    ///
    /// The namespace payload [`NsPayloadOwned`] is included as a hidden field
    /// in the returned [`NsProof`]. A conventional API would instead return
    /// `(NsPayload, NsProof)` and [`NsProof`] would not contain the namespace
    /// payload.
    /// ([`TxProof::new`](crate::block::namespace_payload::TxProof::new)
    /// conforms to this convention.) In the future we should change this API to
    /// conform to convention. But that would require a change to our RPC
    /// endpoint API at [`endpoints`](crate::api::endpoints), which is a hassle.
    pub fn new(payload: &Payload, index: &NsIndex, common: &ADVZCommon) -> Option<ADVZNsProof> {
        let payload_byte_len = payload.byte_len();
        if !payload_byte_len.is_consistent(common) {
            tracing::warn!(
                "payload byte len {} inconsistent with common {}",
                payload_byte_len,
                ADVZScheme::get_payload_byte_len(common)
            );
            return None; // error: payload byte len inconsistent with common
        }
        if !payload.ns_table().in_bounds(index) {
            tracing::warn!("ns_index {:?} out of bounds", index);
            return None; // error: index out of bounds
        }
        let ns_payload_range = payload.ns_table().ns_range(index, &payload_byte_len);

        // TODO vid_scheme() arg should be u32 to match get_num_storage_nodes
        // https://github.com/EspressoSystems/HotShot/issues/3298
        let vid = advz_scheme(
            ADVZScheme::get_num_storage_nodes(common).try_into().ok()?, // error: failure to convert u32 to usize
        );

        let ns_proof = if ns_payload_range.as_block_range().is_empty() {
            None
        } else {
            Some(
                vid.payload_proof(payload.encode(), ns_payload_range.as_block_range())
                    .ok()?, // error: internal to payload_proof()
            )
        };

        Some(ADVZNsProof {
            ns_index: index.clone(),
            ns_payload: payload.read_ns_payload(&ns_payload_range).to_owned(),
            ns_proof,
        })
    }

    /// Verify a [`NsProof`] against a payload commitment. Returns `None` on
    /// error or if verification fails.
    ///
    /// There is no [`NsPayload`](crate::block::namespace_payload::NsPayload)
    /// arg because this data is already included in the [`NsProof`]. See
    /// [`NsProof::new`] for discussion.
    ///
    /// If verification is successful then return `(Vec<Transaction>,
    /// NamespaceId)` obtained by post-processing the underlying
    /// [`NsPayload`](crate::block::namespace_payload::NsPayload). Why? This
    /// method might be run by a client in a WASM environment who might be
    /// running non-Rust code, in which case the client is unable to perform
    /// this post-processing himself.
    pub fn verify(
        &self,
        ns_table: &NsTable,
        commit: &VidCommitment,
        common: &ADVZCommon,
    ) -> Option<(Vec<Transaction>, NamespaceId)> {
        match commit {
            VidCommitment::V0(commit) => {
                ADVZScheme::is_consistent(commit, common).ok()?;
                if !ns_table.in_bounds(&self.ns_index) {
                    return None; // error: index out of bounds
                }

                let range = ns_table
                    .ns_range(&self.ns_index, &PayloadByteLen::from_vid_common(common))
                    .as_block_range();

                match (&self.ns_proof, range.is_empty()) {
                    (Some(proof), false) => {
                        // TODO advz_scheme() arg should be u32 to match get_num_storage_nodes
                        // https://github.com/EspressoSystems/HotShot/issues/3298
                        let vid = advz_scheme(
                            ADVZScheme::get_num_storage_nodes(common).try_into().ok()?, // error: failure to convert u32 to usize
                        );

                        vid.payload_verify(
                            Statement {
                                payload_subslice: self.ns_payload.as_bytes_slice(),
                                range,
                                commit,
                                common,
                            },
                            proof,
                        )
                        .ok()? // error: internal to payload_verify()
                        .ok()?; // verification failure
                    },
                    (None, true) => {}, // 0-length namespace, nothing to verify
                    (None, false) => {
                        tracing::error!(
                            "ns verify: missing proof for nonempty ns payload range {:?}",
                            range
                        );
                        return None;
                    },
                    (Some(_), true) => {
                        tracing::error!("ns verify: unexpected proof for empty ns payload range");
                        return None;
                    },
                }

                // verification succeeded, return some data
                let ns_id = ns_table.read_ns_id_unchecked(&self.ns_index);
                Some((self.ns_payload.export_all_txs(&ns_id), ns_id))
            },
            VidCommitment::V1(_) => None,
        }
    }

    /// Return all transactions in the namespace whose payload is proven by
    /// `self`. The namespace ID for each returned [`Transaction`] is set to
    /// `ns_id`.
    ///
    /// # Design warning
    ///
    /// This method relies on a promise that a [`NsProof`] stores the entire
    /// namespace payload. If in the future we wish to remove the payload from a
    /// [`NsProof`] then this method can no longer be supported.
    ///
    /// In that case, use the following a workaround:
    /// - Given a [`NamespaceId`], get a [`NsIndex`] `i` via
    ///   [`NsTable::find_ns_id`].
    /// - Use `i` to get a
    ///   [`NsPayload`](crate::block::namespace_payload::NsPayload) `p` via
    ///   [`Payload::ns_payload`].
    /// - Use `p` to get the desired [`Vec<Transaction>`] via
    ///   [`NsPayload::export_all_txs`](crate::block::namespace_payload::NsPayload::export_all_txs).
    ///
    /// This workaround duplicates the work done in [`NsProof::new`]. If you
    /// don't like that then you could instead hack [`NsProof::new`] to return a
    /// pair `(NsProof, Vec<Transaction>)`.
    pub fn export_all_txs(&self, ns_id: &NamespaceId) -> Vec<Transaction> {
        self.ns_payload.export_all_txs(ns_id)
    }
}

#[cfg(test)]
mod tests {
    use futures::future;
    use hotshot::{helpers::initialize_logging, traits::BlockPayload};
    use hotshot_types::{
        data::VidCommitment,
        traits::EncodeBytes,
        vid::advz::{ADVZScheme, advz_scheme},
    };
    use jf_vid::{VidDisperse, VidScheme};

    use crate::{Payload, v0::impls::block::test::ValidTest, v0_1::ADVZNsProof};

    #[tokio::test(flavor = "multi_thread")]
    async fn ns_proof() {
        let test_cases = vec![
            vec![
                vec![5, 8, 8],
                vec![7, 9, 11],
                vec![10, 5, 8],
                vec![7, 8, 9],
                vec![],
            ],
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![],
        ];

        initialize_logging();

        let mut rng = jf_utils::test_rng();
        let mut tests = ValidTest::many_from_tx_lengths(test_cases, &mut rng);

        struct BlockInfo {
            block: Payload,
            vid: VidDisperse<ADVZScheme>,
            ns_proofs: Vec<ADVZNsProof>,
        }

        let blocks: Vec<BlockInfo> = {
            // compute blocks separately to avoid async error `captured variable
            // cannot escape `FnMut` closure body` caused by mutable variable `vid`
            // below.
            let blocks_only = future::join_all(tests.iter().map(|t| async {
                Payload::from_transactions(t.all_txs(), &Default::default(), &Default::default())
                    .await
                    .unwrap()
                    .0
            }))
            .await;

            let mut vid = advz_scheme(10);
            blocks_only
                .into_iter()
                .map(|block| {
                    let vid = vid.disperse(block.encode()).unwrap();
                    let ns_proofs: Vec<ADVZNsProof> = block
                        .ns_table()
                        .iter()
                        .map(|ns_index| ADVZNsProof::new(&block, &ns_index, &vid.common).unwrap())
                        .collect();
                    BlockInfo {
                        block,
                        vid,
                        ns_proofs,
                    }
                })
                .collect()
        };

        // sanity: verify all valid namespace proofs
        for (
            BlockInfo {
                block,
                vid,
                ns_proofs,
            },
            test,
        ) in blocks.iter().zip(tests.iter_mut())
        {
            for ns_proof in ns_proofs.iter() {
                let ns_id = block.ns_table().read_ns_id(&ns_proof.ns_index).unwrap();
                let txs = test
                    .nss
                    .remove(&ns_id)
                    .unwrap_or_else(|| panic!("namespace {ns_id} missing from test"));

                // verify ns_proof
                let (ns_proof_txs, ns_proof_ns_id) = ns_proof
                    .verify(
                        block.ns_table(),
                        &VidCommitment::V0(vid.commit),
                        &vid.common,
                    )
                    .unwrap_or_else(|| panic!("namespace {ns_id} proof verification failure"));

                assert_eq!(ns_proof_ns_id, ns_id);
                assert_eq!(ns_proof_txs, txs);
            }
        }

        assert!(blocks.len() >= 2, "need at least 2 test_cases");

        let ns_proof_0_0 = &blocks[0].ns_proofs[0];
        let ns_table_0 = blocks[0].block.ns_table();
        let ns_table_1 = blocks[1].block.ns_table();
        let vid_commit_0 = &VidCommitment::V0(blocks[0].vid.commit);
        let vid_commit_1 = &VidCommitment::V0(blocks[1].vid.commit);
        let vid_common_0 = &blocks[0].vid.common;
        let vid_common_1 = &blocks[1].vid.common;

        // mix and match ns_table, vid_commit, vid_common
        {
            // wrong ns_table
            assert!(
                ns_proof_0_0
                    .verify(ns_table_1, vid_commit_0, vid_common_0)
                    .is_none()
            );

            // wrong vid commitment
            assert!(
                ns_proof_0_0
                    .verify(ns_table_0, vid_commit_1, vid_common_0)
                    .is_none()
            );

            // wrong vid common
            assert!(
                ns_proof_0_0
                    .verify(ns_table_0, vid_commit_0, vid_common_1)
                    .is_none()
            );

            // wrong ns_proof
            assert!(
                ns_proof_0_0
                    .verify(ns_table_1, vid_commit_1, vid_common_1)
                    .is_none()
            );
        }

        // hack the proof
        {
            ns_proof_0_0
                .verify(ns_table_0, vid_commit_0, vid_common_0)
                .expect("sanity: correct proof should succeed");

            let wrong_ns_index_ns_proof_0_0 = ADVZNsProof {
                ns_index: blocks[0].ns_proofs[1].ns_index.clone(),
                ..ns_proof_0_0.clone()
            };
            assert!(
                wrong_ns_index_ns_proof_0_0
                    .verify(ns_table_0, vid_commit_0, vid_common_0)
                    .is_none()
            );

            let wrong_ns_payload_ns_proof_0_0 = ADVZNsProof {
                ns_payload: blocks[0].ns_proofs[1].ns_payload.clone(),
                ..ns_proof_0_0.clone()
            };
            assert!(
                wrong_ns_payload_ns_proof_0_0
                    .verify(ns_table_0, vid_commit_0, vid_common_0)
                    .is_none()
            );

            let wrong_proof_ns_proof_0_0 = ADVZNsProof {
                ns_proof: blocks[0].ns_proofs[1].ns_proof.clone(),
                ..ns_proof_0_0.clone()
            };
            assert!(
                wrong_proof_ns_proof_0_0
                    .verify(ns_table_0, vid_commit_0, vid_common_0)
                    .is_none()
            );
        }
    }
}
