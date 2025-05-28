use serde::{Deserialize, Serialize};
use std::iter::Peekable;

use derive_more::Display;
use std::ops::Range;
use thiserror::Error;

use hotshot_types::vid::advz::{LargeRangeProofType, SmallRangeProofType};

use std::default::Default;

/// Proof of correctness for namespace payload bytes in a block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ADVZNsProof {
    pub(crate) ns_index: NsIndex,
    pub(crate) ns_payload: NsPayloadOwned,
    pub(crate) ns_proof: Option<LargeRangeProofType>, // `None` if ns_payload is empty
}

/// Byte lengths for the different items that could appear in a namespace table.
pub const NUM_NSS_BYTE_LEN: usize = 4;
pub const NS_OFFSET_BYTE_LEN: usize = 4;

// TODO prefer [`NS_ID_BYTE_LEN`] set to `8` because [`NamespaceId`] is a `u64`
// but we need to maintain serialization compatibility.
// https://github.com/EspressoSystems/espresso-sequencer/issues/1574
pub const NS_ID_BYTE_LEN: usize = 4;

/// Raw binary data for a namespace table.
///
/// Any sequence of bytes is a valid [`NsTable`].
///
/// # Binary format of a namespace table
///
/// Byte lengths for the different items that could appear in a namespace table
/// are specified in local private constants [`NUM_NSS_BYTE_LEN`],
/// [`NS_OFFSET_BYTE_LEN`], [`NS_ID_BYTE_LEN`].
///
/// ## Number of entries in the namespace table
///
/// The first [`NUM_NSS_BYTE_LEN`] bytes of the namespace table indicate the
/// number `n` of entries in the table as a little-endian unsigned integer. If
/// the entire table length is smaller than [`NUM_NSS_BYTE_LEN`] then the
/// missing bytes are zero-padded.
///
/// The bytes in the namespace table beyond the first [`NUM_NSS_BYTE_LEN`] bytes
/// encode table entries. Each entry consumes exactly [`NS_ID_BYTE_LEN`] `+`
/// [`NS_OFFSET_BYTE_LEN`] bytes.
///
/// The number `n` could be anything, including a number much larger than the
/// number of entries that could fit in the namespace table. As such, the actual
/// number of entries in the table is defined as the minimum of `n` and the
/// maximum number of whole entries that could fit in the table.
///
/// See [`Self::in_bounds`] for clarification.
///
/// ## Namespace table entry
///
/// ### Namespace ID
///
/// The first [`NS_ID_BYTE_LEN`] bytes of each table entry indicate the
/// [`NamespaceId`] for this namespace. Any table entry whose [`NamespaceId`] is
/// a duplicate of a previous entry is ignored. A correct count of the number of
/// *unique* (non-ignored) entries is given by `NsTable::iter().count()`.
///
/// ### Namespace offset
///
/// The next [`NS_OFFSET_BYTE_LEN`] bytes of each table entry indicate the
/// end-index of a namespace in the block payload bytes
/// [`Payload`](super::payload::Payload). This end-index is a little-endian
/// unsigned integer.
///
/// # How to deduce a namespace's byte range
///
/// In order to extract the payload bytes of a single namespace `N` from the
/// block payload one needs both the start- and end-indices for `N`.
///
/// See [`Self::ns_range`] for clarification. What follows is a description of
/// what's implemented in [`Self::ns_range`].
///
/// If `N` occupies the `i`th entry in the namespace table for `i>0` then the
/// start-index for `N` is defined as the end-index of the `(i-1)`th entry in
/// the table.
///
/// Even if the `(i-1)`the entry would otherwise be ignored (due to a duplicate
/// [`NamespaceId`] or any other reason), that entry's end-index still defines
/// the start-index of `N`. This rule guarantees that both start- and
/// end-indices for any namespace `N` can be read from a constant-size byte
/// range in the namespace table, and it eliminates the need to traverse an
/// unbounded number of previous entries of the namespace table looking for a
/// previous non-ignored entry.
///
/// The start-index of the 0th entry in the table is implicitly defined to be
/// `0`.
///
/// The start- and end-indices `(declared_start, declared_end)` declared in the
/// namespace table could be anything. As such, the actual start- and
/// end-indices `(start, end)` are defined so as to ensure that the byte range
/// is well-defined and in-bounds for the block payload:
/// ```ignore
/// end = min(declared_end, block_payload_byte_length)
/// start = min(declared_start, end)
/// ```
///
/// In a "honestly-prepared" namespace table the end-index of the final
/// namespace equals the byte length of the block payload. (Otherwise the block
/// payload might have bytes that are not included in any namespace.)
///
/// It is possible that a namespace table could indicate two distinct namespaces
/// whose byte ranges overlap, though no "honestly-prepared" namespace table
/// would do this.
///
/// TODO prefer [`NsTable`] to be a newtype like this
/// ```ignore
/// #[repr(transparent)]
/// #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
/// #[serde(transparent)]
/// pub struct NsTable(#[serde(with = "base64_bytes")] Vec<u8>);
/// ```
/// but we need to maintain serialization compatibility.
/// <https://github.com/EspressoSystems/espresso-sequencer/issues/1575>
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
// Boilerplate: `#[serde(remote = "Self")]` needed to check invariants on
// deserialization. See
// https://github.com/serde-rs/serde/issues/1220#issuecomment-382589140
#[serde(remote = "Self")]
pub struct NsTable {
    #[serde(with = "base64_bytes")]
    pub(crate) bytes: Vec<u8>,
}

