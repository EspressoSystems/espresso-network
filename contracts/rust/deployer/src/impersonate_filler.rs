use alloy::{
    network::{Network, TransactionBuilder},
    primitives::Address,
    providers::{
        fillers::{FillerControlFlow, TxFiller},
        Provider, SendableTx,
    },
    transports::TransportResult,
};

/// A filler that sets the `from` field on transactions to impersonate a specific address.
/// This is useful when using Anvil's impersonation features to send transactions from
/// accounts that we don't have the private key for.
///
/// Avoids having to manually set the `from` field on transactions.
#[derive(Clone, Debug, Default)]
pub struct ImpersonateFiller {
    from: Address,
}

impl ImpersonateFiller {
    pub fn new(from: Address) -> Self {
        Self { from }
    }
}

#[derive(Clone, Debug)]
pub struct ImpersonateFillable {
    pub from: Address,
}

impl<N: Network> TxFiller<N> for ImpersonateFiller {
    type Fillable = ImpersonateFillable;

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        if tx.from().is_none() {
            FillerControlFlow::Ready
        } else {
            FillerControlFlow::Finished
        }
    }

    async fn prepare<P: Provider<N>>(
        &self,
        _provider: &P,
        _tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable> {
        Ok(ImpersonateFillable { from: self.from })
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_from(fillable.from);
        }
        Ok(tx)
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_from(self.from);
        }
    }
}
