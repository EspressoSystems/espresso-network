//! The node service: what the store knows of the chain and of the validators.
//!
//! Everything here is answered from the store, its transaction counts and payload sizes from the
//! index the storage instances keep. What only a consensus node knows, such as its VID shares or
//! how often validators took part, is not served.

use std::ops::Range;

use chain_store::{Part, Totals};
use espresso_api::{
    error::to_status,
    proto::{self, node_service_server::NodeService},
    v1::node::StakeTableWithEpochNumber,
};
use espresso_types::{Header, traits::MembershipPersistence};
use hotshot_query_service_types::node::{
    Limits, ResourceSyncStatus, SyncStatus, SyncStatusQueryData, SyncStatusRange,
    TimeWindowQueryData,
};
use hotshot_types::{
    data::EpochNumber, traits::block_contents::BlockHeader, utils::epoch_from_block_number,
};
use tonic::{Request, Response, Status};

use crate::{
    ApiService,
    reader::Wait,
    request::{self, BlockKey, parse, required},
};

/// The most headers a header window holds, as in main's query service.
const WINDOW_LIMIT: usize = 500;

/// The most validators a page of all registered validators holds, as in main's API.
const VALIDATORS_LIMIT: u64 = 1000;

/// How many gaps, or heights missing a part, one question to the store returns.
const GAP_PAGE: usize = 1000;

#[tonic::async_trait]
impl NodeService for ApiService {
    async fn get_node_block_height(
        &self,
        _: Request<proto::GetNodeBlockHeightRequest>,
    ) -> Result<Response<proto::NodeBlockHeightResponse>, Status> {
        let newest = self.reader.newest().await?;
        Ok(Response::new(proto::NodeBlockHeightResponse {
            height: newest.map_or(0, |h| h + 1),
        }))
    }

    async fn get_transaction_count(
        &self,
        request: Request<proto::GetTransactionCountRequest>,
    ) -> Result<Response<proto::TransactionCountResponse>, Status> {
        let r = request.into_inner();
        let totals = self.totals(r.from, r.to, r.namespace).await?;
        Ok(Response::new(proto::TransactionCountResponse {
            count: totals.transactions,
        }))
    }

    async fn get_payload_size(
        &self,
        request: Request<proto::GetPayloadSizeRequest>,
    ) -> Result<Response<proto::PayloadSizeResponse>, Status> {
        let r = request.into_inner();
        let totals = self.totals(r.from, r.to, r.namespace).await?;
        Ok(Response::new(proto::PayloadSizeResponse {
            size: totals.bytes,
        }))
    }

    /// Which heights the store has blocks, leaves and VID common data for, which it lacks, and
    /// which it pruned.
    async fn get_sync_status(
        &self,
        _: Request<proto::GetSyncStatusRequest>,
    ) -> Result<Response<proto::SyncStatusResponse>, Status> {
        let floor = self.reader.store().floor().await.map_err(to_status)?;
        let height = self.reader.newest().await?.map_or(0, |h| h + 1);
        let gaps = self.item_gaps(floor).await?;
        let without_payload = [gaps.clone(), self.part_gaps(Part::Payload, floor).await?].concat();
        let without_common = [gaps.clone(), self.part_gaps(Part::VidCommon, floor).await?].concat();
        let status = SyncStatusQueryData {
            blocks: sync_status(floor, height, without_payload),
            leaves: sync_status(floor, height, gaps),
            vid_common: sync_status(floor, height, without_common),
            pruned_height: floor.checked_sub(1).map(|h| h as usize),
        };
        Ok(Response::new(status.into()))
    }

    /// The block reward of an epoch the store keeps the validators of. The reward used before
    /// rewards depended on the epoch is not stored, so asking without an epoch has no amount.
    async fn get_block_reward(
        &self,
        request: Request<proto::GetBlockRewardRequest>,
    ) -> Result<Response<proto::BlockRewardResponse>, Status> {
        let amount = match request.into_inner().epoch {
            Some(epoch) => self
                .membership
                .load_stake(EpochNumber::new(epoch))
                .await
                .map_err(to_status)?
                .and_then(|(_, reward, _)| reward),
            None => None,
        };
        Ok(Response::new(proto::BlockRewardResponse {
            amount: amount.map(|amount| amount.to_string()),
        }))
    }

    async fn get_header_window(
        &self,
        request: Request<proto::GetHeaderWindowRequest>,
    ) -> Result<Response<proto::HeaderWindowResponse>, Status> {
        let r = request.into_inner();
        let end = required(r.end, "end")?;
        let window = match (r.start_time, r.start_height, r.start_hash) {
            (Some(start), None, None) => self.time_window(start, end).await?,
            (None, Some(height), None) => self.height_window(height, end).await?,
            (None, None, Some(hash)) => {
                let height = self
                    .reader
                    .height(&BlockKey::Block(parse(&hash, "start_hash")?))
                    .await?;
                self.height_window(height, end).await?
            },
            _ => {
                return Err(Status::invalid_argument(
                    "set exactly one of start_time, start_height or start_hash",
                ));
            },
        };
        Ok(Response::new(proto::HeaderWindowResponse::from(&window)))
    }

