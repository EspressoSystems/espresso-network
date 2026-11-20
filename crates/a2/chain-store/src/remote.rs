//! A [`Backend`] that is a client of the store service.

use std::{ops::Range, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::{sync::watch, task::JoinSet, time::sleep};
use tonic::transport::{Channel, Endpoint};

use crate::{
    AppWrite, Backend, Column, KeyBounds, Part, Row, TableName,
    wire::{
        MAX_MESSAGE_SIZE, app_write_to, parts_from, parts_to,
        proto::{self, store_client::StoreClient},
    },
};

/// How long the client waits before following the store's changes again after the stream failed.
const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(1);

/// How often the client pings the store service, so that a connection that died without closing
/// fails the requests and the change stream on it instead of leaving them waiting.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);

/// How long the client waits for the answer to a ping before it drops the connection.
const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct RemoteBackend {
    client: StoreClient<Channel>,
    changes: watch::Sender<u64>,

    /// The task that follows the store's changes, which ends with the backend.
    _follower: JoinSet<()>,
}

impl RemoteBackend {
    /// A client of the store service at `url`, which follows the store's changes from now on.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let channel = Endpoint::from_shared(url.to_string())?
            .connect_timeout(CONNECT_TIMEOUT)
            .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
            .keep_alive_timeout(KEEP_ALIVE_TIMEOUT)
            .keep_alive_while_idle(true)
            .connect()
            .await?;
        let client = StoreClient::new(channel)
            .max_decoding_message_size(MAX_MESSAGE_SIZE)
            .max_encoding_message_size(MAX_MESSAGE_SIZE);
        let changes = watch::channel(0).0;
        let mut follower = JoinSet::new();
        follower.spawn(follow(client.clone(), changes.clone()));
        Ok(Self {
            client,
            changes,
            _follower: follower,
        })
    }

    fn client(&self) -> StoreClient<Channel> {
        self.client.clone()
    }

    /// The store announces the write too, later; readers here need not wait for it.
    fn wrote(&self) {
        self.changes.send_modify(|n| *n += 1);
    }
}

/// Changes `changes` whenever the store reports a write. A stream that ends or fails may have
/// missed writes, so it is followed by a change too.
async fn follow(mut client: StoreClient<Channel>, changes: watch::Sender<u64>) {
    loop {
        match client.changes(proto::Empty {}).await {
            Ok(response) => {
                let mut stream = response.into_inner();
                while let Ok(Some(_)) = stream.message().await {
                    changes.send_modify(|n| *n += 1);
                }
                tracing::warn!("the store's changes ended; following them again");
            },
            Err(status) => tracing::warn!(%status, "cannot follow the store's changes"),
        }
        changes.send_modify(|n| *n += 1);
        sleep(RESUBSCRIBE_DELAY).await;
    }
}

fn status(status: tonic::Status) -> anyhow::Error {
    anyhow::anyhow!("store service: {}", status.message())
}

#[async_trait]
impl Backend for RemoteBackend {
    async fn put(&self, row: Row, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
        self.client()
            .put(proto::PutRequest {
                height: row.height,
                leaf: row.leaf,
                block: row.block,
                payload: row.payload,
                item: row.item.to_vec(),
                parts: parts_to(parts),
            })
            .await
            .map_err(status)?;
        self.wrote();
        Ok(())
    }

    async fn put_parts(&self, height: u64, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
        self.client()
            .put_parts(proto::PutPartsRequest {
                height,
                parts: parts_to(parts),
            })
            .await
            .map_err(status)?;
        self.wrote();
        Ok(())
    }

