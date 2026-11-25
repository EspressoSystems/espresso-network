//! The store over the store service.

use std::{
    ops::{Range, RangeInclusive},
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use chain_api::{BlocksFrontier, RewardSet, SentState, StateRow};
use chain_store::{Item, Lookup, Part, Parts, Record, TipUpdate, Totals};
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    ChainConfig, FeeAccount, FeeAmount, SeqTypes,
    traits::StakeTuple,
    v0_3::{EventKey, StakeTableEvent},
};
use futures::{StreamExt, stream::BoxStream};
use hotshot_types::{
    PeerConfig,
    data::EpochNumber,
    drb::DrbInput,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
};
use membership::{EpochMembership, StakeEvents};
use tonic::transport::{Channel, Endpoint};

use crate::{
    PayloadAt, PayloadData, PayloadStatus, Store,
    wire::{
        MAX_MESSAGE_SIZE, block_parts_to, decode, decode_json, decode_maybe,
        proto::{self, store_client::StoreClient},
        record_from, tip_update,
    },
};

/// How often the client pings the store service, so that a connection that died without closing
/// fails the requests and the tip stream on it instead of leaving them waiting.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);

/// How long the client waits for the answer to a ping before it drops the connection.
const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// A client of the store service.
#[derive(Clone, Debug)]
pub struct RemoteStore {
    client: StoreClient<Channel>,
}

impl RemoteStore {
    /// A client of the store service at `url`.
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
        Ok(Self { client })
    }

    fn client(&self) -> StoreClient<Channel> {
        self.client.clone()
    }
}

#[async_trait]
impl Store for RemoteStore {
    async fn newest(&self) -> anyhow::Result<Option<u64>> {
        let response = self.client().newest(proto::Empty {}).await?;
        Ok(response.into_inner().height)
    }

    async fn floor(&self) -> anyhow::Result<u64> {
        let response = self.client().floor(proto::Empty {}).await?;
        Ok(response.into_inner().height)
    }

    async fn tip_updates(
        &self,
        from: u64,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<TipUpdate>>> {
        let events = self
            .client()
            .follow_tip(proto::FollowTipRequest { from })
            .await?
            .into_inner();
        let updates = events.map(|event| -> anyhow::Result<TipUpdate> { Ok(tip_update(event?)?) });
        Ok(updates.boxed())
    }

    async fn record(&self, height: u64, payload: bool) -> anyhow::Result<Option<Record>> {
        let response = self
            .client()
            .get_record(proto::RecordRequest { height, payload })
            .await?;
        Ok(record_from(height, response.into_inner())?)
    }

    async fn item(&self, height: u64) -> anyhow::Result<Option<Item>> {
        let response = self
            .client()
            .get_item(proto::HeightRequest { height })
            .await?;
        decode_maybe(response.into_inner()).context("decoding an item")
    }

    async fn cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        let response = self
            .client()
            .get_cert2(proto::HeightRequest { height })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a certificate")
    }

    async fn state_cert(
        &self,
        height: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let response = self
            .client()
            .get_state_cert(proto::HeightRequest { height })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a state certificate")
    }

    async fn locate(&self, lookup: &Lookup) -> anyhow::Result<Option<u64>> {
        use proto::locate_request::Commitment;
        let commitment = match lookup {
            Lookup::Leaf(c) => Commitment::Leaf(c.to_string()),
            Lookup::Block(c) => Commitment::Block(c.to_string()),
            Lookup::Payload(c) => Commitment::Payload(c.to_string()),
        };
        let response = self
            .client()
            .locate(proto::LocateRequest {
                commitment: Some(commitment),
            })
            .await?;
        Ok(response.into_inner().height)
    }

