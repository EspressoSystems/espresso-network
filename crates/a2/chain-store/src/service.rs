//! The store service: a [`Backend`] served over gRPC, so that one process alone writes to the
//! database and the ingester and the API service reach the store through [`RemoteBackend`].
//!
//! [`RemoteBackend`]: crate::remote::RemoteBackend

use std::{pin::Pin, sync::Arc};

use futures::{Stream, stream};
use tonic::{Request, Response, Status};

use crate::{
    Backend, Column, KeyBounds, Row, TableName,
    wire::{
        MAX_MESSAGE_SIZE, app_write_from, part_from, parts_from, parts_to,
        proto::{self, store_server::StoreServer},
    },
};

/// The most application table entries one answer holds.
const MAX_APP_ENTRIES: usize = 4096;

/// The gRPC service over `backend`.
pub fn server(backend: Arc<dyn Backend>) -> StoreServer<StoreService> {
    StoreServer::new(StoreService { backend })
        .max_decoding_message_size(MAX_MESSAGE_SIZE)
        .max_encoding_message_size(MAX_MESSAGE_SIZE)
}

pub struct StoreService {
    backend: Arc<dyn Backend>,
}

type ChangeStream = Pin<Box<dyn Stream<Item = Result<proto::Change, Status>> + Send>>;

#[tonic::async_trait]
impl proto::store_server::Store for StoreService {
    type ChangesStream = ChangeStream;

    async fn put(
        &self,
        request: Request<proto::PutRequest>,
    ) -> Result<Response<proto::Done>, Status> {
        let r = request.into_inner();
        let row = Row {
            height: r.height,
            leaf: r.leaf,
            block: r.block,
            payload: r.payload,
            item: r.item.into(),
        };
        self.backend
            .put(row, parts_from(r.parts)?)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::Done {}))
    }

    async fn put_parts(
        &self,
        request: Request<proto::PutPartsRequest>,
    ) -> Result<Response<proto::Done>, Status> {
        let r = request.into_inner();
        self.backend
            .put_parts(r.height, parts_from(r.parts)?)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::Done {}))
    }

    async fn get_item(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Result<Response<proto::MaybeBytes>, Status> {
        let data = self
            .backend
            .item(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::MaybeBytes {
            data: data.map(|d| d.to_vec()),
        }))
    }

    async fn get_part(
        &self,
        request: Request<proto::PartRequest>,
    ) -> Result<Response<proto::MaybeBytes>, Status> {
        let r = request.into_inner();
        let data = self
            .backend
            .part(r.height, part_from(r.part)?)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::MaybeBytes {
            data: data.map(|d| d.to_vec()),
        }))
    }

    async fn get_record(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Result<Response<proto::RecordResponse>, Status> {
        let record = self
            .backend
            .record(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(match record {
            Some((item, parts)) => proto::RecordResponse {
                item: Some(item.to_vec()),
                parts: parts_to(parts),
            },
            None => proto::RecordResponse::default(),
        }))
    }

    async fn height_of(
        &self,
        request: Request<proto::HeightOfRequest>,
    ) -> Result<Response<proto::MaybeHeight>, Status> {
        let r = request.into_inner();
        let column = match proto::Column::try_from(r.column) {
            Ok(proto::Column::Leaf) => Column::Leaf,
            Ok(proto::Column::Block) => Column::Block,
            Ok(proto::Column::Payload) => Column::Payload,
            Err(_) => return Err(Status::invalid_argument(format!("no column {}", r.column))),
        };
        let height = self
            .backend
            .height_of(column, &r.value)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::MaybeHeight { height }))
    }

    async fn newest(
        &self,
        _: Request<proto::Empty>,
    ) -> Result<Response<proto::MaybeHeight>, Status> {
        let height = self.backend.newest().await.map_err(internal)?;
        Ok(Response::new(proto::MaybeHeight { height }))
    }

    async fn floor(&self, _: Request<proto::Empty>) -> Result<Response<proto::Height>, Status> {
        let height = self.backend.floor().await.map_err(internal)?;
        Ok(Response::new(proto::Height { height }))
    }

    async fn missing_items(
        &self,
        request: Request<proto::MissingRequest>,
    ) -> Result<Response<proto::RangesResponse>, Status> {
        let r = request.into_inner();
        let ranges = self
            .backend
            .missing_items(r.from, r.limit as usize)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::RangesResponse {
            ranges: ranges
                .into_iter()
                .map(|range| proto::HeightRange {
                    start: range.start,
                    end: range.end,
                })
                .collect(),
        }))
    }

    async fn newest_gap(
        &self,
        _: Request<proto::Empty>,
    ) -> Result<Response<proto::MaybeRange>, Status> {
        let gap = self.backend.newest_gap().await.map_err(internal)?;
        Ok(Response::new(proto::MaybeRange {
            range: gap.map(|gap| proto::HeightRange {
                start: gap.start,
                end: gap.end,
            }),
        }))
    }

    async fn missing_parts(
        &self,
        request: Request<proto::MissingPartsRequest>,
    ) -> Result<Response<proto::HeightsResponse>, Status> {
        let r = request.into_inner();
        let heights = self
            .backend
            .missing_parts(part_from(r.part)?, r.from, r.limit as usize)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::HeightsResponse { heights }))
    }

    async fn app_get(
        &self,
        request: Request<proto::AppGetRequest>,
    ) -> Result<Response<proto::MaybeBytes>, Status> {
        let r = request.into_inner();
        let data = self
            .backend
            .app_get(&table_name(&r.table)?, &r.key)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::MaybeBytes {
            data: data.map(|d| d.to_vec()),
        }))
    }

    async fn app_entries(
        &self,
        request: Request<proto::AppEntriesRequest>,
    ) -> Result<Response<proto::AppEntriesResponse>, Status> {
        let r = request.into_inner();
        let keys = KeyBounds {
            from: &r.from,
            until: r.until.as_deref(),
            limit: (r.limit as usize).min(MAX_APP_ENTRIES),
        };
        let entries = self
            .backend
            .app_entries(&table_name(&r.table)?, keys)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::AppEntriesResponse {
            entries: entries
                .into_iter()
                .map(|(key, value)| proto::AppEntry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                })
                .collect(),
        }))
    }

    async fn app_write(
        &self,
        request: Request<proto::AppWriteRequest>,
    ) -> Result<Response<proto::Done>, Status> {
        let writes = request
            .into_inner()
            .writes
            .into_iter()
            .map(app_write_from)
            .collect::<Result<Vec<_>, _>>()?;
        self.backend.app_write(writes).await.map_err(internal)?;
        Ok(Response::new(proto::Done {}))
    }

    async fn prune_below(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Result<Response<proto::Done>, Status> {
        self.backend
            .prune_below(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::Done {}))
    }

    /// Sends the counter now, then each value it changes to, until the client goes away.
    async fn changes(
        &self,
        _: Request<proto::Empty>,
    ) -> Result<Response<Self::ChangesStream>, Status> {
        let changes = self.backend.changes();
        let stream = stream::unfold((changes, true), |(mut changes, first)| async move {
            if !first && changes.changed().await.is_err() {
                return None;
            }
            let counter = *changes.borrow_and_update();
            Some((Ok(proto::Change { counter }), (changes, false)))
        });
        Ok(Response::new(Box::pin(stream)))
    }
}

fn internal(err: anyhow::Error) -> Status {
    Status::internal(format!("{err:#}"))
}

/// The application table `name` a client asked for.
fn table_name(name: &str) -> Result<TableName, Status> {
    TableName::new(name).map_err(|err| Status::invalid_argument(format!("{err:#}")))
}
