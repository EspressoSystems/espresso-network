//! Reading what a request asks for.

use std::{ops::Range, str::FromStr};

use committable::Commitment;
use espresso_types::{Header, Leaf2, NamespaceId};
use hotshot_types::data::VidCommitment;
use tonic::Status;

/// How a request names a block.
#[derive(Clone, Debug)]
pub(crate) enum BlockKey {
    Height(u64),
    Block(Commitment<Header>),
    Payload(VidCommitment),
    Leaf(Commitment<Leaf2>),
}

/// A block named by exactly one of a height, a block hash or a payload hash.
pub(crate) fn block_key(
    height: Option<u64>,
    hash: Option<String>,
    payload_hash: Option<String>,
) -> Result<BlockKey, Status> {
    match (height, hash, payload_hash) {
        (Some(height), None, None) => Ok(BlockKey::Height(height)),
        (None, Some(hash), None) => Ok(BlockKey::Block(parse(&hash, "hash")?)),
        (None, None, Some(hash)) => Ok(BlockKey::Payload(parse(&hash, "payload_hash")?)),
        _ => Err(Status::invalid_argument(
            "set exactly one of height, hash or payload_hash",
        )),
    }
}

pub(crate) fn parse<T: FromStr>(value: &str, name: &str) -> Result<T, Status> {
    value
        .parse()
        .map_err(|_| Status::invalid_argument(format!("{name} is not valid: {value}")))
}

pub(crate) fn required<T>(value: Option<T>, name: &str) -> Result<T, Status> {
    value.ok_or_else(|| Status::invalid_argument(format!("{name} is required")))
}

/// The heights `from..until`, at most `limit` of them.
pub(crate) fn range(
    from: Option<u64>,
    until: Option<u64>,
    limit: u64,
) -> Result<Range<u64>, Status> {
    let range = required(from, "from")?..required(until, "until")?;
    if range.end.saturating_sub(range.start) > limit {
        return Err(Status::invalid_argument(format!(
            "at most {limit} heights at a time"
        )));
    }
    Ok(range)
}

/// Namespace ids are 32 bits on chain but travel as uint64; anything wider is a client error.
pub(crate) fn namespace(namespace: Option<u64>) -> Result<NamespaceId, Status> {
    let namespace = u32::try_from(required(namespace, "namespace")?)
        .map_err(|_| Status::invalid_argument("namespace does not fit in 32 bits"))?;
    Ok(NamespaceId::from(namespace))
}