    async fn payload(
        &self,
        at: &PayloadAt,
        range: Option<Range<u64>>,
    ) -> anyhow::Result<Option<PayloadData>> {
        use proto::{payload_request::At, payload_response::Data};
        let at = match at {
            PayloadAt::Height(height) => At::Height(*height),
            PayloadAt::Commitment(commitment) => At::Commitment(commitment.clone()),
        };
        let response = self
            .client()
            .get_payload(proto::PayloadRequest {
                at: Some(at),
                range: range.map(|range| proto::ByteRange {
                    start: range.start,
                    end: range.end,
                }),
            })
            .await?;
        Ok(response.into_inner().data.map(|data| match data {
            Data::Inline(bytes) => PayloadData::Inline(bytes),
            Data::Url(url) => PayloadData::Url(url),
        }))
    }

    async fn payload_status(&self, commitment: &str) -> anyhow::Result<PayloadStatus> {
        let response = self
            .client()
            .payload_status(proto::CommitmentRequest {
                commitment: commitment.to_string(),
            })
            .await?
            .into_inner();
        Ok(PayloadStatus {
            has: response.has,
            held: response.held,
        })
    }

    async fn transaction_position(&self, hash: &str) -> anyhow::Result<Option<(u64, u64)>> {
        let response = self
            .client()
            .transaction_position(proto::TransactionPositionRequest {
                hash: hash.to_string(),
            })
            .await?
            .into_inner();
        Ok(response.height.map(|height| (height, response.position)))
    }

