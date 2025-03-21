// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::HashMap;

use async_broadcast::Receiver;
use async_trait::async_trait;
use futures::Stream;
use hotshot::{traits::BlockPayload, types::Event};
use hotshot_builder_api::{
    v0_1::{
        self,
        block_info::{AvailableBlockData, AvailableBlockInfo},
        builder::{Error, Options},
    },
    v0_2::block_info::AvailableBlockHeaderInputV1,
    v0_99,
};
use hotshot_types::{
    constants::{LEGACY_BUILDER_MODULE, MARKETPLACE_BUILDER_MODULE},
    traits::{
        block_contents::EncodeBytes, node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
};
use tide_disco::{method::ReadState, App, Url};
use tokio::spawn;
use vbs::version::StaticVersionType;

use crate::test_builder::BuilderChange;

pub mod random;
pub use random::RandomBuilderImplementation;

pub mod simple;
pub use simple::SimpleBuilderImplementation;

#[async_trait]
pub trait TestBuilderImplementation<TYPES: NodeType>
where
    <TYPES as NodeType>::InstanceState: Default,
{
    type Config: Default;

    async fn start(
        num_storage_nodes: usize,
        url: Url,
        options: Self::Config,
        changes: HashMap<u64, BuilderChange>,
    ) -> Box<dyn BuilderTask<TYPES>>;
}

pub trait BuilderTask<TYPES: NodeType>: Send + Sync {
    fn start(
        self: Box<Self>,
        stream: Box<dyn Stream<Item = Event<TYPES>> + std::marker::Unpin + Send + 'static>,
    );
}

/// Entry for a built block
#[derive(Debug, Clone)]
struct BlockEntry<TYPES: NodeType> {
    metadata: AvailableBlockInfo<TYPES>,
    payload: Option<AvailableBlockData<TYPES>>,
    header_input: Option<AvailableBlockHeaderInputV1<TYPES>>,
}

/// Construct a tide disco app that mocks the builder API 0.1 + 0.3.
///
/// # Panics
/// If constructing and launching the builder fails for any reason
pub fn run_builder_source<TYPES, Source>(
    url: Url,
    mut change_receiver: Receiver<BuilderChange>,
    source: Source,
) where
    TYPES: NodeType,
    <TYPES as NodeType>::InstanceState: Default,
    Source: Clone + Send + Sync + tide_disco::method::ReadState + 'static,
    <Source as ReadState>::State: Sync
        + Send
        + v0_1::data_source::BuilderDataSource<TYPES>
        + v0_99::data_source::BuilderDataSource<TYPES>,
{
    spawn(async move {
        let start_builder = |url: Url, source: Source| -> _ {
            let builder_api_0_1 = hotshot_builder_api::v0_1::builder::define_api::<Source, TYPES>(
                &Options::default(),
            )
            .expect("Failed to construct the builder API");
            let builder_api_0_3 = hotshot_builder_api::v0_99::builder::define_api::<Source, TYPES>(
                &Options::default(),
            )
            .expect("Failed to construct the builder API");
            let mut app: App<Source, Error> = App::with_state(source);
            app.register_module(LEGACY_BUILDER_MODULE, builder_api_0_1)
                .expect("Failed to register the builder API 0.1")
                .register_module(MARKETPLACE_BUILDER_MODULE, builder_api_0_3)
                .expect("Failed to register the builder API 0.3");
            spawn(app.serve(url, hotshot_builder_api::v0_1::Version::instance()))
        };

        let mut handle = Some(start_builder(url.clone(), source.clone()));

        while let Ok(event) = change_receiver.recv().await {
            match event {
                BuilderChange::Up if handle.is_none() => {
                    handle = Some(start_builder(url.clone(), source.clone()));
                },
                BuilderChange::Down => {
                    if let Some(handle) = handle.take() {
                        handle.abort();
                    }
                },
                _ => {},
            }
        }
    });
}

/// Construct a tide disco app that mocks the builder API 0.1.
///
/// # Panics
/// If constructing and launching the builder fails for any reason
pub fn run_builder_source_0_1<TYPES, Source>(
    url: Url,
    mut change_receiver: Receiver<BuilderChange>,
    source: Source,
) where
    TYPES: NodeType,
    <TYPES as NodeType>::InstanceState: Default,
    Source: Clone + Send + Sync + tide_disco::method::ReadState + 'static,
    <Source as ReadState>::State: Sync + Send + v0_1::data_source::BuilderDataSource<TYPES>,
{
    spawn(async move {
        let start_builder = |url: Url, source: Source| -> _ {
            let builder_api = hotshot_builder_api::v0_1::builder::define_api::<Source, TYPES>(
                &Options::default(),
            )
            .expect("Failed to construct the builder API");
            let mut app: App<Source, Error> = App::with_state(source);
            app.register_module(LEGACY_BUILDER_MODULE, builder_api)
                .expect("Failed to register the builder API");
            spawn(app.serve(url, hotshot_builder_api::v0_1::Version::instance()))
        };

        let mut handle = Some(start_builder(url.clone(), source.clone()));

        while let Ok(event) = change_receiver.recv().await {
            match event {
                BuilderChange::Up if handle.is_none() => {
                    handle = Some(start_builder(url.clone(), source.clone()));
                },
                BuilderChange::Down => {
                    if let Some(handle) = handle.take() {
                        handle.abort();
                    }
                },
                _ => {},
            }
        }
    });
}

/// Helper function to construct all builder data structures from a list of transactions
async fn build_block<TYPES: NodeType>(
    transactions: Vec<TYPES::Transaction>,
    pub_key: TYPES::BuilderSignatureKey,
    priv_key: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,
) -> BlockEntry<TYPES>
where
    <TYPES as NodeType>::InstanceState: Default,
{
    let (block_payload, metadata) = TYPES::BlockPayload::from_transactions(
        transactions,
        &Default::default(),
        &Default::default(),
    )
    .await
    .expect("failed to build block payload from transactions");

    let commitment = block_payload.builder_commitment(&metadata);

    // Get block size from the encoded payload
    let block_size = block_payload.encode().len() as u64;

    let signature_over_block_info =
        TYPES::BuilderSignatureKey::sign_block_info(&priv_key, block_size, 123, &commitment)
            .expect("Failed to sign block info");

    let signature_over_builder_commitment =
        TYPES::BuilderSignatureKey::sign_builder_message(&priv_key, commitment.as_ref())
            .expect("Failed to sign commitment");

    let signature_over_fee_info =
        TYPES::BuilderSignatureKey::sign_fee(&priv_key, 123_u64, &metadata)
            .expect("Failed to sign fee info");

    let block = AvailableBlockData {
        block_payload,
        metadata,
        sender: pub_key.clone(),
        signature: signature_over_builder_commitment,
    };
    let metadata = AvailableBlockInfo {
        sender: pub_key.clone(),
        signature: signature_over_block_info,
        block_hash: commitment,
        block_size,
        offered_fee: 123,
        _phantom: std::marker::PhantomData,
    };
    let header_input = AvailableBlockHeaderInputV1 {
        fee_signature: signature_over_fee_info,
        sender: pub_key,
    };

    BlockEntry {
        metadata,
        payload: Some(block),
        header_input: Some(header_input),
    }
}
