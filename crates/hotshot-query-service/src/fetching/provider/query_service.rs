// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::{collections::HashSet, fmt::Debug, ops::Range};

use anyhow::{Context, ensure};
use async_trait::async_trait;
use futures::{TryFutureExt, future::try_join_all};
use hotshot_types::{data::VidCommon, traits::node_implementation::NodeType};
use http_client::{Client, Url, WireVersion};
use serde::de::DeserializeOwned;

use super::Provider;
use crate::{
    Error, Payload,
    availability::{BlockQueryData, Certificate2, LeafQueryData, VidCommonQueryData},
    fetching::{
        NonEmptyRange,
        request::{
            BlockRangeRequest, BlockRangesRequest, BlockRangesResponse, Certificate2Request,
            LeafRangeRequest, LeafRangesRequest, LeafRequest, PayloadRequest,
            VidCommonRangeRequest, VidCommonRangesRequest, VidCommonRequest,
        },
    },
    types::HeightIndexed,
};

/// Data availability provider backed by another instance of this query service.
///
/// This fetcher implements the [`Provider`] interface by querying the REST API provided by another
/// instance of this query service to try and retrieve missing objects.
///
/// This provider trusts the external query service is connected to: it does not verify the
/// responses it receives. Instantiating this provider adds a trust assumption not only on the
/// external service, but on the correctness of this node's local TLS configuration, etc.: anything
/// needed to ensure that the HTTP client is connected to the intended, trusted server.
///
/// To avoid adding additional trust dependencies, do not use this provider; instead, use a provider
/// implementation that verifies the data it receives, for example using a light client for the
/// protocol.
#[derive(Clone, Debug)]
pub struct TrustedQueryServiceProvider {
    // All peers frame at `WireVersion` (VBS 0.1); the API's own version lives in the URL.
    client: Client<Error, WireVersion>,
}

impl TrustedQueryServiceProvider {
    pub fn new(url: Url) -> Self {
        Self {
            client: Client::new(url),
        }
    }
}

impl TrustedQueryServiceProvider {
    pub async fn fetch_payload<Types: NodeType>(
        &self,
        req: PayloadRequest,
    ) -> anyhow::Result<Payload<Types>> {
        let block = self
            .client
            .get::<BlockQueryData<Types>>(&format!("availability/block/payload-hash/{}", req.0))
            .send()
            .await
            .context("fetching block")?;

        Ok(block.payload)
    }

    pub async fn fetch_payload_range<Types: NodeType>(
        &self,
        req: BlockRangeRequest,
    ) -> anyhow::Result<NonEmptyRange<BlockQueryData<Types>>> {
        let blocks = self
            .client
            .get::<NonEmptyRange<BlockQueryData<Types>>>(&format!(
                "availability/block/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching blocks")?;

        ensure!(
            blocks.start() == req.start && blocks.end() == req.end,
            "wrong block range ({}..{})",
            blocks.start(),
            blocks.end()
        );

        Ok(blocks)
    }

    fn handle_result<R: Debug, T>(&self, req: R, res: anyhow::Result<T>) -> Option<T> {
        match res {
            Ok(res) => Some(res),
            Err(err) => {
                tracing::warn!(upstream = %self.client.base_url(), ?req, "failed to fetch: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<Types> Provider<Types, PayloadRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload<Types>> {
        self.handle_result(req, self.fetch_payload::<Types>(req).await)
    }
}

#[async_trait]
impl<Types> Provider<Types, BlockRangeRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: BlockRangeRequest) -> Option<NonEmptyRange<BlockQueryData<Types>>> {
        self.handle_result(req, self.fetch_payload_range::<Types>(req).await)
    }
}

impl TrustedQueryServiceProvider {
    pub async fn fetch_leaf<Types: NodeType>(
        &self,
        req: LeafRequest,
    ) -> anyhow::Result<LeafQueryData<Types>> {
        let leaf = self
            .client
            .get::<LeafQueryData<Types>>(&format!("availability/leaf/{}", req.height))
            .send()
            .await
            .context("fetching leaf")?;

        ensure!(
            leaf.height() == req.height,
            "received leaf with the wrong height ({})",
            leaf.height(),
        );

        Ok(leaf)
    }

    pub async fn fetch_leaf_range<Types: NodeType>(
        &self,
        req: LeafRangeRequest,
    ) -> anyhow::Result<NonEmptyRange<LeafQueryData<Types>>> {
        let leaves = self
            .client
            .get::<NonEmptyRange<LeafQueryData<Types>>>(&format!(
                "availability/leaf/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching leaf chain")?;

        ensure!(
            leaves.start() == req.start && leaves.end() == req.end,
            "server returned wrong range of leaves ({}..{})",
            leaves.start(),
            leaves.end()
        );

        Ok(leaves)
    }
}

#[async_trait]
impl<Types> Provider<Types, LeafRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRequest) -> Option<LeafQueryData<Types>> {
        self.handle_result(req, self.fetch_leaf(req).await)
    }
}

#[async_trait]
impl<Types> Provider<Types, LeafRangeRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRangeRequest) -> Option<NonEmptyRange<LeafQueryData<Types>>> {
        self.handle_result(req, self.fetch_leaf_range(req).await)
    }
}

