//! An L1 event stream based on a standard JSON-RPC server.

use super::{BlockInput, ResettableStream};
use crate::{
    Result,
    types::common::{Address, L1BlockId, Timestamp},
};
use futures::stream::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tide_disco::Url;

/// An L1 event stream based on a standard JSON-RPC server.
#[derive(Debug)]
pub struct RpcStream;

impl RpcStream {
    /// Construct a new [`RpcStream`].
    ///
    /// The stream will establish a connection to the given providers over HTTP and WebSockets,
    /// respectively. It is recommended to provide both, so that WebSockets can be used to listen
    /// for new L1 heads, and then HTTP can be used to download corresponding block data. This is
    /// the cheapest and most efficient way to use JSON-RPC.
    ///
    /// If multiple providers are given for either protocol or both, the system will rotate between
    /// them if and when one provider fails.
    pub async fn new(
        _http_providers: impl IntoIterator<Item = &Url>,
        _ws_providers: impl IntoIterator<Item = &Url>,
    ) -> Result<Self> {
        todo!()
    }

    /// Get the Espresso stake table genesis block.
    pub async fn genesis(&self, _stake_table: Address) -> Result<(L1BlockId, Timestamp)> {
        todo!()
    }
}

impl Stream for RpcStream {
    type Item = BlockInput;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

impl ResettableStream for RpcStream {
    async fn reset(&mut self, _block: u64) {
        todo!()
    }
}
