//! Light client proofs as a query server returns them.
//!
//! The light client trusts only the genesis stake table, and checks every
//! leaf, header, payload and namespace a server sends against it or against
//! data it already verified. Honest proofs from a simulated chain are
//! serialized, edited, and decoded back. Decoding and verification must not
//! panic, and an edited proof must never verify to data that is not on the
//! chain.

#![no_main]

use std::sync::{Arc, LazyLock};

use arbitrary::Arbitrary;
use espresso_fuzz::{Edit, apply_edits};
use espresso_types::{Header, Leaf2, NamespaceId, Payload, SeqTypes, Transaction};
use futures::executor::block_on;
use hotshot_query_service_types::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
};
use libfuzzer_sys::fuzz_target;
use light_client::{
    client::Client,
    consensus::{
        header::HeaderProof,
        leaf::{LeafProof, LeafProofHint},
        namespace::NamespaceProof,
        payload::PayloadProof,
        quorum::{StakeTable, StakeTableQuorum},
    },
    testing::TestClient,
};
use serde::{Serialize, de::DeserializeOwned};
use versions::NEW_PROTOCOL_VERSION;

const NUM_BLOCKS: usize = 8;
const NAMESPACES: [u32; 4] = [10, 20, 30, 99];

#[derive(Arbitrary, Debug)]
struct Input {
    proof: ProofKind,
    proof_index: u8,
    target_height: u8,
    namespace: u8,
    format: Format,
    edits: Vec<Edit>,
}

#[derive(Arbitrary, Debug)]
enum ProofKind {
    Leaf,
    Header,
    Payload,
    Namespace,
}

#[derive(Arbitrary, Debug)]
enum Format {
    Bincode,
    Json,
}

type Quorum = StakeTableQuorum<(Arc<StakeTable>, Arc<StakeTable>)>;

struct Chain {
    epoch_height: u64,
    quorum: Quorum,
    leaves: Vec<Leaf2>,
    headers: Vec<Header>,
    payloads: Vec<Payload>,
    leaf_proofs: Vec<(LeafProof, Option<Leaf2>)>,
    header_proofs: Vec<(u64, HeaderProof)>,
    payload_proofs: Vec<PayloadProof>,
    namespace_proofs: Vec<NamespaceProof>,
}

static CHAIN: LazyLock<Chain> = LazyLock::new(|| block_on(Chain::new()));