impl TrustedQueryServiceProvider {
    /// Fetch a cert2 from a peer at the given height.
    pub async fn fetch_cert2<Types: NodeType>(
        &self,
        height: u64,
    ) -> anyhow::Result<Certificate2<Types>> {
        let cert2: Certificate2<Types> = self
            .client
            .get(&format!("availability/cert2/{height}"))
            .send()
            .await
            .context("fetching cert2")?;

        ensure!(
            cert2.data.block_number == height,
            "received cert2 with the wrong height ({})",
            cert2.data.block_number,
        );

        Ok(cert2)
    }
}

#[async_trait]
impl<Types> Provider<Types, Certificate2Request> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: Certificate2Request) -> Option<Option<Certificate2<Types>>> {
        // A found cert2 is `Some(Some(_))`; a 404 (or other error) is `None`, which `any_fetch`
        // skips so it tries the next provider instead of resolving as absent.
        self.handle_result(req, self.fetch_cert2(req.height).await.map(Some))
    }
}

impl TrustedQueryServiceProvider {
    pub async fn fetch_vid_common<Types: NodeType>(
        &self,
        req: VidCommonRequest,
    ) -> anyhow::Result<VidCommon> {
        let res = self
            .client
            .get::<VidCommonQueryData<Types>>(&format!(
                "availability/vid/common/payload-hash/{}",
                req.0
            ))
            .send()
            .await
            .context("fetching VID common")?;

        Ok(res.common)
    }

    async fn fetch_vid_common_range_with_fallback<Types: NodeType>(
        &self,
        start: u64,
        end: u64,
    ) -> anyhow::Result<NonEmptyRange<VidCommonQueryData<Types>>> {
        let res = self
            .client
            .get::<NonEmptyRange<VidCommonQueryData<Types>>>(&format!(
                "availability/vid/common/{start}/{end}",
            ))
            .send()
            .await;
        match res {
            Ok(common) => Ok(common),
            Err(Error::Custom { message, .. }) if message.contains("No route matches") => {
                // Old versions of the upstream query service do not support the ranged VID common
                // endpoint. Fall back to fetching each object individually.
                tracing::info!(
                    start,
                    end,
                    "server does not support ranged VID fetching, falling back to individual \
                     fetches"
                );
                let common = try_join_all((start..end).map(|i| {
                    self.client
                        .get::<VidCommonQueryData<Types>>(&format!("availability/vid/common/{i}"))
                        .send()
                        .map_err(move |err| {
                            anyhow::Error::new(err).context(format!("fetching VID common {i}"))
                        })
                }))
                .await?;
                NonEmptyRange::new(common)
                    .context("converting individually fetched VID common into range")
            },
            Err(err) => Err(err).context("fetching VID common range"),
        }
    }

    /// Ask the peer for the objects in `ranges`, in one request.
    ///
    /// A peer answers a ranges request with only what it holds, so the response is checked here the way the
    /// range fetches check their bounds: an incomplete answer is an error, which sends the caller
    /// to the per-range fallback rather than resolving a fetch that is still missing heights.
    async fn fetch_ranges<T: DeserializeOwned + HeightIndexed>(
        &self,
        route: &str,
        ranges: &[Range<u64>],
    ) -> anyhow::Result<Vec<T>> {
        let objs: Vec<T> = self
            .client
            .post(route)
            .body_binary(&ranges)?
            .send()
            .await
            .context("fetching ranges")?;

        // Check the response the way the range fetches check their bounds. It must cover every
        // height asked for, or the caller falls back to the per-range endpoints, and it must
        // contain nothing else, so a peer cannot use a ranges response to write objects we never requested.
        let fetched = objs.iter().map(|obj| obj.height()).collect::<HashSet<_>>();
        ensure!(
            ranges
                .iter()
                .flat_map(|range| range.clone())
                .all(|height| fetched.contains(&height)),
            "server returned {} of the objects in {} requested ranges",
            objs.len(),
            ranges.len()
        );
        ensure!(
            fetched
                .iter()
                .all(|height| ranges.iter().any(|range| range.contains(height))),
            "server returned objects outside the requested ranges"
        );

        Ok(objs)
    }

