use std::{pin::Pin, sync::Arc};

use anyhow::Context;
use async_lock::RwLock;
use espresso_types::SeqTypes;
use futures::{Stream, StreamExt as _};
use hotshot::types::Event;
use hotshot_events_service::{events, events_source::StartupInfo};
use surf_disco::Client;
use tide_disco::Url;
use vbs::version::StaticVersion;

use crate::state::GlobalState;

// TODO (ab): Change
pub struct EventsServiceClient(Client<events::Error, StaticVersion<0, 1>>);

impl EventsServiceClient {
    pub async fn new(url: Url) -> Self {
        let client = Client::new(url.clone());

        client.connect(None).await;

        Self(client)
    }

    pub async fn get_startup_info(
        &self,
    ) -> Result<StartupInfo<SeqTypes>, hotshot_events_service::events::Error> {
        self.0.get("startup_info").send().await
    }

    pub async fn get_event_stream(
        self,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = Result<Event<SeqTypes>, events::Error>> + Send>>>
    {
        let stream = self
            .0
            .socket("events")
            .subscribe::<Event<SeqTypes>>()
            .await
            .context("failed to get event stream")?;

        Ok(stream.boxed())
    }
}

pub async fn handle_events(
    mut stream: Pin<Box<dyn Stream<Item = Result<Event<SeqTypes>, events::Error>> + Send>>,
    _state: Arc<RwLock<GlobalState>>,
) -> anyhow::Result<()> {
    while let Some(event) = stream.next().await {
        let event = event?;

        tracing::debug!("received event {:?}", event.event);

        // TODO ED: Remove this lint later
        #[allow(clippy::single_match)]
        match event.event {
            hotshot::types::EventType::ViewFinished { view_number } => {
                tracing::debug!("received view finished event {view_number:?}")
            },
            _ => (),
        }
    }

    Ok(())
}

#[cfg(any(test, feature = "testing"))]
pub mod mock {
    use std::{sync::Arc, time::Duration};

    use alloy::primitives::U256;
    use async_lock::RwLock;
    use espresso_types::SeqTypes;
    use hotshot::rand::{self};
    use hotshot_events_service::events_source::{EventConsumer, EventsStreamer};
    use hotshot_types::{
        data::ViewNumber,
        event::{Event, EventType},
        light_client::StateKeyPair,
        signature_key::BLSPubKey,
        traits::{node_implementation::ConsensusTime, signature_key::SignatureKey},
        PeerConfig,
    };
    use portpicker::pick_unused_port;
    use rand::{rngs::OsRng, RngCore};
    use tide_disco::{App, Url};
    use tokio::{spawn, task::JoinHandle, time::sleep};
    use vbs::version::{StaticVersion, StaticVersionType};

    const NON_STAKED_NODE_COUNT: usize = 10;
    const NODE_STAKE: u64 = 1;
    const STAKED_NODES: usize = 10;
    pub type StaticVer01 = StaticVersion<0, 1>;

    pub fn generate_stake_table() -> Vec<PeerConfig<SeqTypes>> {
        (0..STAKED_NODES)
            .map(|_| {
                let private_key =
                    <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
                let pub_key = BLSPubKey::from_private(&private_key);
                let state_key_pair = StateKeyPair::generate();

                PeerConfig::<SeqTypes> {
                    stake_table_entry: pub_key.stake_table_entry(U256::from(NODE_STAKE)),
                    state_ver_key: state_key_pair.ver_key(),
                }
            })
            .collect()
    }

    pub async fn generate_view_finished_events(streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>) {
        sleep(Duration::from_secs(2)).await;

        let mut view_number = ViewNumber::new(1);

        let mut count = 1;
        loop {
            while count % 10 != 0 {
                tracing::info!("generating ViewFinished event");

                streamer
                    .write()
                    .await
                    .handle_event(Event {
                        view_number,
                        event: EventType::ViewFinished {
                            view_number: view_number - 1,
                        },
                    })
                    .await;

                view_number += 1;
                count += 1;
            }
            count = 1;

            let delay = 1000 + (u64::from(OsRng.next_u32()) % 1000);
            tracing::info!("sleeping for {delay:?}ms...");

            sleep(Duration::from_millis(delay)).await
        }
    }

    pub fn run_mock_event_service() -> (Url, JoinHandle<()>, JoinHandle<()>) {
        let port = pick_unused_port().expect("no free port");
        let url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let known_nodes_with_stake = generate_stake_table();

        let events_streamer = Arc::new(RwLock::new(EventsStreamer::<SeqTypes>::new(
            known_nodes_with_stake.clone(),
            NON_STAKED_NODE_COUNT,
        )));

        let mut app =
            App::<_, hotshot_events_service::events::Error>::with_state(events_streamer.clone());

        let hotshot_events_api =
            hotshot_events_service::events::define_api::<_, _, StaticVer01>(&Default::default())
                .expect("Failed to define hotshot eventsAPI");

        app.register_module("events_api", hotshot_events_api)
            .expect("Failed to register hotshot events API");

        // cleanup with a function that takes in a a future
        let events_api_handle = spawn({
            let url = url.clone();
            async move {
                {
                    let _ = app.serve(url, StaticVer01::instance()).await;
                }
            }
        });
        let generate_events_handle = spawn(generate_view_finished_events(events_streamer.clone()));

        let url = url.join("events_api").unwrap();

        (url, events_api_handle, generate_events_handle)
    }
}

#[cfg(test)]
mod test {
    use espresso_types::SeqTypes;
    use futures::StreamExt as _;
    use hotshot::{helpers::initialize_logging, types::Event};
    use hotshot_events_service::events_source::StartupInfo;
    use surf_disco::Client;

    use crate::mock::{run_mock_event_service, StaticVer01};

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mock_events_service() {
        // Initialize logging
        initialize_logging();

        let (url, handle1, handle2) = run_mock_event_service();

        tracing::info!("running event service");

        let client = Client::<hotshot_events_service::events::Error, StaticVer01>::new(url);
        client.connect(None).await;

        let startup_info: StartupInfo<SeqTypes> = client
            .get("startup_info")
            .send()
            .await
            .expect("failed to get startup_info");

        tracing::info!("startup info {startup_info:?}");

        let mut events = client
            .socket("events")
            .subscribe::<Event<SeqTypes>>()
            .await
            .unwrap();

        let mut num_of_events_received = 0;
        while let Some(event) = events.next().await {
            let event = event.unwrap();
            tracing::info!("event {event:?}");

            num_of_events_received += 1;

            if num_of_events_received == 15 {
                break;
            }
        }

        handle1.abort();
        handle2.abort();
    }
}
