use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap},
    path::PathBuf,
    time::Duration,
};

use alloy::primitives::U256;
use async_lock::RwLock;
use clap::Args;
use espresso_contract_deployer::network_config::{
    fetch_epoch_config_from_sequencer, fetch_stake_table_from_sequencer,
};
use espresso_types::SeqTypes;
use futures::FutureExt;
use hotshot_types::{
    light_client::{LegacyStateSignatureRequestBody, StateSignaturesBundle, StateVerKey},
    stake_table::one_honest_threshold,
    traits::signature_key::{StakeTableEntryType, StateSignatureKey},
    utils::{epoch_from_block_number, is_gt_epoch_root},
    PeerConfig,
};
use tide_disco::{
    api::ApiError,
    error::ServerError,
    method::{ReadState, WriteState},
    Api, App, Error as _, StatusCode,
};
use tokio::{sync::oneshot, time::sleep};
use url::Url;
use vbs::version::{StaticVersion, StaticVersionType};

use super::{LightClientState, StateSignatureRequestBody};

/// State that checks the light client state update and the signature collection
pub struct StateRelayServerState {
    /// Sequencer endpoint to query for stake table info
    sequencer_url: Url,

    /// Epoch length (fetched from HotShot config)
    blocks_per_epoch: Option<u64>,
    /// the first block where epoch 1 commence
    epoch_start_block: Option<u64>,
    /// Minimum weight to form an available state signature bundle, map: epoch_num -> threshold
    thresholds: HashMap<u64, U256>,
    /// Stake table: map: epoch_num -> map(vk, weight)
    known_nodes: HashMap<u64, HashMap<StateVerKey, U256>>,

    /// Genesis stake table
    genesis_known_nodes: HashMap<StateVerKey, U256>,
    /// Genesis threshold
    genesis_threshold: U256,

    /// Signatures bundles for each block height
    /// NOTE: nested hash-map because state signer could "vote/sign" different light client state for the same height
    bundles: HashMap<u64, HashMap<LightClientState, StateSignaturesBundle>>,

    /// Bundles for legacy light client
    legacy_bundles: HashMap<u64, HashMap<LightClientState, StateSignaturesBundle>>,

    /// The latest state signatures bundle whose total weight exceeds the threshold
    latest_available_bundle: Option<StateSignaturesBundle>,
    /// The latest state signatures bundle for legacy light client
    latest_available_legacy_bundle: Option<StateSignaturesBundle>,
    /// The block height of the latest available state signature bundle
    latest_block_height: Option<u64>,
    /// The block height of the latest available legacy state signature bundle
    latest_block_height_for_legacy: Option<u64>,

    /// A ordered queue of block heights, used for garbage collection.
    queue: BTreeSet<u64>,

    /// A ordered queue of block heights for legacy light client state, used for garbage collection.
    queue_for_legacy: BTreeSet<u64>,

    /// shutdown signal
    shutdown: Option<oneshot::Receiver<()>>,
}

impl StateRelayServerState {
    /// Init the server state
    pub fn new(sequencer_url: Url) -> Self {
        Self {
            sequencer_url,
            blocks_per_epoch: None,
            epoch_start_block: None,
            thresholds: HashMap::new(),
            known_nodes: HashMap::new(),
            genesis_known_nodes: HashMap::new(),
            genesis_threshold: U256::ZERO,
            bundles: HashMap::new(),
            legacy_bundles: HashMap::new(),
            latest_available_bundle: None,
            latest_available_legacy_bundle: None,
            latest_block_height: None,
            latest_block_height_for_legacy: None,
            queue: BTreeSet::new(),
            queue_for_legacy: BTreeSet::new(),
            shutdown: None,
        }
    }

