//! Transaction and namespace proofs as a query server returns them.
//!
//! Clients decode these proofs from an untrusted server and check them against
//! a header they already trust. An honest proof for each VID version is
//! serialized, edited, and decoded back. Decoding and verification must not
//! panic, and an edited proof must never prove a transaction or namespace
//! contents that are not in the block.

#![no_main]

use std::sync::LazyLock;

use arbitrary::Arbitrary;
use espresso_fuzz::{Edit, apply_edits};
use espresso_types::{
    NamespaceId, NsProof, NsTable, Payload, Transaction, TxProof, v0_3::ChainConfig,
};
use hotshot_query_service_types::availability::{QueryablePayload, VerifiableInclusion};
use hotshot_types::{
    data::{VidCommitment, VidCommon, ns_table::parse_ns_table},
    traits::EncodeBytes,
    vid::{
        advz::advz_scheme,
        avidm::{AvidMScheme, init_avidm_param},
        avidm_gf2::{AvidmGf2Scheme, init_avidm_gf2_param},
    },
};
use jf_advz::VidScheme;
use libfuzzer_sys::fuzz_target;
use serde::{Serialize, de::DeserializeOwned};

const NUM_STORAGE_NODES: usize = 4;

#[derive(Arbitrary, Debug)]
struct Input {
    vid_version: u8,
    proof: ProofKind,
    proof_index: u8,
    format: Format,
    edits: Vec<Edit>,
}

#[derive(Arbitrary, Debug)]
enum ProofKind {
    Tx,
    Ns,
}

#[derive(Arbitrary, Debug)]
enum Format {
    Bincode,
    Json,
}

struct Block {
    payload: Payload,
    ns_table: NsTable,
    vids: Vec<Vid>,
}

struct Vid {
    commit: VidCommitment,
    common: VidCommon,
    tx_proofs: Vec<(Transaction, TxProof)>,
    ns_proofs: Vec<NsProof>,
}

static BLOCK: LazyLock<Block> = LazyLock::new(Block::new);

fuzz_target!(init: { LazyLock::force(&BLOCK); }, |input: Input| {
    let block = &*BLOCK;
    let vid = &block.vids[usize::from(input.vid_version) % block.vids.len()];
    match input.proof {
        ProofKind::Tx => {
            let (tx, proof) = &vid.tx_proofs[usize::from(input.proof_index) % vid.tx_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            proof.verify(&block.ns_table, tx, &vid.commit, &vid.common);
            let forged = Transaction::new(tx.namespace(), [tx.payload(), b"x"].concat());
            assert!(
                !proof.verify(&block.ns_table, &forged, &vid.commit, &vid.common),
                "proved a transaction that is not in the block"
            );
        },
        ProofKind::Ns => {
            let proof = &vid.ns_proofs[usize::from(input.proof_index) % vid.ns_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            if let Some((txs, ns_id)) = proof.verify(&block.ns_table, &vid.commit, &vid.common) {
                assert_eq!(
                    txs,
                    block.namespace_txs(ns_id),
                    "proved namespace {ns_id} with contents it does not have"
                );
            }
        },
    }
});

/// Encode `proof` in `format`, apply `edits`, and decode it back.
fn mutate<T: Serialize + DeserializeOwned>(
    proof: &T,
    format: &Format,
    edits: &[Edit],
) -> Option<T> {
    match format {
        Format::Bincode => {
            bincode::deserialize(&apply_edits(bincode::serialize(proof).unwrap(), edits)).ok()
        },
        Format::Json => {
            serde_json::from_slice(&apply_edits(serde_json::to_vec(proof).unwrap(), edits)).ok()
        },
    }
}

impl Block {
    fn new() -> Self {
        let txs = [
            (10, &b"first"[..]),
            (10, b""),
            (10, b"third tx in namespace 10"),
            (20, b"only"),
            (30, b"a"),
            (30, b"bb"),
        ]
        .map(|(ns, bytes)| Transaction::new(NamespaceId::from(ns as u32), bytes.to_vec()));
        let (payload, ns_table) =
            Payload::from_transactions_sync(&txs, ChainConfig::default()).unwrap();
        let bytes = payload.encode();
        let ns_ranges = parse_ns_table(bytes.len(), &ns_table.encode());

        let advz = advz_scheme(NUM_STORAGE_NODES).disperse(&bytes).unwrap();
        let avidm_param = init_avidm_param(NUM_STORAGE_NODES).unwrap();
        let avidm_commit = AvidMScheme::commit(&avidm_param, &bytes, ns_ranges.clone()).unwrap();
        let avidm_gf2_param = init_avidm_gf2_param(NUM_STORAGE_NODES).unwrap();
        let (avidm_gf2_commit, avidm_gf2_common) =
            AvidmGf2Scheme::commit(&avidm_gf2_param, &bytes, ns_ranges).unwrap();

        let vids = [
            (VidCommitment::V0(advz.commit), VidCommon::V0(advz.common)),
            (VidCommitment::V1(avidm_commit), VidCommon::V1(avidm_param)),
            (
                VidCommitment::V2(avidm_gf2_commit),
                VidCommon::V2(avidm_gf2_common),
            ),
        ]
        .into_iter()
        .map(|(commit, common)| Vid {
            tx_proofs: payload
                .iter(&ns_table)
                .map(|index| TxProof::new(&index, &payload, &common).unwrap())
                .collect(),
            ns_proofs: ns_table
                .iter()
                .map(|index| NsProof::new(&payload, &index, &common).unwrap())
                .collect(),
            commit,
            common,
        })
        .collect();
        Self {
            payload,
            ns_table,
            vids,
        }
    }

    fn namespace_txs(&self, ns_id: NamespaceId) -> Vec<Transaction> {
        self.payload
            .iter(&self.ns_table)
            .filter_map(|index| self.payload.transaction(&index))
            .filter(|tx| tx.namespace() == ns_id)
            .collect()
    }
}
