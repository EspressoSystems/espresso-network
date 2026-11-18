//! The availability service: the chain's blocks, from the store.

use std::{future::Future, ops::Range};

use espresso_api::proto::{self, availability_service_server::AvailabilityService};
use futures::stream::BoxStream;
use hotshot_query_service_types::availability::PayloadQueryData;
use tonic::{Request, Response, Status};

use crate::{
    ApiService,
    reader::{Reader, Wait},
    request::{BlockKey, block_key, namespace, parse, range, required},
};

/// The most small objects (leaves, headers, VID common data) a range request returns.
const SMALL_RANGE_LIMIT: u64 = 500;

/// The most blocks or payloads a range request returns.
const LARGE_RANGE_LIMIT: u64 = 100;

/// The most namespace proofs a range request returns.
const NAMESPACE_PROOF_RANGE_LIMIT: u64 = 100;

type Stream<T> = BoxStream<'static, Result<T, Status>>;

impl ApiService {
    /// The values `read` gives for each height of `ranges`, which together cover at most `limit`
    /// heights.
    async fn read_ranges<T, F, Fut>(
        &self,
        ranges: impl IntoIterator<Item = Range<u64>>,
        limit: u64,
        read: F,
    ) -> Result<Vec<T>, Status>
    where
        F: Fn(Reader, u64) -> Fut,
        Fut: Future<Output = Result<T, Status>>,
    {
        let ranges: Vec<_> = ranges.into_iter().collect();
        let total: u64 = ranges.iter().map(|r| r.end.saturating_sub(r.start)).sum();
        if total > limit {
            return Err(Status::invalid_argument(format!(
                "at most {limit} heights at a time"
            )));
        }
        let mut values = Vec::with_capacity(total as usize);
        for height in ranges.into_iter().flatten() {
            values.push(read(self.reader.clone(), height).await?);
        }
        Ok(values)
    }
}

fn unimplemented<T>(what: &str) -> Result<T, Status> {
    Err(Status::unimplemented(format!(
        "this API service does not serve {what} yet"
    )))
}

fn ranges(ranges: Vec<proto::HeightRange>) -> Result<Vec<Range<u64>>, Status> {
    ranges
        .into_iter()
        .map(|r| Ok(required(r.from, "from")?..required(r.until, "until")?))
        .collect()
}

#[tonic::async_trait]
impl AvailabilityService for ApiService {
    async fn get_limits(
        &self,
        _: Request<proto::GetLimitsRequest>,
    ) -> Result<Response<proto::LimitsResponse>, Status> {
        Ok(Response::new(proto::LimitsResponse {
            small_object_range_limit: SMALL_RANGE_LIMIT,
            large_object_range_limit: LARGE_RANGE_LIMIT,
            namespace_proof_range_limit: NAMESPACE_PROOF_RANGE_LIMIT,
        }))
    }

    async fn get_header(
        &self,
        request: Request<proto::GetHeaderRequest>,
    ) -> Result<Response<proto::HeaderResponse>, Status> {
        let r = request.into_inner();
        let height = self
            .reader
            .height(&block_key(r.height, r.hash, r.payload_hash)?)
            .await?;
        let item = self.reader.item(height, Wait::Briefly).await?;
        Ok(Response::new(proto::HeaderResponse::from(
            item.leaf.block_header(),
        )))
    }

    async fn get_header_range(
        &self,
        request: Request<proto::GetHeaderRangeRequest>,
    ) -> Result<Response<proto::HeaderRangeResponse>, Status> {
        let r = request.into_inner();
        let range = range(r.from, r.until, SMALL_RANGE_LIMIT)?;
        let headers = self
            .read_ranges([range], SMALL_RANGE_LIMIT, |reader, height| async move {
                let item = reader.item(height, Wait::Briefly).await?;
                Ok(item.leaf.block_header().clone())
            })
            .await?;
        Ok(Response::new(proto::HeaderRangeResponse::from(&*headers)))
    }

