use anyhow::{Context, Result, anyhow, ensure};
use espresso_types::{Header, Payload};
use hotshot_types::{
    data::{VidCommitment, VidCommon, ns_table::parse_ns_table},
    traits::block_contents::EncodeBytes,
    vid::{
        advz::{ADVZScheme, advz_scheme},
        avidm::{AvidMScheme, init_avidm_param},
        avidm_gf2::{AvidmGf2Scheme, init_avidm_gf2_param},
    },
};
use jf_advz::VidScheme;
use serde::{Deserialize, Serialize};

/// Information required to verify a payload.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PayloadProof {
    /// The payload to be verified.
    payload: Payload,

    /// VID common data.
    ///
    /// This is data necessary to recompute the VID commitment of the payload, which can then be
    /// verified against a commitment in a previously-verified header.
    vid_common: VidCommon,
}

impl PayloadProof {
    /// Construct a [`PayloadProof`].
    ///
    /// Takes the payload to be verified, plus corresponding [`VidCommon`] data to allow a client to
    /// recompute and verify the commitment of the data.
    pub fn new(payload: Payload, vid_common: VidCommon) -> Self {
        Self {
            payload,
            vid_common,
        }
    }

    /// Verify a [`PayloadProof`].
    ///
    /// If the data in this proof matches the expected `header`, the full payload data is returned.
    pub fn verify(self, header: &Header) -> Result<Payload> {
        Ok(self.verify_with_vid_common(header)?.0)
    }

    /// Verify a [`PayloadProof`] and get the corresponding [`VidCommon`].
    ///
    /// If the data in this proof matches the expected `header`, the full payload data is returned.
    pub fn verify_with_vid_common(self, header: &Header) -> Result<(Payload, VidCommon)> {
        ensure!(
            self.payload.ns_table() == header.ns_table(),
            "namespace table of payload does not match namespace table in header"
        );
        let commit = match &self.vid_common {
            VidCommon::V0(common) => {
                advz_scheme(ADVZScheme::get_num_storage_nodes(common) as usize)
                    .commit_only(self.payload.encode())
                    .map(VidCommitment::V0)
                    .context("computing ADVZ commitment")?
            },
            VidCommon::V1(avidm) => {
                let param = init_avidm_param(avidm.total_weights)?;
                let bytes = self.payload.encode();
                AvidMScheme::commit(
                    &param,
                    &bytes,
                    parse_ns_table(bytes.len(), &header.ns_table().encode()),
                )
                .map(VidCommitment::V1)
                .map_err(|err| anyhow!("computing AvidM commitment: {err:#}"))?
            },
            VidCommon::V2(avidm_gf2) => {
                let param = init_avidm_gf2_param(avidm_gf2.param.total_weights)?;
                let bytes = self.payload.encode();
                AvidmGf2Scheme::commit(
                    &param,
                    &bytes,
                    parse_ns_table(bytes.len(), &header.ns_table().encode()),
                )
                .map(|(comm, _)| VidCommitment::V2(comm))
                .map_err(|err| anyhow!("computing AvidM commitment: {err:#}"))?
            },
        };
        ensure!(
            commit == header.payload_commitment(),
            "commitment of payload does not match commitment in header"
        );
        Ok((self.payload, self.vid_common))
    }
}

#[cfg(test)]
mod tests {
    use espresso_types::{NamespaceId, NodeState, Transaction};
    use hotshot_types::traits::block_contents::BlockPayload;

    use super::*;
    use crate::testing::TestClient;

    /// The commitment is checked against the header's namespace table, so the payload that comes
    /// back must carry that table too. A provider can pair genuine payload bytes with a different
    /// table of its own, which would reclassify the transactions into other namespaces.
    #[tokio::test]
    #[test_log::test]
    async fn test_verify_rejects_payload_with_forged_ns_table() {
        let client = TestClient::default();
        let tx = vec![1, 2, 3];
        client
            .add_block(
                1,
                vec![Transaction::new(NamespaceId::from(1u32), tx.clone())],
            )
            .await;
        let header = client.leaf(1).await.header().clone();
        let payload = client.payload(1).await;
        let vid_common = client.vid_common(1).await;

        let verified = PayloadProof::new(payload.clone(), vid_common.clone())
            .verify(&header)
            .unwrap();
        assert_eq!(verified.ns_table(), header.ns_table());

        // The same transaction bytes under a different namespace ID.
        let other = Payload::from_transactions_sync(
            [Transaction::new(NamespaceId::from(2u32), tx)],
            NodeState::mock_v3().chain_config,
        )
        .unwrap()
        .0;
        assert_ne!(other.ns_table(), payload.ns_table());
        let forged = Payload::from_bytes(payload.raw_payload(), other.ns_table());

        let err = PayloadProof::new(forged, vid_common)
            .verify(&header)
            .unwrap_err();
        assert!(err.to_string().contains("namespace table"), "{err:#}");
    }
}
