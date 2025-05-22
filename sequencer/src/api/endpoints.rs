//! Sequencer-specific API endpoint handlers.

use std::{
    collections::{BTreeSet, HashMap},
    env,
    time::Duration,
};

use anyhow::Result;
use committable::Committable;
use espresso_types::{
    v0_1::{ADVZNsProof, RewardAccount, RewardMerkleTree},
    FeeAccount, FeeMerkleTree, NamespaceId, NsProof, PubKey, Transaction,
};
// re-exported here to avoid breaking changes in consumers
// "deprecated" does not work with "pub use": https://github.com/rust-lang/rust/issues/30827
#[deprecated(note = "use espresso_types::ADVZNamespaceProofQueryData")]
pub type ADVZNamespaceProofQueryData = espresso_types::ADVZNamespaceProofQueryData;
#[deprecated(note = "use espresso_types::NamespaceProofQueryData")]
pub type NamespaceProofQueryData = espresso_types::NamespaceProofQueryData;

use futures::{try_join, FutureExt};
use hotshot_query_service::{
    availability::{self, AvailabilityDataSource, CustomSnafu, FetchBlockSnafu},
    explorer::{self, ExplorerDataSource},
    merklized_state::{
        self, MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
    },
    node::{self, NodeDataSource},
    ApiState, Error, VidCommon,
};
use hotshot_types::{
    data::{EpochNumber, VidCommitment, VidShare, ViewNumber},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
    },
    vid::avidm::AvidMShare,
};
use jf_merkle_tree::MerkleTreeScheme;
use serde::de::Error as _;
use snafu::OptionExt;
use tagged_base64::TaggedBase64;
use tide_disco::{method::ReadState, Api, Error as _, StatusCode};
use tracing::warn;
use vbs::version::{StaticVersion, StaticVersionType};
use vid::avid_m::namespaced::NsAvidMScheme;

use super::{
    data_source::{
        CatchupDataSource, HotShotConfigDataSource, NodeStateDataSource, RequestResponseDataSource,
        SequencerDataSource, StakeTableDataSource, StateSignatureDataSource, SubmitDataSource,
    },
    StorageState,
};
use crate::{SeqTypes, SequencerApiVersion, SequencerPersistence};

pub(super) fn fee<State, Ver>(
    api_ver: semver::Version,
) -> Result<Api<State, merklized_state::Error, Ver>>
where
    State: 'static + Send + Sync + ReadState,
    Ver: 'static + StaticVersionType,
    <State as ReadState>::State: Send
        + Sync
        + MerklizedStateDataSource<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence,
{
    let mut options = merklized_state::Options::default();
    let extension = toml::from_str(include_str!("../../api/fee.toml"))?;
    options.extensions.push(extension);

    let mut api =
        merklized_state::define_api::<State, SeqTypes, FeeMerkleTree, Ver, 256>(&options, api_ver)?;

    api.get("getfeebalance", move |req, state| {
        async move {
            let address = req.string_param("address")?;
            let height = state.get_last_state_height().await?;
            let snapshot = Snapshot::Index(height as u64);
            let key = address
                .parse()
                .map_err(|_| merklized_state::Error::Custom {
                    message: "failed to parse address".to_string(),
                    status: StatusCode::BAD_REQUEST,
                })?;
            let path = state.get_path(snapshot, key).await?;
            Ok(path.elem().copied())
        }
        .boxed()
    })?;
    Ok(api)
}

