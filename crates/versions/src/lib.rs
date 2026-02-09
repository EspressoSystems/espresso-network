use std::{borrow::Cow, ops::Deref};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use vbs::version::Version;

const VERSION_SIZE: usize = 4;

pub const VERSION_ZERO: Version = version(0,0);
pub const FEE_VERSION: Version = version(0,2);
pub const EPOCH_VERSION: Version = version(0,3);
pub const DRB_AND_HEADER_UPGRADE_VERSION: Version = version(0,4);
pub const DA_UPGRADE_VERSION: Version = version(0,5);
pub const VID2_UPGRADE_VERSION: Version = version(0,6);
pub const MAX_SUPPORTED_VERSION: Version = DA_UPGRADE_VERSION;

const UPGRADE_HASH: UpgradeHash<'static> =
    UpgradeHash::borrowed(&[1,0,1,0,0,1,0,0,0,1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,0,1,0,0,0,0,0,0]);

pub const fn version(major: u16, minor: u16) -> Version {
    Version { major, minor }
}

pub const fn upgrade_hash<'a>(_base: Version, _upgrade: Version) -> UpgradeHash<'a> {
    UPGRADE_HASH
}

pub fn decode_version(bytes: &[u8]) -> Result<Version, VersionError> {
    let major = bytes
        .get(0 .. 2)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
        .ok_or(VersionError::Decode)?;
    let minor = bytes
        .get(2 .. 4)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
        .ok_or(VersionError::Decode)?;
    Ok(version(major, minor))
}

pub fn encode_version(v: Version) -> [u8; VERSION_SIZE] {
    let mut buf = [0; VERSION_SIZE];
    (&mut buf[0 .. 2]).copy_from_slice(&v.major.to_le_bytes());
    (&mut buf[2 .. 4]).copy_from_slice(&v.minor.to_le_bytes());
    buf
}

pub fn encode<T>(v: Version, val: T) -> Result<Vec<u8>, VersionError>
where
    T: Serialize
{
    let mut buf = Vec::new();
    buf.extend_from_slice(&v.major.to_le_bytes()[..]);
    buf.extend_from_slice(&v.minor.to_le_bytes()[..]);
    bincode::serialize_into(&mut buf, &val)?;
    Ok(buf)
}

pub fn decode<T>(bytes: &[u8]) -> Result<(Version, T), VersionError>
where
    T: DeserializeOwned
{
    let version = decode_version(bytes)?;
    let value = bincode::deserialize(&bytes[VERSION_SIZE..])?;
    Ok((version, value))
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
