//! WebSocket frame payload codecs: binary frames carry VBS, text frames carry JSON. The
//! functions work on raw payloads so any WebSocket implementation can map them onto its own
//! frame type; callers pick the pair matching the negotiated [`ContentType`](crate::ContentType).

use serde::{Serialize, de::DeserializeOwned};
use vbs::{BinarySerializer, Serializer, version::StaticVersionType};

use crate::body::{DecodeFailure, EncodeFailure};

/// Encode a binary data frame payload as VBS.
pub fn encode_binary_frame<Ver: StaticVersionType, T: Serialize + ?Sized>(
    item: &T,
) -> Result<Vec<u8>, EncodeFailure> {
    Serializer::<Ver>::serialize(item).map_err(|err| EncodeFailure::Binary(err.to_string()))
}

/// Encode a text data frame payload as JSON.
pub fn encode_text_frame<T: Serialize + ?Sized>(item: &T) -> Result<String, EncodeFailure> {
    serde_json::to_string(item).map_err(|err| EncodeFailure::Json(err.to_string()))
}

/// Decode a binary data frame payload as VBS. Borrows the payload so transports can decode
/// without copying it.
pub fn decode_binary_frame<Ver: StaticVersionType, T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, DecodeFailure> {
    Serializer::<Ver>::deserialize(bytes).map_err(|err| DecodeFailure::Binary(err.to_string()))
}

/// Decode a text data frame payload as JSON. Borrows the payload so transports can decode
/// without copying it.
pub fn decode_text_frame<T: DeserializeOwned>(text: &str) -> Result<T, DecodeFailure> {
    serde_json::from_str(text).map_err(|err| DecodeFailure::Json(err.to_string()))
}
