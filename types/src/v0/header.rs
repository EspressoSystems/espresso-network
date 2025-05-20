use committable::Commitment;
use serde::{Deserialize, Serialize};
use vbs::version::Version;

use crate::{
    v0_1::{self, ChainConfig},
    v0_2, v0_3,
};

/// Each variant represents a specific minor version header.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Header {
    V1(v0_1::Header),
    V2(v0_2::Header),
    V3(v0_3::Header),
}

/// Enum to represent the first field of different versions of a header
///
/// In v1 headers, the first field is a ChainConfig, which contains either the chain config or its commitment.
/// For versions > 0.1, the first field contains the version.
///
/// This enum has the same variant names and types in the same positions (0 and 1) as the Either enum,
/// ensuring identical serialization and deserialization for the Left and Right variants.
/// However, it will deserialize successfully in one additional case due to the Version variant.
///
/// Variants:
/// - Left: Represents the ChainConfig variant in v1 headers.
/// - Right: Represents the chain config commitment variant in v1 headers.
/// - Version: Represents the versioned header for versions > 0.1.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) enum EitherOrVersion {
    Left(ChainConfig),
    Right(Commitment<ChainConfig>),
    Version(Version),
}

/// Headers with versions greater than 0.1 are serialized as `VersionedHeader` with the version field as `EitherOrVersion`.
/// This ensures that the first field is deserialized as the `EitherOrVersion::Version` variant.
/// This approach is necessary because `serde_flatten()` cannot be used with bincode,
/// as bincode does not support the `deserialize_any()`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionedHeader<Fields> {
    pub(crate) version: EitherOrVersion,
    pub(crate) fields: Fields,
}