    async fn get_leaf(
        &self,
        request: Request<proto::GetLeafRequest>,
    ) -> Result<Response<proto::LeafResponse>, Status> {
        let r = request.into_inner();
        let key = match (r.height, r.hash) {
            (Some(height), None) => BlockKey::Height(height),
            (None, Some(hash)) => BlockKey::Leaf(parse(&hash, "hash")?),
            _ => {
                return Err(Status::invalid_argument(
                    "set exactly one of height or hash",
                ));
            },
        };
        let height = self.reader.height(&key).await?;
        let leaf = self.reader.leaf(height, Wait::Briefly).await?;
        Ok(Response::new(proto::LeafResponse::from(&leaf)))
    }

    async fn get_leaf_range(
        &self,
        request: Request<proto::GetLeafRangeRequest>,
    ) -> Result<Response<proto::LeafRangeResponse>, Status> {
        let r = request.into_inner();
        let range = range(r.from, r.until, SMALL_RANGE_LIMIT)?;
        let leaves = self
            .read_ranges([range], SMALL_RANGE_LIMIT, |reader, height| async move {
                reader.leaf(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::LeafRangeResponse::from(&*leaves)))
    }

    async fn get_leaf_ranges(
        &self,
        request: Request<proto::GetLeafRangesRequest>,
    ) -> Result<Response<proto::LeafRangeResponse>, Status> {
        let ranges = ranges(request.into_inner().ranges)?;
        let leaves = self
            .read_ranges(ranges, SMALL_RANGE_LIMIT, |reader, height| async move {
                reader.leaf(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::LeafRangeResponse::from(&*leaves)))
    }

    async fn get_cert2(
        &self,
        request: Request<proto::GetCert2Request>,
    ) -> Result<Response<proto::Cert2Response>, Status> {
        let height = required(request.into_inner().height, "height")?;
        let item = self.reader.item(height, Wait::Briefly).await?;
        let cert2 = item.cert2.ok_or_else(|| {
            Status::not_found(format!("no cert2 decided height {height} directly"))
        })?;
        Ok(Response::new(proto::Cert2Response {
            certificate: Some((&cert2).into()),
        }))
    }

    async fn get_block(
        &self,
        request: Request<proto::GetBlockRequest>,
    ) -> Result<Response<proto::BlockResponse>, Status> {
        let r = request.into_inner();
        let height = self
            .reader
            .height(&block_key(r.height, r.hash, r.payload_hash)?)
            .await?;
        let block = self.reader.block(height, Wait::Briefly).await?;
        Ok(Response::new(proto::BlockResponse::from(&block)))
    }

    async fn get_block_range(
        &self,
        request: Request<proto::GetBlockRangeRequest>,
    ) -> Result<Response<proto::BlockRangeResponse>, Status> {
        let r = request.into_inner();
        let range = range(r.from, r.until, LARGE_RANGE_LIMIT)?;
        let blocks = self
            .read_ranges([range], LARGE_RANGE_LIMIT, |reader, height| async move {
                reader.block(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::BlockRangeResponse::from(&*blocks)))
    }

    async fn get_block_ranges(
        &self,
        request: Request<proto::GetBlockRangesRequest>,
    ) -> Result<Response<proto::BlockRangeResponse>, Status> {
        let ranges = ranges(request.into_inner().ranges)?;
        let blocks = self
            .read_ranges(ranges, LARGE_RANGE_LIMIT, |reader, height| async move {
                reader.block(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::BlockRangeResponse::from(&*blocks)))
    }

    async fn get_payload(
        &self,
        request: Request<proto::GetPayloadRequest>,
    ) -> Result<Response<proto::PayloadResponse>, Status> {
        let r = request.into_inner();
        let key = match (r.height, r.hash, r.block_hash) {
            (Some(height), None, None) => BlockKey::Height(height),
            (None, Some(hash), None) => BlockKey::Payload(parse(&hash, "hash")?),
            (None, None, Some(hash)) => BlockKey::Block(parse(&hash, "block_hash")?),
            _ => {
                return Err(Status::invalid_argument(
                    "set exactly one of height, hash or block_hash",
                ));
            },
        };
        let height = self.reader.height(&key).await?;
        let payload = PayloadQueryData::from(self.reader.block(height, Wait::Briefly).await?);
        Ok(Response::new(proto::PayloadResponse::from(&payload)))
    }

    async fn get_payload_range(
        &self,
        request: Request<proto::GetPayloadRangeRequest>,
    ) -> Result<Response<proto::PayloadRangeResponse>, Status> {
        let r = request.into_inner();
        let range = range(r.from, r.until, LARGE_RANGE_LIMIT)?;
        let payloads = self
            .read_ranges([range], LARGE_RANGE_LIMIT, |reader, height| async move {
                Ok(PayloadQueryData::from(
                    reader.block(height, Wait::Briefly).await?,
                ))
            })
            .await?;
        Ok(Response::new(proto::PayloadRangeResponse::from(&*payloads)))
    }

    async fn get_vid_common(
        &self,
        request: Request<proto::GetVidCommonRequest>,
    ) -> Result<Response<proto::VidCommonResponse>, Status> {
        let r = request.into_inner();
        let height = self
            .reader
            .height(&block_key(r.height, r.hash, r.payload_hash)?)
            .await?;
        let common = self.reader.vid_common(height, Wait::Briefly).await?;
        Ok(Response::new(proto::VidCommonResponse::try_from(&common)?))
    }

    async fn get_vid_common_range(
        &self,
        request: Request<proto::GetVidCommonRangeRequest>,
    ) -> Result<Response<proto::VidCommonRangeResponse>, Status> {
        let r = request.into_inner();
        let range = range(r.from, r.until, SMALL_RANGE_LIMIT)?;
        let commons = self
            .read_ranges([range], SMALL_RANGE_LIMIT, |reader, height| async move {
                reader.vid_common(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::VidCommonRangeResponse::try_from(
            &*commons,
        )?))
    }

    async fn get_vid_common_ranges(
        &self,
        request: Request<proto::GetVidCommonRangesRequest>,
    ) -> Result<Response<proto::VidCommonRangeResponse>, Status> {
        let ranges = ranges(request.into_inner().ranges)?;
        let commons = self
            .read_ranges(ranges, SMALL_RANGE_LIMIT, |reader, height| async move {
                reader.vid_common(height, Wait::Briefly).await
            })
            .await?;
        Ok(Response::new(proto::VidCommonRangeResponse::try_from(
            &*commons,
        )?))
    }

    async fn get_transaction(
        &self,
        _: Request<proto::GetTransactionRequest>,
    ) -> Result<Response<proto::TransactionResponse>, Status> {
        unimplemented("transactions")
    }

    async fn get_transaction_proof(
        &self,
        _: Request<proto::GetTransactionProofRequest>,
    ) -> Result<Response<proto::TransactionWithProofResponse>, Status> {
        unimplemented("transaction proofs")
    }

    async fn get_block_summary(
        &self,
        _: Request<proto::GetBlockSummaryRequest>,
    ) -> Result<Response<proto::BlockSummaryResponse>, Status> {
        unimplemented("block summaries")
    }

    async fn get_block_summary_range(
        &self,
        _: Request<proto::GetBlockSummaryRangeRequest>,
    ) -> Result<Response<proto::BlockSummaryRangeResponse>, Status> {
        unimplemented("block summaries")
    }

    async fn get_namespace_proof(
        &self,
        request: Request<proto::GetNamespaceProofRequest>,
    ) -> Result<Response<proto::NamespaceProofResponse>, Status> {
        let r = request.into_inner();
        let namespace = namespace(r.namespace)?;
        let height = self
            .reader
            .height(&block_key(r.height, r.hash, r.payload_hash)?)
            .await?;
        let proof = self
            .reader
            .namespace_proof(height, namespace, Wait::Briefly)
            .await?;
        Ok(Response::new(proto::NamespaceProofResponse::try_from(
            &proof,
        )?))
    }

    async fn get_namespace_proof_range(
        &self,
        request: Request<proto::GetNamespaceProofRangeRequest>,
    ) -> Result<Response<proto::NamespaceProofRangeResponse>, Status> {
        let r = request.into_inner();
        let namespace = namespace(r.namespace)?;
        let range = range(r.from, r.until, NAMESPACE_PROOF_RANGE_LIMIT)?;
        let proofs = self
            .read_ranges(
                [range],
                NAMESPACE_PROOF_RANGE_LIMIT,
                |reader, height| async move {
                    reader
                        .namespace_proof(height, namespace, Wait::Briefly)
                        .await
                },
            )
            .await?;
        Ok(Response::new(proto::NamespaceProofRangeResponse::try_from(
            &*proofs,
        )?))
    }

    async fn get_incorrect_encoding_proof(
        &self,
        _: Request<proto::GetIncorrectEncodingProofRequest>,
    ) -> Result<Response<proto::IncorrectEncodingProofResponse>, Status> {
        unimplemented("incorrect encoding proofs")
    }

    async fn get_state_cert(
        &self,
        _: Request<proto::GetStateCertRequest>,
    ) -> Result<Response<proto::StateCertV1Response>, Status> {
        unimplemented("state certificates")
    }

    async fn get_state_cert_v2(
        &self,
        _: Request<proto::GetStateCertV2Request>,
    ) -> Result<Response<proto::StateCertV2Response>, Status> {
        unimplemented("state certificates")
    }

    type StreamLeavesStream = Stream<proto::LeafResponse>;

    async fn stream_leaves(
        &self,
        request: Request<proto::StreamLeavesRequest>,
    ) -> Result<Response<Self::StreamLeavesStream>, Status> {
        let from = required(request.into_inner().from, "from")?;
        Ok(Response::new(self.reader.stream(
            from,
            |reader, height| async move {
                let leaf = reader.leaf(height, Wait::Forever).await?;
                Ok(proto::LeafResponse::from(&leaf))
            },
        )))
    }

    type StreamHeadersStream = Stream<proto::HeaderResponse>;

    async fn stream_headers(
        &self,
        request: Request<proto::StreamHeadersRequest>,
    ) -> Result<Response<Self::StreamHeadersStream>, Status> {
        let from = required(request.into_inner().from, "from")?;
        Ok(Response::new(self.reader.stream(
            from,
            |reader, height| async move {
                let item = reader.item(height, Wait::Forever).await?;
                Ok(proto::HeaderResponse::from(item.leaf.block_header()))
            },
        )))
    }

    type StreamBlocksStream = Stream<proto::BlockResponse>;

    async fn stream_blocks(
        &self,
        request: Request<proto::StreamBlocksRequest>,
    ) -> Result<Response<Self::StreamBlocksStream>, Status> {
        let from = required(request.into_inner().from, "from")?;
        Ok(Response::new(self.reader.stream(
            from,
            |reader, height| async move {
                let block = reader.block(height, Wait::Forever).await?;
                Ok(proto::BlockResponse::from(&block))
            },
        )))
    }

    type StreamPayloadsStream = Stream<proto::PayloadResponse>;

    async fn stream_payloads(
        &self,
        request: Request<proto::StreamPayloadsRequest>,
    ) -> Result<Response<Self::StreamPayloadsStream>, Status> {
        let from = required(request.into_inner().from, "from")?;
        Ok(Response::new(self.reader.stream(
            from,
            |reader, height| async move {
                let payload = PayloadQueryData::from(reader.block(height, Wait::Forever).await?);
                Ok(proto::PayloadResponse::from(&payload))
            },
        )))
    }

    type StreamVidCommonStream = Stream<proto::VidCommonResponse>;

    async fn stream_vid_common(
        &self,
        request: Request<proto::StreamVidCommonRequest>,
    ) -> Result<Response<Self::StreamVidCommonStream>, Status> {
        let from = required(request.into_inner().from, "from")?;
        Ok(Response::new(self.reader.stream(
            from,
            |reader, height| async move {
                let common = reader.vid_common(height, Wait::Forever).await?;
                proto::VidCommonResponse::try_from(&common)
            },
        )))
    }

    type StreamTransactionsStream = Stream<proto::TransactionResponse>;

    async fn stream_transactions(
        &self,
        _: Request<proto::StreamTransactionsRequest>,
    ) -> Result<Response<Self::StreamTransactionsStream>, Status> {
        unimplemented("transactions")
    }

    type StreamNamespaceProofsStream = Stream<proto::NamespaceProofResponse>;

    async fn stream_namespace_proofs(
        &self,
        request: Request<proto::StreamNamespaceProofsRequest>,
    ) -> Result<Response<Self::StreamNamespaceProofsStream>, Status> {
        let r = request.into_inner();
        let from = required(r.from, "from")?;
        let namespace = namespace(r.namespace)?;
        Ok(Response::new(self.reader.stream(
            from,
            move |reader, height| async move {
                let proof = reader
                    .namespace_proof(height, namespace, Wait::Forever)
                    .await?;
                proto::NamespaceProofResponse::try_from(&proof)
            },
        )))
    }
}
