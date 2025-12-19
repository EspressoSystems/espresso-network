//! Sequencer-specific API endpoint handlers.

use std::{
    collections::{BTreeSet, HashMap},
    env,
    time::Duration,
};

use anyhow::Result;
use committable::Committable;
use espresso_types::{FeeAccount, FeeMerkleTree, NamespaceId, NsProof, PubKey, Transaction};
use futures::{stream::StreamExt, try_join, FutureExt, TryFutureExt};
use hotshot_query_service::{
    availability::{
        self, AvailabilityDataSource, BlockId, BlockQueryData, CustomSnafu, FetchBlockSnafu,
        VidCommonQueryData,
    },
    explorer::{self, ExplorerDataSource},
    merklized_state::{
        self, MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence,
    },
    node, ApiState, Error,
};
use hotshot_query_service::{merklized_state::Snapshot, node::NodeDataSource};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
    },
};
use jf_merkle_tree::MerkleTreeScheme;
use serde::{de::Error as _, Deserialize, Serialize};
use snafu::OptionExt;
use tagged_base64::TaggedBase64;
use tide_disco::{method::ReadState, Api, Error as _, RequestParams, StatusCode};
use vbs::version::{StaticVersion, StaticVersionType};

use super::{
    data_source::{
        CatchupDataSource, HotShotConfigDataSource, NodeStateDataSource, SequencerDataSource,
        StakeTableDataSource, StateSignatureDataSource, SubmitDataSource,
    },
    StorageState,
};
use crate::{SeqTypes, SequencerApiVersion, SequencerPersistence};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamespaceProofQueryData {
    pub proof: Option<NsProof>,
    pub transactions: Vec<Transaction>,
}

pub(super) fn get_balance<State, Ver>() -> Result<Api<State, merklized_state::Error, Ver>>
where
    State: 'static + Send + Sync + ReadState,
    Ver: 'static + StaticVersionType,
    <State as ReadState>::State: Send
        + Sync
        + MerklizedStateDataSource<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence,
{
    let mut options = merklized_state::Options::default();
    let extension = toml::from_str(include_str!("../../api/merklized_state.toml"))?;
    options.extensions.push(extension);

    let mut api =
        merklized_state::define_api::<State, SeqTypes, FeeMerkleTree, Ver, 256>(&options)?;

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

pub(super) type AvailState<N, P, D, ApiVer> = ApiState<StorageState<N, P, D, ApiVer>>;

type AvailabilityApi<N, P, D, V, ApiVer> = Api<AvailState<N, P, D, V>, availability::Error, ApiVer>;

// TODO (abdul): replace snafu with `this_error` in  hotshot query service
// Snafu has been replaced by `this_error` everywhere.
// However, the query service still uses snafu
pub(super) fn availability<N, P, D, V: Versions>(
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
    let limit = options.large_object_range_limit;

    let mut api = availability::define_api::<AvailState<N, P, D, _>, SeqTypes, _>(
        &options,
        SequencerApiVersion::instance(),
    )?;

    // Namespace proof endpoint - supports height, hash, or payload-hash as block identifier
    api.at("getnamespaceproof", move |req, state| {
        async move {
            let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
            let (block, common) = get_block_for_ns_proof(&req, state, timeout).await?;
            get_namespace_proof(&block, &common, ns_id)
        }
        .boxed()
    })?;

    // Ranged namespace proof endpoint
    api.at("getnamespaceproof_range", move |req, state| {
        async move {
            let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
            let blocks = get_block_range_for_ns_proof(&req, state, limit, timeout).await?;
            blocks
                .iter()
                .map(|(block, common)| get_namespace_proof(block, common, ns_id))
                .collect::<Result<Vec<_>, _>>()
        }
        .boxed()
    })?;

    // Streaming namespace proof endpoint
    api.stream("stream_namespace_proofs", move |req, state| {
        async move {
            let ns_id = NamespaceId::from(req.integer_param::<_, u32>("namespace")?);
            let from: usize = req.integer_param("from")?;

            state
                .read(|state| {
                    async move {
                        let blocks = state.subscribe_blocks(from).await;
                        let vids = state.subscribe_vid_common(from).await;

                        let stream = blocks
                            .zip(vids)
                            .map(move |(block, vid)| get_namespace_proof(&block, &vid, ns_id));

                        Ok(stream)
                    }
                    .boxed()
                })
                .await
        }
        .try_flatten_stream()
        .boxed()
    })?;

    Ok(api)
}

/// Get namespace proof for a given block and namespace.
fn get_namespace_proof(
    block: &BlockQueryData<SeqTypes>,
    common: &VidCommonQueryData<SeqTypes>,
    ns_id: NamespaceId,
) -> Result<NamespaceProofQueryData, availability::Error> {
    if let Some(ns_index) = block.payload().ns_table().find_ns_id(&ns_id) {
        let proof =
            NsProof::new(block.payload(), &ns_index, common.common()).context(CustomSnafu {
                message: format!("failed to make proof for namespace {ns_id}"),
                status: StatusCode::NOT_FOUND,
            })?;

        Ok(NamespaceProofQueryData {
            transactions: proof.export_all_txs(&ns_id),
            proof: Some(proof),
        })
    } else {
        // ns_id not found in ns_table
        Ok(NamespaceProofQueryData {
            proof: None,
            transactions: Vec::new(),
        })
    }
}

async fn get_block_for_ns_proof<S>(
    req: &RequestParams,
    state: &S,
    timeout: Duration,
) -> Result<(BlockQueryData<SeqTypes>, VidCommonQueryData<SeqTypes>), availability::Error>
where
    S: ReadState,
    S::State: AvailabilityDataSource<SeqTypes> + Sync,
{
    let id = if let Some(height) = req.opt_integer_param("height")? {
        BlockId::Number(height)
    } else if let Some(hash) = req.opt_blob_param("hash")? {
        BlockId::Hash(hash)
    } else {
        BlockId::PayloadHash(req.blob_param("payload-hash")?)
    };

    // Fetch block and VID common - we need to do this sequentially due to borrowing constraints
    let block = state
        .read(|state| state.get_block(id).boxed())
        .await
        .with_timeout(timeout)
        .await
        .context(FetchBlockSnafu {
            resource: format!("{id:?}"),
        })?;

    let vid = state
        .read(|state| state.get_vid_common(id).boxed())
        .await
        .with_timeout(timeout)
        .await
        .context(FetchBlockSnafu {
            resource: format!("{id:?}"),
        })?;

    Ok((block, vid))
}

async fn get_block_range_for_ns_proof<S>(
    req: &RequestParams,
    state: &S,
    limit: usize,
    timeout: Duration,
) -> Result<Vec<(BlockQueryData<SeqTypes>, VidCommonQueryData<SeqTypes>)>, availability::Error>
where
    S: ReadState,
    S::State: AvailabilityDataSource<SeqTypes> + Sync,
{
    let from: usize = req.integer_param("from")?;
    let until: usize = req.integer_param("until")?;
    if until.saturating_sub(from) > limit {
        return Err(availability::Error::RangeLimit { from, until, limit });
    }

    // Fetch blocks and VID commons
    let blocks = state
        .read(|state| state.get_block_range(from..until).boxed())
        .await;
    let vids = state
        .read(|state| state.get_vid_common_range(from..until).boxed())
        .await;

    blocks
        .zip(vids)
        .enumerate()
        .then(|(i, (block, vid))| async move {
            let height = from + i;
            try_join!(
                async move {
                    block.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: height.to_string(),
                    })
                },
                async move {
                    vid.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: height.to_string(),
                    })
                }
            )
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
}

