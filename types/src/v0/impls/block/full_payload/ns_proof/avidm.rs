//! This module contains the namespace proof implementation for the new AvidM scheme.

use hotshot_types::{
    data::VidCommitment,
    vid::avidm::{AvidMCommon, AvidMShare},
};
use vid::avid_m::namespaced::NsAvidMScheme;

use crate::{
    v0_3::{AvidMNsProof, AvidMNsProofV1},
    NamespaceId, NsIndex, NsPayload, NsTable, Payload, Transaction,
};

impl AvidMNsProof {
    pub fn new(payload: &Payload, index: &NsIndex, common: &AvidMCommon) -> Option<AvidMNsProof> {
        let payload_byte_len = payload.byte_len();
        let index = index.0;
        let ns_table = payload.ns_table();
        let ns_table = ns_table
            .iter()
            .map(|index| ns_table.ns_range(&index, &payload_byte_len).0)
            .collect::<Vec<_>>();

        if index >= ns_table.len() {
            tracing::warn!("ns_index {:?} out of bounds", index);
            return None; // error: index out of bounds
        }

        if ns_table[index].is_empty() {
            None
        } else {
            match NsAvidMScheme::namespace_proof(common, &payload.raw_payload, index, ns_table) {
                Ok(proof) => Some(AvidMNsProof(proof)),
                Err(e) => {
                    tracing::error!("error generating namespace proof: {:?}", e);
                    None
                },
            }
        }
    }

    /// Unlike the ADVZ scheme, this function won't fail with a wrong `ns_table`.
    /// It only uses `ns_table` to get the namespace id.
    pub fn verify(
        &self,
        ns_table: &NsTable,
        commit: &VidCommitment,
        common: &AvidMCommon,
    ) -> Option<(Vec<Transaction>, NamespaceId)> {
        match commit {
            VidCommitment::V1(commit) => {
                match NsAvidMScheme::verify_namespace_proof(common, commit, &self.0) {
                    Ok(Ok(_)) => {
                        let ns_id = ns_table.read_ns_id(&NsIndex(self.0.ns_index))?;
                        let ns_payload = NsPayload::from_bytes_slice(&self.0.ns_payload);
                        Some((ns_payload.export_all_txs(&ns_id), ns_id))
                    },
                    Ok(Err(_)) => None,
                    Err(e) => {
                        tracing::warn!("error verifying namespace proof: {:?}", e);
                        None
                    },
                }
            },
            _ => None,
        }
    }
}

impl AvidMNsProofV1 {
    pub fn new_correct_encoding(
        payload: &Payload,
        index: &NsIndex,
        common: &AvidMCommon,
    ) -> Option<AvidMNsProofV1> {
        let payload_byte_len = payload.byte_len();
        let index = index.0;
        let ns_table = payload.ns_table();
        let ns_table = ns_table
            .iter()
            .map(|index| ns_table.ns_range(&index, &payload_byte_len).0)
            .collect::<Vec<_>>();

        if index >= ns_table.len() {
            tracing::warn!("ns_index {:?} out of bounds", index);
            return None; // error: index out of bounds
        }

        if ns_table[index].is_empty() {
            None
        } else {
            match NsAvidMScheme::namespace_proof(common, &payload.raw_payload, index, ns_table) {
                Ok(proof) => Some(AvidMNsProofV1::CorrectEncoding(proof)),
                Err(e) => {
                    tracing::error!("error generating namespace proof: {:?}", e);
                    None
                },
            }
        }
    }

    pub fn new_incorrect_encoding(
        shares: &[AvidMShare],
        ns_table: &NsTable,
        ns_index: &NsIndex,
        commit: &VidCommitment,
        common: &AvidMCommon,
    ) -> Option<AvidMNsProofV1> {
        let VidCommitment::V1(commit) = commit else {
            tracing::error!("Error generating incorrect encoding proof: invalid vid commitment");
            return None;
        };
        if shares.is_empty() {
            tracing::error!("Error generating incorrect encoding proof: no valid shares provided");
            return None;
        }
        let payload_byte_len = crate::PayloadByteLen(shares[0].payload_byte_len());
        let ns_index = ns_index.0;
        let ns_table = ns_table
            .iter()
            .map(|index| ns_table.ns_range(&index, &payload_byte_len).0)
            .collect::<Vec<_>>();

        if ns_index >= ns_table.len() {
            tracing::warn!("ns_index {:?} out of bounds", ns_index);
            return None; // error: index out of bounds
        }

        if ns_table[ns_index].is_empty() {
            None
        } else {
            match NsAvidMScheme::proof_of_incorrect_encoding_for_namespace(
                common, ns_index, commit, shares,
            ) {
                Ok(proof) => Some(AvidMNsProofV1::IncorrectEncoding(proof)),
                Err(e) => {
                    tracing::error!(
                        "error generating incorrect encoding proof for namespace index {ns_index}: {:?}",
                        e
                    );
                    None
                },
            }
        }
    }

