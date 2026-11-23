use std::{fmt, ops::Bound};

use bytes::Bytes;

use crate::Table;

/// Write operations that [`Store::apply`](crate::Store::apply) applies in order, atomically.
///
/// Each operation sees the effects of those before it.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Batch {
    ops: Vec<Op>,
}

impl Batch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the value of `key`, replacing any value it had.
    pub fn put(
        &mut self,
        table: &Table,
        key: impl Into<Bytes>,
        value: impl Into<Bytes>,
    ) -> &mut Self {
        self.push(Op::Put {
            table: table.clone(),
            key: key.into(),
            value: value.into(),
        })
    }

    /// Removes `key`, if present.
    pub fn delete(&mut self, table: &Table, key: impl Into<Bytes>) -> &mut Self {
        self.push(Op::Delete {
            table: table.clone(),
            key: key.into(),
        })
    }

    /// Removes every key in `keys`.
    pub fn delete_range(&mut self, table: &Table, keys: KeyRange) -> &mut Self {
        self.push(Op::DeleteRange {
            table: table.clone(),
            keys,
        })
    }

    /// Removes every key of `table`.
    pub fn drop_table(&mut self, table: &Table) -> &mut Self {
        self.push(Op::DropTable {
            table: table.clone(),
        })
    }

    /// Appends the operations of `other`, which then apply after those already queued.
    pub fn append(&mut self, other: Batch) -> &mut Self {
        self.ops.extend(other.ops);
        self
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    fn push(&mut self, op: Op) -> &mut Self {
        self.ops.push(op);
        self
    }
}

/// See the [`Batch`] method of the same name.
#[derive(Clone, PartialEq, Eq)]
pub enum Op {
    Put {
        table: Table,
        key: Bytes,
        value: Bytes,
    },
    Delete {
        table: Table,
        key: Bytes,
    },
    DeleteRange {
        table: Table,
        keys: KeyRange,
    },
    DropTable {
        table: Table,
    },
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Put { table, key, value } => f
                .debug_struct("Put")
                .field("table", table)
                .field("key", key)
                .field("value_len", &value.len())
                .finish(),
            Self::Delete { table, key } => f
                .debug_struct("Delete")
                .field("table", table)
                .field("key", key)
                .finish(),
            Self::DeleteRange { table, keys } => f
                .debug_struct("DeleteRange")
                .field("table", table)
                .field("keys", keys)
                .finish(),
            Self::DropTable { table } => f.debug_struct("DropTable").field("table", table).finish(),
        }
    }
}

/// The start and end bounds of a non-empty [`KeyRange`].
pub type Bounds<'a> = (Bound<&'a [u8]>, Bound<&'a [u8]>);

/// A range of keys, in bytewise order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRange {
    start: Bound<Bytes>,
    end: Bound<Bytes>,
}

impl KeyRange {
    pub fn new(start: Bound<Bytes>, end: Bound<Bytes>) -> Self {
        Self { start, end }
    }

    pub fn all() -> Self {
        Self::new(Bound::Unbounded, Bound::Unbounded)
    }

    /// Keys at or after `start`.
    pub fn starting_at(start: impl Into<Bytes>) -> Self {
        Self::new(Bound::Included(start.into()), Bound::Unbounded)
    }

    /// Keys before `end`.
    pub fn before(end: impl Into<Bytes>) -> Self {
        Self::new(Bound::Unbounded, Bound::Excluded(end.into()))
    }

    /// Keys at or after `start` and before `end`.
    pub fn between(start: impl Into<Bytes>, end: impl Into<Bytes>) -> Self {
        Self::new(Bound::Included(start.into()), Bound::Excluded(end.into()))
    }

    /// The bounds, or `None` if no key lies in the range.
    ///
    /// Ordered maps may reject an empty range such as `(Excluded(a), Excluded(a))`, so the
    /// bounds are only available for a range that can contain a key.
    pub fn bounds(&self) -> Option<Bounds<'_>> {
        (!self.is_empty()).then(|| (as_slice(&self.start), as_slice(&self.end)))
    }

    pub fn contains(&self, key: &[u8]) -> bool {
        let after_start = match &self.start {
            Bound::Included(s) => key >= s.as_ref(),
            Bound::Excluded(s) => key > s.as_ref(),
            Bound::Unbounded => true,
        };
        let before_end = match &self.end {
            Bound::Included(e) => key <= e.as_ref(),
            Bound::Excluded(e) => key < e.as_ref(),
            Bound::Unbounded => true,
        };
        after_start && before_end
    }

    /// Whether no key lies in the range.
    pub fn is_empty(&self) -> bool {
        match (&self.start, &self.end) {
            (Bound::Unbounded, _) => matches!(&self.end, Bound::Excluded(e) if e.is_empty()),
            (_, Bound::Unbounded) => false,
            (Bound::Included(s), Bound::Included(e)) => s > e,
            (Bound::Included(s), Bound::Excluded(e)) | (Bound::Excluded(s), Bound::Included(e)) => {
                e <= s
            },
            // The smallest key after `s` is `s` followed by a zero byte.
            (Bound::Excluded(s), Bound::Excluded(e)) => {
                e.as_ref() <= [s.as_ref(), &[0]].concat().as_slice()
            },
        }
    }
}

fn as_slice(b: &Bound<Bytes>) -> Bound<&[u8]> {
    match b {
        Bound::Included(k) => Bound::Included(k.as_ref()),
        Bound::Excluded(k) => Bound::Excluded(k.as_ref()),
        Bound::Unbounded => Bound::Unbounded,
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    use super::*;

    fn bound(kind: u8, key: Vec<u8>) -> Bound<Bytes> {
        match kind % 3 {
            0 => Bound::Included(key.into()),
            1 => Bound::Excluded(key.into()),
            _ => Bound::Unbounded,
        }
    }

    quickcheck! {
        /// A key the range contains disproves emptiness, and a non-empty range contains
        /// its smallest member: the start itself, its successor, or the empty key.
        fn is_empty_is_exact(sk: u8, s: Vec<u8>, ek: u8, e: Vec<u8>, probe: Vec<u8>) -> bool {
            let r = KeyRange::new(bound(sk, s.clone()), bound(ek, e));
            let smallest = match sk % 3 {
                0 => s,
                1 => [s.as_slice(), &[0]].concat(),
                _ => Vec::new(),
            };
            let probe_ok = !r.contains(&probe) || !r.is_empty();
            probe_ok && (r.is_empty() == !r.contains(&smallest))
        }
    }

    #[test]
    fn empty_ranges() {
        let r = |s: &'static [u8], e: &'static [u8]| {
            KeyRange::new(
                Bound::Excluded(Bytes::from_static(s)),
                Bound::Excluded(Bytes::from_static(e)),
            )
        };
        assert!(r(b"a", b"a").is_empty());
        assert!(r(b"a", b"a\0").is_empty());
        assert!(!r(b"a", b"a\0\0").is_empty());
        assert!(!r(b"a", b"b").is_empty());
        assert!(r(b"b", b"aaaa").is_empty());
        assert!(KeyRange::before(Bytes::new()).is_empty());
        assert!(!KeyRange::all().is_empty());
    }
}
