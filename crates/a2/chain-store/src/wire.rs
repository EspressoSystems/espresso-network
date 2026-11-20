//! The store protocol's messages and their conversions, which the store service and its client
//! share.

use tonic::Status;

use crate::{AppWrite, Part};

/// The store service's protocol, generated from `proto/espresso/a2/store/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.store.v1");
}

/// The largest message either side sends, which a payload bounds.
pub const MAX_MESSAGE_SIZE: usize = 64 << 20;

#[cfg(feature = "service")]
pub(crate) fn app_write_from(write: proto::AppTableWrite) -> Result<AppWrite, Status> {
    use proto::app_table_write::Op;
    let table = crate::TableName::new(&write.table)
        .map_err(|err| Status::invalid_argument(format!("{err:#}")))?;
    match write.op {
        Some(Op::Put(entry)) => Ok(AppWrite::Put {
            table,
            key: entry.key.into(),
            value: entry.value.into(),
        }),
        Some(Op::DeleteBefore(key)) => Ok(AppWrite::DeleteBefore {
            table,
            key: key.into(),
        }),
        Some(Op::Clear(_)) => Ok(AppWrite::Clear { table }),
        None => Err(Status::invalid_argument("a write without an operation")),
    }
}

#[cfg(feature = "remote")]
pub(crate) fn app_write_to(write: AppWrite) -> proto::AppTableWrite {
    use proto::app_table_write::Op;
    match write {
        AppWrite::Put { table, key, value } => proto::AppTableWrite {
            table: table.to_string(),
            op: Some(Op::Put(proto::AppEntry {
                key: key.to_vec(),
                value: value.to_vec(),
            })),
        },
        AppWrite::DeleteBefore { table, key } => proto::AppTableWrite {
            table: table.to_string(),
            op: Some(Op::DeleteBefore(key.to_vec())),
        },
        AppWrite::Clear { table } => proto::AppTableWrite {
            table: table.to_string(),
            op: Some(Op::Clear(proto::Empty {})),
        },
    }
}

pub(crate) fn part_from(id: i32) -> Result<Part, Status> {
    i16::try_from(id)
        .ok()
        .and_then(Part::from_id)
        .ok_or_else(|| Status::invalid_argument(format!("no part {id}")))
}

pub(crate) fn parts_from(parts: Vec<proto::PartData>) -> Result<Vec<(Part, bytes::Bytes)>, Status> {
    parts
        .into_iter()
        .map(|p| Ok((part_from(p.part)?, p.data.into())))
        .collect()
}

pub(crate) fn parts_to(parts: Vec<(Part, bytes::Bytes)>) -> Vec<proto::PartData> {
    parts
        .into_iter()
        .map(|(part, data)| proto::PartData {
            part: part.id().into(),
            data: data.to_vec(),
        })
        .collect()
}
