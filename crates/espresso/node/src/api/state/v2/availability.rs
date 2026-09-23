//! `AvailabilityService`.

use super::*;

#[tonic::async_trait]
impl<D> proto::availability_service_server::AvailabilityService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    // Delegates to both v1 availability traits, so it needs the bounds of both.
    D::Target: AvailabilityDataSource<SeqTypes>
        + hotshot_query_service::data_source::VersionedDataSource
        + hotshot_query_service::node::NodeDataSource<SeqTypes>
        + RequestResponseDataSource<SeqTypes>
        + StateCertDataSource
        + StateCertFetchingDataSource<SeqTypes>
        + Send
        + Sync,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::AvailabilityStorage<SeqTypes>,
{
    async fn get_limits(
        &self,
        _request: tonic::Request<proto::GetLimitsRequest>,
    ) -> Result<tonic::Response<proto::LimitsResponse>, tonic::Status> {
        let limits = <Self as v1::HotShotAvailabilityApi>::get_limits(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LimitsResponse {
            small_object_range_limit: limits.small_object_range_limit as u64,
            large_object_range_limit: limits.large_object_range_limit as u64,
            namespace_proof_range_limit: NAMESPACE_PROOF_RANGE_LIMIT,
        }))
    }

    async fn get_header(
        &self,
        request: tonic::Request<proto::GetHeaderRequest>,
    ) -> Result<tonic::Response<proto::HeaderResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let header = <Self as v1::HotShotAvailabilityApi>::get_header(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderResponse::from(&header)))
    }

    async fn get_header_range(
        &self,
        request: tonic::Request<proto::GetHeaderRangeRequest>,
    ) -> Result<tonic::Response<proto::HeaderRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let headers = <Self as v1::HotShotAvailabilityApi>::get_header_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderRangeResponse::from(
            &*headers,
        )))
    }

    async fn get_leaf(
        &self,
        request: tonic::Request<proto::GetLeafRequest>,
    ) -> Result<tonic::Response<proto::LeafResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash) {
            (Some(height), None) => v1::availability::LeafId::Height(height),
            (None, Some(hash)) => v1::availability::LeafId::Hash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height or hash",
                ));
            },
        };
        let leaf = <Self as v1::HotShotAvailabilityApi>::get_leaf(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafResponse::from(&leaf)))
    }

    async fn get_leaf_range(
        &self,
        request: tonic::Request<proto::GetLeafRangeRequest>,
    ) -> Result<tonic::Response<proto::LeafRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let leaves = <Self as v1::HotShotAvailabilityApi>::get_leaf_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafRangeResponse::from(
            &*leaves,
        )))
    }

    async fn get_leaf_ranges(
        &self,
        request: tonic::Request<proto::GetLeafRangesRequest>,
    ) -> Result<tonic::Response<proto::LeafRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let leaves = <Self as v1::HotShotAvailabilityApi>::get_leaf_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafRangeResponse::from(
            &*leaves,
        )))
    }

    async fn get_cert2(
        &self,
        request: tonic::Request<proto::GetCert2Request>,
    ) -> Result<tonic::Response<proto::Cert2Response>, tonic::Status> {
        let height = required(request.into_inner().height, "height")?;
        let cert2 = <Self as v1::HotShotAvailabilityApi>::get_cert2(self, height)
            .await
            .map_err(to_status)?
            .ok_or_else(|| {
                tonic::Status::not_found(format!("no cert2 available for height {height}"))
            })?;
        Ok(tonic::Response::new(proto::Cert2Response {
            certificate: Some((&cert2).into()),
        }))
    }

    async fn get_block(
        &self,
        request: tonic::Request<proto::GetBlockRequest>,
    ) -> Result<tonic::Response<proto::BlockResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let block = <Self as v1::HotShotAvailabilityApi>::get_block(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockResponse::from(&block)))
    }

    async fn get_block_range(
        &self,
        request: tonic::Request<proto::GetBlockRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let blocks = <Self as v1::HotShotAvailabilityApi>::get_block_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRangeResponse::from(
            &*blocks,
        )))
    }

    async fn get_block_ranges(
        &self,
        request: tonic::Request<proto::GetBlockRangesRequest>,
    ) -> Result<tonic::Response<proto::BlockRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let blocks = <Self as v1::HotShotAvailabilityApi>::get_block_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRangeResponse::from(
            &*blocks,
        )))
    }

    async fn get_payload(
        &self,
        request: tonic::Request<proto::GetPayloadRequest>,
    ) -> Result<tonic::Response<proto::PayloadResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.block_hash) {
            (Some(height), None, None) => v1::availability::PayloadId::Height(height),
            (None, Some(hash), None) => v1::availability::PayloadId::Hash(hash),
            (None, None, Some(block_hash)) => v1::availability::PayloadId::BlockHash(block_hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or block_hash",
                ));
            },
        };
        let payload = <Self as v1::HotShotAvailabilityApi>::get_payload(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadResponse::from(&payload)))
    }

    async fn get_payload_range(
        &self,
        request: tonic::Request<proto::GetPayloadRangeRequest>,
    ) -> Result<tonic::Response<proto::PayloadRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let payloads = <Self as v1::HotShotAvailabilityApi>::get_payload_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadRangeResponse::from(
            &*payloads,
        )))
    }

    async fn get_vid_common(
        &self,
        request: tonic::Request<proto::GetVidCommonRequest>,
    ) -> Result<tonic::Response<proto::VidCommonResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let common = <Self as v1::HotShotAvailabilityApi>::get_vid_common(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::VidCommonResponse::try_from(
            &common,
        )?))
    }

    async fn get_vid_common_range(
        &self,
        request: tonic::Request<proto::GetVidCommonRangeRequest>,
    ) -> Result<tonic::Response<proto::VidCommonRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let items = <Self as v1::HotShotAvailabilityApi>::get_vid_common_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::VidCommonRangeResponse::try_from(&*items)?,
        ))
    }

    async fn get_vid_common_ranges(
        &self,
        request: tonic::Request<proto::GetVidCommonRangesRequest>,
    ) -> Result<tonic::Response<proto::VidCommonRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let items = <Self as v1::HotShotAvailabilityApi>::get_vid_common_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::VidCommonRangeResponse::try_from(&*items)?,
        ))
    }

    async fn get_transaction(
        &self,
        request: tonic::Request<proto::GetTransactionRequest>,
    ) -> Result<tonic::Response<proto::TransactionResponse>, tonic::Status> {
        let request = request.into_inner();
        let tx = match (request.height, request.index, request.hash) {
            (Some(height), Some(index), None) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_by_position(
                    self, height, index,
                )
                .await
            },
            (None, None, Some(hash)) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_by_hash(self, hash).await
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set height and index, or hash",
                ));
            },
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TransactionResponse::from(&tx)))
    }

    async fn get_transaction_proof(
        &self,
        request: tonic::Request<proto::GetTransactionProofRequest>,
    ) -> Result<tonic::Response<proto::TransactionWithProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let tx = match (request.height, request.index, request.hash) {
            (Some(height), Some(index), None) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_proof_by_position(
                    self, height, index,
                )
                .await
            },
            (None, None, Some(hash)) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_proof_by_hash(self, hash)
                    .await
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set height and index, or hash",
                ));
            },
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::TransactionWithProofResponse::try_from(&tx)?,
        ))
    }

    async fn get_block_summary(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryResponse>, tonic::Status> {
        let height = required(request.into_inner().height, "height")? as usize;
        let summary = <Self as v1::HotShotAvailabilityApi>::get_block_summary(self, height)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockSummaryResponse::from(
            &summary,
        )))
    }

    async fn get_block_summary_range(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let summaries = <Self as v1::HotShotAvailabilityApi>::get_block_summary_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::BlockSummaryRangeResponse::from(&*summaries),
        ))
    }

    async fn get_namespace_proof(
        &self,
        request: tonic::Request<proto::GetNamespaceProofRequest>,
    ) -> Result<tonic::Response<proto::NamespaceProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proof = <Self as v1::AvailabilityApi>::get_namespace_proof(self, id, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::NamespaceProofResponse::try_from(&proof)?,
        ))
    }

    async fn get_namespace_proof_range(
        &self,
        request: tonic::Request<proto::GetNamespaceProofRangeRequest>,
    ) -> Result<tonic::Response<proto::NamespaceProofRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proofs = <Self as v1::AvailabilityApi>::get_namespace_proof_range(
            self,
            range.start,
            range.end,
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::NamespaceProofRangeResponse::try_from(&*proofs)?,
        ))
    }

    async fn get_incorrect_encoding_proof(
        &self,
        request: tonic::Request<proto::GetIncorrectEncodingProofRequest>,
    ) -> Result<tonic::Response<proto::IncorrectEncodingProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let height = required(request.height, "height")?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proof = <Self as v1::AvailabilityApi>::get_incorrect_encoding_proof(
            self,
            v1::availability::BlockId::Height(height),
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::IncorrectEncodingProofResponse {
                proof: Some((&proof).try_into()?),
            },
        ))
    }

    async fn get_state_cert(
        &self,
        request: tonic::Request<proto::GetStateCertRequest>,
    ) -> Result<tonic::Response<proto::StateCertV1Response>, tonic::Status> {
        let cert = <Self as v1::AvailabilityApi>::get_state_cert(
            self,
            required(request.into_inner().epoch, "epoch")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::StateCertV1Response::from(
            &cert,
        )))
    }

    async fn get_state_cert_v2(
        &self,
        request: tonic::Request<proto::GetStateCertV2Request>,
    ) -> Result<tonic::Response<proto::StateCertV2Response>, tonic::Status> {
        let cert = <Self as v1::AvailabilityApi>::get_state_cert_v2(
            self,
            required(request.into_inner().epoch, "epoch")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::StateCertV2Response::from(
            &cert,
        )))
    }

    type StreamLeavesStream = BoxStream<'static, Result<proto::LeafResponse, tonic::Status>>;

    async fn stream_leaves(
        &self,
        request: tonic::Request<proto::StreamLeavesRequest>,
    ) -> Result<tonic::Response<Self::StreamLeavesStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let leaves = <Self as v1::HotShotAvailabilityApi>::stream_leaves(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            leaves
                .map(|leaf| Ok(proto::LeafResponse::from(&leaf)))
                .boxed(),
        ))
    }

    type StreamHeadersStream = BoxStream<'static, Result<proto::HeaderResponse, tonic::Status>>;

    async fn stream_headers(
        &self,
        request: tonic::Request<proto::StreamHeadersRequest>,
    ) -> Result<tonic::Response<Self::StreamHeadersStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let headers = <Self as v1::HotShotAvailabilityApi>::stream_headers(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            headers
                .map(|header| Ok(proto::HeaderResponse::from(&header)))
                .boxed(),
        ))
    }

    type StreamBlocksStream = BoxStream<'static, Result<proto::BlockResponse, tonic::Status>>;

    async fn stream_blocks(
        &self,
        request: tonic::Request<proto::StreamBlocksRequest>,
    ) -> Result<tonic::Response<Self::StreamBlocksStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let blocks = <Self as v1::HotShotAvailabilityApi>::stream_blocks(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            blocks
                .map(|block| Ok(proto::BlockResponse::from(&block)))
                .boxed(),
        ))
    }

    type StreamPayloadsStream = BoxStream<'static, Result<proto::PayloadResponse, tonic::Status>>;

    async fn stream_payloads(
        &self,
        request: tonic::Request<proto::StreamPayloadsRequest>,
    ) -> Result<tonic::Response<Self::StreamPayloadsStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let payloads = <Self as v1::HotShotAvailabilityApi>::stream_payloads(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            payloads
                .map(|payload| Ok(proto::PayloadResponse::from(&payload)))
                .boxed(),
        ))
    }

    type StreamVidCommonStream =
        BoxStream<'static, Result<proto::VidCommonResponse, tonic::Status>>;

    async fn stream_vid_common(
        &self,
        request: tonic::Request<proto::StreamVidCommonRequest>,
    ) -> Result<tonic::Response<Self::StreamVidCommonStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let items = <Self as v1::HotShotAvailabilityApi>::stream_vid_common(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(end_at_first_error(
            items.map(|item| proto::VidCommonResponse::try_from(&item)),
        )))
    }

    type StreamTransactionsStream =
        BoxStream<'static, Result<proto::TransactionResponse, tonic::Status>>;

    async fn stream_transactions(
        &self,
        request: tonic::Request<proto::StreamTransactionsRequest>,
    ) -> Result<tonic::Response<Self::StreamTransactionsStream>, tonic::Status> {
        let request = request.into_inner();
        let transactions = <Self as v1::HotShotAvailabilityApi>::stream_transactions(
            self,
            required(request.from, "from")? as usize,
            namespace_from_query(request.namespace)?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            transactions
                .map(|tx| Ok(proto::TransactionResponse::from(&tx)))
                .boxed(),
        ))
    }

    type StreamNamespaceProofsStream =
        BoxStream<'static, Result<proto::NamespaceProofResponse, tonic::Status>>;

    async fn stream_namespace_proofs(
        &self,
        request: tonic::Request<proto::StreamNamespaceProofsRequest>,
    ) -> Result<tonic::Response<Self::StreamNamespaceProofsStream>, tonic::Status> {
        let request = request.into_inner();
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proofs = <Self as v1::AvailabilityApi>::stream_namespace_proofs(
            self,
            required(request.from, "from")? as usize,
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(end_at_first_error(proofs.map(
            |proof| proto::NamespaceProofResponse::try_from(&proof),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::{super::test_support::*, *};

    /// Covers the four shapes and all seven arms: every version's vector must select the arm named
    /// after it, and the proto message must carry exactly the fields v1 serializes, so neither a
    /// new protocol version nor a proto edit can add or drop a header field without failing here.
    #[test]
    fn every_header_version_maps_to_its_arm_and_fields() {
        for (version, shape) in [
            ("v1", "HeaderV1"),
            ("v2", "HeaderV1"),
            ("v3", "HeaderV3"),
            ("v4", "HeaderV4"),
            ("v5", "HeaderV5"),
            ("v6", "HeaderV5"),
            ("v7", "HeaderV5"),
        ] {
            let (header, fields) = reference_header(version);
            assert_same_fields(shape, &fields);

            use proto::header_response::Header;
            let converted = proto::HeaderResponse::from(&header).header.unwrap();
            // Every shape repeats these assignments in its own struct literal, so each one is
            // compared against the vector: the reference heights, timestamps and l1_head are
            // distinct, so a field wired to its neighbour fails here.
            macro_rules! assert_shared_fields {
                ($header:expr) => {{
                    let header = $header;
                    assert_eq!(header.height, fields["height"].as_u64().unwrap());
                    assert_eq!(header.timestamp, fields["timestamp"].as_u64().unwrap());
                    assert_eq!(header.l1_head, fields["l1_head"].as_u64().unwrap());
                    assert_eq!(
                        header.payload_commitment,
                        fields["payload_commitment"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.builder_commitment,
                        fields["builder_commitment"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.block_merkle_tree_root,
                        fields["block_merkle_tree_root"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.fee_merkle_tree_root,
                        fields["fee_merkle_tree_root"].as_str().unwrap()
                    );
                    let fee_info = header.fee_info.as_ref().unwrap();
                    assert_eq!(
                        fee_info.account,
                        fields["fee_info"]["account"].as_str().unwrap()
                    );
                    assert_eq!(
                        fee_info.amount,
                        fields["fee_info"]["amount"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.ns_table.as_ref().unwrap().bytes,
                        base64::engine::general_purpose::STANDARD
                            .decode(fields["ns_table"]["bytes"].as_str().unwrap())
                            .unwrap()
                    );
                    assert_eq!(
                        header.l1_finalized.is_some(),
                        !fields["l1_finalized"].is_null()
                    );
                    assert_eq!(
                        header.builder_signature.is_some(),
                        !fields["builder_signature"].is_null()
                    );
                    assert!(header.chain_config.is_some());
                }};
            }
            let arm = match converted {
                Header::V1(header) => {
                    assert_shared_fields!(header);
                    "v1"
                },
                Header::V2(header) => {
                    assert_shared_fields!(header);
                    "v2"
                },
                Header::V3(header) => {
                    assert_shared_fields!(&header);
                    assert_eq!(
                        header.reward_merkle_tree_root,
                        fields["reward_merkle_tree_root"].as_str().unwrap()
                    );
                    "v3"
                },
                Header::V4(header) => {
                    assert_shared_fields!(&header);
                    assert_eq!(
                        header.timestamp_millis,
                        fields["timestamp_millis"].as_u64().unwrap()
                    );
                    assert_eq!(
                        header.total_reward_distributed,
                        fields["total_reward_distributed"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.next_stake_table_hash.as_deref(),
                        fields["next_stake_table_hash"].as_str()
                    );
                    "v4"
                },
                Header::V5(header) => {
                    assert_shared_fields!(&header);
                    "v5"
                },
                Header::V6(header) => {
                    assert_shared_fields!(&header);
                    "v6"
                },
                Header::V7(header) => {
                    assert_shared_fields!(&header);
                    "v7"
                },
            };
            assert_eq!(arm, version, "{version} header selected the {arm} arm");
        }
    }

    #[test]
    fn v6_header_mirrors_the_reference_vector() {
        let (header, fields) = reference_header("v6");
        assert_same_fields("HeaderV5", &fields);
        let proto::HeaderResponse { header: converted } = (&header).into();
        let Some(proto::header_response::Header::V6(converted)) = converted else {
            panic!("a 0.6 header must convert to the V6 arm, got {converted:?}");
        };

        assert_eq!(converted.height, fields["height"].as_u64().unwrap());
        assert_eq!(converted.timestamp, fields["timestamp"].as_u64().unwrap());
        assert_eq!(
            converted.timestamp_millis,
            fields["timestamp_millis"].as_u64().unwrap()
        );
        assert_eq!(converted.l1_head, fields["l1_head"].as_u64().unwrap());
        assert_eq!(
            converted.payload_commitment,
            fields["payload_commitment"].as_str().unwrap()
        );
        assert_eq!(
            converted.builder_commitment,
            fields["builder_commitment"].as_str().unwrap()
        );
        assert_eq!(
            converted.block_merkle_tree_root,
            fields["block_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.fee_merkle_tree_root,
            fields["fee_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.reward_merkle_tree_root,
            fields["reward_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.total_reward_distributed,
            fields["total_reward_distributed"].as_str().unwrap()
        );
        assert_eq!(
            converted.next_stake_table_hash.as_deref(),
            fields["next_stake_table_hash"].as_str()
        );

        let fee_info = converted.fee_info.unwrap();
        assert_eq!(fee_info.account, fields["fee_info"]["account"]);
        assert_eq!(fee_info.amount, fields["fee_info"]["amount"]);

        let l1_finalized = converted.l1_finalized.unwrap();
        assert_eq!(
            l1_finalized.number,
            fields["l1_finalized"]["number"].as_u64().unwrap()
        );
        assert_eq!(l1_finalized.timestamp, fields["l1_finalized"]["timestamp"]);
        assert_eq!(l1_finalized.hash, fields["l1_finalized"]["hash"]);

        let signature = converted.builder_signature.unwrap();
        assert_eq!(signature.r, fields["builder_signature"]["r"]);
        assert_eq!(signature.s, fields["builder_signature"]["s"]);
        assert_eq!(
            signature.v,
            fields["builder_signature"]["v"].as_u64().unwrap() as u32
        );

        // protoJSON base64s the bytes, which is how v1 renders the table too.
        let ns_table = converted.ns_table.unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&ns_table.bytes),
            fields["ns_table"]["bytes"].as_str().unwrap()
        );

        let config = match converted.chain_config.unwrap().chain_config.unwrap() {
            proto::resolvable_chain_config::ChainConfig::Full(config) => config,
            other => panic!("the reference header carries a full config, got {other:?}"),
        };
        let expected = &fields["chain_config"]["chain_config"]["Left"];
        assert_same_fields("ChainConfig", expected);

        assert_eq!(config.chain_id, expected["chain_id"]);
        assert_eq!(
            config.max_block_size,
            expected["max_block_size"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(config.base_fee, expected["base_fee"]);
        assert_eq!(config.fee_recipient, expected["fee_recipient"]);
        assert_eq!(
            config.fee_contract.as_deref(),
            expected["fee_contract"].as_str()
        );
        assert_eq!(
            config.stake_table_contract.as_deref(),
            expected["stake_table_contract"].as_str()
        );

        assert_eq!(converted.leader_counts.len(), 100);
        let expected_counts: Vec<u32> = fields["leader_counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|count| count.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(converted.leader_counts, expected_counts);
    }

    /// No reference vector carries a commitment-only chain config, so the `Right` arm is checked
    /// here on its own. `resolve` must report absence rather than `commit` hashing an empty config.
    #[test]
    fn commitment_only_chain_config_keeps_the_commitment() {
        let config = espresso_types::v0_3::ChainConfig::default();
        let commitment = config.commit();
        let resolvable = espresso_types::v0_3::ResolvableChainConfig::from(commitment);

        let converted = proto::ResolvableChainConfig::from(resolvable)
            .chain_config
            .unwrap();
        assert_eq!(
            converted,
            proto::resolvable_chain_config::ChainConfig::Commitment(commitment.to_string())
        );
    }

    /// A 0.1 header has no reward tree, no millisecond timestamp and no leader counts, so the V1
    /// message must not carry them. This is the case where the `reward_merkle_tree_root`
    /// accessor would have reported the commitment of an empty tree instead of nothing.
    #[test]
    fn v1_header_mirrors_the_reference_vector() {
        let (header, fields) = reference_header("v1");
        assert_same_fields("HeaderV1", &fields);
        let proto::HeaderResponse { header: converted } = (&header).into();
        let Some(proto::header_response::Header::V1(converted)) = converted else {
            panic!("a 0.1 header must convert to the V1 arm, got {converted:?}");
        };

        assert_eq!(converted.height, fields["height"].as_u64().unwrap());
        assert_eq!(converted.timestamp, fields["timestamp"].as_u64().unwrap());
        assert_eq!(converted.l1_head, fields["l1_head"].as_u64().unwrap());
        assert_eq!(
            converted.payload_commitment,
            fields["payload_commitment"].as_str().unwrap()
        );
        let fee_info = converted.fee_info.unwrap();
        assert_eq!(fee_info.account, fields["fee_info"]["account"]);
        assert_eq!(fee_info.amount, fields["fee_info"]["amount"]);
    }

    #[tokio::test]
    async fn a_stream_ends_at_its_first_error() {
        let items = futures::stream::iter([
            Ok(1),
            Err(tonic::Status::internal("conversion failed")),
            Ok(2),
        ]);
        let delivered: Vec<_> = end_at_first_error(items).collect().await;
        assert_eq!(delivered.len(), 2);
        assert_eq!(delivered[0].as_ref().unwrap(), &1);
        assert!(delivered[1].is_err());
    }

    /// No vector carries view-change evidence, an upgrade certificate, a phase-2 certificate or a
    /// signed QC, so those are built here. The aggregate signature must print as the TaggedBase64
    /// form v1's serde emits, so a v2 client can hand it back to v1.
    #[test]
    fn synthesized_certificates_convert_arm_by_arm() {
        use std::marker::PhantomData;

        use committable::Commitment;
        use espresso_types::PubKey;
        use hotshot_types::{
            data::{EpochNumber, ViewChangeEvidence2, ViewNumber},
            simple_certificate::{
                TimeoutCertificate2, UpgradeCertificate, ViewSyncFinalizeCertificate2,
            },
            simple_vote::{TimeoutData2, UpgradeProposalData, ViewSyncFinalizeData2, Vote2Data},
            traits::signature_key::SignatureKey as _,
        };
        use proto::view_change_evidence2::Evidence;

        let (_, private_key) = PubKey::generated_from_seed_indexed([7; 32], 0);
        let signature = PubKey::sign(&private_key, b"vote").unwrap();
        let mut signers = bitvec::vec::BitVec::<usize, bitvec::order::Lsb0>::repeat(false, 4);
        signers.set(1, true);
        signers.set(3, true);

        let timeout = TimeoutCertificate2::<SeqTypes>::new(
            TimeoutData2 {
                view: ViewNumber::new(9),
                epoch: Some(EpochNumber::new(2)),
            },
            Commitment::from_raw([1; 32]),
            ViewNumber::new(9),
            Some((signature.clone(), signers)),
            PhantomData,
        );
        let Some(Evidence::Timeout(converted)) =
            proto::ViewChangeEvidence2::from(&ViewChangeEvidence2::Timeout(timeout)).evidence
        else {
            panic!("a timeout certificate must select the timeout arm");
        };
        let data = converted.data.unwrap();
        assert_eq!(data.view, 9);
        assert_eq!(data.epoch, Some(2));
        assert_eq!(converted.view_number, 9);
        assert_eq!(
            converted.vote_commitment,
            Commitment::<TimeoutData2>::from_raw([1; 32]).to_string()
        );
        let signatures = converted.signatures.unwrap();
        assert_eq!(signatures.signers, Vec::from([false, true, false, true]));
        assert_eq!(signatures.signature, signature.to_string());
        assert!(signatures.signature.starts_with("BLS_SIG~"));
        assert_eq!(
            serde_json::to_value(&signature).unwrap(),
            serde_json::Value::String(signatures.signature)
        );

        let view_sync = ViewSyncFinalizeCertificate2::<SeqTypes>::new(
            ViewSyncFinalizeData2 {
                relay: 3,
                round: ViewNumber::new(10),
                epoch: None,
            },
            Commitment::from_raw([2; 32]),
            ViewNumber::new(10),
            None,
            PhantomData,
        );
        let Some(Evidence::ViewSync(converted)) =
            proto::ViewChangeEvidence2::from(&ViewChangeEvidence2::ViewSync(view_sync)).evidence
        else {
            panic!("a view sync certificate must select the view_sync arm");
        };
        let data = converted.data.unwrap();
        assert_eq!((data.relay, data.round, data.epoch), (3, 10, None));
        assert!(converted.signatures.is_none());

        let upgrade = UpgradeCertificate::<SeqTypes>::new(
            UpgradeProposalData {
                old_version: vbs::version::Version { major: 0, minor: 3 },
                new_version: vbs::version::Version { major: 0, minor: 4 },
                decide_by: ViewNumber::new(20),
                new_version_hash: Vec::from([0xab, 0xcd]),
                old_version_last_view: ViewNumber::new(19),
                new_version_first_view: ViewNumber::new(21),
            },
            Commitment::from_raw([3; 32]),
            ViewNumber::new(15),
            None,
            PhantomData,
        );
        let data = proto::UpgradeCertificate::from(&upgrade).data.unwrap();
        assert_eq!(data.old_version.unwrap().minor, 3);
        assert_eq!(data.new_version.unwrap().minor, 4);
        assert_eq!(data.new_version_hash, Vec::from([0xab, 0xcd]));
        assert_eq!(
            (
                data.decide_by,
                data.old_version_last_view,
                data.new_version_first_view
            ),
            (20, 19, 21)
        );

        let cert2 = Certificate2::<SeqTypes>::new(
            Vote2Data {
                leaf_commit: Commitment::from_raw([4; 32]),
                epoch: EpochNumber::new(5),
                block_number: 77,
            },
            Commitment::from_raw([5; 32]),
            ViewNumber::new(30),
            None,
            PhantomData,
        );
        let converted = proto::Certificate2::from(&cert2);
        let data = converted.data.unwrap();
        assert_eq!(
            data.leaf_commit,
            Commitment::<hotshot_types::data::Leaf2<SeqTypes>>::from_raw([4; 32]).to_string()
        );
        assert_eq!((data.epoch, data.block_number), (5, 77));
        assert_eq!(converted.view_number, 30);
    }

    #[test]
    fn namespace_proofs_mirror_the_reference_vectors() {
        use base64::Engine as _;
        use proto::ns_proof::Proof;
        let b64 = base64::engine::general_purpose::STANDARD;

        fn check_transactions(converted: &proto::NamespaceProofResponse, json: &serde_json::Value) {
            let expected = json["transactions"].as_array().unwrap();
            assert_eq!(converted.transactions.len(), expected.len());
            for (tx, expected) in converted.transactions.iter().zip(expected) {
                assert_eq!(tx.namespace, expected["namespace"].as_u64().unwrap());
                assert_eq!(
                    base64::engine::general_purpose::STANDARD.encode(&tx.payload),
                    expected["payload"].as_str().unwrap()
                );
            }
        }

        let (reference, json): (NamespaceProofQueryData, _) =
            load_vector("../../../data/v3/ns_proof_V0.json");
        assert_same_fields("NamespaceProofResponse", &json);
        let expected = &json["proof"]["V0"];
        assert_same_fields("AdvzNsProof", expected);
        let converted = proto::NamespaceProofResponse::try_from(&reference).unwrap();
        check_transactions(&converted, &json);
        let Some(Proof::V0(advz)) = converted.proof.unwrap().proof else {
            panic!("the ADVZ vector must select the v0 arm");
        };
        assert_eq!(advz.ns_index, json_bytes(&expected["ns_index"]));
        assert_eq!(
            b64.encode(&advz.ns_payload),
            expected["ns_payload"].as_str().unwrap()
        );
        let range_proof = advz.ns_proof.unwrap();
        let expected_proof = &expected["ns_proof"];
        assert_same_fields("LargeRangeProof", expected_proof);
        assert_eq!(range_proof.prefix_elems, expected_proof["prefix_elems"]);
        assert_eq!(range_proof.suffix_elems, expected_proof["suffix_elems"]);
        assert_eq!(
            range_proof.prefix_bytes,
            json_bytes(&expected_proof["prefix_bytes"])
        );
        assert_eq!(
            range_proof.suffix_bytes,
            json_bytes(&expected_proof["suffix_bytes"])
        );

        for (path, arm) in [
            ("../../../data/v4/ns_proof_V1.json", "V1"),
            ("../../../data/v6/ns_proof_V2.json", "V2"),
        ] {
            let (reference, json): (NamespaceProofQueryData, _) = load_vector(path);
            let expected = &json["proof"][arm];
            assert_same_fields("NsProofPayload", expected);
            let converted = proto::NamespaceProofResponse::try_from(&reference).unwrap();
            check_transactions(&converted, &json);
            let payload = match (arm, converted.proof.unwrap().proof) {
                ("V1", Some(Proof::V1(payload))) | ("V2", Some(Proof::V2(payload))) => payload,
                (arm, other) => panic!("the {arm} vector selected the wrong arm: {other:?}"),
            };
            assert_eq!(payload.ns_index, expected["ns_index"].as_u64().unwrap());
            assert_eq!(
                b64.encode(&payload.ns_payload),
                expected["ns_payload"].as_str().unwrap()
            );
            assert_eq!(payload.ns_proof, expected["ns_proof"]);
        }
    }

    /// No test can build this proof, since it needs a malicious dispersal and the vid crate keeps
    /// the items for one private. Deserializing the JSON v1 would serve pins the two field names
    /// the conversion reads, so an upstream rename fails here rather than as a 500.
    #[test]
    fn bad_encoding_namespace_proof_mirrors_its_v1_rendering() {
        // ark-serialize writes a `Vec` as a little-endian u64 length followed by its elements, so
        // eight zero bytes is the empty vector both fields hold here.
        let empty = tagged_base64::TaggedBase64::new("FIELD", &0u64.to_le_bytes())
            .unwrap()
            .to_string();
        let commitment = reference_header("v3").1["payload_commitment"]
            .as_str()
            .unwrap()
            .to_string();
        let merkle_proof: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/ns_proof_V1.json").unwrap(),
        )
        .unwrap();
        let merkle_proof = merkle_proof["proof"]["V1"]["ns_proof"].as_str().unwrap();

        let json = serde_json::json!({
            "ns_index": 1,
            "ns_commit": commitment,
            "ns_mt_proof": merkle_proof,
            "ns_proof": { "recovered_poly": empty, "raw_shares": empty },
        });
        let reference: espresso_types::v0_3::AvidMIncorrectEncodingNsProof =
            serde_json::from_value(json.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&reference).unwrap(),
            json,
            "v1 no longer renders the proof the way this test claims"
        );

        let converted = proto::AvidmBadEncodingNsProof::try_from(&reference).unwrap();
        assert_eq!(converted.ns_index, 1);
        assert_eq!(converted.ns_commit, commitment);
        assert_eq!(converted.ns_mt_proof, merkle_proof);
        let inner = converted.ns_proof.unwrap();
        assert_eq!(inner.recovered_poly, empty);
        assert_eq!(inner.raw_shares, empty);
    }

    /// Neither vector carries signatures, so the signature mapping is pinned only by its types.
    #[test]
    fn state_certs_mirror_the_reference_vectors() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/state_cert.json").unwrap(),
        )
        .unwrap();
        let reference: espresso_types::v0_3::StateCertQueryDataV1<SeqTypes> =
            serde_json::from_value(json.clone()).unwrap();
        assert_same_fields("StateCertV1Response", &json);
        let converted = proto::StateCertV1Response::from(&reference);
        assert_eq!(converted.epoch, json["epoch"].as_u64().unwrap());
        assert_eq!(converted.light_client_state, json["light_client_state"]);
        assert_eq!(
            converted.next_stake_table_state,
            json["next_stake_table_state"]
        );
        assert_eq!(
            converted.signatures.len(),
            json["signatures"].as_array().unwrap().len()
        );

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v4/state_cert.json").unwrap(),
        )
        .unwrap();
        let reference: espresso_types::v0_4::StateCertQueryDataV2<SeqTypes> =
            serde_json::from_value(json.clone()).unwrap();
        assert_same_fields("StateCertV2Response", &json);
        let converted = proto::StateCertV2Response::from(&reference);
        assert_eq!(converted.epoch, json["epoch"].as_u64().unwrap());
        assert_eq!(converted.light_client_state, json["light_client_state"]);
        assert_eq!(converted.auth_root, json["auth_root"]);
        assert_eq!(
            converted.signatures.len(),
            json["signatures"].as_array().unwrap().len()
        );
    }

    /// Every proof in the vector is AvidM. The ADVZ arm is covered by
    /// `advz_transaction_proof_mirrors_its_v1_rendering`.
    #[test]
    fn transaction_with_proof_mirrors_the_reference_vector() {
        use base64::Engine as _;
        use proto::tx_proof::Proof;

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/transaction_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: Vec<TransactionWithProofQueryData<SeqTypes>> =
            serde_json::from_value(json.clone()).unwrap();
        let first = &json[0];
        assert_same_fields("TransactionWithProofResponse", first);

        let converted = proto::TransactionWithProofResponse::try_from(&reference[0]).unwrap();
        assert_eq!(converted.hash, first["hash"]);
        assert_eq!(converted.index, first["index"].as_u64().unwrap());
        assert_eq!(converted.block_hash, first["block_hash"]);
        assert_eq!(
            converted.block_height,
            first["block_height"].as_u64().unwrap()
        );
        assert_eq!(converted.namespace, first["namespace"].as_u64().unwrap());
        assert_eq!(
            u64::from(converted.pos_in_namespace),
            first["pos_in_namespace"].as_u64().unwrap()
        );
        let transaction = converted.transaction.unwrap();
        assert_eq!(
            transaction.namespace,
            first["transaction"]["namespace"].as_u64().unwrap()
        );
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&transaction.payload),
            first["transaction"]["payload"].as_str().unwrap()
        );

        let Some(Proof::V1(proof)) = converted.proof.unwrap().proof else {
            panic!("the AvidM vector must select the v1 proof arm");
        };
        let expected = &first["proof"]["V1"];
        assert_same_fields("AvidmTxProof", expected);
        assert_eq!(proof.tx_index, json_bytes(&expected["tx_index"]));
        let ns_proof = proof.ns_proof.unwrap();
        let expected_ns = &expected["ns_proof"];
        assert_same_fields("NsProofPayload", expected_ns);
        assert_eq!(ns_proof.ns_index, expected_ns["ns_index"].as_u64().unwrap());
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&ns_proof.ns_payload),
            expected_ns["ns_payload"].as_str().unwrap()
        );
        assert_eq!(ns_proof.ns_proof, expected_ns["ns_proof"]);
    }

    #[test]
    fn vid_common_mirrors_the_reference_vectors() {
        use proto::vid_common_response::Common;

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v1/vid_common_v0.json");
        assert_same_fields("VidCommonResponse", &json);
        assert_same_fields("AdvzCommon", &json["common"]["V0"]);
        let converted = proto::VidCommonResponse::try_from(&reference).unwrap();
        assert_eq!(converted.height, json["height"].as_u64().unwrap());
        assert_eq!(converted.block_hash, json["block_hash"]);
        assert_eq!(converted.payload_hash, json["payload_hash"]);
        let Some(Common::V0(advz)) = converted.common else {
            panic!("the ADVZ vector must select the v0 arm");
        };
        let expected = &json["common"]["V0"];
        assert_eq!(advz.poly_commits, expected["poly_commits"]);
        assert_eq!(advz.all_evals_digest, expected["all_evals_digest"]);
        assert_eq!(
            u64::from(advz.payload_byte_len),
            expected["payload_byte_len"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(advz.num_storage_nodes),
            expected["num_storage_nodes"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(advz.multiplicity),
            expected["multiplicity"].as_u64().unwrap()
        );

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v1/vid_common_v1.json");
        assert_same_fields("AvidmCommon", &json["common"]["V1"]);
        let Some(Common::V1(avidm)) = proto::VidCommonResponse::try_from(&reference)
            .unwrap()
            .common
        else {
            panic!("the AvidM vector must select the v1 arm");
        };
        let expected = &json["common"]["V1"];
        assert_eq!(
            avidm.total_weights,
            expected["total_weights"].as_u64().unwrap()
        );
        assert_eq!(
            avidm.recovery_threshold,
            expected["recovery_threshold"].as_u64().unwrap()
        );

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v2/vid_common_v2.json");
        assert_same_fields("AvidmGf2Common", &json["common"]["V2"]);
        let Some(Common::V2(gf2)) = proto::VidCommonResponse::try_from(&reference)
            .unwrap()
            .common
        else {
            panic!("the AvidmGf2 vector must select the v2 arm");
        };
        let expected = &json["common"]["V2"];
        let param = gf2.param.unwrap();
        assert_eq!(
            param.total_weights,
            expected["param"]["total_weights"].as_u64().unwrap()
        );
        assert_eq!(
            param.recovery_threshold,
            expected["param"]["recovery_threshold"].as_u64().unwrap()
        );
        let expected_commits: Vec<&str> = expected["ns_commits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|commit| commit.as_str().unwrap())
            .collect();
        assert_eq!(gf2.ns_commits, expected_commits);
        let expected_lens: Vec<u64> = expected["ns_lens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|len| len.as_u64().unwrap())
            .collect();
        assert_eq!(gf2.ns_lens, expected_lens);
    }

    /// The payload bytes and namespace table are compared through base64, which is how v1 renders
    /// them and how protoJSON renders `bytes`.
    #[test]
    fn block_and_payload_mirror_the_reference_vectors() {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD;

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/block_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: BlockQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let block = proto::BlockResponse::from(&reference);
        assert_same_fields("BlockResponse", &json);
        assert_eq!(block.hash, json["hash"]);
        assert_eq!(block.size, json["size"].as_u64().unwrap());
        assert_eq!(
            block.num_transactions,
            json["num_transactions"].as_u64().unwrap()
        );
        let Some(proto::header_response::Header::V1(header)) = block.header.unwrap().header else {
            panic!("an unwrapped 0.1-shaped header must convert to the V1 arm");
        };
        assert_eq!(header.height, json["header"]["height"].as_u64().unwrap());
        let payload = block.payload.unwrap();
        assert_eq!(
            b64.encode(&payload.raw_payload),
            json["payload"]["raw_payload"].as_str().unwrap()
        );
        assert_eq!(
            b64.encode(&payload.ns_table.unwrap().bytes),
            json["payload"]["ns_table"]["bytes"].as_str().unwrap()
        );

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/payload_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: PayloadQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let payload = proto::PayloadResponse::from(&reference);
        assert_same_fields("PayloadResponse", &json);
        assert_eq!(payload.height, json["height"].as_u64().unwrap());
        assert_eq!(payload.block_hash, json["block_hash"]);
        assert_eq!(payload.hash, json["hash"]);
        assert_eq!(payload.size, json["size"].as_u64().unwrap());
        let data = payload.data.unwrap();
        assert_eq!(
            b64.encode(&data.raw_payload),
            json["data"]["raw_payload"].as_str().unwrap()
        );
        assert_eq!(
            b64.encode(&data.ns_table.unwrap().bytes),
            json["data"]["ns_table"]["bytes"].as_str().unwrap()
        );
    }

    /// The per-namespace map is the one summary field that comes from the payload, so no header
    /// vector covers it.
    #[test]
    fn block_summary_mirrors_its_v1_rendering() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/block_query_data.json").unwrap(),
        )
        .unwrap();
        let block: BlockQueryData<SeqTypes> = serde_json::from_value(json).unwrap();
        let summary = BlockSummaryQueryData::from(block);
        let expected = serde_json::to_value(&summary).unwrap();
        let converted = proto::BlockSummaryResponse::from(&summary);

        assert_same_fields("BlockSummaryResponse", &expected);
        assert_eq!(converted.hash, expected["hash"]);
        assert_eq!(converted.size, expected["size"].as_u64().unwrap());
        assert_eq!(
            converted.num_transactions,
            expected["num_transactions"].as_u64().unwrap()
        );

        // protoJSON writes a map key as a string whatever its proto type, which is also what
        // serde_json does with v1's `NamespaceId` keys.
        let expected_namespaces = expected["namespaces"].as_object().unwrap();
        assert!(
            expected_namespaces.len() > 1,
            "the vector should span several namespaces"
        );
        assert_eq!(converted.namespaces.len(), expected_namespaces.len());
        let mut counted = 0;
        for (namespace, expected_info) in expected_namespaces {
            let info = &converted.namespaces[&namespace.parse::<u64>().unwrap()];
            assert_eq!(
                info.num_transactions,
                expected_info["num_transactions"].as_u64().unwrap()
            );
            assert_eq!(info.size, expected_info["size"].as_u64().unwrap());
            counted += info.num_transactions;
        }
        assert_eq!(counted, converted.num_transactions);
    }

    #[test]
    fn leaf_mirrors_the_reference_vector() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/leaf_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: LeafQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let converted = proto::LeafResponse::from(&reference);

        assert_same_fields("Leaf2", &json["leaf"]);
        assert_same_fields("QuorumCertificate2", &json["qc"]);

        let leaf = converted.leaf.unwrap();
        let expected = &json["leaf"];
        assert_eq!(leaf.view_number, expected["view_number"].as_u64().unwrap());
        assert_eq!(leaf.parent_commitment, expected["parent_commitment"]);
        assert_eq!(leaf.with_epoch, expected["with_epoch"].as_bool().unwrap());
        assert!(leaf.next_epoch_justify_qc.is_none());
        assert!(leaf.upgrade_certificate.is_none());
        assert!(leaf.view_change_evidence.is_none());
        assert!(leaf.next_drb_result.is_empty());

        // The v3-era vector embeds a 0.1-shaped header: twelve flat fields, no version wrapper,
        // no reward root. Header versions and leaf versions moved independently.
        let header_json = &expected["block_header"];
        assert!(
            header_json.get("fields").is_none(),
            "vector header grew a version wrapper; update the expected arm"
        );
        let Some(proto::header_response::Header::V1(header)) = leaf.block_header.unwrap().header
        else {
            panic!("an unwrapped 0.1-shaped header must convert to the V1 arm");
        };
        assert_eq!(header.height, header_json["height"].as_u64().unwrap());

        // `LeafQueryData` deserializes through `new`, which drops the payload the JSON carries,
        // so the conversion must report none.
        assert!(leaf.block_payload.is_none());

        let justify = leaf.justify_qc.unwrap();
        let expected_justify = &expected["justify_qc"];
        assert_eq!(
            justify.view_number,
            expected_justify["view_number"].as_u64().unwrap()
        );
        assert_eq!(justify.vote_commitment, expected_justify["vote_commitment"]);
        assert_eq!(
            justify.data.as_ref().unwrap().leaf_commit,
            expected_justify["data"]["leaf_commit"]
        );

        let qc = converted.qc.unwrap();
        let expected_qc = &json["qc"];
        assert_eq!(qc.view_number, expected_qc["view_number"].as_u64().unwrap());
        assert_eq!(qc.vote_commitment, expected_qc["vote_commitment"]);
        let data = qc.data.unwrap();
        assert_eq!(data.leaf_commit, expected_qc["data"]["leaf_commit"]);
        assert_eq!(data.epoch, expected_qc["data"]["epoch"].as_u64());
        assert_eq!(
            data.block_number,
            expected_qc["data"]["block_number"].as_u64()
        );
        // The vector's certificates are unsigned, so absence must map to absence.
        assert!(qc.signatures.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn advz_transaction_proof_mirrors_its_v1_rendering() {
        use hotshot_query_service::availability::QueryablePayload;
        use hotshot_types::{
            data::VidCommon,
            traits::{EncodeBytes, block_contents::BlockPayload},
            vid::advz::advz_scheme,
        };
        use jf_advz::VidScheme;
        use proto::tx_proof::Proof;

        // No reference vector carries an ADVZ transaction proof, so build one the way a V0 node
        // did and compare against v1's JSON. The accessor-backed fields are the real check. The
        // range proofs are read from that same JSON, so their loop only pins that the keys exist
        // and that an absent proof stays absent.
        let namespace = espresso_types::NamespaceId::from(7_u32);
        let transactions: Vec<_> = (1_u8..=3)
            .map(|byte| espresso_types::Transaction::new(namespace, vec![byte; 8]))
            .collect();
        let (payload, _) = espresso_types::Payload::from_transactions(
            transactions,
            &Default::default(),
            &Default::default(),
        )
        .await
        .unwrap();
        let common = VidCommon::V0(advz_scheme(10).disperse(payload.encode()).unwrap().common);
        let index = payload.iter(payload.ns_table()).next().unwrap();
        let (_, proof) = espresso_types::TxProof::new(&index, &payload, &common).unwrap();
        let rendered = serde_json::to_value(&proof).unwrap();
        let rendered = &rendered["V0"];

        let Some(Proof::V0(converted)) = proto::TxProof::try_from(&proof).unwrap().proof else {
            panic!("an ADVZ common yields the V0 arm");
        };
        assert_eq!(converted.tx_index, json_bytes(&rendered["tx_index"]));
        assert_eq!(
            converted.payload_num_txs,
            json_bytes(&rendered["payload_num_txs"])
        );
        assert_eq!(
            converted.payload_tx_table_entries,
            json_bytes(&rendered["payload_tx_table_entries"])
        );
        for (proof, key) in [
            (converted.payload_proof_num_txs, "payload_proof_num_txs"),
            (
                converted.payload_proof_tx_table_entries,
                "payload_proof_tx_table_entries",
            ),
            (converted.payload_proof_tx, "payload_proof_tx"),
        ] {
            let Some(proof) = proof else {
                assert!(
                    rendered[key].is_null(),
                    "{key} is present in v1's rendering"
                );
                continue;
            };
            assert_eq!(
                proof.proofs,
                rendered[key]["proofs"].as_str().unwrap(),
                "{key}"
            );
            assert_eq!(
                proof.prefix_bytes,
                json_bytes(&rendered[key]["prefix_bytes"]),
                "{key}"
            );
            assert_eq!(
                proof.suffix_bytes,
                json_bytes(&rendered[key]["suffix_bytes"]),
                "{key}"
            );
        }
    }
}