    /// after relay server started, when the first signature arrive, we query sequencer for the genesis and update local state.
    /// The main reason we don't initialize at constructor (i.e. `Self::new()`) is due to cyclic dependency:
    /// seq0 depends on relay server to be running to post light client signatures to;
    /// relay server depends on seq0 to be running to query stake tables.
    /// Thus, our strategy is to starts relay server with `None` and empty states and fill it only when needed.
    ///
    /// Another subtlety is our epoch doesn't starts from 1, because PoS will be activated at some block height,
    /// thus `first_epoch` is not necessarily 1, but the `epoch_from_block_number(epoch_start_block, blocks_per_epoch)`.
    async fn init_genesis(&mut self) -> anyhow::Result<()> {
        // fetch genesis info from sequencer
        if self.blocks_per_epoch.is_none() || self.epoch_start_block.is_none() {
            let (blocks_per_epoch, epoch_start_block) =
                fetch_epoch_config_from_sequencer(&self.sequencer_url).await?;
            // set local state
            self.blocks_per_epoch.get_or_insert(blocks_per_epoch);
            self.epoch_start_block.get_or_insert(epoch_start_block);
        }
        let (blocks_per_epoch, epoch_start_block) = (
            // both safe unwrap
            self.blocks_per_epoch.unwrap(),
            self.epoch_start_block.unwrap(),
        );

        let first_epoch = epoch_from_block_number(epoch_start_block, blocks_per_epoch);
        tracing::info!(%blocks_per_epoch, %epoch_start_block, "Initializing genesis stake table with ");

        if self.genesis_threshold.is_zero() {
            self.init_genesis_stake_table().await?;
        }

        // init local state
        self.thresholds.insert(first_epoch, self.genesis_threshold);
        self.known_nodes
            .insert(first_epoch, self.genesis_known_nodes.clone());

        Ok(())
    }

    async fn init_genesis_stake_table(&mut self) -> anyhow::Result<()> {
        let genesis_stake_table =
            fetch_stake_table_from_sequencer(&self.sequencer_url, None).await?;
        let genesis_total_stake = genesis_stake_table.total_stakes();

        for entry in genesis_stake_table.0 {
            self.genesis_known_nodes
                .insert(entry.state_ver_key.clone(), entry.stake_table_entry.stake());
        }

        self.genesis_threshold = one_honest_threshold(genesis_total_stake);

        tracing::info!("Genesis stake table initialized with total stake {genesis_total_stake}");
        Ok(())
    }

    /// sync the stake table at `epoch` for the relayer server, fetching from the sequencer.
    /// If the requested `epoch` is older than `latest_epoch`, then does nothing.
    ///
    /// NOTE: should not be publicly invocable, always in-sync with `self.queue` for easier garbage collection.
    async fn sync_stake_table(&mut self, epoch: u64) -> anyhow::Result<()> {
        let blocks_per_epoch = self.blocks_per_epoch.expect("forget to init genesis");
        let epoch_start_block = self.epoch_start_block.expect("forget to init genesis");
        let first_epoch = epoch_from_block_number(epoch_start_block, blocks_per_epoch);
        let latest_epoch = epoch_from_block_number(
            self.latest_block_height.unwrap_or_default(),
            blocks_per_epoch,
        );
        if epoch <= latest_epoch {
            tracing::debug!(
                "Skipped stake table sync: requested epoch: {}, latest: {}",
                epoch,
                latest_epoch
            );
            return Ok(());
        }
        if self.known_nodes.contains_key(&epoch) {
            tracing::debug!(%epoch, "Skipped stake table sync: already synced ");
            return Ok(());
        }

        tracing::info!(%epoch, "Syncing stake table");

        // Stake table for the first epoch will be the same as the genesis stake table.
        if epoch > first_epoch {
            let peer_configs = {
                let client = surf_disco::Client::<ServerError, StaticVersion<0, 1>>::new(
                    self.sequencer_url.clone(),
                );
                loop {
                    match client
                        .get::<Vec<PeerConfig<SeqTypes>>>(&format!("node/stake-table/{epoch}"))
                        .send()
                        .await
                    {
                        Ok(config) => break config,
                        Err(e) => {
                            tracing::error!("Failed to fetch stake table: {e}");
                            sleep(Duration::from_secs(5)).await;
                        },
                    }
                }
            };

            // now update the local state for that epoch
            let mut total_weights = U256::ZERO;
            let mut new_nodes = HashMap::<StateVerKey, U256>::new();
            for peer in peer_configs.iter() {
                let weight = peer.stake_table_entry.stake_amount;
                new_nodes.insert(peer.state_ver_key.clone(), weight);
                total_weights += weight;
            }
            self.known_nodes.insert(epoch, new_nodes);
            self.thresholds
                .insert(epoch, one_honest_threshold(total_weights));
        } else {
            self.known_nodes
                .insert(epoch, self.genesis_known_nodes.clone());
            self.thresholds.insert(epoch, self.genesis_threshold);
        }

        tracing::info!(%epoch, "Stake table synced ");
        Ok(())
    }

