//! Type-erased mirrors of the [`crate::v1`] API traits.
//!
//! The `router_*` builders in [`crate::axum`] used to be generic over the state type, so every
//! handler closure, and the aide/axum machinery behind it, was codegened once per concrete state
//! (three in the node binary, plus the test mock). The traits here repeat the `v1` traits with
//! their associated types replaced by [`Erased`], which makes them object safe: the routers take
//! an `Arc<dyn ...>` and are compiled once. Cost at runtime is one virtual call and one box per
//! response.
//!
//! Each trait has a blanket impl forwarding to the corresponding `v1` trait, so every state type
//! gets the erased view for free, and only those forwarders are monomorphized per state.
//!
//! [`erase_api`] generates both from one list of method names. That is not only brevity: erasure
//! collapses distinct return types onto [`Erased`], so a mirror method wired to the wrong `v1`
//! method type-checks where the typed `v1` trait would have rejected it. Six of
//! [`DynHotShotAvailabilityApi`]'s range getters share one erased signature, and so do
//! [`DynAvailabilityApi`]'s `get_state_cert` and `get_state_cert_v2`. Naming each method once
//! makes that particular mismatch unspellable rather than merely unlikely.
//!
//! It does not make the mirror wrong-proof: argument order is still positional, a method left off
//! a list is simply not mirrored, and labelling an `Option<Self::Assoc>` method `erased` turns a
//! 404 into a `null` body. Those need a test asserting a payload, which `axum.rs` does not have.

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::HeaderMap;
use futures::stream::{BoxStream, StreamExt};

use crate::{
    axum::decode_body,
    error::{ApiError, classify},
    v1,
};

/// A response value whose type has been erased. `Box<dyn erased_serde::Serialize>` serializes
/// exactly like the value it holds, so JSON and VBS bodies are unchanged.
pub(crate) type Erased = Box<dyn erased_serde::Serialize + Send + Sync>;

fn erase<T: serde::Serialize + Send + Sync + 'static>(value: T) -> Erased {
    Box::new(value)
}

/// Declare a type-erased mirror of a `v1` trait, plus the blanket impl forwarding to it.
///
/// Methods are grouped by the shape of the `v1` return type, which is what you read off the `v1`
/// trait when adding one:
///
/// - `concrete f(..) -> T` names no associated type, and is forwarded unchanged
/// - `assoc f(..)` returns `Self::Assoc`, which becomes [`Erased`]
/// - `optional f(..)` returns `Option<Self::Assoc>`, where `None` is a 404 rather than a body
/// - `stream f(..)` returns `BoxStream<'static, Self::Assoc>`
///
/// The groups must appear in that order. Per-method `///` docs would make the group boundaries
/// ambiguous to the macro parser, so annotate an entry with a `//` comment instead.
///
/// A method whose *argument* has an erased type cannot be generated, because the body must be
/// decoded where the concrete type is still known; write those by hand.
// spellchecker:off
macro_rules! erase_api {
    (
        $dyn_trait:ident => $v1_trait:ident;
        $(concrete $cn:ident($($ca:ident: $ct:ty),* $(,)?) -> $crt:ty;)*
        $(assoc $an:ident($($aa:ident: $at:ty),* $(,)?);)*
        $(optional $on:ident($($oa:ident: $ot:ty),* $(,)?);)*
        $(stream $sn:ident($($sa:ident: $st:ty),* $(,)?);)*
    ) => {
        #[async_trait]
        pub(crate) trait $dyn_trait: Send + Sync {
            $(async fn $cn(&self, $($ca: $ct),*) -> anyhow::Result<$crt>;)*
            $(async fn $an(&self, $($aa: $at),*) -> anyhow::Result<Erased>;)*
            $(async fn $on(&self, $($oa: $ot),*) -> anyhow::Result<Option<Erased>>;)*
            $(async fn $sn(&self, $($sa: $st),*) -> anyhow::Result<BoxStream<'static, Erased>>;)*
        }

        #[async_trait]
        impl<T: v1::$v1_trait + Send + Sync> $dyn_trait for T {
            $(async fn $cn(&self, $($ca: $ct),*) -> anyhow::Result<$crt> {
                <T as v1::$v1_trait>::$cn(self, $($ca),*).await
            })*
            $(async fn $an(&self, $($aa: $at),*) -> anyhow::Result<Erased> {
                <T as v1::$v1_trait>::$an(self, $($aa),*).await.map(erase)
            })*
            $(async fn $on(&self, $($oa: $ot),*) -> anyhow::Result<Option<Erased>> {
                Ok(<T as v1::$v1_trait>::$on(self, $($oa),*).await?.map(erase))
            })*
            $(async fn $sn(&self, $($sa: $st),*) -> anyhow::Result<BoxStream<'static, Erased>> {
                Ok(<T as v1::$v1_trait>::$sn(self, $($sa),*).await?.map(erase).boxed())
            })*
        }
    };
}
// spellchecker:on

