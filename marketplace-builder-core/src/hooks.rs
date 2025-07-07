use std::marker::PhantomData;

use async_trait::async_trait;
use hotshot::types::LegacyEvent;
use hotshot_types::traits::node_implementation::NodeType;

/// A trait for hooks into the builder service. Used to further customize
/// builder behaviour in ways not possible in builder core.
/// If you don't need such customisation, use [`NoHooks`].
///
/// A simple example filtering incoming transactions based on some imaginary
/// application-specific magic byte:
/// ```rust
/// # type MyTypes = hotshot_example_types::node_types::TestTypes;
/// # type MyTransaction = hotshot_example_types::block_types::TestTransaction;
/// use marketplace_builder_core::hooks::BuilderHooks;
/// struct MyBuilderHooks { magic: u8 };
///
/// #[async_trait::async_trait]
/// impl BuilderHooks<MyTypes> for MyBuilderHooks {
///     async fn process_transactions(&self, transactions: Vec<MyTransaction>) -> Vec<MyTransaction> {
///         transactions.into_iter().filter(|tx| tx.bytes()[0] == self.magic).collect()
///     }
/// }
/// ```
#[async_trait]
pub trait BuilderHooks<Types: NodeType>: Sync + Send + 'static {
    /// Implement this to process transactions before
    /// they'll be passed on to the builder.
    #[inline(always)]
    async fn process_transactions(
        &self,
        transactions: Vec<Types::Transaction>,
    ) -> Vec<Types::Transaction> {
        transactions
    }

    /// Handle any hotshot event _before_ the builder event loop handles it.
    /// Event handling is done sequentially, i.e. you can rely on the fact
    /// that the builder will process this event _after_ the hooks have finished
    /// processing it. Accordingly, if this property is not important to you,
    /// and especially if you're doing anything involved with this hook
    /// it is advisable to spawn a task doing the actual work and return,
    /// so that builder's event loop isn't blocked for too long.
    #[inline(always)]
    async fn handle_hotshot_event(&self, _event: &LegacyEvent<Types>) {}
}

#[async_trait]
impl<T: ?Sized, Types> BuilderHooks<Types> for Box<T>
where
    Types: NodeType,
    T: BuilderHooks<Types>,
{
    #[inline(always)]
    async fn process_transactions(
        &self,
        transactions: Vec<Types::Transaction>,
    ) -> Vec<Types::Transaction> {
        (**self).process_transactions(transactions).await
    }

    #[inline(always)]
    async fn handle_hotshot_event(&self, event: &LegacyEvent<Types>) {
        (**self).handle_hotshot_event(event).await
    }
}

/// Hooks that do nothing
pub struct NoHooks<Types: NodeType>(pub PhantomData<Types>);

impl<Types: NodeType> BuilderHooks<Types> for NoHooks<Types> {}
