//! Byte-centric storage interface for consensus.
//!
//! A store holds named tables of byte keys and byte values, ordered bytewise by key. All
//! writes go through [`Store::apply`], which applies a [`Batch`] atomically and durably.
//!
//! This crate performs no I/O. Backends live in crates of their own.

mod batch;
mod table;

pub mod key;

#[cfg(any(test, feature = "testing"))]
pub mod conformance;
#[cfg(any(test, feature = "testing"))]
pub mod mem;

pub use batch::{Batch, Bounds, KeyRange, Op};
pub use bytes::Bytes;
pub use table::{InvalidTableName, Table};

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub rows: u64,

    /// The keys and values.
    pub bytes: u64,
}

impl Usage {
    /// The usage of one row with the given key and value lengths.
    pub fn of_row(key_len: usize, value_len: usize) -> Self {
        Self {
            rows: 1,
            bytes: (key_len + value_len) as u64,
        }
    }

    /// Saturates instead of overflowing, since stored usage may be corrupt.
    pub fn add(&mut self, other: Self) {
        self.rows = self.rows.saturating_add(other.rows);
        self.bytes = self.bytes.saturating_add(other.bytes);
    }

    /// Removes `other`, which should be part of `self`; saturates at zero.
    pub fn sub(&mut self, other: Self) {
        self.rows = self.rows.saturating_sub(other.rows);
        self.bytes = self.bytes.saturating_sub(other.bytes);
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
