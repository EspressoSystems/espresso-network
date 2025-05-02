use std::{future::Future, pin::Pin, time::Duration};

use anyhow::Context;
use either::Either::{self, Left, Right};
use futures::{stream::unfold, Stream, StreamExt};
use hotshot::types::Event;
use hotshot_events_service::events::Error as EventStreamError;
use hotshot_types::traits::node_implementation::NodeType;
use surf_disco::{client::HealthStatus, Client};
use tokio::time::{sleep, timeout};
use tracing::{error, warn};
use url::Url;
use vbs::version::StaticVersionType;

type EventServiceConnection<Types, ApiVer> = surf_disco::socket::Connection<
    Event<Types>,
    surf_disco::socket::Unsupported,
    EventStreamError,
    ApiVer,
>;

type EventServiceReconnect<Types, ApiVer> = Pin<
    Box<dyn Future<Output = anyhow::Result<EventServiceConnection<Types, ApiVer>>> + Send + Sync>,
>;

/// A wrapper around event streaming API that provides auto-reconnection capability
pub struct EventServiceStream<Types: NodeType, V: StaticVersionType> {
    api_url: Url,
    connection: Either<EventServiceConnection<Types, V>, EventServiceReconnect<Types, V>>,
}

impl<Types: NodeType, ApiVer: StaticVersionType + 'static> EventServiceStream<Types, ApiVer> {
    /// Maximum period between events, once it elapsed we assume
    /// udnerlying connection silently went down and attempt to reconnect
    const MAX_WAIT_PERIOD: Duration = Duration::from_secs(10);
    const RETRY_PERIOD: Duration = Duration::from_secs(1);
    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

    async fn connect_inner(
        url: Url,
    ) -> anyhow::Result<
        surf_disco::socket::Connection<
            Event<Types>,
            surf_disco::socket::Unsupported,
            EventStreamError,
            ApiVer,
        >,
    > {
        let client = Client::<hotshot_events_service::events::Error, ApiVer>::new(url.clone());

        timeout(Self::CONNECTION_TIMEOUT, async {
            loop {
                match client.healthcheck::<HealthStatus>().await {
                    Ok(_) => break,
                    Err(err) => {
                        tracing::debug!(?err, "Healthcheck failed, retrying");
                    },
                }
                sleep(Self::RETRY_PERIOD).await;
            }
        })
        .await
        .context("Couldn't connect to hotshot events API")?;

        tracing::info!("Builder client connected to the hotshot events API");

        Ok(client
            .socket("hotshot-events/events")
            .subscribe::<Event<Types>>()
            .await?)
    }

    /// Establish initial connection to the events service at `api_url`
    pub async fn connect(api_url: Url) -> anyhow::Result<impl Stream<Item = Event<Types>> + Unpin> {
        let connection = Self::connect_inner(api_url.clone()).await?;

        let this = Self {
            api_url,
            connection: Left(connection),
        };

        let stream = unfold(this, |mut this| async move {
            loop {
                match &mut this.connection {
                    Left(connection) => {
                        match tokio::time::timeout(Self::MAX_WAIT_PERIOD, connection.next()).await {
                            Ok(Some(Ok(event))) => {
                                return Some((event, this));
                            },
                            Ok(Some(Err(err))) => {
                                warn!(?err, "Error in event stream");
                                continue;
                            },
                            Ok(None) => {
                                warn!("Event stream ended, attempting reconnection");
                                let fut = Self::connect_inner(this.api_url.clone());
                                let _ =
                                    std::mem::replace(&mut this.connection, Right(Box::pin(fut)));
                                continue;
                            },
                            Err(_) => {
                                // Timeout occurred, reconnect
                                warn!("Timeout waiting for next event; reconnecting");
                                let fut = Self::connect_inner(this.api_url.clone());
                                let _ =
                                    std::mem::replace(&mut this.connection, Right(Box::pin(fut)));
                                continue;
                            },
                        }
                    },
                    Right(reconnection) => match reconnection.await {
                        Ok(connection) => {
                            let _ = std::mem::replace(&mut this.connection, Left(connection));
                            continue;
                        },
                        Err(err) => {
                            error!(?err, "Error while reconnecting, will retry in a while");
                            sleep(Self::RETRY_PERIOD).await;
                            let fut = Self::connect_inner(this.api_url.clone());
                            let _ = std::mem::replace(&mut this.connection, Right(Box::pin(fut)));
                            continue;
                        },
                    },
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use futures::{future::BoxFuture, stream, StreamExt};
    use hotshot::types::{Event, EventType};
    use hotshot_events_service::{
        events::define_api,
        events_source::{EventFilterSet, EventsSource, StartupInfo},
    };
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{data::ViewNumber, traits::node_implementation::ConsensusTime};
    use tide_disco::{method::ReadState, App};
    use tokio::{spawn, task::JoinHandle, time::timeout};
    use tracing::debug;
    use url::Url;
    use vbs::version::StaticVersion;

    use crate::utils::EventServiceStream;

    type MockVersion = StaticVersion<0, 1>;

    struct MockEventsSource {
        counter: AtomicU64,
    }

    #[async_trait]
    impl EventsSource<TestTypes> for MockEventsSource {
        type EventStream = futures::stream::Iter<std::vec::IntoIter<Arc<Event<TestTypes>>>>;

        async fn get_event_stream(
            &self,
            _filter: Option<EventFilterSet<TestTypes>>,
        ) -> Self::EventStream {
            let view = ViewNumber::new(self.counter.load(Ordering::SeqCst));
            let test_event = Arc::new(Event {
                view_number: view,
                event: EventType::ViewFinished { view_number: view },
            });
            self.counter.fetch_add(1, Ordering::SeqCst);
            stream::iter(vec![test_event])
        }

        async fn get_startup_info(&self) -> StartupInfo<TestTypes> {
            StartupInfo {
                known_node_with_stake: Vec::new(),
                non_staked_node_count: 0,
            }
        }
    }

    #[async_trait]
    impl ReadState for MockEventsSource {
        type State = Self;

        async fn read<T>(
            &self,
            op: impl Send + for<'a> FnOnce(&'a Self::State) -> BoxFuture<'a, T> + 'async_trait,
        ) -> T {
            op(self).await
        }
    }

    fn run_app(path: &'static str, bind_url: Url) -> JoinHandle<()> {
        let source = MockEventsSource {
            counter: AtomicU64::new(0),
        };
        let api = define_api::<MockEventsSource, _, MockVersion>(
            &Default::default(),
            "0.0.1".parse().unwrap(),
        )
        .unwrap();

        let mut app: App<MockEventsSource, hotshot_events_service::events::Error> =
            App::with_state(source);

        app.register_module(path, api).unwrap();

        spawn(async move { app.serve(bind_url, MockVersion {}).await.unwrap() })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_event_stream_wrapper() {
        const TIMEOUT: Duration = Duration::from_secs(3);

        let url: Url = format!(
            "http://localhost:{}",
            portpicker::pick_unused_port().unwrap()
        )
        .parse()
        .unwrap();

        let app_handle = run_app("hotshot-events", url.clone());

        let mut stream = EventServiceStream::<TestTypes, MockVersion>::connect(url.clone())
            .await
            .unwrap();

        timeout(TIMEOUT, stream.next())
            .await
            .expect("When mock event server is spawned, stream should work")
            .unwrap();

        app_handle.abort();

        timeout(TIMEOUT, stream.next())
            .await
            .expect_err("When mock event server is killed, stream should be in reconnecting state and never return");

        let app_handle = run_app("hotshot-events", url.clone());

        timeout(TIMEOUT, stream.next())
            .await
            .expect("When mock event server is restarted, stream should work again")
            .unwrap();

        app_handle.abort();

        run_app("wrong-path", url.clone());

        timeout(TIMEOUT, stream.next())
            .await
            .expect_err("API is reachable, but is on wrong path");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_event_stream_wrapper_with_idle_timeout() {
        const TIMEOUT: Duration = Duration::from_secs(3);

        let url: Url = format!(
            "http://localhost:{}",
            portpicker::pick_unused_port().unwrap()
        )
        .parse()
        .unwrap();

        let app_handle = run_app("hotshot-events", url.clone());

        let mut stream = EventServiceStream::<TestTypes, MockVersion>::connect(url.clone())
            .await
            .unwrap();

        // The stream should work when the server is running
        timeout(TIMEOUT, stream.next())
            .await
            .expect("When mock event server is spawned, stream should work")
            .unwrap();

        // Simulate idle timeout by stopping the server and waiting
        app_handle.abort();
        tokio::time::sleep(
            EventServiceStream::<TestTypes, MockVersion>::RETRY_PERIOD + Duration::from_millis(500),
        )
        .await; // Wait longer than idle timeout
                // Check whether stream returns Err(_) after idle timeout
        match timeout(
            EventServiceStream::<TestTypes, MockVersion>::RETRY_PERIOD,
            stream.next(),
        )
        .await
        {
            Ok(Some(_)) => panic!("Expected error after idle timeout but got an event"),
            Ok(None) => panic!("Expected error but got None"),
            Err(err) => debug!("Stream returned an error after idle timeout: {:?}", err),
        }

        // Stream should reconnect after idle timeout
        let new_app_handle = run_app("hotshot-events", url.clone());

        timeout(TIMEOUT, stream.next())
            .await
            .expect("After idle timeout, stream should reconnect when the server restarts")
            .unwrap();

        // Cleanup
        new_app_handle.abort();
    }
}
