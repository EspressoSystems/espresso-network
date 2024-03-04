#![allow(unused_imports)]
use async_broadcast::broadcast;
use async_compatibility_layer::{
    art::async_spawn,
    channel::{unbounded, UnboundedReceiver},
    logging::{setup_backtrace, setup_logging},
};
use async_lock::RwLock;
use async_trait::async_trait;
use builder::{init_node, BuilderContext};
use clap::Parser;
use commit::{Commitment, CommitmentBoundsArkless};
use futures::{future::FutureExt, stream::StreamExt};
pub use hotshot::traits::election::static_committee::{
    GeneralStaticCommittee, StaticElectionConfig,
};
use hotshot::types::SignatureKey;
use hotshot_example_types::block_types::genesis_vid_commitment;
use hotshot_types::{
    data::{Leaf, ViewNumber},
    signature_key::BLSPubKey,
    traits::{
        election::Membership,
        metrics::NoMetrics,
        node_implementation::{ConsensusTime, NodeType},
    },
};
use hs_builder_api::builder::Options as BuilderApiOptions;
use hs_builder_core::{
    builder_state::{BuilderProgress, BuilderState, MessageType},
    service::{run_standalone_builder_service, GlobalState},
};
use sequencer::{
    api, network,
    options::{Modules, Options as SeqOptions},
    NetworkParams, SeqTypes,
};
use std::sync::{Arc, Mutex};
use tagged_base64::TaggedBase64;
use tide_disco::{app, method::ReadState, App, Url};

// /// Construct a tide disco app that mocks the builder API.
// ///
// /// # Panics
// /// If constructing and launching the builder fails for any reason
// pub fn run_builder_api<Types: NodeType>(url: Url, mut global_state: GlobalState<Types>) {
//     let builder_api = hs_builder_api::builder::define_api::<GlobalState<Types>, SeqTypes>(
//         &BuilderApiOptions::default(),
//     )
//     .expect("Failed to construct the builder API");

//     // let global_state = Arc::new(RwLock::new(GlobalState::<SeqTypes>::new(
//     //     req_sender.clone(),
//     //     res_receiver,
//     // )));

//     // let mut app: App<GlobalState<SeqTypes>, hs_builder_api::builder::Error> = App::with_state(GlobalState::<SeqTypes>::new(
//     //     req_sender.clone(),
//     //     res_receiver,
//     // ));
//     //let mut app: App<GlobalState<SeqTypes>, hs_builder_api::builder::Error> = App::with_state(global_state);
//     let mut app: App<GlobalState<Types>, hs_builder_api::builder::Error> =
//         App::with_state(global_state);

//     app.register_module("/", builder_api)
//         .expect("Failed to register the builder API");

//     async_spawn({ app.serve(url) });
// }

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    // setup logging and backtrace
    setup_logging();
    setup_backtrace();

    tracing::info!("Starting Builder Core from main.rs");
    // get options
    let opt = SeqOptions::parse();
    // call the init_node function and get the builder context

    // first create network params
    let network_params = NetworkParams {
        da_server_url: opt.da_server_url,
        consensus_server_url: opt.consensus_server_url,
        orchestrator_url: opt.orchestrator_url,
        state_relay_server_url: opt.state_relay_server_url,
        webserver_poll_interval: opt.webserver_poll_interval,
        private_staking_key: opt.private_staking_key,
    };

    let seed = [201_u8; 32];

    let (builder_pub_key, builder_private_key) =
        BLSPubKey::generated_from_seed_indexed(seed, 2011_u64);

    let (tx_sender, tx_receiver) = broadcast::<MessageType<SeqTypes>>(usize::MAX);
    let (decide_sender, decide_receiver) = broadcast::<MessageType<SeqTypes>>(usize::MAX);
    let (da_sender, da_receiver) = broadcast::<MessageType<SeqTypes>>(usize::MAX);
    let (qc_sender, qc_receiver) = broadcast::<MessageType<SeqTypes>>(usize::MAX);

    let (req_sender, req_receiver) = broadcast::<MessageType<SeqTypes>>(usize::MAX);

    let (res_sender, res_receiver) = unbounded();

    let global_state = GlobalState::<SeqTypes>::new(req_sender, res_receiver);
    let arc_global_state = Arc::new(RwLock::new(global_state));

    let mut commitee_stake_table_entries = vec![];
    // form the quorum election config, required for the VID computation inside the builder_state
    let quorum_election_config: StaticElectionConfig =
        <<SeqTypes as NodeType>::Membership as Membership<SeqTypes>>::default_election_config(
            8 as u64,
        );

    let quorum_membership: GeneralStaticCommittee<
        SeqTypes,
        hotshot_stake_table::vec_based::config::QCVerKey,
    > = <<SeqTypes as NodeType>::Membership as Membership<SeqTypes>>::create_election(
        commitee_stake_table_entries,
        quorum_election_config,
    );

    let builder_state = BuilderState::<SeqTypes>::new(
        (builder_pub_key, builder_private_key),
        (
            ViewNumber::new(0),
            genesis_vid_commitment(),
            Commitment::<Leaf<SeqTypes>>::default_commitment_no_preimage(),
        ),
        tx_receiver,
        decide_receiver,
        da_receiver,
        qc_receiver,
        req_receiver,
        arc_global_state,
        res_sender,
        Arc::new(quorum_membership),
    );

    let port = portpicker::pick_unused_port().expect("Could not find an open port");
    let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

    // get handle to the hotshot context
    let mut builder_context = init_node(network_params, &NoMetrics).await?;

    // start doing consensus i.e. in this case be passive member of the consensus network
    builder_context.start_consensus().await;

    async_spawn(async move {
        let builder_api = hs_builder_api::builder::define_api::<GlobalState<SeqTypes>, SeqTypes>(
            &BuilderApiOptions::default(),
        )
        .expect("Failed to construct the builder API");

        let mut app: App<GlobalState<SeqTypes>, hs_builder_api::builder::Error> =
            App::with_state(global_state);

        app.register_module("builder", builder_api)
            .expect("Failed to register the builder API");

        async_spawn(async move {
            app.serve(api_url).await;
        });
        async_spawn(async move {
            run_standalone_builder_service(
                builder_context.hotshot_handle,
                tx_sender,
                decide_sender,
                da_sender,
                qc_sender,
            )
            .await;
        });
        builder_state.event_loop();
    })
    .await;

    Ok(())
}
