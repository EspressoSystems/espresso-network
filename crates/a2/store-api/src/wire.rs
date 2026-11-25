//! The store protocol's messages and their conversions, which the server and the client share.

use bytes::Bytes;
use chain_store::{Part, Parts, Record, TipUpdate};
use serde::{Serialize, de::DeserializeOwned};
use tonic::Status;

/// The store service's protocol, generated from `proto/espresso/a2/store/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.store.v1");
}

/// The largest message either side sends, which a payload bounds.
pub const MAX_MESSAGE_SIZE: usize = 64 << 20;

pub(crate) fn part_from(id: i32) -> Result<Part, Status> {
    i16::try_from(id)
        .ok()
        .and_then(Part::from_id)
        .ok_or_else(|| Status::invalid_argument(format!("no part {id}")))
}

pub(crate) fn parts_from(parts: Vec<proto::PartData>) -> Result<Vec<(Part, Bytes)>, Status> {
    parts
        .into_iter()
        .map(|p| Ok((part_from(p.part)?, p.data)))
        .collect()
}

pub(crate) fn parts_to(parts: Vec<(Part, Bytes)>) -> Vec<proto::PartData> {
    parts
        .into_iter()
        .map(|(part, data)| proto::PartData {
            part: part.id().into(),
            data,
        })
        .collect()
}

pub(crate) fn record_to(record: Option<Record>) -> proto::MaybeRecord {
    match record {
        Some(record) => proto::MaybeRecord {
            item: Some(record.item),
            parts: parts_to(record.parts),
        },
        None => proto::MaybeRecord::default(),
    }
}

pub(crate) fn record_from(
    height: u64,
    record: proto::MaybeRecord,
) -> Result<Option<Record>, Status> {
    let Some(item) = record.item else {
        return Ok(None);
    };
    Ok(Some(Record {
        height,
        item,
        parts: parts_from(record.parts)?,
    }))
}

pub(crate) fn tip_event(update: TipUpdate) -> proto::TipEvent {
    use proto::tip_event::Event;
    let event = match update {
        TipUpdate::Height {
            height,
            item,
            parts,
        } => Event::Height(proto::TipHeight {
            height,
            item,
            parts: parts_to(parts),
        }),
        TipUpdate::Bounds { newest, floor } => Event::Bounds(proto::TipBounds { newest, floor }),
    };
    proto::TipEvent { event: Some(event) }
}

pub(crate) fn tip_update(event: proto::TipEvent) -> Result<TipUpdate, Status> {
    use proto::tip_event::Event;
    match event.event {
        Some(Event::Height(height)) => Ok(TipUpdate::Height {
            height: height.height,
            item: height.item,
            parts: parts_from(height.parts)?,
        }),
        Some(Event::Bounds(bounds)) => Ok(TipUpdate::Bounds {
            newest: bounds.newest,
            floor: bounds.floor,
        }),
        None => Err(Status::invalid_argument("a tip event without an event")),
    }
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Bytes, Status> {
    bincode::serialize(value)
        .map(Bytes::from)
        .map_err(|err| Status::internal(format!("encoding an answer: {err}")))
}

pub(crate) fn encode_maybe<T: Serialize>(value: Option<T>) -> Result<proto::MaybeEncoded, Status> {
    Ok(proto::MaybeEncoded {
        data: value.as_ref().map(encode).transpose()?,
    })
}

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
    Ok(bincode::deserialize(bytes)?)
}

pub(crate) fn decode_maybe<T: DeserializeOwned>(
    value: proto::MaybeEncoded,
) -> anyhow::Result<Option<T>> {
    value.data.map(|bytes| decode(&bytes)).transpose()
}

pub(crate) fn encode_json<T: Serialize>(value: &T) -> Result<Bytes, Status> {
    serde_json::to_vec(value)
        .map(Bytes::from)
        .map_err(|err| Status::internal(format!("encoding an answer: {err}")))
}

pub(crate) fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

/// The parts a writer gives, as sent: the store computes what describes a payload, so only what
/// a writer can know is sent.
pub(crate) fn block_parts_to(parts: &Parts) -> anyhow::Result<proto::BlockParts> {
    Ok(proto::BlockParts {
        payload: parts.payload.clone(),
        vid_common: parts.vid_common.as_ref().map(to_bincode).transpose()?,
        cert2: parts.cert2.as_ref().map(to_bincode).transpose()?,
        state_cert: parts.state_cert.as_ref().map(to_bincode).transpose()?,
        certs_known: parts.certs_known,
    })
}

fn to_bincode<T: Serialize>(value: &T) -> anyhow::Result<Bytes> {
    Ok(bincode::serialize(value)?.into())
}

pub(crate) fn block_parts_from(parts: Option<proto::BlockParts>) -> Result<Parts, Status> {
    let Some(parts) = parts else {
        return Ok(Parts::default());
    };
    let invalid = |what: &str| {
        let what = what.to_string();
        move |err: bincode::Error| Status::invalid_argument(format!("{what}: {err}"))
    };
    Ok(Parts {
        payload: parts.payload,
        vid_common: parts
            .vid_common
            .map(|bytes| bincode::deserialize(&bytes))
            .transpose()
            .map_err(invalid("VID common data"))?,
        cert2: parts
            .cert2
            .map(|bytes| bincode::deserialize(&bytes))
            .transpose()
            .map_err(invalid("a certificate"))?,
        state_cert: parts
            .state_cert
            .map(|bytes| bincode::deserialize(&bytes))
            .transpose()
            .map_err(invalid("a state certificate"))?,
        certs_known: parts.certs_known,
        ..Default::default()
    })
}

/// `bytes` decoded as bincode, an invalid argument if they are not.
pub(crate) fn argument<T: DeserializeOwned>(bytes: &[u8], what: &str) -> Result<T, Status> {
    bincode::deserialize(bytes).map_err(|err| Status::invalid_argument(format!("{what}: {err}")))
}

/// `bytes` decoded as JSON, an invalid argument if they are not.
pub(crate) fn json_argument<T: DeserializeOwned>(bytes: &[u8], what: &str) -> Result<T, Status> {
    serde_json::from_slice(bytes).map_err(|err| Status::invalid_argument(format!("{what}: {err}")))
}
