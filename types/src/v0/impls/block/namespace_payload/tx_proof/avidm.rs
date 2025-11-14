use hotshot_types::{
    data::{VidCommitment, VidCommon},
    vid::avidm::AvidMCommon,
};

use crate::{
    v0_3::{AvidMNsProof, AvidMTxProof},
    Index, NsTable, NumTxs, NumTxsRange, Payload, Transaction, TxIndex, TxPayloadRange,
    TxTableEntriesRange,
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
        if self.tx_index.0 > txs.len() {
            return false;
        }
        &txs[self.tx_index.0] == tx
    }
}
