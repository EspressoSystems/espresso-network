use hotshot_types::{
    data::{VidCommitment, VidCommon},
    vid::avidm::AvidMCommon,
};

use crate::{
    Index, NsTable, NumTxs, NumTxsRange, Payload, Transaction, TxIndex, TxPayloadRange,
    TxTableEntriesRange,
    v0_3::{AvidMNsProof, AvidMTxProof},
};

impl AvidMTxProof {
    pub fn new(
        index: &Index,
        payload: &Payload,
        common: &AvidMCommon,
    ) -> Option<(Transaction, Self)> {
        let ns_index = &index.ns_index;
        let tx_index = &TxIndex(index.position as usize);

        let payload_byte_len = payload.byte_len();
        if !payload.ns_table().in_bounds(ns_index) {
            tracing::warn!("ns_index {:?} out of bounds", ns_index);
            return None; // error: ns index out of bounds
        }
        // check tx index below

        let ns_range = payload.ns_table().ns_range(ns_index, &payload_byte_len);
        let ns_byte_len = ns_range.byte_len();
        let ns_payload = payload.read_ns_payload(&ns_range);
        let ns_proof = AvidMNsProof::new(payload, ns_index, common)?;

        // Read the tx table len from this namespace's tx table and compute a
        // proof of correctness.
        let num_txs_range = NumTxsRange::new(&ns_byte_len);
        let payload_num_txs = ns_payload.read(&num_txs_range);

        // Check tx index.
        //
        // TODO the next line of code (and other code) could be easier to read
        // if we make a helpers that repeat computation we've already done.
        if !NumTxs::new(&payload_num_txs, &ns_byte_len).in_bounds(tx_index) {
            return None; // error: tx index out of bounds
        }

        // Read the tx table entries for this tx and compute a proof of
        // correctness.
        let tx_table_entries_range = TxTableEntriesRange::new(tx_index);
        let payload_tx_table_entries = ns_payload.read(&tx_table_entries_range);

        // Read the tx payload and compute a proof of correctness.
        let tx_payload_range =
            TxPayloadRange::new(&payload_num_txs, &payload_tx_table_entries, &ns_byte_len);

        let tx = {
            let ns_id = payload.ns_table().read_ns_id_unchecked(ns_index);
            let tx_payload = ns_payload
                .read(&tx_payload_range)
                .to_payload_bytes()
                .to_vec();
            Transaction::new(ns_id, tx_payload)
        };

        Some((
            tx,
            AvidMTxProof {
                tx_index: tx_index.clone(),
                ns_proof,
            },
        ))
    }

    pub fn verify(
        &self,
        ns_table: &NsTable,
        tx: &Transaction,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> bool {
        let VidCommon::V1(common) = common else {
            tracing::info!("VID version mismatch");
            return false;
        };

        let Some((txs, _)) = self.ns_proof.verify(ns_table, commit, common) else {
            return false;
        };
        txs.get(self.tx_index.0) == Some(tx)
    }
}

#[cfg(test)]
mod tests {
    use hotshot::traits::BlockPayload;
    use hotshot_query_service::availability::QueryablePayload;
    use hotshot_types::{
        data::{VidCommitment, VidCommon},
        traits::EncodeBytes,
        vid::avidm::{AvidMParam, AvidMScheme},
    };

    use crate::{Payload, TxIndex, v0::impls::block::test::ValidTest, v0_3::AvidMTxProof};

    /// A proof is untrusted input, so `verify` must reject a `tx_index` that is
    /// not in the namespace rather than panic. `tx_index == num_txs` is the
    /// boundary an off-by-one in the bounds check lets through.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn verify_rejects_out_of_range_tx_index() {
        let mut rng = jf_utils::test_rng();
        let test = ValidTest::from_tx_lengths(vec![vec![5, 8, 8]], &mut rng);
        let num_txs = test.all_txs().len();
        let block =
            Payload::from_transactions(test.all_txs(), &Default::default(), &Default::default())
                .await
                .unwrap()
                .0;

        let param = || AvidMParam::new(5usize, 10usize).unwrap();
        let payload_byte_len = block.byte_len();
        let ns_table = block.ns_table();
        let ns_ranges = ns_table
            .iter()
            .map(|index| ns_table.ns_range(&index, &payload_byte_len).0)
            .collect::<Vec<_>>();
        let vid_commit =
            VidCommitment::V1(AvidMScheme::commit(&param(), &block.encode(), ns_ranges).unwrap());
        let vid_common = VidCommon::V1(param());

        let index = block.iter(block.ns_table()).next().unwrap();
        let (tx, mut proof) = AvidMTxProof::new(&index, &block, &param()).unwrap();
        assert!(proof.verify(block.ns_table(), &tx, &vid_commit, &vid_common));

        // A valid index that names a different transaction is rejected.
        proof.tx_index = TxIndex(1);
        assert!(!proof.verify(block.ns_table(), &tx, &vid_commit, &vid_common));

        for out_of_range in [num_txs, num_txs + 1, usize::MAX] {
            proof.tx_index = TxIndex(out_of_range);
            assert!(
                !proof.verify(block.ns_table(), &tx, &vid_commit, &vid_common),
                "tx_index {out_of_range} of {num_txs} must be rejected"
            );
        }
    }
}