// The state types the `router_*` builders take, one per router.
pub(crate) type RewardState = Arc<dyn DynRewardApi>;
pub(crate) type AvailabilityState = Arc<dyn DynAvailability>;
pub(crate) type BlockState = Arc<dyn DynBlockStateApi>;
pub(crate) type FeeState = Arc<dyn DynFeeStateApi>;
pub(crate) type StatusState = Arc<dyn DynStatusApi>;
pub(crate) type ConfigState = Arc<dyn DynConfigApi>;
pub(crate) type NodeState = Arc<dyn DynNodeApi>;
pub(crate) type CatchupState = Arc<dyn DynCatchupApi>;
pub(crate) type SubmitState = Arc<dyn DynSubmitApi>;
pub(crate) type StateSignatureState = Arc<dyn DynStateSignatureApi>;
pub(crate) type HotShotEventsState = Arc<dyn DynHotShotEventsApi>;
pub(crate) type LightClientState = Arc<dyn DynLightClientApi>;
pub(crate) type ExplorerState = Arc<dyn DynExplorerApi>;
pub(crate) type DatabaseState = Arc<dyn DynDatabaseApi>;
/// [`v1::TokenApi`] has no associated types, so it is already object safe.
pub(crate) type TokenState = Arc<dyn v1::TokenApi + Send + Sync>;

/// The availability router serves both availability traits from one state object.
pub(crate) trait DynAvailability: DynAvailabilityApi + DynHotShotAvailabilityApi {}
impl<T: DynAvailabilityApi + DynHotShotAvailabilityApi> DynAvailability for T {}

erase_api! {
    DynRewardApi => RewardApi;

    concrete get_reward_state_height() -> u64;
    concrete get_reward_state_v2_height() -> u64;

    assoc get_reward_account_proof_v1(height: u64, address: String);
    assoc get_reward_claim_input(block_height: u64, address: String);
    assoc get_reward_balance(height: u64, address: String);
    assoc get_latest_reward_balance(address: String);
    assoc get_reward_account_proof(height: u64, address: String);
    assoc get_latest_reward_account_proof(address: String);
    assoc get_reward_amounts(height: u64, offset: u64, limit: u64);
    assoc get_reward_merkle_tree_v2(height: u64);
    assoc get_reward_state_path_v1(snapshot: v1::Snapshot, key: String);
    assoc get_reward_state_path_v2(snapshot: v1::Snapshot, key: String);
}

erase_api! {
    DynAvailabilityApi => AvailabilityApi;

    assoc get_namespace_proof(block_id: v1::BlockId, namespace: u32);
    assoc get_namespace_proof_range(from: u64, until: u64, namespace: u32);
    assoc get_incorrect_encoding_proof(block_id: v1::BlockId, namespace: u32);
    assoc get_state_cert(epoch: u64);
    assoc get_state_cert_v2(epoch: u64);

    stream stream_namespace_proofs(from: usize, namespace: u32);
}

