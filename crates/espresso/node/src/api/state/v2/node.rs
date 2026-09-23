//! `NodeService`.

use super::*;

#[tonic::async_trait]
impl<D> proto::node_service_server::NodeService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::node::NodeDataSource<SeqTypes>
        + StakeTableDataSource<SeqTypes>
        + PruningDataSource
        + Send
        + Sync,
{
    async fn get_transaction_count(
        &self,
        request: tonic::Request<proto::GetTransactionCountRequest>,
    ) -> Result<tonic::Response<proto::TransactionCountResponse>, tonic::Status> {
        let proto::GetTransactionCountRequest {
            from,
            to,
            namespace,
        } = request.into_inner();
        let count = <Self as v1::NodeApi>::count_transactions(self, from, to, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TransactionCountResponse {
            count,
        }))
    }

    async fn get_payload_size(
        &self,
        request: tonic::Request<proto::GetPayloadSizeRequest>,
    ) -> Result<tonic::Response<proto::PayloadSizeResponse>, tonic::Status> {
        let proto::GetPayloadSizeRequest {
            from,
            to,
            namespace,
        } = request.into_inner();
        let size = <Self as v1::NodeApi>::payload_size(self, from, to, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadSizeResponse { size }))
    }

    async fn get_sync_status(
        &self,
        _request: tonic::Request<proto::GetSyncStatusRequest>,
    ) -> Result<tonic::Response<proto::SyncStatusResponse>, tonic::Status> {
        let status = <Self as v1::NodeApi>::sync_status(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::SyncStatusResponse::from(
            status,
        )))
    }

    async fn get_block_reward(
        &self,
        request: tonic::Request<proto::GetBlockRewardRequest>,
    ) -> Result<tonic::Response<proto::BlockRewardResponse>, tonic::Status> {
        let reward = <Self as v1::NodeApi>::get_block_reward(self, request.into_inner().epoch)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRewardResponse {
            amount: reward.map(|amount| amount.to_string()),
        }))
    }

    async fn get_vid_share(
        &self,
        request: tonic::Request<proto::GetVidShareRequest>,
    ) -> Result<tonic::Response<proto::VidShareResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.payload_hash) {
            (Some(height), None, None) => v1::VidShareId::Height(height),
            (None, Some(hash), None) => v1::VidShareId::Hash(hash),
            (None, None, Some(hash)) => v1::VidShareId::PayloadHash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or payload_hash",
                ));
            },
        };
        let share = <Self as v1::NodeApi>::get_vid_share(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::VidShareResponse::try_from(
            &share,
        )?))
    }

    async fn get_header_window(
        &self,
        request: tonic::Request<proto::GetHeaderWindowRequest>,
    ) -> Result<tonic::Response<proto::HeaderWindowResponse>, tonic::Status> {
        let request = request.into_inner();
        let start = match (request.start_time, request.start_height, request.start_hash) {
            (Some(time), None, None) => v1::HeaderWindowStart::Time(time),
            (None, Some(height), None) => v1::HeaderWindowStart::Height(height),
            (None, None, Some(hash)) => v1::HeaderWindowStart::Hash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of start_time, start_height or start_hash",
                ));
            },
        };
        let end = required(request.end, "end")?;
        let window = <Self as v1::NodeApi>::get_header_window(self, start, end)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderWindowResponse::from(
            &window,
        )))
    }

    async fn get_node_block_height(
        &self,
        _request: tonic::Request<proto::GetNodeBlockHeightRequest>,
    ) -> Result<tonic::Response<proto::NodeBlockHeightResponse>, tonic::Status> {
        let height = <Self as v1::NodeApi>::block_height(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeBlockHeightResponse {
            height,
        }))
    }

    async fn get_node_limits(
        &self,
        _request: tonic::Request<proto::GetNodeLimitsRequest>,
    ) -> Result<tonic::Response<proto::NodeLimitsResponse>, tonic::Status> {
        let limits = <Self as v1::NodeApi>::limits(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeLimitsResponse::from(
            limits,
        )))
    }

    async fn get_stake_table(
        &self,
        request: tonic::Request<proto::GetStakeTableRequest>,
    ) -> Result<tonic::Response<proto::StakeTableResponse>, tonic::Status> {
        let table = match request.into_inner().epoch {
            Some(epoch) => StakeTableWithEpochNumber {
                epoch: Some(EpochNumber::new(epoch)),
                stake_table: <Self as v1::NodeApi>::stake_table(self, epoch)
                    .await
                    .map_err(to_status)?,
            },
            None => <Self as v1::NodeApi>::stake_table_current(self)
                .await
                .map_err(to_status)?,
        };
        Ok(tonic::Response::new(table.into()))
    }

    async fn get_validators(
        &self,
        request: tonic::Request<proto::GetValidatorsRequest>,
    ) -> Result<tonic::Response<proto::ValidatorsResponse>, tonic::Status> {
        let epoch = required(request.into_inner().epoch, "epoch")?;
        let validators = <Self as v1::NodeApi>::get_validators(self, epoch)
            .await
            .map_err(to_status)?;
        let mut validators: Vec<proto::Validator> = validators
            .into_values()
            .map(|authenticated| authenticated.into_inner().into())
            .collect();
        // v1 serves a map, so the order is its own; the paged route reports account order.
        validators.sort_by(|a, b| a.account.cmp(&b.account));
        Ok(tonic::Response::new(proto::ValidatorsResponse {
            validators,
        }))
    }

    async fn get_all_validators(
        &self,
        request: tonic::Request<proto::GetAllValidatorsRequest>,
    ) -> Result<tonic::Response<proto::ValidatorsResponse>, tonic::Status> {
        let request = request.into_inner();
        let validators = <Self as v1::NodeApi>::get_all_validators(
            self,
            required(request.epoch, "epoch")?,
            required(request.offset, "offset")?,
            required(request.limit, "limit")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::ValidatorsResponse {
            validators: validators.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_proposal_participation(
        &self,
        request: tonic::Request<proto::GetProposalParticipationRequest>,
    ) -> Result<tonic::Response<proto::ParticipationResponse>, tonic::Status> {
        let fractions = match request.into_inner().epoch {
            Some(epoch) => <Self as v1::NodeApi>::proposal_participation(self, epoch).await,
            None => <Self as v1::NodeApi>::current_proposal_participation(self).await,
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(fractions.into()))
    }

    async fn get_vote_participation(
        &self,
        request: tonic::Request<proto::GetVoteParticipationRequest>,
    ) -> Result<tonic::Response<proto::ParticipationResponse>, tonic::Status> {
        let fractions = match request.into_inner().epoch {
            Some(epoch) => <Self as v1::NodeApi>::vote_participation(self, epoch).await,
            None => <Self as v1::NodeApi>::current_vote_participation(self).await,
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(fractions.into()))
    }
}

// Stays here rather than in the api crate's `render`: the source type is this crate's, and the
// api crate cannot depend on this one.
impl From<StakeTableWithEpochNumber<SeqTypes>> for proto::StakeTableResponse {
    fn from(table: StakeTableWithEpochNumber<SeqTypes>) -> Self {
        Self {
            epoch: table.epoch.map(|epoch| *epoch),
            stake_table: table.stake_table.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};

    use alloy::primitives::{Address, U256};
    use espresso_types::{PubKey, v0_3::RegisteredValidator};
    use hotshot_query_service::node::{ResourceSyncStatus, SyncStatus, SyncStatusRange};
    use hotshot_types::{
        addr::NetAddr,
        vid::{
            advz::advz_scheme,
            avidm::{AvidMScheme, init_avidm_param},
            avidm_gf2::{AvidmGf2Scheme, init_avidm_gf2_param},
        },
        x25519,
    };
    use jf_advz::VidScheme as _;
    use proto::vid_share_response::Share;

    use super::*;

    // A test network only disperses with ADVZ, so the AvidM arms run only here, against the
    // namespaced wrappers the node stores rather than the inner per-namespace shares.
    #[test]
    fn every_vid_share_arm_maps_to_its_own_shape() {
        let payload = b"two namespaces worth of payload bytes, dispersed";
        let weights = [1u32, 1, 1];
        let ns_table = vec![0..24usize, 24..payload.len()];

        let mut advz = advz_scheme(3);
        let share = VidShare::V0(advz.disperse(payload).unwrap().shares.remove(0));
        let Share::V0(advz) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V0 arm");
        };
        assert!(advz.aggregate_proofs.starts_with("FIELD~"));
        assert!(!advz.evals_proof.unwrap().proof.is_empty());

        let param = init_avidm_param(3).unwrap();
        let (_, mut shares) =
            AvidMScheme::ns_disperse(&param, &weights, payload, ns_table.clone()).unwrap();
        let share = VidShare::V1(shares.remove(0));
        let Share::V1(avidm) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V1 arm");
        };
        assert_eq!(avidm.ns_lens, [24, 24]);
        assert_eq!(avidm.ns_commits.len(), 2);
        assert!(avidm.ns_commits[0].starts_with("AvidMCommit~"));
        assert_eq!(avidm.content.len(), 2);
        assert!(avidm.content[0].payload.starts_with("FIELD~"));

        let param = init_avidm_gf2_param(3).unwrap();
        let (_, _, mut shares) =
            AvidmGf2Scheme::ns_disperse(&param, &weights, payload, ns_table).unwrap();
        let share = VidShare::V2(shares.remove(0));
        let Share::V2(gf2) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V2 arm");
        };
        assert_eq!(gf2.namespaces.len(), 2);
        assert!(!gf2.namespaces[0].payload.is_empty());
        assert!(gf2.namespaces[0].mt_proofs[0].starts_with("MERKLE_PROOF~"));
    }

    // No test network registers a validator, so this mapping is only exercised here.
    #[test]
    fn validator_maps_hex_quantities_and_sorts_delegators() {
        let delegator = |byte: u8| Address::from([byte; 20]);
        let key = x25519::Keypair::generated_from_seed_indexed([3; 32], 0)
            .unwrap()
            .public_key();
        let stake = U256::from(1_000_000_000_000_000_000u64);
        let p2p_addr = NetAddr::Inet(IpAddr::V6(Ipv6Addr::LOCALHOST), 9977);
        let registered = RegisteredValidator::<PubKey> {
            account: delegator(0xab),
            stake_table_key: None,
            state_ver_key: None,
            stake,
            commission: 1234,
            delegators: HashMap::from([
                (delegator(0xff), U256::from(10)),
                (delegator(0x01), U256::from(255)),
            ]),
            authenticated: true,
            x25519_key: Some(key),
            p2p_addr: Some(p2p_addr.clone()),
        };

        let proto = proto::Validator::from(registered);

        // serde renders this key in x25519's own base58, so the tagged form is worth pinning.
        let x25519_key = proto.x25519_key.as_deref().unwrap();
        assert_eq!(x25519_key.parse::<x25519::PublicKey>().unwrap(), key);
        assert!(x25519_key.starts_with("X25519_PK~"));
        assert_ne!(
            Some(x25519_key),
            serde_json::to_value(key).unwrap().as_str()
        );
        // v1 serializes the pre-bracketing form, so `to_string` would give `[::1]:9977`.
        assert_eq!(
            proto.p2p_addr.as_deref(),
            serde_json::to_value(&p2p_addr).unwrap().as_str()
        );
        assert_ne!(
            proto.p2p_addr.as_deref(),
            Some(p2p_addr.to_string().as_str())
        );

        assert_eq!(proto.account, "0xabababababababababababababababababababab");
        // v1 serializes a U256 as a hex quantity, where `to_string` would give it in decimal.
        assert_eq!(
            proto.stake,
            serde_json::to_value(stake).unwrap().as_str().unwrap()
        );
        assert_eq!(proto.commission, 1234);
        assert!(proto.authenticated);
        assert_eq!(proto.stake_table_key, None);
        assert_eq!(proto.state_ver_key, None);
        assert_eq!(
            proto
                .delegators
                .iter()
                .map(|delegator| (delegator.account.as_str(), delegator.amount.as_str()))
                .collect::<Vec<_>>(),
            [
                ("0x0101010101010101010101010101010101010101", "0xff"),
                ("0xffffffffffffffffffffffffffffffffffffffff", "0xa"),
            ]
        );
    }

    // `test_node_api_v2_agrees_with_v1` compares a fresh node's sync status, which the query
    // service caches at startup with no ranges, so this match is only exercised here.
    #[test]
    fn sync_status_ranges_keep_their_bounds_and_status() {
        let converted = proto::ResourceSyncStatus::from(ResourceSyncStatus {
            missing: 7,
            ranges: vec![
                SyncStatusRange {
                    start: 0,
                    end: 3,
                    status: SyncStatus::Pruned,
                },
                SyncStatusRange {
                    start: 3,
                    end: 5,
                    status: SyncStatus::Present,
                },
                SyncStatusRange {
                    start: 5,
                    end: 12,
                    status: SyncStatus::Missing,
                },
            ],
        });

        assert_eq!(converted.missing, 7);
        let ranges: Vec<_> = converted
            .ranges
            .iter()
            .map(|range| (range.start, range.end, range.status()))
            .collect();
        assert_eq!(
            ranges,
            [
                (0, 3, proto::SyncStatus::Pruned),
                (3, 5, proto::SyncStatus::Present),
                (5, 12, proto::SyncStatus::Missing),
            ]
        );
    }
}
