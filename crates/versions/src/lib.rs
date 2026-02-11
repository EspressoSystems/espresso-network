use std::{borrow::Cow, ops::Deref};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vbs::version::Version;

// Known versions:

pub const VERSION_0_0: Version = version(0, 0);
pub const VERSION_0_1: Version = version(0, 1);
pub const FEE_VERSION: Version = version(0, 2);
pub const EPOCH_VERSION: Version = version(0, 3);
pub const DRB_AND_HEADER_UPGRADE_VERSION: Version = version(0, 4);
pub const DA_UPGRADE_VERSION: Version = version(0, 5);
pub const VID2_UPGRADE_VERSION: Version = version(0, 6);
pub const MAX_SUPPORTED_VERSION: Version = DA_UPGRADE_VERSION;

// Known upgrade hashes:

const UPGRADE_HASH: UpgradeHash<'static> = UpgradeHash::borrowed(&[
    1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
]);

/// Version constructor.
pub const fn version(major: u16, minor: u16) -> Version {
    Version { major, minor }
}

/// Serialize a `Version` and a value.
pub fn encode<T>(v: Version, val: T) -> Result<Vec<u8>, VersionError>
where
    T: Serialize,
{
    let mut buf = Version::serialize(&v);
    bincode::serialize_into(&mut buf, &val)?;
    Ok(buf)
}

/// Deserialize a `Version` and a value.
pub fn decode<T>(bytes: &[u8]) -> Result<(Version, T), VersionError>
where
    T: DeserializeOwned,
{
    let (version, bytes) = Version::deserialize(bytes).map_err(|_| VersionError::Decode)?;
    let value = bincode::deserialize(bytes)?;
    Ok((version, value))
}

/// A version upgrade from some base to some target version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Upgrade {
    pub base: Version,
    pub target: Version,
}

impl Upgrade {
    pub const fn new(base: Version, target: Version) -> Self {
        debug_assert! {
            base.major < target.major || (base.major == target.major && base.minor <= target.minor)
        }
        Self { base, target }
    }

    /// A version upgrade where `base` == `target`.
    pub const fn trivial(base: Version) -> Self {
        Self { base, target: base }
    }

    /// Get the upgrade hash of this `base`, `target` pair.
    pub const fn hash(&self) -> UpgradeHash<'_> {
        // Currently only one upgrade hash is used. Eventually there could
        // be a `match (base, target) { ... }` here that returns a unique
        // hash per combination, or else some default hash.
        UPGRADE_HASH
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UpgradeHash<'a>(Cow<'a, [u8; 32]>);

impl<'a> UpgradeHash<'a> {
    pub const fn borrowed(hash: &'a [u8; 32]) -> Self {
        Self(Cow::Borrowed(hash))
    }

    pub fn new(hash: [u8; 32]) -> Self {
        Self(Cow::Owned(hash))
    }
}

impl<'a> From<UpgradeHash<'a>> for [u8; 32] {
    fn from(value: UpgradeHash) -> Self {
        *value.0
    }
}

impl<'a> From<&UpgradeHash<'a>> for [u8; 32] {
    fn from(value: &UpgradeHash) -> Self {
        *value.0
    }
}

impl<'a> From<UpgradeHash<'a>> for Vec<u8> {
    fn from(value: UpgradeHash) -> Self {
        value.0.to_vec()
    }
}

impl<'a> From<&UpgradeHash<'a>> for Vec<u8> {
    fn from(value: &UpgradeHash) -> Self {
        value.0.to_vec()
    }
}

impl<'a> Deref for UpgradeHash<'a> {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VersionError {
    #[error("failed to decode version")]
    Decode,

    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),
}
