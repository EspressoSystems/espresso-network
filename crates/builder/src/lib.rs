use std::{sync::Arc, time::Duration};

use async_lock::{Mutex, RwLock};
use espresso_types::{v0_1::NoStorage, v0_3::Fetcher, EpochCommittees, NodeState};
use hotshot_types::{epoch_membership::EpochMembershipCoordinator, traits::metrics::NoMetrics};
use sequencer::{catchup::StatePeers, L1Params, SequencerApiVersion};
use url::Url;

pub fn build_instance_state(
    genesis: sequencer::Genesis,
    l1_params: L1Params,
    state_peers: Vec<Url>,
) -> NodeState {
    let chain_config = genesis.chain_config;
    let genesis_version = genesis.genesis_version;
    let l1_client = l1_params
        .options
        .connect(l1_params.urls)
        .expect("failed to create L1 client");

    let peers = Arc::new(StatePeers::<SequencerApiVersion>::from_urls(
        state_peers,
        Default::default(),
        Duration::from_secs(2),
        &NoMetrics,
    ));

    let fetcher = Fetcher::new(
        peers.clone(),
        Arc::new(Mutex::new(NoStorage)),
        l1_client.clone(),
        chain_config,
    );

    let coordinator = EpochMembershipCoordinator::new(
        Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            Default::default(),
            None,
            fetcher,
            genesis.epoch_height.unwrap_or_default(),
        ))),
        genesis.epoch_height.unwrap_or_default(),
        &Arc::new(sequencer::persistence::no_storage::NoStorage),
    );

    NodeState::new(
        u64::MAX, // dummy node ID, only used for debugging
        chain_config,
        l1_client,
        peers,
        genesis.base_version,
        coordinator,
        genesis_version,
    )
}