    /// Centralizing all garbage-collection logic, won't panic, won't error, simply do nothing if nothing to prune.
    /// `until_height` is inclusive, meaning that would also be pruned.
    pub fn prune(&mut self, until_height: u64) {
        let blocks_per_epoch = self.blocks_per_epoch.expect("forget to init genesis");
        let oldest_epoch = if let Some(&height) = self.queue.first() {
            epoch_from_block_number(height, blocks_per_epoch)
        } else {
            1
        };

        while let Some(&height) = self.queue.first() {
            if height > until_height {
                return;
            }
            self.bundles.remove(&height);
            self.queue.pop_first();
            tracing::debug!(%height, "garbage collected for ");
        }

        let newest_epoch = epoch_from_block_number(until_height + 1, blocks_per_epoch);
        if newest_epoch > oldest_epoch {
            for epoch in oldest_epoch..newest_epoch {
                self.thresholds.remove(&epoch);
                self.known_nodes.remove(&epoch);
                tracing::debug!(%epoch, "garbage collected for ");
            }
        }
    }

    /// Centralizing all garbage-collection logic, won't panic, won't error, simply do nothing if nothing to prune.
    /// `until_height` is inclusive, meaning that would also be pruned.
    pub fn prune_for_legacy(&mut self, until_height: u64) {
        while let Some(&height) = self.queue_for_legacy.first() {
            if height > until_height {
                return;
            }
            self.legacy_bundles.remove(&height);
            self.queue_for_legacy.pop_first();
            tracing::debug!(%height, "garbage collected for ");
        }
    }

    pub fn with_shutdown_signal(
        mut self,
        shutdown_listener: Option<oneshot::Receiver<()>>,
    ) -> Self {
        if self.shutdown.is_some() {
            panic!("A shutdown signal is already registered and can not be registered twice");
        }
        self.shutdown = shutdown_listener;
        self
    }

    pub fn with_blocks_per_epoch(mut self, blocks_per_epoch: u64) -> Self {
        self.blocks_per_epoch = Some(blocks_per_epoch);
        self
    }
    pub fn with_epoch_start_block(mut self, epoch_start_block: u64) -> Self {
        self.epoch_start_block = Some(epoch_start_block);
        self
    }
    pub fn with_thresholds(mut self, thresholds: HashMap<u64, U256>) -> Self {
        self.thresholds = thresholds;
        self
    }
    pub fn with_known_nodes(
        mut self,
        known_nodes: HashMap<u64, HashMap<StateVerKey, U256>>,
    ) -> Self {
        self.known_nodes = known_nodes;
        self
    }
}

#[async_trait::async_trait]
pub trait StateRelayServerDataSource {
    /// Get the latest available signatures bundle.
    /// # Errors
    /// Errors if there's no available signatures bundle.
    fn get_latest_signature_bundle(&self) -> Result<StateSignaturesBundle, ServerError>;

    /// Post a signature to the relay server
    /// # Errors
    /// Errors if the signature is invalid, already posted, or no longer needed.
    async fn post_signature(&mut self, req: StateSignatureRequestBody) -> Result<(), ServerError>;

    /// Get the latest available signatures bundle for the legacy light client.
    /// # Errors
    /// Errors if there's no available signatures bundle.
    fn get_latest_legacy_signature_bundle(&self) -> Result<StateSignaturesBundle, ServerError>;

    /// Post a signature to the relay server for the legacy light client
    /// # Errors
    /// Errors if the signature is invalid, already posted, or no longer needed.
    async fn post_legacy_signature(
        &mut self,
        req: StateSignatureRequestBody,
    ) -> Result<(), ServerError>;
}