/// Return type for [`NsTable::validate`].
#[derive(Error, Debug, Display, Eq, PartialEq)]
pub enum NsTableValidationError {
    InvalidByteLen,
    NonIncreasingEntries,
    DuplicateNamespaceId,
    InvalidHeader, // TODO this variant obsolete after https://github.com/EspressoSystems/espresso-sequencer/issues/1604
    InvalidFinalOffset, // TODO this variant obsolete after https://github.com/EspressoSystems/espresso-sequencer/issues/1604
    ExpectNonemptyNsTable,
}

pub struct NsTableBuilder {
    pub(crate) bytes: Vec<u8>,
    pub(crate) num_entries: usize,
}

/// Index for an entry in a ns table.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NsIndex(pub(crate) usize);

/// Number of entries in a namespace table.
pub struct NumNss(pub(crate) usize);

/// Return type for [`Payload::ns_iter`].
pub struct NsIter(pub(crate) Range<usize>);

/// Raw payload data for an entire block.
///
/// A block consists of two sequences of arbitrary bytes:
/// - `ns_table`: namespace table
/// - `ns_payloads`: namespace payloads
///
/// Any sequence of bytes is a valid `ns_table`. Any sequence of bytes is a
/// valid `ns_payloads`. The contents of `ns_table` determine how to interpret
/// `ns_payload`.
///
/// # Namespace table
///
/// See [`NsTable`] for the format of a namespace table.
///
/// # Namespace payloads
///
/// A concatenation of payload bytes for multiple individual namespaces.
/// Namespace boundaries are dictated by `ns_table`. See [`NsPayload`] for the
/// format of a namespace payload.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Payload {
    // Concatenated payload bytes for each namespace
    //
    // TODO want to rename thisfield to `ns_payloads`, but can't due to
    // serialization compatibility.
    #[serde(with = "base64_bytes")]
    pub(crate) raw_payload: Vec<u8>,

    pub(crate) ns_table: NsTable,
}

/// Byte length of a block payload, which includes all namespaces but *not* the
/// namespace table.
#[derive(Clone, Debug, Display, Eq, Hash, PartialEq)]
pub struct PayloadByteLen(pub(crate) usize);

pub use hotshot_query_service::availability::TransactionIndex as Index;

/// Cartesian product of [`NsIter`], [`TxIter`].
pub struct Iter<'a> {
    pub(crate) ns_iter: Peekable<NsIter>,
    pub(crate) tx_iter: Option<TxIter>,
    pub(crate) block: &'a Payload,
}

/// Index range for a namespace payload inside a block payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NsPayloadRange(pub(crate) Range<usize>);

/// Raw binary data for a single namespace's payload.
///
/// Any sequence of bytes is a valid [`NsPayload`].
///
/// See module-level documentation [`types`](super::types) for a full
/// specification of the binary format of a namespace.
pub struct NsPayload(pub(crate) [u8]);

#[repr(transparent)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NsPayloadOwned(#[serde(with = "base64_bytes")] pub(crate) Vec<u8>);

/// Proof of correctness for transaction bytes in a block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TxProof {
    // Naming conventions for this struct's fields:
    // - `payload_x`: bytes from the payload
    // - `payload_proof_x`: a proof of those bytes from the payload
    pub(crate) tx_index: TxIndex,

    // Number of txs declared in the tx table
    pub(crate) payload_num_txs: NumTxsUnchecked,
    pub(crate) payload_proof_num_txs: SmallRangeProofType,

    // Tx table entries for this tx
    pub(crate) payload_tx_table_entries: TxTableEntries,
    pub(crate) payload_proof_tx_table_entries: SmallRangeProofType,

    // This tx's payload bytes.
    // `None` if this tx has zero length.
    pub(crate) payload_proof_tx: Option<SmallRangeProofType>,
}

/// Byte lengths for the different items that could appear in a tx table.
pub const NUM_TXS_BYTE_LEN: usize = 4;
pub const TX_OFFSET_BYTE_LEN: usize = 4;

/// Number of txs in a namespace.
///
/// Like [`NumTxsUnchecked`] but checked against a [`NsPayloadByteLen`].
pub struct NumTxs(pub(crate) usize);

/// Byte length of a namespace payload.
pub struct NsPayloadByteLen(pub(crate) usize);

/// The part of a tx table that declares the number of txs in the payload.
///
/// "Unchecked" because this quantity might exceed the number of tx table
/// entries that could fit into the namespace that contains it.
///
/// Use [`NumTxs`] for the actual number of txs in this namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumTxsUnchecked(pub(crate) usize);

/// Byte range for the part of a tx table that declares the number of txs in the
/// payload.
pub struct NumTxsRange(pub(crate) Range<usize>);

/// Entries from a tx table in a namespace for use in a transaction proof.
///
/// Contains either one or two entries according to whether it was derived from
/// the first transaction in the namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxTableEntries {
    pub(crate) cur: usize,
    pub(crate) prev: Option<usize>, // `None` if derived from the first transaction
}

/// Byte range for entries from a tx table for use in a transaction proof.
///
/// This range covers either one or two entries from a tx table according to
/// whether it was derived from the first transaction in the namespace.
pub struct TxTableEntriesRange(pub(crate) Range<usize>);

/// A transaction's payload data.
pub struct TxPayload<'a>(pub(crate) &'a [u8]);

/// Byte range for a transaction's payload data.
pub struct TxPayloadRange(pub(crate) Range<usize>);

/// Index for an entry in a tx table.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TxIndex(pub(crate) usize);

pub struct TxIter(pub(crate) Range<usize>);

/// Build an individual namespace payload one transaction at a time.
///
/// Use [`Self::append_tx`] to add each transaction. Use [`Self::into_bytes`]
/// when you're done. The returned bytes include a well-formed tx table and all
/// tx payloads.
#[derive(Default)]
pub struct NsPayloadBuilder {
    pub(crate) tx_table_entries: Vec<u8>,
    pub(crate) tx_bodies: Vec<u8>,
}
