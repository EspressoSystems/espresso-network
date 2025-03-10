//! Types related to a namespace table.
//!
//! All code that needs to know the binary format of a namespace table is
//! restricted to this file.
//!
//! See [`NsTable`] for a full specification of the binary format of a namespace
//! table.
use std::{collections::HashSet, sync::Arc};

use committable::{Commitment, Committable, RawCommitmentBuilder};
use hotshot_types::traits::EncodeBytes;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    v0::impls::block::uint_bytes::{
        bytes_serde_impl, u32_from_bytes, u32_to_bytes, usize_from_bytes, usize_to_bytes,
    },
    NamespaceId, NsIndex, NsIter, NsPayloadRange, NsTable, NsTableBuilder, NsTableValidationError,
    NumNss, PayloadByteLen, NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN,
};

// Boilerplate: `#[serde(remote = "Self")]` allows invariant checking on
// deserialization via re-implementation of `Deserialize` in terms of default
// derivation. See
// https://github.com/serde-rs/serde/issues/1220#issuecomment-382589140
impl<'de> Deserialize<'de> for NsTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let unchecked = NsTable::deserialize(deserializer)?;
        unchecked
            .validate_deserialization_invariants()
            .map_err(de::Error::custom)?;
        Ok(unchecked)
    }
}

// Boilerplate: use of `#[serde(remote = "Self")]` must include a trivial
// `Serialize` impl. See
// https://github.com/serde-rs/serde/issues/1220#issuecomment-382589140
impl Serialize for NsTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        NsTable::serialize(self, serializer)
    }
}

impl NsTable {
    /// Search the namespace table for the ns_index belonging to `ns_id`.
    pub fn find_ns_id(&self, ns_id: &NamespaceId) -> Option<NsIndex> {
        self.iter()
            .find(|index| self.read_ns_id_unchecked(index) == *ns_id)
    }

    /// Number of entries in the namespace table.
    ///
    /// Defined as the maximum number of entries that could fit in the namespace
    /// table, ignoring what's declared in the table header.
    pub fn len(&self) -> NumNss {
        NumNss(
            self.bytes.len().saturating_sub(NUM_NSS_BYTE_LEN)
                / NS_ID_BYTE_LEN.saturating_add(NS_OFFSET_BYTE_LEN),
        )
    }