pub(super) fn reward<State, Ver>(
    api_ver: semver::Version,
) -> Result<Api<State, merklized_state::Error, Ver>>
where
    State: 'static + Send + Sync + ReadState,
    Ver: 'static + StaticVersionType,
    <State as ReadState>::State: Send
        + Sync
        + MerklizedStateDataSource<SeqTypes, RewardMerkleTree, { RewardMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence,
{
    let mut options = merklized_state::Options::default();
    let extension = toml::from_str(include_str!("../../api/reward.toml"))?;
    options.extensions.push(extension);

    let mut api = merklized_state::define_api::<
        State,
        SeqTypes,
        RewardMerkleTree,
        Ver,
        { RewardMerkleTree::ARITY },
    >(&options, api_ver)?;

    api.get("get_latest_reward_balance", move |req, state| {
        async move {
            let address = req.string_param("address")?;
            let height = state.get_last_state_height().await?;
            let snapshot = Snapshot::Index(height as u64);
            let key = address
                .parse()
                .map_err(|_| merklized_state::Error::Custom {
                    message: "failed to parse reward address".to_string(),
                    status: StatusCode::BAD_REQUEST,
                })?;
            let path = state.get_path(snapshot, key).await?;
            Ok(path.elem().copied())
        }
        .boxed()
    })?
    .get("get_reward_balance", move |req, state| {
        async move {
            let address = req.string_param("address")?;
            let height: usize = req.integer_param("height")?;
            let snapshot = Snapshot::Index(height as u64);
            let key = address
                .parse()
                .map_err(|_| merklized_state::Error::Custom {
                    message: "failed to parse reward address".to_string(),
                    status: StatusCode::BAD_REQUEST,
                })?;
            let path = state.get_path(snapshot, key).await?;
            Ok(path.elem().copied())
        }
        .boxed()
    })?;
    Ok(api)
}

pub(super) type AvailState<N, P, D, ApiVer> = ApiState<StorageState<N, P, D, ApiVer>>;

type AvailabilityApi<N, P, D, V, ApiVer> = Api<AvailState<N, P, D, V>, availability::Error, ApiVer>;

// TODO (abdul): replace snafu with `this_error` in  hotshot query service
// Snafu has been replaced by `this_error` everywhere.
// However, the query service still uses snafu
pub(super) fn availability<N, P, D, V: Versions>(
    api_ver: semver::Version,
) -> Result<AvailabilityApi<N, P, D, V, SequencerApiVersion>>
where
    N: ConnectedNetwork<PubKey>,
    D: SequencerDataSource + Send + Sync + 'static,
    P: SequencerPersistence,
{
    let mut options = availability::Options::default();
    let extension = toml::from_str(include_str!("../../api/availability.toml"))?;
    options.extensions.push(extension);
    let timeout = options.fetch_timeout;

    let mut api = availability::define_api::<AvailState<N, P, D, _>, SeqTypes, _>(
        &options,
        SequencerApiVersion::instance(),
        api_ver.clone(),
    )?;

    if api_ver.major == 1 {
        if api_ver.minor >= 1 {
            // >= V1.1 api returns both correct and incorrect encoding proofs
            api.get("getnamespaceproof", move |req, state| {
                async move {
                    let height: usize = req.integer_param("height")?;
                    let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
                    let (block, common) = try_join!(
                        async move {
                            state
                                .get_block(height)
                                .await
                                .with_timeout(timeout)
                                .await
                                .context(FetchBlockSnafu {
                                    resource: height.to_string(),
                                })
                        },
                        async move {
                            state
                                .get_vid_common(height)
                                .await
                                .with_timeout(timeout)
                                .await
                                .context(FetchBlockSnafu {
                                    resource: height.to_string(),
                                })
                        }
                    )?;

                    let ns_table = block.payload().ns_table();
                    if let Some(ns_index) = ns_table.find_ns_id(&ns_id) {
                        match NsProof::v1_1_new_with_correct_encoding(
                            block.payload(),
                            &ns_index,
                            common.common(),
                        ) {
                            Some(proof) => Ok(espresso_types::NamespaceProofQueryData {
                                transactions: proof.export_all_txs(&ns_id),
                                proof: Some(proof),
                            }),
                            None => {
                                // if we fail to generate the correct encoding proof, we try to generate the incorrect encoding proof
                                tracing::debug!("Failed to generate namespace proof for block {height} and namespace {ns_id}, trying to generate incorrect encoding proof");
                                let mut vid_shares = state
                                    .request_vid_shares(
                                        height as u64,
                                        common.clone(),
                                        Duration::from_secs(40),
                                    )
                                    .await
                                    .map_err(|err| {
                                        warn!("Failed to request VID shares from network: {err:#}");
                                        hotshot_query_service::availability::Error::Custom {
                                            message: "Failed to request VID shares from network"
                                                .to_string(),
                                            status: StatusCode::NOT_FOUND,
                                        }
                                    })?;
                                let vid_share = state.vid_share(height).await;
                                if let Ok(vid_share) = vid_share {
                                    vid_shares.push(vid_share);
                                };

                                // Collect the shares as V1 shares
                                let vid_shares: Vec<AvidMShare> = vid_shares
                                    .into_iter()
                                    .filter_map(|share| {
                                        if let VidShare::V1(share) = share {
                                            Some(share)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();

                                match NsProof::v1_1_new_with_incorrect_encoding(
                                    &vid_shares,
                                    ns_table,
                                    &ns_index,
                                    &common.payload_hash(),
                                    common.common(),
                                ) {
                                    Some(proof) => Ok(espresso_types::NamespaceProofQueryData {
                                        transactions: vec![],
                                        proof: Some(proof),
                                    }),
                                    None => {
                                        warn!("Failed to generate proof of incorrect encoding");
                                        Err(availability::Error::Custom {
                                            message:
                                                "Failed to generate proof of incorrect encoding"
                                                    .to_string(),
                                            status: StatusCode::INTERNAL_SERVER_ERROR,
                                        })
                                    },
                                }
                            },
                        }
                    } else {
                        // ns_id not found in ns_table
                        Err(availability::Error::Custom {
                            message: "Namespace not found".to_string(),
                            status: StatusCode::NOT_FOUND,
                        })
                    }
                }
                .boxed()
            })?
            .at("incorrect_encoding_proof", |req, state| {
                async move {
                    // Get the block number from the request
                    let block_number =
                        req.integer_param::<_, u64>("block_number").map_err(|_| {
                            hotshot_query_service::availability::Error::Custom {
                                message: "Block number is required".to_string(),
                                status: StatusCode::BAD_REQUEST,
                            }
                        })?;

                    // Get or fetch the VID common data for the given block number
                    // TODO: Time this out
                    let vid_common = state
                        .read(|state| state.get_vid_common(block_number as usize).boxed())
                        .await
                        .await;

                    // Request the VID shares from other nodes. Use the VID common and common metadata to
                    // verify that they are correct
                    let vid_common_clone = vid_common.clone();
                    let mut vid_shares = state
                        .read(|state| {
                            state.request_vid_shares(
                                block_number,
                                vid_common_clone,
                                Duration::from_secs(40),
                            )
                        })
                        .await
                        .map_err(|err| {
                            warn!("Failed to request VID shares from network: {err:#}");
                            hotshot_query_service::availability::Error::Custom {
                                message: "Failed to request VID shares from network".to_string(),
                                status: StatusCode::NOT_FOUND,
                            }
                        })?;

                    // Get our own share and add it. We don't need to verify here
                    let vid_share = state
                        .read(|state| state.vid_share(block_number as usize).boxed())
                        .await;
                    if let Ok(vid_share) = vid_share {
                        vid_shares.push(vid_share);
                    };

                    // Get the total VID weight based on the VID common data
                    let avidm_param = match vid_common.common() {
                        VidCommon::V0(_) => {
                            // TODO: This needs to be done via the stake table
                            return Err(hotshot_query_service::availability::Error::Custom {
                                message: "V0 shares not supported yet".to_string(),
                                status: StatusCode::NOT_FOUND,
                            });
                        },
                        VidCommon::V1(v1) => v1,
                    };

                    // Get the payload hash
                    let VidCommitment::V1(local_payload_hash) = vid_common.payload_hash() else {
                        return Err(hotshot_query_service::availability::Error::Custom {
                            message: "V0 shares not supported yet".to_string(),
                            status: StatusCode::NOT_FOUND,
                        });
                    };

                    // Collect the shares as V1 shares
                    let avidm_shares: Vec<AvidMShare> = vid_shares
                        .into_iter()
                        .filter_map(|share| {
                            if let VidShare::V1(share) = share {
                                Some(share)
                            } else {
                                None
                            }
                        })
                        .collect();

                    match NsAvidMScheme::proof_of_incorrect_encoding(
                        avidm_param,
                        &local_payload_hash,
                        &avidm_shares,
                    ) {
                        Ok(proof) => Ok(proof),
                        Err(err) => {
                            warn!("Failed to generate proof of incorrect encoding: {err:#}");
                            Err(hotshot_query_service::availability::Error::Custom {
                                message: "Failed to generate proof of incorrect encoding"
                                    .to_string(),
                                status: StatusCode::INTERNAL_SERVER_ERROR,
                            })
                        },
                    }
                }
                .boxed()
            })?;
        } else {
            // V1.0 api only returns the correct encoding proof
            api.get("getnamespaceproof", move |req, state| {
                async move {
                    let height: usize = req.integer_param("height")?;
                    let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
                    let (block, common) = try_join!(
                        async move {
                            state
                                .get_block(height)
                                .await
                                .with_timeout(timeout)
                                .await
                                .context(FetchBlockSnafu {
                                    resource: height.to_string(),
                                })
                        },
                        async move {
                            state
                                .get_vid_common(height)
                                .await
                                .with_timeout(timeout)
                                .await
                                .context(FetchBlockSnafu {
                                    resource: height.to_string(),
                                })
                        }
                    )?;

                    if let Some(ns_index) = block.payload().ns_table().find_ns_id(&ns_id) {
                        let proof = NsProof::new(block.payload(), &ns_index, common.common())
                            .context(CustomSnafu {
                                message: format!("failed to make proof for namespace {ns_id}"),
                                status: StatusCode::NOT_FOUND,
                            })?;

                        Ok(espresso_types::NamespaceProofQueryData {
                            transactions: proof.export_all_txs(&ns_id),
                            proof: Some(proof),
                        })
                    } else {
                        // ns_id not found in ns_table
                        Ok(espresso_types::NamespaceProofQueryData {
                            proof: None,
                            transactions: Vec::new(),
                        })
                    }
                }
                .boxed()
            })?;
        }
    } else {
        api.get("getnamespaceproof", move |req, state| {
            async move {
                let height: usize = req.integer_param("height")?;
                let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
                let (block, common) = try_join!(
                    async move {
                        state
                            .get_block(height)
                            .await
                            .with_timeout(timeout)
                            .await
                            .context(FetchBlockSnafu {
                                resource: height.to_string(),
                            })
                    },
                    async move {
                        state
                            .get_vid_common(height)
                            .await
                            .with_timeout(timeout)
                            .await
                            .context(FetchBlockSnafu {
                                resource: height.to_string(),
                            })
                    }
                )?;

                if let Some(ns_index) = block.payload().ns_table().find_ns_id(&ns_id) {
                    let VidCommon::V0(common) = &common.common().clone() else {
                        return Err(availability::Error::Custom {
                            message: "Unsupported VID version, use new API version instead."
                                .to_string(),
                            status: StatusCode::NOT_FOUND,
                        });
                    };
                    let proof = ADVZNsProof::new(block.payload(), &ns_index, common).context(
                        CustomSnafu {
                            message: format!("failed to make proof for namespace {ns_id}"),
                            status: StatusCode::NOT_FOUND,
                        },
                    )?;

                    Ok(espresso_types::ADVZNamespaceProofQueryData {
                        transactions: proof.export_all_txs(&ns_id),
                        proof: Some(proof),
                    })
                } else {
                    // ns_id not found in ns_table
                    Ok(espresso_types::ADVZNamespaceProofQueryData {
                        proof: None,
                        transactions: Vec::new(),
                    })
                }
            }
            .boxed()
        })?;
    }

    Ok(api)
}

type ExplorerApi<N, P, D, V, ApiVer> = Api<AvailState<N, P, D, V>, explorer::Error, ApiVer>;

pub(super) fn explorer<N, P, D, V: Versions>(
    api_ver: semver::Version,
) -> Result<ExplorerApi<N, P, D, V, SequencerApiVersion>>
where
    N: ConnectedNetwork<PubKey>,
    D: ExplorerDataSource<SeqTypes> + Send + Sync + 'static,
    P: SequencerPersistence,
{
    let api = explorer::define_api::<AvailState<N, P, D, V>, SeqTypes, _>(
        SequencerApiVersion::instance(),
        api_ver,
    )?;
    Ok(api)
}

pub(super) fn node<S>(api_ver: semver::Version) -> Result<Api<S, node::Error, StaticVersion<0, 1>>>
where
    S: 'static + Send + Sync + ReadState,
    <S as ReadState>::State: Send
        + Sync
        + StakeTableDataSource<SeqTypes>
        + NodeDataSource<SeqTypes>
        + AvailabilityDataSource<SeqTypes>,
{
    // Extend the base API
    let mut options = node::Options::default();
    let extension = toml::from_str(include_str!("../../api/node.toml"))?;
    options.extensions.push(extension);

    // Create the base API with our extensions
    let mut api =
        node::define_api::<S, SeqTypes, _>(&options, SequencerApiVersion::instance(), api_ver)?;

    // Tack on the application logic
    api.at("stake_table", |req, state| {
        async move {
            // Try to get the epoch from the request. If this fails, error
            // as it was probably a mistake
            let epoch = req
                .opt_integer_param("epoch_number")
                .map_err(|_| hotshot_query_service::node::Error::Custom {
                    message: "Epoch number is required".to_string(),
                    status: StatusCode::BAD_REQUEST,
                })?
                .map(EpochNumber::new);

            state
                .read(|state| state.get_stake_table(epoch).boxed())
                .await
                .map_err(|err| node::Error::Custom {
                    message: format!("failed to get stake table for epoch={epoch:?}. err={err:#}"),
                    status: StatusCode::NOT_FOUND,
                })
        }
        .boxed()
    })?
    .at("stake_table_current", |_, state| {
        async move {
            state
                .read(|state| state.get_stake_table_current().boxed())
                .await
                .map_err(|err| node::Error::Custom {
                    message: format!("failed to get current stake table. err={err:#}"),
                    status: StatusCode::NOT_FOUND,
                })
        }
        .boxed()
    })?
    .at("get_validators", |req, state| {
        async move {
            let epoch = req.integer_param::<_, u64>("epoch_number").map_err(|_| {
                hotshot_query_service::node::Error::Custom {
                    message: "Epoch number is required".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            })?;

            state
                .read(|state| state.get_validators(EpochNumber::new(epoch)).boxed())
                .await
                .map_err(|err| hotshot_query_service::node::Error::Custom {
                    message: format!("failed to get validators mapping: err: {err}"),
                    status: StatusCode::NOT_FOUND,
                })
        }
        .boxed()
    })?;

    Ok(api)
}
pub(super) fn submit<N, P, S, ApiVer: StaticVersionType + 'static>(
    api_ver: semver::Version,
) -> Result<Api<S, Error, ApiVer>>
where
    N: ConnectedNetwork<PubKey>,
    S: 'static + Send + Sync + ReadState,
    P: SequencerPersistence,
    S::State: Send + Sync + SubmitDataSource<N, P>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/submit.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;

    api.with_version(api_ver).at("submit", |req, state| {
        async move {
            let tx = req
                .body_auto::<Transaction, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            let hash = tx.commit();
            state
                .read(|state| state.submit(tx).boxed())
                .await
                .map_err(|err| Error::internal(err.to_string()))?;
            Ok(hash)
        }
        .boxed()
    })?;

    Ok(api)
}

pub(super) fn state_signature<N, S, ApiVer: StaticVersionType + 'static>(
    _: ApiVer,
    api_ver: semver::Version,
) -> Result<Api<S, Error, ApiVer>>
where
    N: ConnectedNetwork<PubKey>,
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + StateSignatureDataSource<N>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/state_signature.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;
    api.with_version(api_ver);

    api.get("get_state_signature", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            state
                .get_state_signature(height)
                .await
                .ok_or(tide_disco::Error::catch_all(
                    StatusCode::NOT_FOUND,
                    "Signature not found.".to_owned(),
                ))
        }
        .boxed()
    })?;

    Ok(api)
}

