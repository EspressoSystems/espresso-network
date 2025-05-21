#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use alloy::primitives::U256;
    use async_lock::RwLock;
    use futures::stream::StreamExt;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::ViewNumber,
        event::{Event, EventType},
        light_client::StateKeyPair,
        signature_key::BLSPubKey,
        traits::{
            node_implementation::{ConsensusTime, NodeType},
            signature_key::SignatureKey,
        },
        PeerConfig,
    };
    use surf_disco::Client;
    use tide_disco::{App, Url};
    use tokio::spawn;
    use tracing_test::traced_test;
    use vbs::version::{StaticVersion, StaticVersionType};

    //use crate::fetch::Fetch;
    use crate::events::{define_api, Error, Options};
    use crate::events_source::{EventConsumer, EventsStreamer, StartupInfo}; // EventsUpdater};

    // return a empty transaction event
    fn generate_event<Types: NodeType<View = ViewNumber>>(view_number: u64) -> Event<Types> {
        Event {
            view_number: ViewNumber::new(view_number),
            event: EventType::Transactions {
                transactions: vec![],
            },
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_no_active_receiver() {
        tracing::info!("Starting test_no_active_receiver");
        let port = portpicker::pick_unused_port().expect("Could not find an open port");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let known_nodes_with_stake = vec![];
        let non_staked_node_count = 0;
        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake,
            non_staked_node_count,
        )));

        // Start the web server.
        let mut app = App::<_, Error>::with_state(events_streamer.clone());

        let hotshot_events_api = define_api::<
            Arc<RwLock<EventsStreamer<TestTypes>>>,
            TestTypes,
            StaticVersion<0, 1>,
        >(&Options::default(), "0.0.1".parse().unwrap())
        .expect("Failed to define hotshot eventsAPI");

        app.register_module("hotshot_events", hotshot_events_api)
            .expect("Failed to register hotshot events API");

        spawn(app.serve(api_url, StaticVersion::<0, 1>::instance()));
        let total_count = 5;
        let send_handle = spawn(async move {
            let mut send_count = 0;
            loop {
                let tx_event = generate_event(send_count);
                tracing::debug!("Before writing to events_source");
                events_streamer
                    .write()
                    .await
                    .handle_event(tx_event.clone())
                    .await;
                send_count += 1;
                tracing::debug!("After writing to events_source");
                if send_count >= total_count {
                    break;
                }
            }
        });

        send_handle.await.unwrap();
    }

    #[tokio::test]
    #[traced_test]
    async fn test_startup_info_endpoint() {
        let port = portpicker::pick_unused_port().expect("Could not find an open port");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let private_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
        let pub_key = BLSPubKey::from_private(&private_key);
        let state_key_pair = StateKeyPair::generate();

        let peer_config = PeerConfig::<TestTypes> {
            stake_table_entry: pub_key.stake_table_entry(U256::from(1)),
            state_ver_key: state_key_pair.ver_key(),
        };

        let known_nodes_with_stake = vec![peer_config];
        let non_staked_node_count = 10;

        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake.clone(),
            non_staked_node_count,
        )));

        // Start the web server.
        let mut app = App::<_, Error>::with_state(events_streamer.clone());

        let hotshot_events_api = define_api::<
            Arc<RwLock<EventsStreamer<TestTypes>>>,
            TestTypes,
            StaticVersion<0, 1>,
        >(&Options::default(), "0.0.1".parse().unwrap())
        .expect("Failed to define hotshot eventsAPI");

        app.register_module("api", hotshot_events_api)
            .expect("Failed to register hotshot events API");

        spawn(app.serve(api_url.clone(), StaticVersion::<0, 1>::instance()));

        let client = Client::<Error, StaticVersion<0, 1>>::new(
            format!("http://localhost:{}/api", port).parse().unwrap(),
        );
        client.connect(None).await;

        let startup_info: StartupInfo<TestTypes> = client
            .get("startup_info")
            .send()
            .await
            .expect("failed to get startup_info");

        assert_eq!(startup_info.known_node_with_stake, known_nodes_with_stake);
        assert_eq!(startup_info.non_staked_node_count, non_staked_node_count);
    }

    #[tokio::test]
    #[traced_test]
    async fn test_event_stream() {
        tracing::info!("Starting test_event_stream");

        let port = portpicker::pick_unused_port().expect("Could not find an open port");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let known_nodes_with_stake = vec![];
        let non_staked_node_count = 0;
        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake,
            non_staked_node_count,
        )));

        // Start the web server.
        let mut app = App::<_, Error>::with_state(events_streamer.clone());

        let hotshot_events_api = define_api::<
            Arc<RwLock<EventsStreamer<TestTypes>>>,
            TestTypes,
            StaticVersion<0, 1>,
        >(&Options::default(), "1.0.0".parse().unwrap())
        .expect("Failed to define hotshot eventsAPI");

        app.register_module("hotshot_events", hotshot_events_api)
            .expect("Failed to register hotshot events API");

        spawn(app.serve(api_url, StaticVersion::<0, 1>::instance()));

        // Start Client 1
        let client_1 = Client::<Error, StaticVersion<0, 1>>::new(
            format!("http://localhost:{}/hotshot_events", port)
                .parse()
                .unwrap(),
        );
        client_1.connect(None).await;

        tracing::info!("Client 1 Connected to server");

        // client 1 subscribe to hotshot events
        let mut events_1 = client_1
            .socket("events")
            .subscribe::<Event<TestTypes>>()
            .await
            .unwrap();

        tracing::info!("Client 1 Subscribed to events");

        // Start Client 2
        let client_2 = Client::<Error, StaticVersion<0, 1>>::new(
            format!("http://localhost:{}/hotshot_events", port)
                .parse()
                .unwrap(),
        );
        client_2.connect(None).await;

        tracing::info!("Client 2 Connected to server");

        // client 2 subscrive to hotshot events
        let mut events_2 = client_2
            .socket("events")
            .subscribe::<Event<TestTypes>>()
            .await
            .unwrap();

        tracing::info!("Client 2 Subscribed to events");

        let total_count = 5;
        // wait for these events to receive on client 1
        let receive_handle_1 = spawn(async move {
            let mut receive_count = 0;
            while let Some(event) = events_1.next().await {
                let event = event.unwrap();
                tracing::info!("Received event in Client 1: {:?}", event);

                receive_count += 1;

                if receive_count == total_count {
                    tracing::info!("Client1 Received all sent events, exiting loop");
                    break;
                }
            }

            assert_eq!(receive_count, total_count);

            tracing::info!("stream ended");
        });

        // wait for these events to receive on client 2
        let receive_handle_2 = spawn(async move {
            let mut receive_count = 0;
            while let Some(event) = events_2.next().await {
                let event = event.unwrap();

                tracing::info!("Received event in Client 2: {:?}", event);
                receive_count += 1;

                if receive_count == total_count {
                    tracing::info!("Client 2 Received all sent events, exiting loop");
                    break;
                }
            }

            assert_eq!(receive_count, total_count);

            tracing::info!("stream ended");
        });

        let send_handle = spawn(async move {
            let mut send_count = 0;
            loop {
                let tx_event = generate_event(send_count);
                tracing::debug!("Before writing to events_source");
                events_streamer
                    .write()
                    .await
                    .handle_event(tx_event.clone())
                    .await;
                send_count += 1;
                tracing::debug!("After writing to events_source");
                tracing::info!("Event sent: {:?}", tx_event);
                if send_count >= total_count {
                    break;
                }
            }
        });

        send_handle.await.unwrap();
        receive_handle_1.await.unwrap();
        receive_handle_2.await.unwrap();
    }
}