    async fn totals(
        &self,
        heights: RangeInclusive<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<Totals> {
        let response = self
            .client()
            .totals(proto::TotalsRequest {
                from: *heights.start(),
                to: *heights.end(),
                namespace,
            })
            .await?
            .into_inner();
        Ok(Totals {
            transactions: response.transactions,
            bytes: response.bytes,
        })
    }

    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        let response = self
            .client()
            .missing_items(proto::MissingRequest {
                from,
                limit: u32::try_from(limit).unwrap_or(u32::MAX),
            })
            .await?;
        Ok(response
            .into_inner()
            .ranges
            .into_iter()
            .map(|range| range.start..range.end)
            .collect())
    }

    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        let response = self.client().newest_gap(proto::Empty {}).await?;
        Ok(response
            .into_inner()
            .range
            .map(|range| range.start..range.end))
    }

    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        let response = self
            .client()
            .missing_parts(proto::MissingPartsRequest {
                part: part.id().into(),
                from,
                limit: u32::try_from(limit).unwrap_or(u32::MAX),
            })
            .await?;
        Ok(response.into_inner().heights)
    }

    async fn put_block(&self, item: &Item, parts: &Parts) -> anyhow::Result<()> {
        self.client()
            .put_block(proto::PutBlockRequest {
                item: bincode::serialize(item)?.into(),
                parts: Some(block_parts_to(parts)?),
            })
            .await?;
        Ok(())
    }

    async fn put_parts(&self, height: u64, parts: &Parts) -> anyhow::Result<()> {
        self.client()
            .put_parts(proto::PutPartsRequest {
                height,
                parts: Some(block_parts_to(parts)?),
            })
            .await?;
        Ok(())
    }

    async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool> {
        let response = self
            .client()
            .lease(proto::LeaseRequest {
                name: name.to_string(),
                holder: holder.to_string(),
                ttl_ms: u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX),
            })
            .await?;
        Ok(response.into_inner().held)
    }

    async fn state(&self, height: u64) -> anyhow::Result<Option<StateRow>> {
        let response = self
            .client()
            .get_state(proto::HeightRequest { height })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a state")
    }

    async fn states(&self) -> anyhow::Result<Vec<(u64, StateRow)>> {
        let response = self.client().get_states(proto::Empty {}).await?;
        decode(&response.into_inner().data).context("decoding states")
    }

    async fn rewards(&self, at: u64) -> anyhow::Result<Option<RewardSet>> {
        let response = self
            .client()
            .get_rewards(proto::HeightRequest { height: at })
            .await?;
        decode_maybe(response.into_inner()).context("decoding reward balances")
    }

    async fn sent_state(&self, height: u64) -> anyhow::Result<Option<SentState>> {
        let response = self
            .client()
            .get_sent_state(proto::HeightRequest { height })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a sent state")
    }

    async fn put_state(
        &self,
        height: u64,
        frontier: Option<BlocksFrontier>,
        chain_config: ChainConfig,
        rewards: Option<RewardSet>,
    ) -> anyhow::Result<bool> {
        let response = self
            .client()
            .put_state(proto::PutStateRequest {
                height,
                frontier: bincode::serialize(&frontier)?.into(),
                chain_config: bincode::serialize(&chain_config)?.into(),
                rewards: rewards
                    .map(|rewards| bincode::serialize(&rewards))
                    .transpose()?
                    .map(Into::into),
            })
            .await?;
        Ok(response.into_inner().stored)
    }

    async fn chain_config(&self, commitment: &str) -> anyhow::Result<Option<ChainConfig>> {
        let response = self
            .client()
            .get_chain_config(proto::CommitmentRequest {
                commitment: commitment.to_string(),
            })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a chain config")
    }

    async fn put_chain_config(&self, config: &ChainConfig) -> anyhow::Result<()> {
        self.client()
            .put_chain_config(proto::Encoded {
                data: bincode::serialize(config)?.into(),
            })
            .await?;
        Ok(())
    }

    async fn fee_accounts(&self) -> anyhow::Result<Option<Vec<(FeeAccount, FeeAmount)>>> {
        let response = self.client().get_fee_accounts(proto::Empty {}).await?;
        decode_maybe(response.into_inner()).context("decoding the fee accounts")
    }

    async fn put_fee_accounts(&self, accounts: &[(FeeAccount, FeeAmount)]) -> anyhow::Result<()> {
        self.client()
            .put_fee_accounts(proto::Encoded {
                data: bincode::serialize(accounts)?.into(),
            })
            .await?;
        Ok(())
    }

    async fn stake_table(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<Vec<PeerConfig<SeqTypes>>>> {
        let response = self
            .client()
            .get_stake_table(proto::EpochRequest { epoch: *epoch })
            .await?;
        decode_maybe(response.into_inner()).context("decoding a stake table")
    }

    async fn service_records(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<(EpochNumber, Vec<SignedServiceRecord<SeqTypes>>)>> {
        let response = self
            .client()
            .get_service_records(proto::MaybeEpochRequest {
                epoch: epoch.map(|epoch| *epoch),
            })
            .await?;
        decode_maybe(response.into_inner()).context("decoding service records")
    }

    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership> {
        let response = self
            .client()
            .get_epoch(proto::EpochRequest { epoch: *epoch })
            .await?;
        decode_json(&response.into_inner().data).context("decoding an epoch's membership")
    }

    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>> {
        let response = self
            .client()
            .get_epochs(proto::EpochsRequest {
                limit: u64::try_from(limit).unwrap_or(u64::MAX),
            })
            .await?;
        decode_json(&response.into_inner().data).context("decoding epochs' membership")
    }

    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>> {
        let response = self
            .client()
            .get_latest_stakes(proto::EpochsRequest {
                limit: u64::try_from(limit).unwrap_or(u64::MAX),
            })
            .await?;
        decode_json(&response.into_inner().data).context("decoding stake tables")
    }

    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()> {
        self.client()
            .put_epoch(proto::PutEpochRequest {
                epoch: *epoch,
                update: serde_json::to_vec(&update)?.into(),
            })
            .await?;
        Ok(())
    }

    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents> {
        let response = self
            .client()
            .get_events(proto::EventsRequest { from, to })
            .await?;
        decode_json(&response.into_inner().data).context("decoding stake-table events")
    }

    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.client()
            .put_events(proto::PutEventsRequest {
                l1_finalized,
                events: serde_json::to_vec(&events)?.into(),
            })
            .await?;
        Ok(())
    }

    async fn clear_stake_tables(&self) -> anyhow::Result<()> {
        self.client().clear_stake_tables(proto::Empty {}).await?;
        Ok(())
    }

    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>> {
        let response = self
            .client()
            .get_drb_input(proto::EpochRequest { epoch })
            .await?;
        decode_maybe(response.into_inner()).context("decoding DRB progress")
    }

    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()> {
        self.client()
            .put_drb_input(proto::Encoded {
                data: bincode::serialize(&input)?.into(),
            })
            .await?;
        Ok(())
    }
}
