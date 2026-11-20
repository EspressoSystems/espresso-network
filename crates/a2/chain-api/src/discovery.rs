//! Discovery records as the discovery service lists them.

use anyhow::Context;
use consensus_api::discovery::{ServiceRecord, SignedServiceRecord};
use espresso_types::SeqTypes;
use hotshot_types::data::EpochNumber;
use tagged_base64::TaggedBase64;

use crate::proto;

/// `signed` as clients receive it; they check the signature against the stake table of the
/// record's epoch themselves.
pub fn listed(signed: SignedServiceRecord<SeqTypes>) -> proto::ServiceRecord {
    proto::ServiceRecord {
        signer: signed.signer.to_string(),
        epoch: *signed.record.epoch,
        signed_message: signed.record.signed_bytes(),
        api_url: signed.record.api_url,
        api_key: signed.record.api_key.map(|key| key.to_vec()),
        sequence: signed.record.sequence,
        signature: TaggedBase64::from(signed.signature).to_string(),
    }
}

/// The record `listed` describes, as [`listed`] lists it; its signature is not checked here.
pub fn from_listed(listed: proto::ServiceRecord) -> anyhow::Result<SignedServiceRecord<SeqTypes>> {
    let api_key = listed
        .api_key
        .map(|key| <[u8; 32]>::try_from(key.as_slice()))
        .transpose()
        .context("an API key that is not 32 bytes")?;
    Ok(SignedServiceRecord {
        record: ServiceRecord {
            epoch: EpochNumber::new(listed.epoch),
            api_url: listed.api_url,
            api_key,
            sequence: listed.sequence,
        },
        signer: listed.signer.parse().context("the signer")?,
        signature: listed.signature.parse().context("the signature")?,
    })
}
