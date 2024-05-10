//! The only thing [`NsPayload2`] does is naively read from its payload given a
//! byte range. It doesn't know anything about the underlying binary format.
//! That's all done in `NsPayloadRange2`.
use crate::block2::num_txs::NumTxs;

use super::{
    newtypes::{NumTxsRangeRelative, TxOffset, TxOffsetRangeRelative},
    uint_bytes::usize_from_bytes,
    TX_OFFSET_BYTE_LEN,
};

pub struct NsPayload2([u8]);

impl NsPayload2 {
    /// Read the number of txs declared in the tx table.
    pub fn read_num_txs(&self, range: &NumTxsRangeRelative) -> NumTxs {
        NumTxs::from_bytes2(&self.0[range.0.clone()])
    }

    /// Read the `index`th entry from the tx table.
    pub fn read_tx_offset(&self, range: &TxOffsetRangeRelative) -> TxOffset {
        // TODO do not use `usize_from_bytes` or any constants. Instead make a
        // TxOffset::from_bytes method.
        TxOffset(usize_from_bytes::<TX_OFFSET_BYTE_LEN>(
            &self.0[range.0.clone()],
        ))
    }

    // TODO write helper wrappers for `NsPayloadRange`, eg `num_txs()`?
}
