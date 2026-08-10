#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use alloy::primitives::U256;
    use async_lock::RwLock;
    use futures::stream::StreamExt;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        PeerConfig,
        data::ViewNumber,
        event::{Event, EventType},
        light_client::StateKeyPair,
        signature_key::BLSPubKey,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
    };
    use http_client::{Client, Url};
    use test_utils::reserve_tcp_port;
    use tokio::{spawn, time::sleep};
    use tracing_test::traced_test;
    use vbs::version::StaticVersion;

    use crate::{
        events::{self, Error},
        events_source::{EventConsumer, EventsStreamer, StartupInfo},
    };

    // return a empty transaction event
    fn generate_event<Types: NodeType>(view_number: u64) -> Event<Types> {
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
        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let known_nodes_with_stake = vec![];
        let non_staked_node_count = 0;
        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake,
            non_staked_node_count,
        )));

        // Start the web server.
        let router = events::app(axum::Router::new().nest(
            "/hotshot_events",
            events::legacy_events_router::<TestTypes, _, StaticVersion<0, 1>>(
                events_streamer.clone(),
            ),
        ));
        events::serve(&api_url, router);
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
        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let private_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
        let pub_key = BLSPubKey::from_private(&private_key);
        let state_key_pair = StateKeyPair::generate();

        let peer_config = PeerConfig::<TestTypes> {
            stake_table_entry: pub_key.stake_table_entry(U256::from(1)),
            state_ver_key: state_key_pair.ver_key(),
            connect_info: None,
        };

        let known_nodes_with_stake = vec![peer_config];
        let non_staked_node_count = 10;

        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake.clone(),
            non_staked_node_count,
        )));

        // Start the web server.
        let router = events::app(axum::Router::new().nest(
            "/api",
            events::legacy_events_router::<TestTypes, _, StaticVersion<0, 1>>(
                events_streamer.clone(),
            ),
        ));
        events::serve(&api_url, router);

        let client = Client::<Error, StaticVersion<0, 1>>::new(
            format!("http://localhost:{port}/api").parse().unwrap(),
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

        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let api_url = Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        let known_nodes_with_stake = vec![];
        let non_staked_node_count = 0;
        let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
            known_nodes_with_stake,
            non_staked_node_count,
        )));

        // Start the web server.
        let router = events::app(axum::Router::new().nest(
            "/hotshot_events",
            events::events_router::<TestTypes, _, StaticVersion<0, 1>>(events_streamer.clone()),
        ));
        events::serve(&api_url, router);

        // Start Client 1
        let client_1 = Client::<Error, StaticVersion<0, 1>>::new(
            format!("http://localhost:{port}/hotshot_events")
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
            format!("http://localhost:{port}/hotshot_events")
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

        // The server registers a subscriber only when its socket handler starts serving the
        // stream, asynchronously after `subscribe` returns; events broadcast before that are
        // lost. Wait until both subscribers are registered before publishing.
        while events_streamer.read().await.subscriber_count() < 2 {
            sleep(Duration::from_millis(10)).await;
        }

        let total_count = 5;
        // wait for these events to receive on client 1
        let receive_handle_1 = spawn(async move {
            let mut receive_count = 0;
            while let Some(event) = events_1.next().await {
                let event = event.unwrap();
                tracing::info!("Received event in Client 1: {event:?}");

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

                tracing::info!("Received event in Client 2: {event:?}");
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
                tracing::info!("Event sent: {tx_event:?}");
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