#[async_trait::async_trait]
impl StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<StateSignaturesBundle, ServerError> {
        match &self.latest_available_bundle {
            Some(bundle) => Ok(bundle.clone()),
            None => Err(ServerError::catch_all(
                StatusCode::NOT_FOUND,
                "The light client state signatures are not ready.".to_owned(),
            )),
        }
    }

    fn get_latest_legacy_signature_bundle(&self) -> Result<StateSignaturesBundle, ServerError> {
        match &self.latest_available_legacy_bundle {
            Some(bundle) => Ok(bundle.clone()),
            None => Err(ServerError::catch_all(
                StatusCode::NOT_FOUND,
                "The legacy light client state signatures are not ready.".to_owned(),
            )),
        }
    }

    async fn post_signature(&mut self, req: StateSignatureRequestBody) -> Result<(), ServerError> {
        // sanity check the signature validity first before adding in
        if !req
            .key
            .verify_state_sig(&req.signature, &req.state, &req.next_stake)
        {
            // If it's a legacy signature, handle it separately
            if req.key.legacy_verify_state_sig(&req.signature, &req.state) {
                return self.post_legacy_signature(req).await;
            }
            tracing::warn!("Received invalid signature: {:?}", req);
            return Err(ServerError::catch_all(
                StatusCode::BAD_REQUEST,
                "The posted signature is not valid.".to_owned(),
            ));
        }

        let block_height = req.state.block_height;
        if block_height <= self.latest_block_height.unwrap_or(0) {
            // This signature is no longer needed
            return Ok(());
        }

        let blocks_per_epoch = match self.blocks_per_epoch {
            Some(v) => v,
            None => {
                self.init_genesis().await.map_err(|e| {
                    ServerError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
                })?;
                self.blocks_per_epoch
                    .expect("internal err, init_genesis() wrong")
            },
        };
        let epoch = epoch_from_block_number(block_height, blocks_per_epoch);

        // if epoch is activated and block_height is greater than epoch root, ignore and drop
        if block_height > self.epoch_start_block.expect("init_genesis wrong")
            && is_gt_epoch_root(block_height, blocks_per_epoch)
        {
            return Ok(());
        }

        if !self.known_nodes.contains_key(&epoch) {
            self.sync_stake_table(epoch).await.map_err(|e| {
                ServerError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
            })?;
        }

        // retrieve the signer/sender's weight from the correct stake table for that epoch
        let Some(nodes) = self.known_nodes.get(&epoch) else {
            return Err(ServerError::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Stake table not found".to_owned(),
            ));
        };
        let Some(threshold) = self.thresholds.get(&epoch) else {
            return Err(ServerError::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Threshold not found".to_owned(),
            ));
        };
        let Some(weight) = nodes.get(&req.key) else {
            return Err(ServerError::catch_all(
                StatusCode::UNAUTHORIZED,
                "Signature posted by nodes not on the stake table".to_owned(),
            ));
        };

        let bundles_at_height = self.bundles.entry(block_height).or_default();
        self.queue.insert(block_height);

        let bundle = bundles_at_height
            .entry(req.state)
            .or_insert(StateSignaturesBundle {
                state: req.state,
                next_stake: req.next_stake,
                signatures: Default::default(),
                accumulated_weight: U256::from(0),
            });
        tracing::debug!(
            "Accepting new signature for block height {} from {}.",
            block_height,
            req.key
        );
        match bundle.signatures.entry(req.key) {
            Entry::Occupied(_) => {
                // A signature is already posted for this key with this state
                return Err(ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "A signature of this light client state is already posted at this block \
                     height for this key."
                        .to_owned(),
                ));
            },
            Entry::Vacant(entry) => {
                entry.insert(req.signature);
                bundle.accumulated_weight += *weight;
            },
        }

        if bundle.accumulated_weight >= *threshold {
            tracing::info!(
                "State signature bundle at block height {} is ready to serve.",
                block_height
            );
            self.latest_block_height = Some(block_height);
            self.latest_available_bundle = Some(bundle.clone());

            // garbage collect
            self.prune(block_height);
        }

        Ok(())
    }

    async fn post_legacy_signature(
        &mut self,
        req: StateSignatureRequestBody,
    ) -> Result<(), ServerError> {
        // Initialize the stake table if not yet done
        if self.genesis_threshold.is_zero() {
            if let Err(e) = self.init_genesis_stake_table().await {
                tracing::error!("Failed to initialize genesis stake table: {e}");
                return Err(ServerError::catch_all(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Relay server is not ready to receive signatures: {e}"),
                ));
            }
        }
        let block_height = req.state.block_height;
        if block_height <= self.latest_block_height_for_legacy.unwrap_or(0) {
            // This signature is no longer needed
            return Ok(());
        }
        let Some(weight) = self.genesis_known_nodes.get(&req.key) else {
            tracing::warn!(
                "Received invalid legacy signature from unknown node: {:?}",
                req
            );
            return Err(ServerError::catch_all(
                StatusCode::UNAUTHORIZED,
                "Legacy signature posted by nodes not on the stake table".to_owned(),
            ));
        };

        // sanity check the signature validity first before adding in
        if !req.key.legacy_verify_state_sig(&req.signature, &req.state) {
            tracing::warn!("Received invalid legacy signature: {:?}", req);
            return Err(ServerError::catch_all(
                StatusCode::BAD_REQUEST,
                "The posted legacy signature is not valid.".to_owned(),
            ));
        }

        let bundles_at_height = self.legacy_bundles.entry(block_height).or_default();
        self.queue_for_legacy.insert(block_height);

        let bundle = bundles_at_height
            .entry(req.state)
            .or_insert(StateSignaturesBundle {
                state: req.state,
                next_stake: req.next_stake,
                signatures: Default::default(),
                accumulated_weight: U256::from(0),
            });
        tracing::debug!(
            "Accepting new legacy signature for block height {} from {}.",
            block_height,
            req.key
        );
        match bundle.signatures.entry(req.key) {
            Entry::Occupied(_) => {
                // A signature is already posted for this key with this state
                return Err(ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "A legacy signature of this light client state is already posted at this \
                     block height for this key."
                        .to_owned(),
                ));
            },
            Entry::Vacant(entry) => {
                entry.insert(req.signature);
                bundle.accumulated_weight += *weight;
            },
        }

        if bundle.accumulated_weight >= self.genesis_threshold {
            tracing::info!(
                "Legacy state signature bundle at block height {} is ready to serve.",
                block_height
            );
            self.latest_block_height_for_legacy = Some(block_height);
            self.latest_available_legacy_bundle = Some(bundle.clone());

            // garbage collect
            self.prune_for_legacy(block_height);
        }

        Ok(())
    }
}

