//! Sequencer-specific API endpoint handlers.

use std::{
    collections::{BTreeSet, HashMap},
    env,
};

use anyhow::Result;
use committable::Committable;
use espresso_types::{
    v0_3::RewardAccountV1,
    v0_4::{RewardAccountV2, RewardClaimError},
    FeeAccount, FeeMerkleTree, PubKey, Transaction,
};
// re-exported here to avoid breaking changes in consumers
// "deprecated" does not work with "pub use": https://github.com/rust-lang/rust/issues/30827
#[deprecated(note = "use espresso_types::ADVZNamespaceProofQueryData")]
pub type ADVZNamespaceProofQueryData = espresso_types::ADVZNamespaceProofQueryData;
#[deprecated(note = "use espresso_types::NamespaceProofQueryData")]
pub type NamespaceProofQueryData = espresso_types::NamespaceProofQueryData;

use futures::FutureExt;
use hotshot_query_service::{
    availability::AvailabilityDataSource,
    explorer::{self, ExplorerDataSource},
    merklized_state::{
        self, MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
    },
    node::{self, NodeDataSource},
    Error,
};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
    },
};
use jf_merkle_tree_compat::MerkleTreeScheme;
use serde::de::Error as _;
use tagged_base64::TaggedBase64;
use tide_disco::{method::ReadState, Api, Error as _, RequestParams, StatusCode};
use vbs::version::{StaticVersion, StaticVersionType};

use super::data_source::{
    CatchupDataSource, HotShotConfigDataSource, NodeStateDataSource, StakeTableDataSource,
    StateSignatureDataSource, SubmitDataSource,
};
use crate::{
    api::RewardAccountProofDataSource, SeqTypes, SequencerApiVersion, SequencerPersistence,
};

mod availability;
pub(super) use availability::*;

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

pub enum RewardMerkleTreeVersion {
    V1,
    V2,
}

