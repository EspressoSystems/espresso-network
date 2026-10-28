use ::light_client::{
    LightClient,
    state::LightClientOptions,
    storage::{SqlStorage, Storage},
};
use anyhow::Context as _;
use async_trait::async_trait;
use espresso_types::SeqTypes;
use hotshot_query_service::fetching::{self, Provider};
use tokio::sync::OnceCell;

use super::PeerClient;
use crate::api::{ApiState, light_client_genesis};

type PeerLightClient = LightClient<SqlStorage, PeerClient>;

/// A [`Provider`] that fetches from staked peers over the query lane, verified by a light client.
///
/// Like [`LightClientProvider`](crate::api::LightClientProvider) it is created before the node
/// context exists and builds its light client on first use. Its light client database is a
/// private temporary one, so it never contends with the HTTP light client's.
#[derive(Debug)]
pub(crate) struct PeerProvider {
    light_client: OnceCell<PeerLightClient>,
    state: ApiState,
    client: PeerClient,
    opt: LightClientOptions,
}

impl PeerProvider {
    pub(in crate::api) fn new(
        state: ApiState,
        client: PeerClient,
        opt: LightClientOptions,
    ) -> PeerProvider {
        PeerProvider {
            light_client: OnceCell::new(),
            state,
            client,
            opt,
        }
    }

    async fn light_client(&self) -> Option<&PeerLightClient> {
        self.light_client
            .get_or_try_init(|| self.init())
            .await
            .inspect_err(|err| {
                tracing::warn!("peer light client unavailable, retrying on the next fetch: {err:#}")
            })
            .ok()
    }

    async fn init(&self) -> anyhow::Result<PeerLightClient> {
        let ctx = self.state.context().await;
        let db = <SqlStorage as Storage>::default()
            .await
            .context("opening the peer light client database")?;
        let chain_id = ctx.node_state().genesis_chain_config.chain_id;
        let genesis = light_client_genesis(&ctx.network_config(), chain_id);
        Ok(LightClient::from_genesis_with_options(
            db,
            self.client.clone(),
            genesis,
            self.opt.clone(),
        ))
    }
}

#[async_trait]
impl<T> Provider<SeqTypes, T> for PeerProvider
where
    T: fetching::Request<SeqTypes> + 'static,
    PeerLightClient: Provider<SeqTypes, T>,
{
    async fn fetch(&self, req: T) -> Option<T::Response> {
        self.light_client().await?.fetch(req).await
    }
}