erase_api! {
    DynHotShotAvailabilityApi => HotShotAvailabilityApi;

    assoc get_leaf(id: v1::LeafId);
    assoc get_leaf_range(from: usize, until: usize);
    assoc get_header(id: v1::BlockId);
    assoc get_header_range(from: usize, until: usize);
    assoc get_block(id: v1::BlockId);
    assoc get_block_range(from: usize, until: usize);
    assoc get_payload(id: v1::PayloadId);
    assoc get_payload_range(from: usize, until: usize);
    assoc get_vid_common(id: v1::BlockId);
    assoc get_vid_common_range(from: usize, until: usize);
    assoc get_transaction_by_position(height: u64, index: u64);
    assoc get_transaction_by_hash(hash: String);
    assoc get_transaction_proof_by_position(height: u64, index: u64);
    assoc get_transaction_proof_by_hash(hash: String);
    assoc get_block_summary(height: usize);
    assoc get_block_summary_range(from: usize, until: usize);
    assoc get_limits();

    // None is a 404 rather than a body, so the option survives erasure.
    optional get_cert2(height: u64);

    stream stream_leaves(from: usize);
    stream stream_headers(from: usize);
    stream stream_blocks(from: usize);
    stream stream_payloads(from: usize);
    stream stream_vid_common(from: usize);
    stream stream_transactions(from: usize, namespace: Option<u32>);
}

erase_api! {
    DynBlockStateApi => BlockStateApi;

    concrete get_block_state_height() -> u64;

    assoc get_block_state_path(snapshot: v1::Snapshot, key: String);
}

erase_api! {
    DynFeeStateApi => FeeStateApi;

    concrete get_fee_state_height() -> u64;

    assoc get_fee_state_path(snapshot: v1::Snapshot, key: String);
    assoc get_fee_balance_latest(address: String);
}

erase_api! {
    DynStatusApi => StatusApi;

    concrete block_height() -> u64;
    concrete success_rate() -> f64;
    concrete time_since_last_decide() -> u64;
    concrete metrics() -> String;

    assoc keys();
}

erase_api! {
    DynConfigApi => ConfigApi;

    concrete env() -> Vec<String>;

    assoc hotshot_config();
    assoc runtime_config();
}

erase_api! {
    DynNodeApi => NodeApi;

    concrete block_height() -> u64;
    concrete count_transactions(from: Option<u64>, to: Option<u64>, namespace: Option<u64>) -> u64;
    concrete payload_size(from: Option<u64>, to: Option<u64>, namespace: Option<u64>) -> u64;

    assoc get_vid_share(id: v1::VidShareId);
    assoc sync_status();
    assoc get_header_window(start: v1::HeaderWindowStart, end: u64);
    assoc limits();
    assoc stake_table(epoch: u64);
    assoc stake_table_current();
    assoc da_stake_table(epoch: u64);
    assoc da_stake_table_current();
    assoc get_validators(epoch: u64);
    assoc get_all_validators(epoch: u64, offset: u64, limit: u64);
    assoc current_proposal_participation();
    assoc proposal_participation(epoch: u64);
    assoc current_vote_participation();
    assoc vote_participation(epoch: u64);
    assoc get_block_reward(epoch: Option<u64>);
    assoc get_oldest_block();
    assoc get_oldest_leaf();
}

erase_api! {
    DynCatchupGet => CatchupApi;

    assoc get_account(height: u64, view: u64, address: String);
    assoc get_blocks_frontier(height: u64, view: u64);
    assoc get_chain_config(commitment: String);
    assoc get_leaf_chain(height: u64);
    assoc get_cert2(height: u64);
    assoc get_reward_account_v1(height: u64, view: u64, address: String);
    assoc get_reward_account_v2(height: u64, view: u64, address: String);
    assoc get_reward_merkle_tree_v2(height: u64, view: u64);
    assoc get_state_cert(epoch: u64);
}

