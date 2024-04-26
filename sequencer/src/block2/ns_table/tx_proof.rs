use crate::{
    block2::{
        iter::Index,
        payload_bytes::{
            ns_offset_as_bytes, ns_offset_from_bytes, tx_offset_as_bytes, NS_OFFSET_BYTE_LEN,
            NUM_TXS_BYTE_LEN, TX_OFFSET_BYTE_LEN,
        },
        Payload,
    },
    Transaction,
};
use hotshot_query_service::{VidCommitment, VidCommon};
use hotshot_types::vid::{vid_scheme, SmallRangeProofType, VidSchemeType};
use jf_primitives::vid::{
    payload_prover::{PayloadProver, Statement},
    VidScheme,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TxProof {
    // Conventions:
    // - `payload_x`: bytes from the payload
    // - `payload_proof_x`: a proof of those bytes from the payload

    // TODO can we trust ns_range claims? Or do we need to take the ns table as
    // a separate arg, and replace ns_range_x here with ns_index into the ns
    // table. I think we can trust them because payload proofs are tied to a
    // specific location
    ns_range_start: [u8; NS_OFFSET_BYTE_LEN], // serialized usize
    ns_range_end: [u8; NS_OFFSET_BYTE_LEN],   // serialized usize

    payload_num_txs: [u8; NUM_TXS_BYTE_LEN], // serialized usize
    payload_proof_num_txs: SmallRangeProofType,

    payload_tx_table_entry_prev: Option<[u8; TX_OFFSET_BYTE_LEN]>, // serialized usize, `None` for the 0th tx
    payload_tx_table_entry: [u8; TX_OFFSET_BYTE_LEN],              // serialized usize
    payload_proof_tx_range: SmallRangeProofType,
    // payload_proof_tx: Option<SmallRangeProofType>, // `None` if the tx has zero length
}

impl Payload {
    pub fn transaction(&self, index: &Index) -> Option<Transaction> {
        // TODO check index.ns_index in bounds
        // TODO don't copy the tx bytes into the return value
        // https://github.com/EspressoSystems/hotshot-query-service/issues/267
        Some(
            self.ns_payload(&index.ns_index)
                .export_tx(&self.ns_table.read_ns_id(&index.ns_index), &index.tx_index),
        )
    }

    pub fn transaction_with_proof(
        &self,
        index: &Index,
        common: &VidCommon,
    ) -> Option<(Transaction, TxProof)> {
        if self.payload.len() != VidSchemeType::get_payload_byte_len(common) {
            return None; // error: common inconsistent with self
        }

        // TODO check index.ns_index in bounds
        let ns_payload = self.ns_payload(&index.ns_index);
        let ns_payload_range = self
            .ns_table
            .ns_payload_range(&index.ns_index, self.payload.len());

        let vid = vid_scheme(VidSchemeType::get_num_storage_nodes(common));

        // BEGIN WIP
        // range of contiguous bytes in this namespace's tx table
        // TODO refactor as a function of `index`?
        // let tx_table_range = {
        //     let start = if index.tx_index.index == 0 {
        //         // Special case: the desired range includes only one entry from
        //         // the tx table: the first entry. This entry starts immediately
        //         // following the bytes that encode the tx table length.
        //         NUM_NSS_BYTE_LEN
        //     } else {
        //         // the desired range starts at the beginning of the previous
        //         // transaction's tx table entry
        //         (index.tx_index.index - 1)
        //             .saturating_mul(TX_OFFSET_BYTE_LEN)
        //             .saturating_add(NUM_TXS_BYTE_LEN)
        //     };
        //     // The desired range ends at the end of this transaction's tx table
        //     // entry
        //     let end = index
        //         .tx_index
        //         .index
        //         .saturating_add(1)
        //         .saturating_mul(TX_OFFSET_BYTE_LEN)
        //         .saturating_add(NUM_TXS_BYTE_LEN);
        //     Range {
        //         start: start.saturating_add(index.ns_index.ns_range.start),
        //         end: end.saturating_add(index.ns_index.ns_range.start),
        //     }
        // };

        // let payload_tx_range_start: Option<[u8; TX_OFFSET_BYTE_LEN]> = if index.tx_index.index == 0
        // {
        //     None
        // } else {
        //     Some(
        //         self.payload
        //             .get(
        //                 tx_table_range.start
        //                     ..tx_table_range.start.saturating_add(TX_OFFSET_BYTE_LEN),
        //             )?
        //             .try_into()
        //             .unwrap(), // panic is impossible
        //     )
        // };

        // let payload_tx_range_end: [u8; TX_OFFSET_BYTE_LEN] = self
        //     .payload
        //     .get(tx_table_range.end.saturating_sub(TX_OFFSET_BYTE_LEN)..tx_table_range.end)?
        //     .try_into()
        //     .unwrap(); // panic is impossible

        // let tx_range = Range {
        //     start: index
        //         .tx_index
        //         .range
        //         .start
        //         .saturating_add(index.ns_index.ns_range.start),
        //     end: index
        //         .tx_index
        //         .range
        //         .end
        //         .saturating_add(index.ns_index.ns_range.start),
        // };
        // END WIP

        // Read the tx table len from this namespace's tx table and compute a
        // proof of correctness.
        let (payload_num_txs, payload_proof_num_txs) = {
            // TODO make range_num_txs a method (of NsPayload)?
            let range_num_txs = Range {
                start: ns_payload_range.start,
                end: ns_payload_range
                    .start
                    .saturating_add(NUM_TXS_BYTE_LEN)
                    .min(ns_payload_range.end),
            };
            (
                // TODO make read_num_txs a method (of NsPayload)? Careful not to correct the original bytes!
                // TODO should be safe to read NUM_TXS_BYTE_LEN from payload; we would have exited by now otherwise.
                self.payload.get(range_num_txs.clone())?.try_into().unwrap(), // panic is impossible [TODO after we fix ns iterator])
                vid.payload_proof(&self.payload, range_num_txs).ok()?,
            )
        };

        // Read the tx table entries for this tx and compute a proof of
        // correctness.
        let payload_tx_table_entry_prev = ns_payload
            .read_tx_offset_prev(&index.tx_index)
            .map(tx_offset_as_bytes);
        let payload_tx_table_entry = tx_offset_as_bytes(ns_payload.read_tx_offset(&index.tx_index));
        let payload_proof_tx_range = {
            // TODO add a method Payload::tx_payload_range(index: Index) that automatically translates NsPayload::tx_payload_range by the namespace offset?
            let range = ns_payload.tx_payload_range(&index.tx_index);
            let range = range.start.saturating_add(ns_payload_range.start)
                ..range.end.saturating_add(ns_payload_range.start);
            vid.payload_proof(&self.payload, range).ok()?
        };

        Some((
            self.transaction(index)?,
            TxProof {
                ns_range_start: ns_offset_as_bytes(ns_payload_range.start),
                ns_range_end: ns_offset_as_bytes(ns_payload_range.end),
                payload_num_txs,
                payload_proof_num_txs,
                payload_tx_table_entry_prev,
                payload_tx_table_entry,
                payload_proof_tx_range,
                //     payload_proof_tx: if tx_range.is_empty() {
                //         None
                //     } else {
                //         Some(vid.payload_proof(&self.payload, tx_range).ok()?)
                //     },
            },
        ))
    }
}

impl TxProof {
    // - Returns `None` if an error occurred.
    // - `bool` result, or should we use `Result<(),()>` ?
    pub fn verify(
        &self,
        _tx: &Transaction,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> Option<bool> {
        VidSchemeType::is_consistent(commit, common).ok()?;

        let vid = vid_scheme(VidSchemeType::get_num_storage_nodes(common));

        // BEGIN WIP
        let ns_range =
            ns_offset_from_bytes(&self.ns_range_start)..ns_offset_from_bytes(&self.ns_range_end);
        // let tx_table_byte_len = (); // from num_txs bytes, capped by namespace size, offset by namespace.start

        // Verify proof for tx table len
        {
            let num_txs_range = Range {
                start: ns_range.start,
                end: ns_range
                    .start
                    .saturating_add(NUM_TXS_BYTE_LEN)
                    .min(ns_range.end),
            };

            tracing::info!("verify {:?}, {:?}", num_txs_range, self.payload_num_txs);

            if vid
                .payload_verify(
                    Statement {
                        payload_subslice: &self.payload_num_txs,
                        range: num_txs_range,
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
            // let tx_range = {
            //     // TODO translate by ns offset and tx_table_byte_len
            //     let end = tx_offset_from_bytes(&self.payload_tx_table_entry).;
            //     let start = tx_offset_from_bytes(
            //         &self
            //             .payload_tx_table_entry_prev
            //             .unwrap_or([0; TX_OFFSET_BYTE_LEN]),
            //     )
            //     .saturating_add(ns_range.start);
            // };
        }

        Some(true)
    }
}
