use hotshot_types::{
    traits::EncodeBytes,
    vid::advz::{advz_scheme, ADVZCommitment, ADVZCommon, ADVZScheme},
};
use jf_vid::{
    payload_prover::{PayloadProver, Statement},
    VidScheme,
};

use crate::{
    Index, NsIndex, NsTable, NumTxs, NumTxsRange, Payload, PayloadByteLen, Transaction, TxIndex,
    TxPayloadRange, TxProof, TxTableEntriesRange,
};

impl TxProof {
    /// Returns the [`Transaction`] indicated by `index`, along with a proof of
    /// correctness for that transaction. Returns `None` on error.
    pub fn new(
        index: &Index,
        payload: &Payload,
        common: &ADVZCommon,
    ) -> Option<(Transaction, Self)> {
        let ns_index = &NsIndex(index.namespace as usize);
        let tx_index = &TxIndex(index.position as usize);

        let payload_byte_len = payload.byte_len();
        if !payload_byte_len.is_consistent(common) {
            tracing::warn!(
                "payload byte len {} inconsistent with common {}",
                payload_byte_len,
                ADVZScheme::get_payload_byte_len(common)
            );
            return None; // error: payload byte len inconsistent with common
        }
        if !payload.ns_table().in_bounds(ns_index) {
            tracing::warn!("ns_index {:?} out of bounds", ns_index);
            return None; // error: ns index out of bounds
        }
        // check tx index below

        let payload_bytes_arc = payload.encode(); // pacify borrow checker
        let payload_bytes = payload_bytes_arc.as_ref();
        let ns_range = payload.ns_table().ns_range(ns_index, &payload_byte_len);
        let ns_byte_len = ns_range.byte_len();
        let ns_payload = payload.read_ns_payload(&ns_range);
        let vid = advz_scheme(
            ADVZScheme::get_num_storage_nodes(common)
                .try_into()
                .unwrap(),
        );

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

        let payload_proof_num_txs = vid
            .payload_proof(payload_bytes, ns_range.block_range(&num_txs_range))
            .ok()?;

        // Read the tx table entries for this tx and compute a proof of
        // correctness.
        let tx_table_entries_range = TxTableEntriesRange::new(tx_index);
        let payload_tx_table_entries = ns_payload.read(&tx_table_entries_range);
        let payload_proof_tx_table_entries = {
            vid.payload_proof(payload_bytes, ns_range.block_range(&tx_table_entries_range))
                .ok()?
        };

        // Read the tx payload and compute a proof of correctness.
        let tx_payload_range =
            TxPayloadRange::new(&payload_num_txs, &payload_tx_table_entries, &ns_byte_len);
        let payload_proof_tx = {
            let range = ns_range.block_range(&tx_payload_range);
            if range.is_empty() {
                None
            } else {
                Some(vid.payload_proof(payload_bytes, range).ok()?)
            }
        };

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
            TxProof {
                tx_index: tx_index.clone(),
                payload_num_txs,
                payload_proof_num_txs,
                payload_tx_table_entries,
                payload_proof_tx_table_entries,
                payload_proof_tx,
            },
        ))
    }

    /// Verify a [`TxProof`] for `tx` against a payload commitment. Returns
    /// `None` on error.
    pub fn verify(
        &self,
        ns_table: &NsTable,
        tx: &Transaction,
        commit: &ADVZCommitment,
        common: &ADVZCommon,
    ) -> Option<bool> {
        ADVZScheme::is_consistent(commit, common).ok()?;
        let Some(ns_index) = ns_table.find_ns_id(&tx.namespace()) else {
            tracing::info!("ns id {} does not exist", tx.namespace());
            return None; // error: ns id does not exist
        };
        let ns_range = ns_table.ns_range(&ns_index, &PayloadByteLen::from_vid_common(common));
        let ns_byte_len = ns_range.byte_len();

        if !NumTxs::new(&self.payload_num_txs, &ns_byte_len).in_bounds(&self.tx_index) {
            tracing::info!("tx index {:?} out of bounds", self.tx_index);
            return None; // error: tx index out of bounds
        }

        let vid = advz_scheme(
            ADVZScheme::get_num_storage_nodes(common)
                .try_into()
                .unwrap(),
        );

        // Verify proof for tx table len
        {
            let range = ns_range.block_range(&NumTxsRange::new(&ns_byte_len));
            if vid
                .payload_verify(
                    Statement {
                        payload_subslice: &self.payload_num_txs.to_payload_bytes(),
                        range,
                        commit,
                        common,
                    },
                    &self.payload_proof_num_txs,
                )
                .ok()?
                .is_err()
            {
                return Some(false);
            }
        }

        // Verify proof for tx table entries
        {
            let range = ns_range.block_range(&TxTableEntriesRange::new(&self.tx_index));
            if vid
                .payload_verify(
                    Statement {
                        payload_subslice: &self.payload_tx_table_entries.to_payload_bytes(),
                        range,
                        commit,
                        common,
                    },
                    &self.payload_proof_tx_table_entries,
                )
                .ok()?
                .is_err()
            {
                return Some(false);
            }
        }

        // Verify proof for tx payload
        {
            let range = ns_range.block_range(&TxPayloadRange::new(
                &self.payload_num_txs,
                &self.payload_tx_table_entries,
                &ns_byte_len,
            ));

            match (&self.payload_proof_tx, range.is_empty()) {
                (Some(proof), false) => {
                    if vid
                        .payload_verify(
                            Statement {
                                payload_subslice: tx.payload(),
                                range,
                                commit,
                                common,
                            },
                            proof,
                        )
                        .ok()?
                        .is_err()
                    {
                        return Some(false);
                    }
                },
                (None, true) => {}, // 0-length tx, nothing to verify
                (None, false) => {
                    tracing::error!(
                        "tx verify: missing proof for nonempty tx payload range {:?}",
                        range
                    );
                    return None;
                },
                (Some(_), true) => {
                    tracing::error!("tx verify: unexpected proof for empty tx payload range");
                    return None;
                },
            }
        }

        Some(true)
    }
}