    pub async fn fetch_vid_common_range<Types: NodeType>(
        &self,
        req: VidCommonRangeRequest,
    ) -> anyhow::Result<NonEmptyRange<VidCommonQueryData<Types>>> {
        let common = self
            .fetch_vid_common_range_with_fallback(req.start, req.end)
            .await?;

        ensure!(
            common.start() == req.start && common.end() == req.end,
            "server returned wrong VID common ({}..{})",
            common.start(),
            common.end()
        );

        Ok(common)
    }
}

#[async_trait]
impl<Types> Provider<Types, VidCommonRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        self.handle_result(req, self.fetch_vid_common::<Types>(req).await)
    }
}

#[async_trait]
impl<Types> Provider<Types, VidCommonRangeRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(
        &self,
        req: VidCommonRangeRequest,
    ) -> Option<NonEmptyRange<VidCommonQueryData<Types>>> {
        self.handle_result(req, self.fetch_vid_common_range::<Types>(req).await)
    }
}

#[async_trait]
impl<Types> Provider<Types, LeafRangesRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRangesRequest) -> Option<Vec<LeafQueryData<Types>>> {
        let route = "availability/leaf/ranges";
        match self.fetch_ranges(route, &req.0).await {
            Ok(leaves) => Some(leaves),
            Err(err) => {
                let ranges = req.0.iter().map(|range| {
                    self.fetch_leaf_range::<Types>(LeafRangeRequest {
                        start: range.start,
                        end: range.end,
                    })
                });
                self.handle_result(route, fall_back(err, ranges).await)
            },
        }
    }
}

#[async_trait]
impl<Types> Provider<Types, BlockRangesRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: BlockRangesRequest) -> Option<BlockRangesResponse<Types>> {
        // One range is a range fetch, and that endpoint is a cacheable GET.
        if let [range] = req.0.as_slice() {
            let req = BlockRangeRequest {
                start: range.start,
                end: range.end,
            };
            let blocks = self.handle_result(req, self.fetch_payload_range(req).await)?;
            return Some(BlockRangesResponse {
                blocks: blocks.into_iter().collect(),
                vid_common: vec![],
            });
        }

        let route = "availability/block/ranges";
        let blocks = match self.fetch_ranges(route, &req.0).await {
            Ok(blocks) => Some(blocks),
            Err(err) => {
                let ranges = req.0.iter().map(|range| {
                    self.fetch_payload_range::<Types>(BlockRangeRequest {
                        start: range.start,
                        end: range.end,
                    })
                });
                self.handle_result(route, fall_back(err, ranges).await)
            },
        }?;
        Some(BlockRangesResponse {
            blocks,
            vid_common: vec![],
        })
    }
}

#[async_trait]
impl<Types> Provider<Types, VidCommonRangesRequest> for TrustedQueryServiceProvider
where
    Types: NodeType,
{
    async fn fetch(&self, req: VidCommonRangesRequest) -> Option<Vec<VidCommonQueryData<Types>>> {
        let route = "availability/vid/common/ranges";
        match self.fetch_ranges(route, &req.0).await {
            Ok(common) => Some(common),
            Err(err) => {
                let ranges = req.0.iter().map(|range| {
                    self.fetch_vid_common_range::<Types>(VidCommonRangeRequest {
                        start: range.start,
                        end: range.end,
                    })
                });
                self.handle_result(route, fall_back(err, ranges).await)
            },
        }
    }
}

/// Serve a ranges request from the per-range endpoints instead.
///
/// A peer that predates the ranges endpoints answers them with a 404 or a 405, and its error body
/// is not always in the envelope this client decodes, so any failure falls back rather than only
/// the ones that can be identified. The ranges request is an optimization over these, so the
/// fallback is exactly the work the caller would otherwise have done.
async fn fall_back<T, F>(
    err: anyhow::Error,
    ranges: impl Iterator<Item = F>,
) -> anyhow::Result<Vec<T>>
where
    F: Future<Output = anyhow::Result<NonEmptyRange<T>>>,
{
    tracing::info!(%err, "peer did not serve ranges request, falling back to per-range fetches");

    // One at a time. The whole set was one request, and fanning it out to a range fetch per
    // height all at once would hit the peer harder than the scanner did before batching.
    let mut objs = vec![];
    for range in ranges {
        objs.extend(range.await?);
    }
    Ok(objs)
}

// The SQL test databases (`TmpDb`) are not built on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod postgres;
#[cfg(all(test, not(target_os = "windows")))]
mod sqlite;
