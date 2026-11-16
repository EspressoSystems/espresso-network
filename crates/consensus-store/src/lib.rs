//! Byte-centric storage interface for consensus.
//!
//! A store holds named tables of byte keys and byte values, ordered bytewise by key. All
//! writes go through [`Store::apply`], which applies a [`Batch`] atomically and durably.
//! Values longer than [`EXTERN_THRESHOLD`] are stored outside the store's index, in files of
//! their own; callers see no difference except in [`Usage`].
//!
//! This crate performs no I/O. Backends live in crates of their own.

mod batch;
mod table;

pub mod key;

#[cfg(any(test, feature = "testing"))]
pub mod conformance;
#[cfg(any(test, feature = "testing"))]
pub mod mem;

pub use batch::{Batch, Bounds, KeyRange, Op, UpdateFn};
pub use bytes::Bytes;
pub use table::{InvalidTableName, Table};

/// Values longer than this many bytes are stored extern, all others inline.
pub const EXTERN_THRESHOLD: usize = 64 * 1024;

/// A storage backend.
pub trait Store: Send + Sync + 'static {
    type Snapshot: Snapshot + Send;

    /// Applies the operations of `batch` in order, atomically and durably.
    ///
    /// When this returns `Ok`, every operation is on disk. When it returns any error other
    /// than [`Error::Poisoned`], none is, and the same batch can be applied again.
    ///
    /// [`Error::Poisoned`] means the outcome is unknown: the batch may or may not be on disk.
    /// The store then refuses every further `apply` until it is reopened, and reopening
    /// shows which.
    fn apply(&self, batch: &Batch) -> Result<(), Error>;

    /// A consistent view of the store as of the latest completed `apply`.
    ///
    /// A snapshot never blocks `apply`, and later writes are invisible to it.
    fn snapshot(&self) -> Result<Self::Snapshot, Error>;
}

/// A consistent read-only view of a [`Store`].
///
/// A table that was never written, or was dropped, reads as empty.
pub trait Snapshot {
    fn get(&self, table: &Table, key: &[u8]) -> Result<Option<Bytes>, Error>;

    /// The length of the value of `key`, without reading an extern value.
    fn value_len(&self, table: &Table, key: &[u8]) -> Result<Option<u64>, Error>;

    /// The entries of `table` whose keys lie in `keys`, in ascending key order.
    fn range(
        &self,
        table: &Table,
        keys: &KeyRange,
    ) -> Result<impl Iterator<Item = Result<(Bytes, Bytes), Error>> + Send, Error>;

    /// The space `table` takes up.
    fn usage(&self, table: &Table) -> Result<Usage, Error>;
}

/// The space a table takes up.
///
/// Rows that share an extern value through [`Batch::copy`] each count its full length.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub rows: u64,

    /// Keys, and values of at most [`EXTERN_THRESHOLD`] bytes.
    pub inline_bytes: u64,

    /// Values longer than [`EXTERN_THRESHOLD`] bytes.
    pub extern_bytes: u64,
}

impl Usage {
    /// The usage of one row with the given key and value lengths.
    pub fn of_row(key_len: usize, value_len: usize) -> Self {
        if value_len > EXTERN_THRESHOLD {
            Self {
                rows: 1,
                inline_bytes: key_len as u64,
                extern_bytes: value_len as u64,
            }
        } else {
            Self {
                rows: 1,
                inline_bytes: (key_len + value_len) as u64,
                extern_bytes: 0,
            }
        }
    }

    /// Saturates instead of overflowing, since stored usage may be corrupt.
    pub fn add(&mut self, other: Self) {
        self.rows = self.rows.saturating_add(other.rows);
        self.inline_bytes = self.inline_bytes.saturating_add(other.inline_bytes);
        self.extern_bytes = self.extern_bytes.saturating_add(other.extern_bytes);
    }

    /// Removes `other`, which should be part of `self`; saturates at zero.
    pub fn sub(&mut self, other: Self) {
        self.rows = self.rows.saturating_sub(other.rows);
        self.inline_bytes = self.inline_bytes.saturating_sub(other.inline_bytes);
        self.extern_bytes = self.extern_bytes.saturating_sub(other.extern_bytes);
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// Stored data does not match what was written.
    #[error("corrupt value in table {table} at key {key:x?}: {reason}")]
    Corrupt {
        table: Table,
        key: Bytes,
        reason: &'static str,
    },

    #[error("backend error: {0}")]
    Backend(Box<dyn std::error::Error + Send + Sync>),

    /// A commit failed after it may have reached disk. See [`Store::apply`].
    #[error("store must be reopened, a commit may or may not have reached disk: {0}")]
    Poisoned(Box<dyn std::error::Error + Send + Sync>),
}