fuzz_target!(init: { LazyLock::force(&CHAIN); }, |input: Input| {
    let chain = &*CHAIN;
    let index = usize::from(input.proof_index);
    let header = chain.header(usize::from(input.target_height) % NUM_BLOCKS);
    match input.proof {
        ProofKind::Leaf => {
            let (proof, finalized) = &chain.leaf_proofs[index % chain.leaf_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            if let Ok(leaf) = block_on(chain.verify_leaf_proof(&proof, finalized.as_ref())) {
                assert!(
                    chain.leaves.contains(leaf.leaf()),
                    "proved a leaf that is not on the chain: {leaf:?}"
                );
            }
        },
        ProofKind::Header => {
            let (root, proof) = &chain.header_proofs[index % chain.header_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            let root_commit = chain.header(*root as usize).block_merkle_tree_root();
            if let Ok(proved) = proof.verify(root_commit) {
                assert!(proved.height() < *root);
                assert_eq!(
                    &proved,
                    chain.header(proved.height() as usize),
                    "proved a header that is not on the chain"
                );
            }
        },
        ProofKind::Payload => {
            let proof = &chain.payload_proofs[index % chain.payload_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            if let Ok(payload) = proof.verify(header) {
                assert_eq!(
                    payload,
                    chain.payloads[header.height() as usize],
                    "proved a payload that is not the block's"
                );
            }
        },
        ProofKind::Namespace => {
            let proof = &chain.namespace_proofs[index % chain.namespace_proofs.len()];
            let Some(proof) = mutate(proof, &input.format, &input.edits) else {
                return;
            };
            let namespace = NamespaceId::from(NAMESPACES[usize::from(input.namespace) % NAMESPACES.len()]);
            if let Ok(txs) = proof.verify(header, namespace) {
                assert_eq!(
                    txs,
                    chain.namespace_txs(header.height() as usize, namespace),
                    "proved namespace {namespace} with contents it does not have"
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

impl Chain {
    async fn new() -> Self {
        let client = TestClient::default();
        let new_protocol = TestClient::default();
        new_protocol.set_upgrade(1, NEW_PROTOCOL_VERSION).await;
        for height in 0..NUM_BLOCKS + 3 {
            client.add_block(height, block_txs(height)).await;
            new_protocol.add_block(height, block_txs(height)).await;
        }

        let genesis = client.genesis().await;
        let stake_table = Arc::new(StakeTable::from(genesis.stake_table));
        let quorum =
            StakeTableQuorum::new((stake_table.clone(), stake_table), genesis.epoch_height);

        let mut leaves = vec![];
        let mut headers = vec![];
        let mut payloads = vec![];
        let mut leaf_proofs = vec![];
        let mut header_proofs = vec![];
        let mut payload_proofs = vec![];
        let mut namespace_proofs = vec![];
        for height in 0..NUM_BLOCKS {
            let id = LeafId::Number(height);
            let leaf = client.leaf(height).await;
            headers.push(leaf.header().clone());
            leaves.push(leaf.leaf().clone());
            leaves.push(new_protocol.leaf(height).await.leaf().clone());
            payloads.push(client.payload(height).await);

            leaf_proofs.push((client.leaf_proof(id, None).await.unwrap(), None));
            leaf_proofs.push((
                client
                    .leaf_proof(id, Some(height as u64 + 1))
                    .await
                    .unwrap(),
                Some(client.leaf(height + 1).await.leaf().clone()),
            ));
            if height > 0 {
                leaf_proofs.push((new_protocol.leaf_proof(id, None).await.unwrap(), None));
            }

            for root in height + 1..NUM_BLOCKS {
                let proof = client
                    .header_proof(root as u64, BlockId::Number(height))
                    .await
                    .unwrap();
                header_proofs.push((root as u64, proof));
            }

            payload_proofs.push(client.payload_proof(height as u64).await.unwrap());
            for ns in NAMESPACES {
                namespace_proofs.push(
                    client
                        .namespace_proof(height as u64, NamespaceId::from(ns))
                        .await
                        .unwrap(),
                );
            }
        }

        let chain = Self {
            epoch_height: genesis.epoch_height,
            quorum,
            leaves,
            headers,
            payloads,
            leaf_proofs,
            header_proofs,
            payload_proofs,
            namespace_proofs,
        };
        chain.assert_honest_proofs_verify().await;
        chain
    }

    /// The soundness checks only mean something if the untouched proofs
    /// verify against this chain.
    async fn assert_honest_proofs_verify(&self) {
        for (proof, finalized) in &self.leaf_proofs {
            self.verify_leaf_proof(proof, finalized.as_ref())
                .await
                .expect("honest leaf proof verifies");
        }
        for (root, proof) in &self.header_proofs {
            let root_commit = self.header(*root as usize).block_merkle_tree_root();
            proof
                .verify_ref(root_commit)
                .expect("honest header proof verifies");
        }
        for (height, proof) in self.payload_proofs.iter().enumerate() {
            assert_eq!(
                proof.clone().verify(self.header(height)).unwrap(),
                self.payloads[height]
            );
        }
        for (i, proof) in self.namespace_proofs.iter().enumerate() {
            let height = i / NAMESPACES.len();
            let namespace = NamespaceId::from(NAMESPACES[i % NAMESPACES.len()]);
            assert_eq!(
                proof.verify(self.header(height), namespace).unwrap(),
                self.namespace_txs(height, namespace)
            );
        }
    }

    /// Picks the hint the way the light client does: the trusted quorum when
    /// the proof carries certificates, and a leaf it already trusts when the
    /// proof claims to end right before it.
    async fn verify_leaf_proof(
        &self,
        proof: &LeafProof,
        finalized: Option<&Leaf2>,
    ) -> anyhow::Result<LeafQueryData<SeqTypes>> {
        let hint = match proof.epoch(self.epoch_height)? {
            Some(_) => LeafProofHint::Quorum(&self.quorum),
            None => LeafProofHint::<Quorum>::Assumption(
                finalized.ok_or_else(|| anyhow::anyhow!("no finalized leaf to assume"))?,
            ),
        };
        proof.verify(hint).await
    }

    fn header(&self, height: usize) -> &Header {
        &self.headers[height]
    }

    fn namespace_txs(&self, height: usize, namespace: NamespaceId) -> Vec<Transaction> {
        block_txs(height)
            .into_iter()
            .filter(|tx| tx.namespace() == namespace)
            .collect()
    }
}

/// Blocks mix namespaces, empty transactions and empty blocks, so proofs
/// cover each shape of namespace table.
fn block_txs(height: usize) -> Vec<Transaction> {
    let txs = match height % 4 {
        0 => vec![],
        1 => vec![(10, vec![height as u8; 3])],
        2 => vec![
            (10, vec![]),
            (10, b"second".to_vec()),
            (20, vec![height as u8]),
        ],
        _ => vec![
            (20, b"tx".to_vec()),
            (30, vec![height as u8; 40]),
            (30, vec![7]),
        ],
    };
    txs.into_iter()
        .map(|(ns, bytes)| Transaction::new(NamespaceId::from(ns as u32), bytes))
        .collect()
}