type ExplorerApi<N, P, D, V, ApiVer> = Api<AvailState<N, P, D, V>, explorer::Error, ApiVer>;

pub(super) fn explorer<N, P, D, V: Versions>(
) -> Result<ExplorerApi<N, P, D, V, SequencerApiVersion>>
where
    N: ConnectedNetwork<PubKey>,
    D: ExplorerDataSource<SeqTypes> + Send + Sync + 'static,
    P: SequencerPersistence,
{
    let api = explorer::define_api::<AvailState<N, P, D, V>, SeqTypes, _>(
        SequencerApiVersion::instance(),
    )?;
    Ok(api)
}

pub(super) fn node<S>() -> Result<Api<S, node::Error, StaticVersion<0, 1>>>
where
    S: 'static + Send + Sync + ReadState,
    <S as ReadState>::State:
        Send + Sync + StakeTableDataSource<SeqTypes> + NodeDataSource<SeqTypes>,
{
    // Extend the base API
    let mut options = node::Options::default();
    let extension = toml::from_str(include_str!("../../api/node.toml"))?;
    options.extensions.push(extension);

    // Create the base API with our extensions
    let mut api = node::define_api::<S, SeqTypes, _>(&options, SequencerApiVersion::instance())?;

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

            Ok(state
                .read(|state| state.get_stake_table(epoch).boxed())
                .await)
        }
        .boxed()
    })?
    .at("stake_table_current", |_, state| {
        async move {
            Ok(state
                .read(|state| state.get_stake_table_current().boxed())
                .await)
        }
        .boxed()
    })?;

    Ok(api)
}
pub(super) fn submit<N, P, S, ApiVer: StaticVersionType + 'static>() -> Result<Api<S, Error, ApiVer>>
where
    N: ConnectedNetwork<PubKey>,
    S: 'static + Send + Sync + ReadState,
    P: SequencerPersistence,
    S::State: Send + Sync + SubmitDataSource<N, P>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/submit.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;

    api.at("submit", |req, state| {
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
) -> Result<Api<S, Error, ApiVer>>
where
    N: ConnectedNetwork<PubKey>,
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + StateSignatureDataSource<N>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/state_signature.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;

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
) -> Result<Api<S, Error, ApiVer>>
where
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + NodeStateDataSource + CatchupDataSource,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/catchup.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;

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
                    state.node_state().await,
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
                                state.node_state().await,
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
                .get_frontier(state.node_state().await, height, ViewNumber::new(view))
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
    })?;

    Ok(api)
}

type MerklizedStateApi<N, P, D, V, ApiVer> =
    Api<AvailState<N, P, D, V>, merklized_state::Error, ApiVer>;
pub(super) fn merklized_state<N, P, D, S, V: Versions, const ARITY: usize>(
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
    >(&Default::default())?;
    Ok(api)
}

pub(super) fn config<S, ApiVer: StaticVersionType + 'static>(
    _: ApiVer,
) -> Result<Api<S, Error, ApiVer>>
where
    S: 'static + Send + Sync + ReadState,
    S::State: Send + Sync + HotShotConfigDataSource,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/config.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;

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