pub(super) fn catchup<S, ApiVer: StaticVersionType + 'static>(
    _: ApiVer,
    api_ver: semver::Version,
) -> Result<Api<S, Error, ApiVer>>
where
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + NodeStateDataSource + CatchupDataSource,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/catchup.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;
    api.with_version(api_ver);

    api.get("account", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            let view = req
                .integer_param("view")
                .map_err(Error::from_request_error)?;
            let account = req
                .string_param("address")
                .map_err(Error::from_request_error)?;
            let account = account.parse().map_err(|err| {
                Error::catch_all(
                    StatusCode::BAD_REQUEST,
                    format!("malformed account {account}: {err}"),
                )
            })?;

            state
                .get_account(
                    &state.node_state().await,
                    height,
                    ViewNumber::new(view),
                    account,
                )
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .at("accounts", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            let view = req
                .integer_param("view")
                .map_err(Error::from_request_error)?;
            let accounts = req
                .body_auto::<Vec<FeeAccount>, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            state
                .read(|state| {
                    async move {
                        state
                            .get_accounts(
                                &state.node_state().await,
                                height,
                                ViewNumber::new(view),
                                &accounts,
                            )
                            .await
                            .map_err(|err| {
                                Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}"))
                            })
                    }
                    .boxed()
                })
                .await
        }
        .boxed()
    })?
    .get("reward_account", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            let view = req
                .integer_param("view")
                .map_err(Error::from_request_error)?;
            let account = req
                .string_param("address")
                .map_err(Error::from_request_error)?;
            let account = account.parse().map_err(|err| {
                Error::catch_all(
                    StatusCode::BAD_REQUEST,
                    format!("malformed account {account}: {err}"),
                )
            })?;

            state
                .get_reward_account(
                    &state.node_state().await,
                    height,
                    ViewNumber::new(view),
                    account,
                )
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .at("reward_accounts", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            let view = req
                .integer_param("view")
                .map_err(Error::from_request_error)?;
            let accounts = req
                .body_auto::<Vec<RewardAccount>, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            state
                .read(|state| {
                    async move {
                        state
                            .get_reward_accounts(
                                &state.node_state().await,
                                height,
                                ViewNumber::new(view),
                                &accounts,
                            )
                            .await
                            .map_err(|err| {
                                Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}"))
                            })
                    }
                    .boxed()
                })
                .await
        }
        .boxed()
    })?
    .get("blocks", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            let view = req
                .integer_param("view")
                .map_err(Error::from_request_error)?;

            state
                .get_frontier(&state.node_state().await, height, ViewNumber::new(view))
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .get("chainconfig", |req, state| {
        async move {
            let commitment = req
                .blob_param("commitment")
                .map_err(Error::from_request_error)?;

            state
                .get_chain_config(commitment)
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .get("leafchain", |req, state| {
        async move {
            let height = req
                .integer_param("height")
                .map_err(Error::from_request_error)?;
            state
                .get_leaf_chain(height)
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?;

    Ok(api)
}

type MerklizedStateApi<N, P, D, V, ApiVer> =
    Api<AvailState<N, P, D, V>, merklized_state::Error, ApiVer>;
pub(super) fn merklized_state<N, P, D, S, V: Versions, const ARITY: usize>(
    api_ver: semver::Version,
) -> Result<MerklizedStateApi<N, P, D, V, SequencerApiVersion>>
where
    N: ConnectedNetwork<PubKey>,
    D: MerklizedStateDataSource<SeqTypes, S, ARITY>
        + Send
        + Sync
        + MerklizedStateHeightPersistence
        + 'static,
    S: MerklizedState<SeqTypes, ARITY>,
    P: SequencerPersistence,
    for<'a> <S::Commit as TryFrom<&'a TaggedBase64>>::Error: std::fmt::Display,
{
    let api = merklized_state::define_api::<
        AvailState<N, P, D, V>,
        SeqTypes,
        S,
        SequencerApiVersion,
        ARITY,
    >(&Default::default(), api_ver)?;
    Ok(api)
}

pub(super) fn config<S, ApiVer: StaticVersionType + 'static>(
    _: ApiVer,
    api_ver: semver::Version,
) -> Result<Api<S, Error, ApiVer>>
where
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + HotShotConfigDataSource,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/config.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;
    api.with_version(api_ver);

    let env_variables = get_public_env_vars()
        .map_err(|err| Error::catch_all(StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")))?;

    api.get("hotshot", |_, state| {
        async move { Ok(state.get_config().await) }.boxed()
    })?
    .get("env", move |_, _| {
        {
            let env_variables = env_variables.clone();
            async move { Ok(env_variables) }
        }
        .boxed()
    })?;

    Ok(api)
}

fn get_public_env_vars() -> Result<Vec<String>> {
    let toml: toml::Value = toml::from_str(include_str!("../../api/public-env-vars.toml"))?;

    let keys = toml
        .get("variables")
        .ok_or_else(|| toml::de::Error::custom("variables not found"))?
        .as_array()
        .ok_or_else(|| toml::de::Error::custom("variables is not an array"))?
        .clone()
        .into_iter()
        .map(|v| v.try_into())
        .collect::<Result<BTreeSet<String>, toml::de::Error>>()?;

    let hashmap: HashMap<String, String> = env::vars().collect();
    let mut public_env_vars: Vec<String> = Vec::new();
    for key in keys {
        let value = hashmap.get(&key).cloned().unwrap_or_default();
        public_env_vars.push(format!("{key}={value}"));
    }

    Ok(public_env_vars)
}