erase_api! {
    DynStateSignatureApi => StateSignatureApi;

    assoc get_state_signature(height: u64);
}

erase_api! {
    DynHotShotEventsApi => HotShotEventsApi;

    assoc startup_info();

    stream events();
}

erase_api! {
    DynLightClientApi => LightClientApi;

    assoc get_leaf_proof(query: v1::LeafQuery, finalized: Option<u64>);
    assoc get_header_proof(root: u64, requested: v1::HeaderQuery);
    assoc get_light_client_stake_table(epoch: u64);
    assoc get_payload_proof(height: u64);
    assoc get_payload_proof_range(start: u64, end: u64);
    assoc get_lc_namespace_proof(height: u64, namespace: u64);
    assoc get_lc_namespace_proof_range(start: u64, end: u64, namespace: u64);
    assoc get_lc_namespaces_proof_range(start: u64, end: u64, namespaces: String);
}

erase_api! {
    DynExplorerApi => ExplorerApi;

    assoc get_block_detail(ident: v1::BlockIdent);
    assoc get_block_summaries(target: v1::BlockIdent, limit: u64);
    assoc get_transaction_detail(ident: v1::TxIdent);
    assoc get_transaction_summaries(target: v1::TxIdent, limit: u64, filter: v1::TxSummaryFilter);
    assoc get_explorer_summary();
    assoc get_search_result(query: String);
}

erase_api! {
    DynDatabaseApi => DatabaseApi;

    assoc get_table_sizes();
    assoc get_migration_status();
}

/// The catchup router serves both halves from one state object; the split exists because the two
/// POST endpoints below cannot be generated, not because it is an API boundary.
pub(crate) trait DynCatchupApi: DynCatchupGet + DynCatchupPost {}
impl<T: DynCatchupGet + DynCatchupPost> DynCatchupApi for T {}

/// `get_accounts` and `get_reward_accounts_v1` decode a list whose element type is erased, so the
/// decode happens behind the trait object where the concrete type is still in scope. Errors match
/// the typed path: a bad body is the 400 [`decode_body`] returns, a handler failure goes through
/// [`classify`].
#[async_trait]
pub(crate) trait DynCatchupPost: Send + Sync {
    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError>;
    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError>;
}

#[async_trait]
impl<T: v1::CatchupApi + Send + Sync> DynCatchupPost for T {
    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError> {
        let accounts: Vec<<T as v1::CatchupApi>::FeeAccount> = decode_body(headers, body)?;
        v1::CatchupApi::get_accounts(self, height, view, accounts)
            .await
            .map(erase)
            .map_err(classify)
    }
    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError> {
        let accounts: Vec<<T as v1::CatchupApi>::RewardAccountV1> = decode_body(headers, body)?;
        v1::CatchupApi::get_reward_accounts_v1(self, height, view, accounts)
            .await
            .map(erase)
            .map_err(classify)
    }
}

#[async_trait]
pub(crate) trait DynSubmitApi: Send + Sync {
    /// The transaction type is erased, so the body is decoded behind the trait object; a bad
    /// body is the 400 [`decode_body`] returns and a submit failure a 500, as on the typed path.
    async fn submit(&self, headers: &HeaderMap, body: &[u8]) -> Result<Erased, ApiError>;
}

#[async_trait]
impl<T: v1::SubmitApi + Send + Sync> DynSubmitApi for T {
    async fn submit(&self, headers: &HeaderMap, body: &[u8]) -> Result<Erased, ApiError> {
        let tx: <T as v1::SubmitApi>::Transaction = decode_body(headers, body)?;
        v1::SubmitApi::submit(self, tx)
            .await
            .map(erase)
            .map_err(ApiError::Internal)
    }
}
