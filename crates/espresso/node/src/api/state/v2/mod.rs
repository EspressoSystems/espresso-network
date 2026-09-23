//! The v2 tonic services, each delegating to the v1 trait for the same module.

mod availability;
mod config;
mod database;
mod node;
mod status;
mod token;

use super::*;

fn block_id_from_query(
    height: Option<u64>,
    hash: Option<String>,
    payload_hash: Option<String>,
) -> Result<v1::availability::BlockId, tonic::Status> {
    match (height, hash, payload_hash) {
        (Some(height), None, None) => Ok(v1::availability::BlockId::Height(height)),
        (None, Some(hash), None) => Ok(v1::availability::BlockId::Hash(hash)),
        (None, None, Some(payload_hash)) => {
            Ok(v1::availability::BlockId::PayloadHash(payload_hash))
        },
        _ => Err(tonic::Status::invalid_argument(
            "set exactly one of height, hash or payload_hash",
        )),
    }
}

/// Namespace ids are 32 bits on chain but travel as uint64 so they round-trip with the
/// responses' `namespace` fields. Anything wider is a client error, not a truncation.
fn namespace_from_query(namespace: Option<u64>) -> Result<Option<u32>, tonic::Status> {
    namespace
        .map(|namespace| {
            u32::try_from(namespace)
                .map_err(|_| tonic::Status::invalid_argument("namespace does not fit in 32 bits"))
        })
        .transpose()
}

fn required<T>(value: Option<T>, name: &str) -> Result<T, tonic::Status> {
    value.ok_or_else(|| tonic::Status::invalid_argument(format!("{name} is required")))
}

fn range_from_query(from: Option<u64>, until: Option<u64>) -> Result<Range<u64>, tonic::Status> {
    Ok(required(from, "from")?..required(until, "until")?)
}

/// A conversion error is sent as an `event: error` frame. Ending the stream there means a
/// subscriber that skips the frame cannot miss a height without noticing.
fn end_at_first_error<T, S>(items: S) -> BoxStream<'static, Result<T, tonic::Status>>
where
    T: Send + 'static,
    S: futures::Stream<Item = Result<T, tonic::Status>> + Send + 'static,
{
    items
        .scan(false, |failed, item| {
            if *failed {
                return futures::future::ready(None);
            }
            *failed = item.is_err();
            futures::future::ready(Some(item))
        })
        .boxed()
}

fn ranges_from_body(ranges: Vec<proto::HeightRange>) -> Result<Vec<Range<u64>>, tonic::Status> {
    ranges
        .into_iter()
        .map(|range| range_from_query(range.from, range.until))
        .collect()
}

#[cfg(test)]
mod test_support {
    use super::*;

    /// v1 renders its byte-encoded fields as JSON integer arrays.
    pub(super) fn json_bytes(value: &serde_json::Value) -> Vec<u8> {
        <Vec<u8> as serde::Deserialize>::deserialize(value).unwrap()
    }

    pub(super) fn load_vector<T>(path: &str) -> (T, serde_json::Value)
    where
        T: serde::de::DeserializeOwned,
    {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        (T::deserialize(&json).unwrap(), json)
    }

    /// The reference vectors are the canonical v1 encoding, so comparing the converted header
    /// against them is what makes "the v2 header mirrors v1" a checked claim rather than a
    /// reviewed one. Every representation the conversion picks by hand is pinned here: `0x`
    /// addresses, the hex L1 timestamp, decimal fee and reward amounts, TaggedBase64
    /// commitments, and the base64 namespace table.
    pub(super) fn reference_header(version: &str) -> (espresso_types::Header, serde_json::Value) {
        let path = format!("../../../data/{version}/header.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let header: espresso_types::Header = serde_json::from_value(json.clone()).unwrap();
        // v1 headers are stored flat; later versions wrap their fields alongside the version.
        let fields = json.get("fields").cloned().unwrap_or(json);
        (header, fields)
    }

    /// Fails when the proto message and the reference vector disagree about which fields exist,
    /// which value-by-value assertions cannot catch: they only check the fields already declared.
    /// The declared side is read from the descriptor, so a field the proto lacks fails too.
    pub(super) fn assert_same_fields(message: &str, reference: &serde_json::Value) {
        use prost::Message as _;
        let descriptors =
            prost_types::FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
        let descriptor = descriptors
            .file
            .iter()
            .flat_map(|file| &file.message_type)
            .find(|candidate| candidate.name() == message)
            .unwrap_or_else(|| panic!("no proto message {message}"));
        let declared: std::collections::BTreeSet<&str> = descriptor
            .field
            .iter()
            .map(|field| match field.oneof_index {
                // v1 writes a tagged union as one key naming the union, not one per arm, so an
                // arm is compared under its oneof's name. A `proto3_optional` field is a
                // synthetic one-arm oneof and keeps its own name.
                Some(index) if !field.proto3_optional() => {
                    descriptor.oneof_decl[index as usize].name()
                },
                _ => field.name(),
            })
            .collect();
        let referenced: std::collections::BTreeSet<&str> = reference
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            // `_pd` is how v1 serializes a certificate's `PhantomData`, and carries nothing a
            // client can read, so no proto message mirrors it.
            .filter(|name| *name != "_pd")
            .collect();
        assert_eq!(
            declared, referenced,
            "{message} fields drifted from the reference vector"
        );
    }
}
