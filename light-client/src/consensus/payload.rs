use anyhow::{anyhow, ensure, Context, Result};
use espresso_types::{Header, Payload};
use hotshot_types::{
    data::{ns_table::parse_ns_table, VidCommitment, VidCommon},
    traits::block_contents::EncodeBytes,
    vid::{
        advz::{advz_scheme, ADVZScheme},
        avidm::{init_avidm_param, AvidMScheme},
        avidm_gf2::{init_avidm_gf2_param, AvidmGf2Scheme},
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
        Ok(self.payload)
    }
}
