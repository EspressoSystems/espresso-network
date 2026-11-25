//! The store service: a [`Store`] served over gRPC, which the storage instances run over their own
//! [`LocalStore`](crate::LocalStore) for the API instances' [`RemoteStore`]s.
//!
//! [`RemoteStore`]: crate::RemoteStore

use std::{pin::Pin, sync::Arc, time::Duration};

use chain_store::{Item, Lookup};
use espresso_types::v0_3::{EventKey, StakeTableEvent};
use futures::{Stream, StreamExt};
use hotshot_types::data::EpochNumber;
use membership::EpochMembership;
use tonic::{Request, Response, Status};

use crate::{
    PayloadAt, PayloadData, Store,
    wire::{
        MAX_MESSAGE_SIZE, argument, block_parts_from, encode, encode_json, encode_maybe,
        json_argument, part_from,
        proto::{self, store_server::StoreServer},
        record_to, tip_event,
    },
};

/// The gRPC service over `store`.
pub fn server(store: Arc<dyn Store>) -> StoreServer<StoreService> {
    StoreServer::new(StoreService { store })
        .max_decoding_message_size(MAX_MESSAGE_SIZE)
        .max_encoding_message_size(MAX_MESSAGE_SIZE)
}

pub struct StoreService {
    store: Arc<dyn Store>,
}

type TipStream = Pin<Box<dyn Stream<Item = Result<proto::TipEvent, Status>> + Send>>;

type Answer<T> = Result<Response<T>, Status>;

fn done() -> Answer<proto::Done> {
    Ok(Response::new(proto::Done {}))
}

#[tonic::async_trait]
impl proto::store_server::Store for StoreService {
    type FollowTipStream = TipStream;

    async fn newest(&self, _: Request<proto::Empty>) -> Answer<proto::MaybeHeight> {
        let height = self.store.newest().await.map_err(internal)?;
        Ok(Response::new(proto::MaybeHeight { height }))
    }

    async fn floor(&self, _: Request<proto::Empty>) -> Answer<proto::Height> {
        let height = self.store.floor().await.map_err(internal)?;
        Ok(Response::new(proto::Height { height }))
    }

    async fn follow_tip(
        &self,
        request: Request<proto::FollowTipRequest>,
    ) -> Answer<Self::FollowTipStream> {
        let updates = self
            .store
            .tip_updates(request.into_inner().from)
            .await
            .map_err(internal)?;
        let events = updates.map(|update| update.map(tip_event).map_err(internal));
        Ok(Response::new(Box::pin(events)))
    }

    async fn get_record(
        &self,
        request: Request<proto::RecordRequest>,
    ) -> Answer<proto::MaybeRecord> {
        let r = request.into_inner();
        let record = self
            .store
            .record(r.height, r.payload)
            .await
            .map_err(internal)?;
        Ok(Response::new(record_to(record)))
    }