pub(super) fn reward<State, Ver, MT, const ARITY: usize>(
    api_ver: semver::Version,
    merkle_tree_version: RewardMerkleTreeVersion,
) -> Result<Api<State, merklized_state::Error, Ver>>
where
    State: 'static + Send + Sync + ReadState,
    Ver: 'static + StaticVersionType,
    MT: MerklizedState<SeqTypes, ARITY>,
    for<'a> <MT::Commit as TryFrom<&'a TaggedBase64>>::Error: std::fmt::Display,
    <MT as MerklizedState<SeqTypes, ARITY>>::Entry: std::marker::Copy,
    <State as ReadState>::State: Send
        + Sync
        + RewardAccountProofDataSource
        + MerklizedStateDataSource<SeqTypes, MT, ARITY>
        + MerklizedStateHeightPersistence,
{
    let mut options = merklized_state::Options::default();
    let extension = toml::from_str(include_str!("../../api/reward.toml"))?;
    options.extensions.push(extension);

    let mut api =
        merklized_state::define_api::<State, SeqTypes, MT, Ver, ARITY>(&options, api_ver)?;

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

            let last_height = state.get_last_state_height().await?;

            if height > last_height {
                return Err(merklized_state::Error::Custom {
                    message: format!(
                        "requested height {height} is greater than last known height {last_height}"
                    ),
                    status: StatusCode::BAD_REQUEST,
                });
            }

            Ok(path.elem().copied())
        }
        .boxed()
    })?;

    match merkle_tree_version {
        RewardMerkleTreeVersion::V1 => {
            api.get("get_reward_account_proof", move |req, state| {
                async move {
                    let address = req.string_param("address")?;
                    let height = req.integer_param("height")?;
                    let account = address
                        .parse()
                        .map_err(|_| merklized_state::Error::Custom {
                            message: format!("invalid reward address: {address}"),
                            status: StatusCode::BAD_REQUEST,
                        })?;

                    state
                        .load_v1_reward_account_proof(height, account)
                        .await
                        .map_err(|err| merklized_state::Error::Custom {
                            message: format!(
                                "failed to load v1 reward account {address} at height {height}: \
                                 {err}"
                            ),
                            status: StatusCode::NOT_FOUND,
                        })
                }
                .boxed()
            })?;
        },
        RewardMerkleTreeVersion::V2 => {
            api.get("get_reward_account_proof", move |req, state| {
                async move {
                    let address = req.string_param("address")?;
                    let height = req.integer_param("height")?;
                    let account = address
                        .parse()
                        .map_err(|_| merklized_state::Error::Custom {
                            message: format!("invalid reward address: {address}"),
                            status: StatusCode::BAD_REQUEST,
                        })?;

                    state
                        .load_v2_reward_account_proof(height, account)
                        .await
                        .map_err(|err| merklized_state::Error::Custom {
                            message: format!(
                                "failed to load v2 reward account {address} at height {height}: \
                                 {err}"
                            ),
                            status: StatusCode::NOT_FOUND,
                        })
                }
                .boxed()
            })?;

            api.get("get_reward_claim_input", move |req, state| {
                async move {
                    let address = req.string_param("address")?;
                    let height = req.integer_param("height")?;
                    let account = address
                        .parse()
                        .map_err(|_| merklized_state::Error::Custom {
                            message: format!("invalid reward address: {address}"),
                            status: StatusCode::BAD_REQUEST,
                        })?;

                    let proof = state
                        .load_v2_reward_account_proof(height, account)
                        .await
                        .map_err(|err| merklized_state::Error::Custom {
                            message: format!(
                                "failed to load v2 reward account {address} at height {height}: \
                                 {err}"
                            ),
                            status: StatusCode::NOT_FOUND,
                        })?;

                    // Auth root inputs (other than the reward merkle tree root) are currently
                    // all zero placeholder values. This may be extended in the future.
                    let claim_input = match proof.to_reward_claim_input() {
                        Ok(input) => input,
                        Err(RewardClaimError::ZeroRewardError) => {
                            return Err(merklized_state::Error::Custom {
                                message: format!(
                                    "zero reward balance for {address} at height {height}"
                                ),
                                status: StatusCode::NOT_FOUND,
                            })
                        },
                        Err(RewardClaimError::ProofConversionError(err)) => {
                            let message = format!(
                                "failed to create solidity proof for {address} at height \
                                 {height}: {err}",
                            );
                            tracing::warn!("{message}");
                            // Normally we would not want to return the internal error via the
                            // API response but this is an error that should never occur. No
                            // secret data involved so it seems fine to return it.
                            return Err(merklized_state::Error::Custom {
                                message,
                                status: StatusCode::INTERNAL_SERVER_ERROR,
                            });
                        },
                    };

                    Ok(claim_input)
                }
                .boxed()
            })?;
        },
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
    })?
    .at("get_all_validators", |req, state| {
        async move {
            let epoch = req.integer_param::<_, u64>("epoch_number").map_err(|_| {
                hotshot_query_service::node::Error::Custom {
                    message: "Epoch number is required".to_string(),
                    status: StatusCode::BAD_REQUEST,
                }
            })?;

            let offset = req.integer_param::<_, u64>("offset")?;

            let limit = req.integer_param::<_, u64>("limit")?;
            if limit > 1000 {
                return Err(hotshot_query_service::node::Error::Custom {
                    message: "Limit cannot be greater than 1000".to_string(),
                    status: StatusCode::BAD_REQUEST,
                });
            }

            state
                .read(|state| {
                    state
                        .get_all_validators(EpochNumber::new(epoch), offset, limit)
                        .boxed()
                })
                .await
                .map_err(|err| hotshot_query_service::node::Error::Custom {
                    message: format!("failed to get all validators : err: {err}"),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })
        }
        .boxed()
    })?
    .at("current_proposal_participation", |_, state| {
        async move {
            Ok(state
                .read(|state| state.current_proposal_participation().boxed())
                .await)
        }
        .boxed()
    })?
    .at("previous_proposal_participation", |_, state| {
        async move {
            Ok(state
                .read(|state| state.previous_proposal_participation().boxed())
                .await)
        }
        .boxed()
    })?
    .at("get_block_reward", |req, state| {
        async move {
            let epoch = req
                .opt_integer_param::<_, u64>("epoch_number")?
                .map(EpochNumber::new);

            state
                .read(|state| state.get_block_reward(epoch).boxed())
                .await
                .map_err(|err| node::Error::Custom {
                    message: format!("failed to get block reward. err={err:#}"),
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

    let parse_height_view = |req: &RequestParams| -> Result<(u64, ViewNumber), Error> {
        let height = req
            .integer_param("height")
            .map_err(Error::from_request_error)?;
        let view = req
            .integer_param("view")
            .map_err(Error::from_request_error)?;
        Ok((height, ViewNumber::new(view)))
    };

    let parse_fee_account = |req: &RequestParams| -> Result<FeeAccount, Error> {
        let raw = req
            .string_param("address")
            .map_err(Error::from_request_error)?;
        raw.parse().map_err(|err| {
            Error::catch_all(
                StatusCode::BAD_REQUEST,
                format!("malformed fee account {raw}: {err}"),
            )
        })
    };

    let parse_reward_account = |req: &RequestParams| -> Result<RewardAccountV2, Error> {
        let raw = req
            .string_param("address")
            .map_err(Error::from_request_error)?;
        raw.parse().map_err(|err| {
            Error::catch_all(
                StatusCode::BAD_REQUEST,
                format!("malformed reward account {raw}: {err}"),
            )
        })
    };

    api.get("account", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let account = parse_fee_account(&req)?;
            state
                .get_account(&state.node_state().await, height, view, account)
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .at("accounts", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let accounts = req
                .body_auto::<Vec<FeeAccount>, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            state
                .read(|state| {
                    async move {
                        state
                            .get_accounts(&state.node_state().await, height, view, &accounts)
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
    .get("reward_account", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let account = parse_reward_account(&req)?;
            state
                .get_reward_account_v1(&state.node_state().await, height, view, account.into())
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .at("reward_accounts", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let accounts = req
                .body_auto::<Vec<RewardAccountV1>, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            state
                .read(|state| {
                    async move {
                        state
                            .get_reward_accounts_v1(
                                &state.node_state().await,
                                height,
                                view,
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
    .get("reward_account_v2", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let account = parse_reward_account(&req)?;

            state
                .get_reward_account_v2(&state.node_state().await, height, view, account)
                .await
                .map_err(|err| Error::catch_all(StatusCode::NOT_FOUND, format!("{err:#}")))
        }
        .boxed()
    })?
    .at("reward_accounts_v2", move |req, state| {
        async move {
            let (height, view) = parse_height_view(&req)?;
            let accounts = req
                .body_auto::<Vec<RewardAccountV2>, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;

            state
                .read(|state| {
                    async move {
                        state
                            .get_reward_accounts_v2(
                                &state.node_state().await,
                                height,
                                view,
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