    /// Iterator over all unique namespaces in the namespace table.
    pub fn iter(&self) -> impl Iterator<Item = NsIndex> + '_ {
        NsIter::new(&self.len())
    }

    /// Read the namespace id from the `index`th entry from the namespace table.
    /// Returns `None` if `index` is out of bounds.
    ///
    /// TODO I want to restrict visibility to `pub(crate)` or lower but this
    /// method is currently used in `nasty-client`.
    pub fn read_ns_id(&self, index: &NsIndex) -> Option<NamespaceId> {
        if !self.in_bounds(index) {
            None
        } else {
            Some(self.read_ns_id_unchecked(index))
        }
    }

    /// Like [`Self::read_ns_id`] except `index` is not checked. Use [`Self::in_bounds`] as needed.
    pub fn read_ns_id_unchecked(&self, index: &NsIndex) -> NamespaceId {
        let start = index.0 * (NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN) + NUM_NSS_BYTE_LEN;

        // TODO hack to deserialize `NamespaceId` from `NS_ID_BYTE_LEN` bytes
        // https://github.com/EspressoSystems/espresso-sequencer/issues/1574
        NamespaceId::from(u32_from_bytes::<NS_ID_BYTE_LEN>(
            &self.bytes[start..start + NS_ID_BYTE_LEN],
        ))
    }

    /// Does the `index`th entry exist in the namespace table?
    pub fn in_bounds(&self, index: &NsIndex) -> bool {
        self.len().in_bounds(index)
    }

    /// Instantiate an `NsTable` from a byte slice.
    pub fn from_bytes_unchecked(bytes: &[u8]) -> NsTable {
        NsTable {
            bytes: bytes.to_vec(),
        }
    }

    /// Are the bytes of this [`NsTable`] uncorrupted?
    ///
    /// # Checks
    /// 1. Byte length must hold a whole number of entries.
    /// 2. All offsets must increase monotonically. Offsets
    ///    must be nonzero. Namespace IDs must be unique.
    /// 3. Header consistent with byte length. (Obsolete after
    ///    <https://github.com/EspressoSystems/espresso-sequencer/issues/1604>.)
    /// 4. Final offset must equal `payload_byte_len`. (Obsolete after
    ///    <https://github.com/EspressoSystems/espresso-sequencer/issues/1604>.)
    ///    If the namespace table is empty then `payload_byte_len` must be 0.
    pub fn validate(
        &self,
        payload_byte_len: &PayloadByteLen,
    ) -> Result<(), NsTableValidationError> {
        use NsTableValidationError::*;

        // conditions 1-3
        self.validate_deserialization_invariants()?;

        // condition 4
        let len = self.len().0;
        if len > 0 {
            let final_ns_index = NsIndex(len - 1);
            let final_offset = self.read_ns_offset_unchecked(&final_ns_index);
            if final_offset != payload_byte_len.as_usize() {
                return Err(InvalidFinalOffset);
            }
        } else if payload_byte_len.as_usize() != 0 {
            return Err(ExpectNonemptyNsTable);
        }

        Ok(())
    }

    // CRATE-VISIBLE HELPERS START HERE

    /// Read subslice range for the `index`th namespace from the namespace
    /// table.
    pub fn ns_range(&self, index: &NsIndex, payload_byte_len: &PayloadByteLen) -> NsPayloadRange {
        let end = self
            .read_ns_offset_unchecked(index)
            .min(payload_byte_len.as_usize());
        let start = if index.0 == 0 {
            0
        } else {
            self.read_ns_offset_unchecked(&NsIndex(index.0 - 1))
        }
        .min(end);
        NsPayloadRange::new(start, end)
    }

    // PRIVATE HELPERS START HERE

    /// Read the number of namespaces declared in the namespace table. THIS
    /// QUANTITY IS NEVER USED. Instead use [`NsTable::len`].
    ///
    /// TODO Delete this method after
    /// <https://github.com/EspressoSystems/espresso-sequencer/issues/1604>
    fn read_num_nss(&self) -> usize {
        let num_nss_byte_len = NUM_NSS_BYTE_LEN.min(self.bytes.len());
        usize_from_bytes::<NUM_NSS_BYTE_LEN>(&self.bytes[..num_nss_byte_len])
    }

    /// Read the namespace offset from the `index`th entry from the namespace table.
    fn read_ns_offset_unchecked(&self, index: &NsIndex) -> usize {
        let start =
            index.0 * (NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN) + NUM_NSS_BYTE_LEN + NS_ID_BYTE_LEN;
        usize_from_bytes::<NS_OFFSET_BYTE_LEN>(&self.bytes[start..start + NS_OFFSET_BYTE_LEN])
    }

    /// Helper for [`NsTable::validate`], used in our custom [`serde`]
    /// implementation.
    ///
    /// Checks conditions 1-3 of [`NsTable::validate`]. Those conditions can be
    /// checked by looking only at the contents of the [`NsTable`].
    fn validate_deserialization_invariants(&self) -> Result<(), NsTableValidationError> {
        use NsTableValidationError::*;

        // Byte length for a table with `x` entries must be exactly `x *
        // NsTableBuilder::entry_byte_len() +
        // NsTableBuilder::header_byte_len()`.
        //
        // Explanation for the following `if` condition:
        //
        // The above condition is equivalent to `[byte length] -
        // header_byte_len` equals 0 modulo `entry_byte_len`. In order to
        // compute `[byte length] - header_byte_len` we must first check that
        // `[byte length]` is not exceeded by `header_byte_len`
        if self.bytes.len() < NsTableBuilder::header_byte_len()
            || (self.bytes.len() - NsTableBuilder::header_byte_len())
                % NsTableBuilder::entry_byte_len()
                != 0
        {
            return Err(InvalidByteLen);
        }

        // Header must declare the correct number of namespaces
        //
        // TODO this check obsolete after
        // https://github.com/EspressoSystems/espresso-sequencer/issues/1604
        if self.len().0 != self.read_num_nss() {
            return Err(InvalidHeader);
        }

        // Offsets must increase monotonically. Offsets must
        // be nonzero. Namespace IDs must be unique
        {
            let mut prev_offset = 0;
            let mut repeat_ns_ids = HashSet::<NamespaceId>::new();
            for (ns_id, offset) in self.iter().map(|i| {
                (
                    self.read_ns_id_unchecked(&i),
                    self.read_ns_offset_unchecked(&i),
                )
            }) {
                if !repeat_ns_ids.insert(ns_id) {
                    return Err(DuplicateNamespaceId);
                }
                if offset <= prev_offset {
                    return Err(NonIncreasingEntries);
                }
                prev_offset = offset;
            }
        }

        Ok(())
    }
}

