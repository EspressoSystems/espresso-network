//! The v1 paths that the state relay server and the light client prover read from a node's API.
//!
//! The API service serves v2, but the relay server and the prover are main's and ask for v1: the
//! stake table of an epoch and the hotshot config, to know whose signatures count, and the state
//! certificate of an epoch, which a proof carries across an epoch boundary. These paths answer
//! them, in the format their client decodes, and nothing else of v1.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chain_store::ChainStore;
use espresso_types::{SeqTypes, v0_4::StateCertQueryDataV2};
use hotshot_types::{PeerConfig, data::EpochNumber, network::NetworkConfig};
use serde_json::json;

use crate::Membership;

/// What the v1 paths answer from.
pub struct Compat {
    store: ChainStore,
    membership: Membership,

    /// The chain's configuration from its genesis file: its epochs and genesis stake table.
    config: NetworkConfig<SeqTypes>,
}

impl Compat {
    pub fn new(store: ChainStore, membership: Membership, config: NetworkConfig<SeqTypes>) -> Self {
        Self {
            store,
            membership,
            config,
        }
    }

    /// The v1 paths, both with the `/v1` prefix and without, as main's API serves them.
    pub fn router(self: Arc<Self>) -> Router {
        let routes = Router::new()
            .route("/node/stake-table/{epoch}", get(stake_table))
            .route("/config/hotshot", get(hotshot_config))
            .route("/availability/state-cert-v2/{epoch}", get(state_cert))
            .with_state(self);
        Router::new().merge(routes.clone()).nest("/v1", routes)
    }
}

async fn stake_table(State(compat): State<Arc<Compat>>, Path(epoch): Path<u64>) -> Response {
    match compat.membership.stake_table(EpochNumber::new(epoch)) {
        Some(table) => Json::<Vec<PeerConfig<SeqTypes>>>(table.0).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            format!("no stake table of epoch {epoch} yet"),
        )
            .into_response(),
    }
}

/// The parts of the hotshot config the relay server reads: the epoch layout and the genesis
/// stake table.
async fn hotshot_config(State(compat): State<Arc<Compat>>) -> Response {
    let config = &compat.config.config;
    Json(json!({
        "config": {
            "epoch_height": config.epoch_height,
            "epoch_start_block": config.epoch_start_block,
            "known_nodes_with_stake": config.known_nodes_with_stake,
            "known_da_nodes": config.known_da_nodes,
        }
    }))
    .into_response()
}

async fn state_cert(State(compat): State<Arc<Compat>>, Path(epoch): Path<u64>) -> Response {
    let root = compat.membership.root_of(EpochNumber::new(epoch));
    match compat.store.state_cert(root).await {
        Ok(Some(cert)) => Json(StateCertQueryDataV2::<SeqTypes>(cert)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            format!("no state certificate of epoch {epoch}"),
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use chain_store::{Parts, mem::MemBackend};
    use espresso_contract_deployer::network_config::{
        fetch_epoch_config_from_sequencer, fetch_stake_table_from_sequencer,
    };
    use http_client::{Client, error::ClientErr};
    use tokio::net::TcpListener;
    use url::Url;
    use vbs::version::StaticVersion;

    use super::*;
    use crate::{ApiService, membership::testing::*};

    /// Serves the v1 paths over `store`, returning the API's URL as the relay server is given it.
    async fn serve(store: ChainStore) -> Url {
        let mut config = NetworkConfig::<SeqTypes>::default();
        config.config.known_nodes_with_stake = committee();
        config.config.known_da_nodes = committee();
        config.config.epoch_height = EPOCH_HEIGHT;
        config.config.epoch_start_block = 1;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let service = ApiService::new(store.clone(), "127.0.0.1:1".parse().unwrap())
            .with_compat(Compat::new(store, membership(), config));
        tokio::spawn(service.serve(listener));
        url.parse().unwrap()
    }

    /// The relay server reads the epoch layout and the stake tables with these very functions.
    #[tokio::test]
    async fn answers_the_relay_server() {
        let url = serve(ChainStore::new(MemBackend::default())).await;
        let (epoch_height, start) = fetch_epoch_config_from_sequencer(&url).await.unwrap();
        assert_eq!((epoch_height, start), (EPOCH_HEIGHT, 1));

        let genesis = fetch_stake_table_from_sequencer(&url, None).await.unwrap();
        assert_eq!(genesis.0, committee());
        let epoch = EpochNumber::new(FIRST_EPOCH + 1);
        let table = fetch_stake_table_from_sequencer(&url, Some(epoch))
            .await
            .unwrap();
        assert_eq!(table.0, committee());
    }

    /// The prover reads the state certificate of an epoch as JSON, as here.
    #[tokio::test]
    async fn answers_the_prover() {
        let store = ChainStore::new(MemBackend::default());
        let membership = membership();
        let epoch = FIRST_EPOCH + 1;
        let root = membership.root_of(EpochNumber::new(epoch));
        let cert = crate::membership::testing::state_cert(root, &[0, 1]);
        let parts = Parts {
            state_cert: Some(cert.clone()),
            ..Default::default()
        };
        store.put_parts(root, &parts).await.unwrap();
        let url = serve(store).await;

        let client = Client::<ClientErr, StaticVersion<0, 1>>::new(url);
        let answer = client
            .get::<StateCertQueryDataV2<SeqTypes>>(&format!("availability/state-cert-v2/{epoch}"))
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap();
        assert_eq!(answer.0, cert);
        client
            .get::<StateCertQueryDataV2<SeqTypes>>(&format!(
                "availability/state-cert-v2/{}",
                epoch + 1
            ))
            .header("Accept", "application/json")
            .send()
            .await
            .expect_err("no certificate of the next epoch yet");
    }
}