    async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        let response = self
            .client()
            .get_item(proto::HeightRequest { height })
            .await
            .map_err(status)?;
        Ok(response.into_inner().data.map(Bytes::from))
    }

    async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>> {
        let response = self
            .client()
            .get_part(proto::PartRequest {
                height,
                part: part.id().into(),
            })
            .await
            .map_err(status)?;
        Ok(response.into_inner().data.map(Bytes::from))
    }

    async fn record(&self, height: u64) -> anyhow::Result<Option<(Bytes, Vec<(Part, Bytes)>)>> {
        let response = self
            .client()
            .get_record(proto::HeightRequest { height })
            .await
            .map_err(status)?
            .into_inner();
        let Some(item) = response.item else {
            return Ok(None);
        };
        Ok(Some((
            item.into(),
            parts_from(response.parts).map_err(status)?,
        )))
    }

    async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>> {
        let column = match column {
            Column::Leaf => proto::Column::Leaf,
            Column::Block => proto::Column::Block,
            Column::Payload => proto::Column::Payload,
        };
        let response = self
            .client()
            .height_of(proto::HeightOfRequest {
                column: column.into(),
                value: value.to_string(),
            })
            .await
            .map_err(status)?;
        Ok(response.into_inner().height)
    }

    async fn newest(&self) -> anyhow::Result<Option<u64>> {
        let response = self
            .client()
            .newest(proto::Empty {})
            .await
            .map_err(status)?;
        Ok(response.into_inner().height)
    }

    async fn floor(&self) -> anyhow::Result<u64> {
        let response = self.client().floor(proto::Empty {}).await.map_err(status)?;
        Ok(response.into_inner().height)
    }

    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        let response = self
            .client()
            .missing_items(proto::MissingRequest {
                from,
                limit: limit as u64,
            })
            .await
            .map_err(status)?;
        Ok(response
            .into_inner()
            .ranges
            .into_iter()
            .map(|range| range.start..range.end)
            .collect())
    }

    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        let response = self
            .client()
            .newest_gap(proto::Empty {})
            .await
            .map_err(status)?;
        Ok(response.into_inner().range.map(|gap| gap.start..gap.end))
    }

    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        let response = self
            .client()
            .missing_parts(proto::MissingPartsRequest {
                part: part.id().into(),
                from,
                limit: limit as u64,
            })
            .await
            .map_err(status)?;
        Ok(response.into_inner().heights)
    }

    async fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        let response = self
            .client()
            .app_get(proto::AppGetRequest {
                table: table.to_string(),
                key: key.to_vec(),
            })
            .await
            .map_err(status)?;
        Ok(response.into_inner().data.map(Bytes::from))
    }

    async fn app_entries(
        &self,
        table: &TableName,
        keys: KeyBounds<'_>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        let response = self
            .client()
            .app_entries(proto::AppEntriesRequest {
                table: table.to_string(),
                from: keys.from.to_vec(),
                until: keys.until.map(<[u8]>::to_vec),
                limit: u32::try_from(keys.limit).unwrap_or(u32::MAX),
            })
            .await
            .map_err(status)?;
        Ok(response
            .into_inner()
            .entries
            .into_iter()
            .map(|entry| (entry.key.into(), entry.value.into()))
            .collect())
    }

    async fn app_write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.client()
            .app_write(proto::AppWriteRequest {
                writes: writes.into_iter().map(app_write_to).collect(),
            })
            .await
            .map_err(status)?;
        self.wrote();
        Ok(())
    }

    async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
        self.client()
            .prune_below(proto::HeightRequest { height })
            .await
            .map_err(status)?;
        self.wrote();
        Ok(())
    }

    fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::Arc};

    use tokio::{net::TcpListener, time::timeout};
    use tonic::transport::{Server, server::TcpIncoming};

    use super::*;
    use crate::{mem::MemBackend, service::server, testing::row};

    /// Serves a store service over an empty store in memory, returning its URL.
    async fn serve() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let backend: Arc<dyn Backend> = Arc::new(MemBackend::default());
        tokio::spawn(
            Server::builder()
                .add_service(server(backend))
                .serve_with_incoming(TcpIncoming::from(listener)),
        );
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn conforms() {
        let url = serve().await;
        let backend = RemoteBackend::connect(&url).await.unwrap();
        crate::testing::conformance(&backend).await;
    }

    /// A write through one client reaches the changes of another.
    #[tokio::test]
    async fn announces_writes_to_other_clients() {
        let url = serve().await;
        let writer = RemoteBackend::connect(&url).await.unwrap();
        let reader = RemoteBackend::connect(&url).await.unwrap();
        let mut changes = reader.changes();
        // The first change is the stream's initial value; it says nothing about writes.
        timeout(Duration::from_secs(10), changes.changed())
            .await
            .unwrap()
            .unwrap();
        changes.borrow_and_update();

        writer.put(row(0, "x"), Vec::new()).await.unwrap();
        timeout(Duration::from_secs(10), changes.changed())
            .await
            .expect("the reader hears of the write")
            .unwrap();
        assert!(reader.item(0).await.unwrap().is_some());
    }
}