    async fn get_vid_share(
        &self,
        _: Request<proto::GetVidShareRequest>,
    ) -> Result<Response<proto::VidShareResponse>, Status> {
        unimplemented("VID shares")
    }

    async fn get_node_limits(
        &self,
        _: Request<proto::GetNodeLimitsRequest>,
    ) -> Result<Response<proto::NodeLimitsResponse>, Status> {
        Ok(Response::new(
            Limits {
                window_limit: WINDOW_LIMIT,
            }
            .into(),
        ))
    }

    /// The stake table of an epoch, or of the epoch of the newest stored block.
    async fn get_stake_table(
        &self,
        request: Request<proto::GetStakeTableRequest>,
    ) -> Result<Response<proto::StakeTableResponse>, Status> {
        let epoch = match request.into_inner().epoch {
            Some(epoch) => EpochNumber::new(epoch),
            None => {
                let newest = self
                    .reader
                    .newest()
                    .await?
                    .ok_or_else(|| Status::not_found("no block is stored yet"))?;
                EpochNumber::new(epoch_from_block_number(newest, self.epoch_height))
            },
        };
        let stake_table = self
            .tables
            .stake_table(epoch)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found(format!("no stake table of epoch {epoch}")))?;
        Ok(Response::new(
            StakeTableWithEpochNumber {
                epoch: Some(epoch),
                stake_table,
            }
            .into(),
        ))
    }

    async fn get_validators(
        &self,
        request: Request<proto::GetValidatorsRequest>,
    ) -> Result<Response<proto::ValidatorsResponse>, Status> {
        let epoch = EpochNumber::new(required(request.into_inner().epoch, "epoch")?);
        let (validators, ..) = self
            .membership
            .load_stake(epoch)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found(format!("no validators of epoch {epoch}")))?;
        let mut validators: Vec<proto::Validator> = validators
            .into_values()
            .map(|validator| validator.into_inner().into())
            .collect();
        validators.sort_by(|a, b| a.account.cmp(&b.account));
        Ok(Response::new(proto::ValidatorsResponse { validators }))
    }

    async fn get_all_validators(
        &self,
        request: Request<proto::GetAllValidatorsRequest>,
    ) -> Result<Response<proto::ValidatorsResponse>, Status> {
        let r = request.into_inner();
        let epoch = EpochNumber::new(required(r.epoch, "epoch")?);
        let limit = required(r.limit, "limit")?;
        if limit > VALIDATORS_LIMIT {
            return Err(Status::invalid_argument(format!(
                "limit cannot be greater than {VALIDATORS_LIMIT}"
            )));
        }
        let validators = self
            .membership
            .load_all_validators(epoch, required(r.offset, "offset")?, limit)
            .await
            .map_err(|err| Status::not_found(format!("{err:#}")))?;
        Ok(Response::new(proto::ValidatorsResponse {
            validators: validators.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_proposal_participation(
        &self,
        _: Request<proto::GetProposalParticipationRequest>,
    ) -> Result<Response<proto::ParticipationResponse>, Status> {
        unimplemented("participation")
    }

    async fn get_vote_participation(
        &self,
        _: Request<proto::GetVoteParticipationRequest>,
    ) -> Result<Response<proto::ParticipationResponse>, Status> {
        unimplemented("participation")
    }
}

impl ApiService {
    /// The transactions and payload bytes of the stored heights from `from` to `to`, inclusive, of
    /// `namespace` only if given: from genesis and to the newest height when absent.
    async fn totals(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> Result<Totals, Status> {
        let namespace = namespace
            .map(|namespace| request::namespace(Some(namespace)).map(u32::from))
            .transpose()?;
        self.reader
            .store()
            .totals(from.unwrap_or(0)..=to.unwrap_or(u64::MAX), namespace)
            .await
            .map_err(to_status)
    }

    /// The headers whose timestamps are at least `start` and before `end`, at most
    /// [`WINDOW_LIMIT`] of them, with the header before them and, unless the limit cut the
    /// window short, the one after.
    ///
    /// Timestamps do not decrease with height, so the window's first height is found by a binary
    /// search over the stored heights.
    async fn time_window(
        &self,
        start: u64,
        end: u64,
    ) -> Result<TimeWindowQueryData<Header>, Status> {
        let newest = self
            .reader
            .newest()
            .await?
            .ok_or_else(|| Status::not_found("no block is stored yet"))?;
        let floor = self.reader.store().floor().await.map_err(to_status)?;
        let (mut low, mut high) = (floor, newest + 1);
        while low < high {
            let middle = low + (high - low) / 2;
            if self.header(middle).await?.timestamp() < start {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        let window = self.window_from(low, end, newest, low > floor).await?;
        if window.window.is_empty() && window.next.is_none() {
            return Err(Status::not_found(
                "no block is stored at or after the window yet",
            ));
        }
        Ok(window)
    }

    /// The headers from `height` on whose timestamps are before `end`, at most [`WINDOW_LIMIT`]
    /// of them, with the header before them and, unless the limit cut the window short, the one
    /// after.
    async fn height_window(
        &self,
        height: u64,
        end: u64,
    ) -> Result<TimeWindowQueryData<Header>, Status> {
        let newest = self
            .reader
            .newest()
            .await?
            .ok_or_else(|| Status::not_found("no block is stored yet"))?;
        self.window_from(height, end, newest, height > 0).await
    }

    /// The window starting at `first`, up to `newest`, with the header before it if `with_prev`;
    /// see [`Self::height_window`].
    async fn window_from(
        &self,
        first: u64,
        end: u64,
        newest: u64,
        with_prev: bool,
    ) -> Result<TimeWindowQueryData<Header>, Status> {
        let prev = match with_prev {
            true => Some(self.header(first - 1).await?),
            false => None,
        };
        let mut window = Vec::new();
        let mut next = None;
        for height in first..=newest {
            if window.len() == WINDOW_LIMIT {
                break;
            }
            let header = self.header(height).await?;
            if header.timestamp() >= end {
                next = Some(header);
                break;
            }
            window.push(header);
        }
        Ok(TimeWindowQueryData { window, prev, next })
    }

    async fn header(&self, height: u64) -> Result<Header, Status> {
        let item = self.reader.item(height, Wait::Briefly).await?;
        Ok(item.leaf.block_header().clone())
    }

    /// The ranges of heights from `floor` on without an item, in height order.
    async fn item_gaps(&self, floor: u64) -> Result<Vec<Range<u64>>, Status> {
        let mut gaps = Vec::new();
        let mut from = floor;
        loop {
            let page = self
                .reader
                .store()
                .missing_items(from, GAP_PAGE)
                .await
                .map_err(to_status)?;
            let full = page.len() == GAP_PAGE;
            if let Some(last) = page.last() {
                from = last.end;
            }
            gaps.extend(page);
            if !full {
                return Ok(gaps);
            }
        }
    }

    /// The ranges of heights from `floor` on that have an item but not `part`, in height order.
    async fn part_gaps(&self, part: Part, floor: u64) -> Result<Vec<Range<u64>>, Status> {
        let mut gaps: Vec<Range<u64>> = Vec::new();
        let mut from = floor;
        loop {
            let page = self
                .reader
                .store()
                .missing_parts(part, from, GAP_PAGE)
                .await
                .map_err(to_status)?;
            let full = page.len() == GAP_PAGE;
            for height in page {
                match gaps.last_mut() {
                    Some(gap) if gap.end == height => gap.end += 1,
                    _ => gaps.push(height..height + 1),
                }
                from = height + 1;
            }
            if !full {
                return Ok(gaps);
            }
        }
    }
}

/// The status of the heights before `height`: pruned before `floor`, missing in `missing`, and
/// present otherwise.
fn sync_status(floor: u64, height: u64, mut missing: Vec<Range<u64>>) -> ResourceSyncStatus {
    missing.sort_by_key(|gap| gap.start);
    let range = |start: u64, end: u64, status| SyncStatusRange {
        start: start as usize,
        end: end as usize,
        status,
    };
    let mut ranges = Vec::new();
    if floor > 0 {
        ranges.push(range(0, floor, SyncStatus::Pruned));
    }
    let mut cursor = floor;
    let mut count = 0;
    for gap in missing {
        let start = gap.start.max(cursor);
        if start >= gap.end {
            continue;
        }
        if start > cursor {
            ranges.push(range(cursor, start, SyncStatus::Present));
        }
        match ranges.last_mut() {
            Some(last) if last.status == SyncStatus::Missing && last.end == start as usize => {
                last.end = gap.end as usize;
            },
            _ => ranges.push(range(start, gap.end, SyncStatus::Missing)),
        }
        count += gap.end - start;
        cursor = gap.end;
    }
    if cursor < height {
        ranges.push(range(cursor, height, SyncStatus::Present));
    }
    ResourceSyncStatus {
        missing: count as usize,
        ranges,
    }
}

fn unimplemented<T>(what: &str) -> Result<T, Status> {
    Err(Status::unimplemented(format!(
        "this API service does not serve {what} yet"
    )))
}
