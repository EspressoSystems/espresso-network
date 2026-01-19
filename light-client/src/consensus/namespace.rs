use anyhow::{bail, ensure, Context, Result};
use espresso_types::{Header, NamespaceId, NsProof, Transaction};
use hotshot_query_service::VidCommon;
use serde::{Deserialize, Serialize};

/// Information required to verify a payload.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]

pub struct NamespaceProof {
    proof: Option<NonEmptyNamespaceProof>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct NonEmptyNamespaceProof {
    proof: NsProof,
    common: VidCommon,
}

impl NamespaceProof {
    /// Construct a [`NamespaceProof`].
    ///
    /// Takes the underlying [`NsProof`], plus corresponding [`VidCommon`] data to allow a client to
    /// verify the proof.
    pub fn new(proof: NsProof, common: VidCommon) -> Self {
        Self {
            proof: Some(NonEmptyNamespaceProof { proof, common }),
        }
    }

    /// Create a trivial proof for a namespace which is not present in a given block.
    pub fn not_present() -> Self {
        Self { proof: None }
    }

    /// Verify a [`NamespaceProof`].
    ///
    /// If the data in this proof matches the expected `header` and belongs to `namespace`, the list
    /// of transactions from the namespace is returned.
    pub fn verify(&self, header: &Header, namespace: NamespaceId) -> Result<Vec<Transaction>> {
        let Some(proof) = &self.proof else {
            // A trivial proof is a claim that the requested namespace is not present in the
            // namespace table. We need to verify this claim.
            if let Some(ix) = header.ns_table().find_ns_id(&namespace) {
                bail!(
                    "received trivial proof for missing namespace, but requested namespace \
                     {namespace} is present at position {ix:?}"
                );
            }
            return Ok(vec![]);
        };

        let (txs, ns) = proof
            .proof
            .verify(
                header.ns_table(),
                &header.payload_commitment(),
                &proof.common,
            )
            .context("invalid namespace proof")?;
        ensure!(
            ns == namespace,
            "proof is for wrong namespace {ns}, expected namespace {namespace}"
        );
        Ok(txs)
    }
}

#[cfg(test)]
mod test {
    use espresso_types::{Leaf2, NodeState};
    use hotshot_query_service::availability::TransactionIndex;

    use super::*;
    use crate::testing::{EnableEpochs, TestClient};

    #[tokio::test]
    #[test_log::test]
    async fn test_namespace_proof_non_empty() {
        let client = TestClient::default();
        let leaf = client.leaf(1).await;
        let payload = client.payload(1).await;
        let common = client.vid_common(1).await;

        let tx = payload
            .transaction(&TransactionIndex {
                ns_index: 0.into(),
                position: 0,
            })
            .unwrap();
        let proof =
            NamespaceProof::new(NsProof::new(&payload, &0.into(), &common).unwrap(), common);
        assert_eq!(
            proof.verify(leaf.header(), tx.namespace()).unwrap(),
            vec![tx.clone()]
        );

        // Check that a trivial proof is not accepted for a non-trivial namespace.
        let err = NamespaceProof::not_present()
            .verify(leaf.header(), tx.namespace())
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("received trivial proof for missing namespace"),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_namespace_proof_empty() {
        let leaf = Leaf2::genesis::<EnableEpochs>(&Default::default(), &NodeState::mock()).await;
        let proof = NamespaceProof::not_present();

        assert_eq!(
            proof.verify(leaf.block_header(), 0u64.into()).unwrap(),
            vec![]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_namespace_proof_invalid_wrong_payload() {
        let client = TestClient::default();

        let payload1 = client.payload(1).await;
        let common1 = client.vid_common(1).await;
        let tx = payload1
            .transaction(&TransactionIndex {
                ns_index: 0.into(),
                position: 0,
            })
            .unwrap();
        let proof = NamespaceProof::new(
            NsProof::new(&payload1, &0.into(), &common1).unwrap(),
            common1,
        );

        let leaf2 = client.leaf(2).await;
        let err = proof.verify(leaf2.header(), tx.namespace()).unwrap_err();
        assert!(
            err.to_string().contains("invalid namespace proof"),
            "{err:#}"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_namespace_proof_invalid_wrong_namespace() {
        let client = TestClient::default();
        let leaf = client.leaf(1).await;
        let payload = client.payload(1).await;
        let common = client.vid_common(1).await;

        let tx = payload
            .transaction(&TransactionIndex {
                ns_index: 0.into(),
                position: 0,
            })
            .unwrap();
        let proof =
            NamespaceProof::new(NsProof::new(&payload, &0.into(), &common).unwrap(), common);
        let err = proof
            .verify(
                leaf.header(),
                NamespaceId::from(u64::from(tx.namespace()) + 1),
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("proof is for wrong namespace"),
            "{err:#}"
        );
    }
}
