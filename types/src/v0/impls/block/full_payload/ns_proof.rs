use hotshot_query_service::VidCommitment;
use hotshot_types::{
    traits::EncodeBytes,
    vid::advz::{advz_scheme, ADVZCommon, ADVZScheme},
};
use jf_vid::{
    payload_prover::{PayloadProver, Statement},
    VidScheme,
};

use crate::{NamespaceId, NsIndex, NsProof, NsTable, Payload, PayloadByteLen, Transaction};

impl NsProof {
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
    pub fn new(payload: &Payload, index: &NsIndex, common: &ADVZCommon) -> Option<NsProof> {
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

        Some(NsProof {
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
                    }
                    (None, true) => {} // 0-length namespace, nothing to verify
                    (None, false) => {
                        tracing::error!(
                            "ns verify: missing proof for nonempty ns payload range {:?}",
                            range
                        );
                        return None;
                    }
                    (Some(_), true) => {
                        tracing::error!("ns verify: unexpected proof for empty ns payload range");
                        return None;
                    }
                }

                // verification succeeded, return some data
                let ns_id = ns_table.read_ns_id_unchecked(&self.ns_index);
                Some((self.ns_payload.export_all_txs(&ns_id), ns_id))
            }
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
mod test;