    /// Unlike the ADVZ scheme, this function won't fail with a wrong `ns_table`.
    /// It only uses `ns_table` to get the namespace id.
    pub fn verify(
        &self,
        ns_table: &NsTable,
        commit: &VidCommitment,
        common: &AvidMCommon,
    ) -> Option<(Vec<Transaction>, NamespaceId)> {
        match (commit, self) {
            (VidCommitment::V1(commit), AvidMNsProofV1::CorrectEncoding(proof)) => {
                // correct encoding proof
                match NsAvidMScheme::verify_namespace_proof(common, commit, proof) {
                    Ok(Ok(_)) => {
                        let ns_id = ns_table.read_ns_id(&NsIndex(proof.ns_index))?;
                        let ns_payload = NsPayload::from_bytes_slice(&proof.ns_payload);
                        Some((ns_payload.export_all_txs(&ns_id), ns_id))
                    },
                    Ok(Err(_)) => None,
                    Err(e) => {
                        tracing::warn!("error verifying namespace proof: {:?}", e);
                        None
                    },
                }
            },
            (VidCommitment::V1(commit), AvidMNsProofV1::IncorrectEncoding(proof)) => {
                // incorrect encoding proof
                match proof.verify(common, commit) {
                    Ok(Ok(_)) => {
                        let ns_id = ns_table.read_ns_id(&NsIndex(proof.ns_index))?;
                        Some((vec![], ns_id))
                    },
                    Ok(Err(_)) => None,
                    Err(e) => {
                        tracing::warn!("error verifying namespace proof: {:?}", e);
                        None
                    },
                }
            },
            _ => None,
        }
    }
}

/// Copied from ADVZNsProof tests.
#[cfg(test)]
mod tests {
    use futures::future;
    use hotshot::{helpers::initialize_logging, traits::BlockPayload};
    use hotshot_types::{
        data::VidCommitment,
        traits::EncodeBytes,
        vid::avidm::{AvidMParam, AvidMScheme},
    };

    use crate::{v0::impls::block::test::ValidTest, v0_3::AvidMNsProof, NsIndex, Payload};

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

        let param = AvidMParam::new(5usize, 10usize).unwrap();

        struct BlockInfo {
            block: Payload,
            vid_commit: VidCommitment,
            ns_proofs: Vec<AvidMNsProof>,
        }

        let blocks: Vec<BlockInfo> = future::join_all(tests.iter().map(|t| async {
            let block =
                Payload::from_transactions(t.all_txs(), &Default::default(), &Default::default())
                    .await
                    .unwrap()
                    .0;
            let payload_byte_len = block.byte_len();
            let ns_table = block.ns_table();
            let ns_table = ns_table
                .iter()
                .map(|index| ns_table.ns_range(&index, &payload_byte_len).0)
                .collect::<Vec<_>>();
            let vid_commit = AvidMScheme::commit(&param, &block.encode(), ns_table).unwrap();
            let ns_proofs: Vec<AvidMNsProof> = block
                .ns_table()
                .iter()
                .map(|ns_index| AvidMNsProof::new(&block, &ns_index, &param).unwrap())
                .collect();
            BlockInfo {
                block,
                vid_commit: VidCommitment::V1(vid_commit),
                ns_proofs,
            }
        }))
        .await;

        // sanity: verify all valid namespace proofs
        for (
            BlockInfo {
                block,
                vid_commit,
                ns_proofs,
            },
            test,
        ) in blocks.iter().zip(tests.iter_mut())
        {
            for ns_proof in ns_proofs.iter() {
                let ns_id = block
                    .ns_table()
                    .read_ns_id(&NsIndex(ns_proof.0.ns_index))
                    .unwrap();
                let txs = test
                    .nss
                    .remove(&ns_id)
                    .unwrap_or_else(|| panic!("namespace {} missing from test", ns_id));

                // verify ns_proof
                let (ns_proof_txs, ns_proof_ns_id) = ns_proof
                    .verify(block.ns_table(), vid_commit, &param)
                    .unwrap_or_else(|| panic!("namespace {} proof verification failure", ns_id));

                assert_eq!(ns_proof_ns_id, ns_id);
                assert_eq!(ns_proof_txs, txs);
            }
        }

        assert!(blocks.len() >= 2, "need at least 2 test_cases");

        let ns_proof_0_0 = &blocks[0].ns_proofs[0];
        let ns_table_0 = blocks[0].block.ns_table();
        let ns_table_1 = blocks[1].block.ns_table();
        let vid_commit_1 = &blocks[1].vid_commit;

        // mix and match ns_table, vid_commit, vid_common
        {
            // wrong vid commitment
            assert!(ns_proof_0_0
                .verify(ns_table_0, vid_commit_1, &param)
                .is_none());

            // wrong ns_proof
            assert!(ns_proof_0_0
                .verify(ns_table_1, vid_commit_1, &param)
                .is_none());
        }
    }
}