    async fn get_item(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let item = self
            .store
            .item(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(item)?))
    }

    async fn get_cert2(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let cert2 = self
            .store
            .cert2(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(cert2)?))
    }

    async fn get_state_cert(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let cert = self
            .store
            .state_cert(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(cert)?))
    }

    async fn locate(&self, request: Request<proto::LocateRequest>) -> Answer<proto::MaybeHeight> {
        use proto::locate_request::Commitment;
        let lookup = match request.into_inner().commitment {
            Some(Commitment::Leaf(c)) => Lookup::Leaf(parse(&c)?),
            Some(Commitment::Block(c)) => Lookup::Block(parse(&c)?),
            Some(Commitment::Payload(c)) => Lookup::Payload(parse(&c)?),
            None => return Err(Status::invalid_argument("a lookup without a commitment")),
        };
        let height = self.store.locate(&lookup).await.map_err(internal)?;
        Ok(Response::new(proto::MaybeHeight { height }))
    }

    async fn get_payload(
        &self,
        request: Request<proto::PayloadRequest>,
    ) -> Answer<proto::PayloadResponse> {
        use proto::{payload_request::At, payload_response::Data};
        let r = request.into_inner();
        let at = match r.at {
            Some(At::Height(height)) => PayloadAt::Height(height),
            Some(At::Commitment(commitment)) => PayloadAt::Commitment(commitment),
            None => {
                return Err(Status::invalid_argument(
                    "a payload request without a payload",
                ));
            },
        };
        let range = r.range.map(|range| range.start..range.end);
        let data = self.store.payload(&at, range).await.map_err(internal)?;
        Ok(Response::new(proto::PayloadResponse {
            data: data.map(|data| match data {
                PayloadData::Inline(bytes) => Data::Inline(bytes),
                PayloadData::Url(url) => Data::Url(url),
            }),
        }))
    }

    async fn payload_status(
        &self,
        request: Request<proto::CommitmentRequest>,
    ) -> Answer<proto::PayloadStatusResponse> {
        let status = self
            .store
            .payload_status(&request.into_inner().commitment)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::PayloadStatusResponse {
            has: status.has,
            held: status.held,
        }))
    }

    async fn transaction_position(
        &self,
        request: Request<proto::TransactionPositionRequest>,
    ) -> Answer<proto::MaybePosition> {
        let position = self
            .store
            .transaction_position(&request.into_inner().hash)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::MaybePosition {
            height: position.map(|(height, _)| height),
            position: position.map_or(0, |(_, position)| position),
        }))
    }

    async fn totals(
        &self,
        request: Request<proto::TotalsRequest>,
    ) -> Answer<proto::TotalsResponse> {
        let r = request.into_inner();
        let totals = self
            .store
            .totals(r.from..=r.to, r.namespace)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::TotalsResponse {
            transactions: totals.transactions,
            bytes: totals.bytes,
        }))
    }

    async fn missing_items(
        &self,
        request: Request<proto::MissingRequest>,
    ) -> Answer<proto::RangesResponse> {
        let r = request.into_inner();
        let ranges = self
            .store
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

    async fn newest_gap(&self, _: Request<proto::Empty>) -> Answer<proto::MaybeRange> {
        let gap = self.store.newest_gap().await.map_err(internal)?;
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
    ) -> Answer<proto::HeightsResponse> {
        let r = request.into_inner();
        let heights = self
            .store
            .missing_parts(part_from(r.part)?, r.from, r.limit as usize)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::HeightsResponse { heights }))
    }

    async fn put_block(&self, request: Request<proto::PutBlockRequest>) -> Answer<proto::Done> {
        let r = request.into_inner();
        let item: Item = argument(&r.item, "an item")?;
        let parts = block_parts_from(r.parts)?;
        self.store
            .put_block(&item, &parts)
            .await
            .map_err(internal)?;
        done()
    }

    async fn put_parts(&self, request: Request<proto::PutPartsRequest>) -> Answer<proto::Done> {
        let r = request.into_inner();
        let parts = block_parts_from(r.parts)?;
        self.store
            .put_parts(r.height, &parts)
            .await
            .map_err(internal)?;
        done()
    }

    async fn lease(&self, request: Request<proto::LeaseRequest>) -> Answer<proto::LeaseResponse> {
        let r = request.into_inner();
        let ttl = Duration::from_millis(r.ttl_ms);
        let held = self
            .store
            .lease(&r.name, &r.holder, ttl)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::LeaseResponse { held }))
    }

    async fn get_state(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let state = self
            .store
            .state(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(state)?))
    }

    async fn get_states(&self, _: Request<proto::Empty>) -> Answer<proto::Encoded> {
        let states = self.store.states().await.map_err(internal)?;
        Ok(Response::new(proto::Encoded {
            data: encode(&states)?,
        }))
    }

    async fn get_rewards(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let rewards = self
            .store
            .rewards(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(rewards)?))
    }

    async fn get_sent_state(
        &self,
        request: Request<proto::HeightRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let state = self
            .store
            .sent_state(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(state)?))
    }

    async fn put_state(
        &self,
        request: Request<proto::PutStateRequest>,
    ) -> Answer<proto::PutStateResponse> {
        let r = request.into_inner();
        let frontier = argument(&r.frontier, "a frontier")?;
        let chain_config = argument(&r.chain_config, "a chain config")?;
        let rewards = r
            .rewards
            .map(|rewards| argument(&rewards, "reward balances"))
            .transpose()?;
        let stored = self
            .store
            .put_state(r.height, frontier, chain_config, rewards)
            .await
            .map_err(internal)?;
        Ok(Response::new(proto::PutStateResponse { stored }))
    }

    async fn get_chain_config(
        &self,
        request: Request<proto::CommitmentRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let config = self
            .store
            .chain_config(&request.into_inner().commitment)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(config)?))
    }

    async fn put_chain_config(&self, request: Request<proto::Encoded>) -> Answer<proto::Done> {
        let config = argument(&request.into_inner().data, "a chain config")?;
        self.store
            .put_chain_config(&config)
            .await
            .map_err(internal)?;
        done()
    }

    async fn get_fee_accounts(&self, _: Request<proto::Empty>) -> Answer<proto::MaybeEncoded> {
        let accounts = self.store.fee_accounts().await.map_err(internal)?;
        Ok(Response::new(encode_maybe(accounts)?))
    }

    async fn put_fee_accounts(&self, request: Request<proto::Encoded>) -> Answer<proto::Done> {
        let accounts: Vec<_> = argument(&request.into_inner().data, "fee accounts")?;
        self.store
            .put_fee_accounts(&accounts)
            .await
            .map_err(internal)?;
        done()
    }

    async fn get_stake_table(
        &self,
        request: Request<proto::EpochRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let epoch = EpochNumber::new(request.into_inner().epoch);
        let table = self.store.stake_table(epoch).await.map_err(internal)?;
        Ok(Response::new(encode_maybe(table)?))
    }

    async fn get_service_records(
        &self,
        request: Request<proto::MaybeEpochRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let epoch = request.into_inner().epoch.map(EpochNumber::new);
        let records = self.store.service_records(epoch).await.map_err(internal)?;
        Ok(Response::new(encode_maybe(records)?))
    }

    async fn get_epoch(&self, request: Request<proto::EpochRequest>) -> Answer<proto::Encoded> {
        let epoch = EpochNumber::new(request.into_inner().epoch);
        let membership = self.store.epoch(epoch).await.map_err(internal)?;
        Ok(Response::new(proto::Encoded {
            data: encode_json(&membership)?,
        }))
    }

    async fn get_epochs(&self, request: Request<proto::EpochsRequest>) -> Answer<proto::Encoded> {
        let limit = usize::try_from(request.into_inner().limit).unwrap_or(usize::MAX);
        let epochs = self.store.epochs(limit).await.map_err(internal)?;
        Ok(Response::new(proto::Encoded {
            data: encode_json(&epochs)?,
        }))
    }

    async fn get_latest_stakes(
        &self,
        request: Request<proto::EpochsRequest>,
    ) -> Answer<proto::Encoded> {
        let limit = usize::try_from(request.into_inner().limit).unwrap_or(usize::MAX);
        let stakes = self.store.latest_stakes(limit).await.map_err(internal)?;
        Ok(Response::new(proto::Encoded {
            data: encode_json(&stakes)?,
        }))
    }

    async fn put_epoch(&self, request: Request<proto::PutEpochRequest>) -> Answer<proto::Done> {
        let r = request.into_inner();
        let update: EpochMembership = json_argument(&r.update, "an epoch's membership")?;
        self.store
            .put_epoch(EpochNumber::new(r.epoch), update)
            .await
            .map_err(internal)?;
        done()
    }

    async fn get_events(&self, request: Request<proto::EventsRequest>) -> Answer<proto::Encoded> {
        let r = request.into_inner();
        let events = self.store.events(r.from, r.to).await.map_err(internal)?;
        Ok(Response::new(proto::Encoded {
            data: encode_json(&events)?,
        }))
    }

    async fn put_events(&self, request: Request<proto::PutEventsRequest>) -> Answer<proto::Done> {
        let r = request.into_inner();
        let events: Vec<(EventKey, StakeTableEvent)> =
            json_argument(&r.events, "stake-table events")?;
        self.store
            .put_events(r.l1_finalized, events)
            .await
            .map_err(internal)?;
        done()
    }

    async fn clear_stake_tables(&self, _: Request<proto::Empty>) -> Answer<proto::Done> {
        self.store.clear_stake_tables().await.map_err(internal)?;
        done()
    }

    async fn get_drb_input(
        &self,
        request: Request<proto::EpochRequest>,
    ) -> Answer<proto::MaybeEncoded> {
        let input = self
            .store
            .drb_input(request.into_inner().epoch)
            .await
            .map_err(internal)?;
        Ok(Response::new(encode_maybe(input)?))
    }

    async fn put_drb_input(&self, request: Request<proto::Encoded>) -> Answer<proto::Done> {
        let input = argument(&request.into_inner().data, "DRB progress")?;
        self.store.put_drb_input(input).await.map_err(internal)?;
        done()
    }
}

fn internal(err: anyhow::Error) -> Status {
    Status::internal(format!("{err:#}"))
}

fn parse<T: std::str::FromStr>(commitment: &str) -> Result<T, Status> {
    commitment
        .parse()
        .map_err(|_| Status::invalid_argument(format!("not a commitment: {commitment}")))
}