/// configurability options for the web server
#[derive(Args, Default)]
pub struct Options {
    #[arg(
        long = "state-relay-server-api-path",
        env = "STATE_RELAY_SERVER_API_PATH"
    )]
    /// path to API
    pub api_path: Option<PathBuf>,
}

/// Set up APIs for relay server
fn define_api<State, ApiVer: StaticVersionType + 'static>(
    options: &Options,
    _: ApiVer,
) -> Result<Api<State, ServerError, ApiVer>, ApiError>
where
    State: 'static + Send + Sync + ReadState + WriteState,
    <State as ReadState>::State: Send + Sync + StateRelayServerDataSource,
{
    let mut api = match &options.api_path {
        Some(path) => Api::<State, ServerError, ApiVer>::from_file(path)?,
        None => {
            let toml: toml::Value = toml::from_str(include_str!(
                "../../api/state_relay_server.toml"
            ))
            .map_err(|err| ApiError::CannotReadToml {
                reason: err.to_string(),
            })?;
            Api::<State, ServerError, ApiVer>::new(toml)?
        },
    };

    api.get("getlateststate", |_req, state| {
        async move { state.get_latest_signature_bundle() }.boxed()
    })?
    .post("poststatesignature", move |req, state| {
        async move {
            if let Ok(body) = req.body_auto::<StateSignatureRequestBody, ApiVer>(ApiVer::instance())
            {
                state.post_signature(body).await
            } else if let Ok(legacy_body) =
                req.body_auto::<LegacyStateSignatureRequestBody, ApiVer>(ApiVer::instance())
            {
                state.post_legacy_signature(legacy_body.into()).await
            } else {
                Err(ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "Invalid request body".to_string(),
                ))
            }
        }
        .boxed()
    })?
    .get("getlatestlegacystate", |_req, state| {
        async move { state.get_latest_legacy_signature_bundle() }.boxed()
    })?
    .post("postlegacystatesignature", move |req, state| {
        async move {
            let body = req
                .body_auto::<StateSignatureRequestBody, ApiVer>(ApiVer::instance())
                .map_err(ServerError::from_request_error)?;
            state.post_legacy_signature(body).await?;
            Ok(())
        }
        .boxed()
    })?;

    Ok(api)
}

pub async fn run_relay_server<ApiVer: StaticVersionType + 'static>(
    shutdown_listener: Option<oneshot::Receiver<()>>,
    sequencer_url: Url,
    url: Url,
    bind_version: ApiVer,
) -> anyhow::Result<()> {
    let options = Options::default();
    let api = define_api(&options, bind_version).unwrap();

    let state = RwLock::new(
        StateRelayServerState::new(sequencer_url).with_shutdown_signal(shutdown_listener),
    );
    let mut app = App::<RwLock<StateRelayServerState>, ServerError>::with_state(state);

    app.register_module("api", api).unwrap();

    let app_future = app.serve(url.clone(), bind_version);
    app_future.await?;

    tracing::info!(%url, "Relay server starts serving at ");

    Ok(())
}

pub async fn run_relay_server_with_state<ApiVer: StaticVersionType + 'static>(
    server_url: Url,
    bind_version: ApiVer,
    state: StateRelayServerState,
) -> anyhow::Result<()> {
    let options = Options::default();
    let api = define_api(&options, bind_version).unwrap();

    let mut app = App::<RwLock<StateRelayServerState>, ServerError>::with_state(RwLock::new(state));
    app.register_module("api", api).unwrap();

    let app_future = app.serve(server_url.clone(), bind_version);
    app_future.await?;

    tracing::info!(%server_url, "Relay server starts serving at ");

    Ok(())
}