impl EncodeBytes for NsTable {
    fn encode(&self) -> Arc<[u8]> {
        Arc::from(self.bytes.as_ref())
    }
}

impl Committable for NsTable {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new(&Self::tag())
            .var_size_bytes(&self.bytes)
            .finalize()
    }

    fn tag() -> String {
        "NSTABLE".into()
    }
}

impl NsTableBuilder {
    // >>>> change
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // pre-allocate space for the ns table header
        Self {
            bytes: Vec::from([0; NUM_NSS_BYTE_LEN]),
            num_entries: 0,
        }
    }

    /// Add an entry to the namespace table.
    pub fn append_entry(&mut self, ns_id: NamespaceId, offset: usize) {
        // hack to serialize `NamespaceId` to `NS_ID_BYTE_LEN` bytes
        self.bytes
            .extend(u32_to_bytes::<NS_ID_BYTE_LEN>(u32::from(ns_id)));
        self.bytes
            .extend(usize_to_bytes::<NS_OFFSET_BYTE_LEN>(offset));
        self.num_entries += 1;
    }

    /// Serialize to bytes and consume self.
    pub fn into_ns_table(self) -> NsTable {
        let mut bytes = self.bytes;
        // write the number of entries to the ns table header
        bytes[..NUM_NSS_BYTE_LEN]
            .copy_from_slice(&usize_to_bytes::<NUM_NSS_BYTE_LEN>(self.num_entries));
        NsTable { bytes }
    }

    /// Byte length of a namespace table header.
    pub const fn header_byte_len() -> usize {
        NUM_NSS_BYTE_LEN
    }

    /// Byte length of a single namespace table entry.
    pub const fn entry_byte_len() -> usize {
        NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN
    }
}

bytes_serde_impl!(NsIndex, to_bytes, [u8; NUM_NSS_BYTE_LEN], from_bytes);

impl NsIndex {
    pub fn to_bytes(&self) -> [u8; NUM_NSS_BYTE_LEN] {
        usize_to_bytes::<NUM_NSS_BYTE_LEN>(self.0)
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        Self(usize_from_bytes::<NUM_NSS_BYTE_LEN>(bytes))
    }
}

impl NumNss {
    pub fn in_bounds(&self, index: &NsIndex) -> bool {
        index.0 < self.0
    }
}

impl NsIter {
    pub fn new(num_nss: &NumNss) -> Self {
        Self(0..num_nss.0)
    }
}

// Simple `impl Iterator` delegates to `Range`.
impl Iterator for NsIter {
    type Item = NsIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(NsIndex)
    }
}

#[cfg(test)]
mod test;
