use std::{cmp::Ordering, collections::HashMap, fmt::Display, sync::Arc, time::Duration};

use anyhow::{Result, bail, ensure};
use committable::{Commitment, Committable};
use disco_types::status::StatusCode;
use espresso_types::{BlockMerkleTree, Header, NsIndex, NsProof, SeqTypes};
use futures::{TryStreamExt, future::try_join, stream::StreamExt};
use hotshot_query_service::{
    Error, QueryError,
    availability::{AvailabilityDataSource, LeafQueryData, PayloadQueryData, VidCommonQueryData},
    data_source::{VersionedDataSource, storage::NodeStorage},
    merklized_state::{MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot},
    node::BlockId,
    types::HeightIndexed,
};
use hotshot_types::simple_certificate::QuorumCertificate2;
use itertools::izip;
use jf_merkle_tree_compat::{
    DigestAlgorithm, MerkleTreeScheme,
    internal::MerkleNode,
    prelude::{Sha3Digest, Sha3Node},
};
use light_client::{
    client::NAMESPACES_PARAM_TAG,
    consensus::{header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof},
};
use tagged_base64::TaggedBase64;
use tokio::spawn;
use versions::NEW_PROTOCOL_VERSION;

/// Construct a proof that the requested leaf is finalized.
///
/// The `finalized` hint is honored when it yields a bounded proof no longer than a direct finality
/// proof, since the client can verify hint-based proofs without signature checks.
pub(crate) async fn get_leaf_proof<State>(
    state: &State,
    requested_leaf: LeafQueryData<SeqTypes>,
    finalized: Option<usize>,
    fetch_timeout: Duration,
    chain_limit: usize,
) -> Result<LeafProof, Error>
where
    State: AvailabilityDataSource<SeqTypes> + VersionedDataSource + Clone + Send + Sync + 'static,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let requested = requested_leaf.height() as usize;
    let new_protocol = requested_leaf.header().version() >= NEW_PROTOCOL_VERSION;

    let mut hint = finalized.filter(|finalized| finalized.saturating_sub(requested) <= chain_limit);

    // New-protocol chains cannot terminate early, so a nearby cert2 may yield a shorter proof.
    if let Some(finalized) = hint
        && new_protocol
        && let Some(cert2_height) = earliest_cert2_height(state, requested as u64).await
        && cert2_height + 1 < finalized as u64
    {
        hint = None;
    }

    if let Some(finalized) = hint {
        get_leaf_proof_with_finalized_assumption(
            state,
            requested_leaf,
            finalized,
            fetch_timeout,
            chain_limit,
        )
        .await
    } else if new_protocol {
        get_leaf_proof_with_cert2(state, requested_leaf, fetch_timeout, chain_limit).await
    } else {
        get_leaf_proof_with_leaf_chain(state, requested_leaf, fetch_timeout, chain_limit).await
    }
}

/// The earliest stored cert2 height at or after `height`. Errors are treated as no cert2: this
/// only chooses between proof strategies, and the chosen path will surface any persistent error.
async fn earliest_cert2_height<State>(state: &State, height: u64) -> Option<u64>
where
    State: VersionedDataSource,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let cert2 = state
        .read()
        .await
        .ok()?
        .load_earliest_cert2(height)
        .await
        .ok()??;
    Some(cert2.data.block_number)
}

/// Build a leaf proof for a legacy leaf from the justify QCs of the leaves that follow it.
pub(crate) async fn get_leaf_proof_with_leaf_chain<State>(
    state: &State,
    requested_leaf: LeafQueryData<SeqTypes>,
    fetch_timeout: Duration,
    chain_limit: usize,
) -> Result<LeafProof, Error>
where
    State: AvailabilityDataSource<SeqTypes> + VersionedDataSource + Clone + Send + Sync + 'static,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let requested = requested_leaf.height() as usize;
    let mut tx = state.read().await.map_err(internal)?;
    let latest_height = NodeStorage::block_height(&mut tx).await.map_err(internal)?;
    drop(tx);

    let mut leaves = state.get_leaf_range(requested + 1..latest_height).await;
    let mut proof = LeafProof::default();
    let requested_leaf_qc = requested_leaf.qc().clone();
    proof.push(requested_leaf);

    let mut remaining = chain_limit;
    while let Some(leaf) = leaves.next().await {
        let leaf = leaf
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found("missing leaves"))?;

        remaining = remaining
            .checked_sub(1)
            .ok_or_else(|| chain_too_long(requested, chain_limit))?;

        // HotStuff commit rules cannot terminate a chain of new-protocol leaves; switch to cert2.
        if leaf.header().version() >= NEW_PROTOCOL_VERSION {
            complete_proof_with_cert2(
                state,
                &mut proof,
                leaf,
                requested_leaf_qc.clone(),
                fetch_timeout,
                remaining,
            )
            .await?;
            return Ok(proof);
        }

        if proof.push(leaf) {
            return Ok(proof);
        }
    }

    // There is no stored legacy QC chain to fall back on: it could only close a chain whose tip was
    // legacy, and every network is past the 0.6 cutover, so running out of leaves is a 404.
    Err(not_found("not enough leaves to prove finality"))
}

/// Build a leaf proof for the new protocol using certificate2 finality.
///
/// Certificate2 directly commits the leaf at `cert2.data.block_number`. The indirect commit rule then
/// commits all of that leaf's uncommitted ancestors. If cert2 directly commits the requested leaf,
/// the proof contains only that leaf plus cert2. Otherwise, the proof includes the chain from the
/// requested leaf through the directly committed descendant; the verifier checks parent commitments
/// to prove the requested leaf is on that finalized chain.
pub(crate) async fn get_leaf_proof_with_cert2<State>(
    state: &State,
    requested_leaf: LeafQueryData<SeqTypes>,
    fetch_timeout: Duration,
    chain_limit: usize,
) -> Result<LeafProof, Error>
where
    State: AvailabilityDataSource<SeqTypes> + VersionedDataSource + Clone + Send + Sync + 'static,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let mut proof = LeafProof::default();
    let requested_leaf_qc = requested_leaf.qc().clone();
    complete_proof_with_cert2(
        state,
        &mut proof,
        requested_leaf,
        requested_leaf_qc,
        fetch_timeout,
        chain_limit,
    )
    .await?;
    Ok(proof)
}

/// Extend `proof` from `leaf` to the leaf directly committed by the earliest stored cert2, adding
/// at most `chain_limit` leaves.
async fn complete_proof_with_cert2<State>(
    state: &State,
    proof: &mut LeafProof,
    leaf: LeafQueryData<SeqTypes>,
    requested_leaf_qc: QuorumCertificate2<SeqTypes>,
    fetch_timeout: Duration,
    chain_limit: usize,
) -> Result<(), Error>
where
    State: AvailabilityDataSource<SeqTypes> + VersionedDataSource + Clone + Send + Sync + 'static,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let start_height = leaf.height();
    let start_commit = leaf.leaf().commit();
    let limit = chain_limit as u64;

    // The new leaf may already complete a HotStuff chain begun before the protocol cutover.
    if proof.push(leaf) {
        return Ok(());
    }

    let stored = state
        .read()
        .await
        .map_err(internal)?
        .load_earliest_cert2(start_height)
        .await
        .map_err(internal)?;

    let earliest = stored.as_ref().map(|cert2| cert2.data.block_number);
    let Some(cert2) = stored.filter(|cert2| cert2.data.block_number <= start_height + limit) else {
        // An indirectly finalized leaf has no cert2 of its own, so its descendants are asked, one
        // at a time. A cert2 no peer has never resolves, hence the timeout per step.
        let known_height = async {
            let mut tx = state.read().await?;
            anyhow::Ok(NodeStorage::block_height(&mut tx).await? as u64)
        }
        .await
        // A miss either way, and a failing store should not be asked once more per height.
        .inspect_err(|err| {
            tracing::warn!(
                %err,
                height = start_height,
                "failed to load block height, skipping cert2 fetch"
            )
        })
        .ok();
        if let Some(known_height) = known_height {
            let end = (start_height + limit + 1).min(known_height);
            let state = state.clone();
            spawn(async move {
                for height in start_height..end {
                    if state
                        .get_cert2(height)
                        .await
                        .with_timeout(fetch_timeout)
                        .await
                        .is_some()
                    {
                        break;
                    }
                }
            });
        }
        return Err(not_found(match earliest {
            None => format!("no cert2 finality proof available at or after height {start_height}"),
            Some(height) => format!(
                "earliest cert2 finality proof (height {height}) is more than {chain_limit} \
                 leaves past height {start_height}"
            ),
        }));
    };

    let cert2_height = cert2.data.block_number;
    if cert2_height < start_height {
        return Err(not_found(
            "cert2 finality proof is older than requested leaf",
        ));
    }

    if cert2_height == start_height {
        if start_commit != cert2.data.leaf_commit {
            return Err(internal("stored cert2 does not finalize the expected leaf"));
        }
        proof.add_certificate(Arc::new(cert2), requested_leaf_qc);
        return Ok(());
    }

    let mut leaves = state
        .get_leaf_range(start_height as usize + 1..cert2_height as usize + 1)
        .await;
    // Extend the proof chain until we reach the leaf directly committed by cert2. Once that leaf is
    // present and matches cert2's commitment, the requested leaf is finalized by the indirect
    // commit rule.
    while let Some(leaf) = leaves.next().await {
        let leaf = leaf
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found("missing leaves"))?;

        if leaf.height() == cert2_height {
            if leaf.leaf().commit() != cert2.data.leaf_commit {
                return Err(internal("stored cert2 does not finalize the expected leaf"));
            }
            if !proof.push(leaf) {
                proof.add_certificate(Arc::new(cert2), requested_leaf_qc);
            }
            return Ok(());
        }
        if proof.push(leaf) {
            return Ok(());
        }
    }

    Err(not_found("missing cert2 leaf"))
}

pub(crate) async fn get_leaf_proof_with_finalized_assumption<State>(
    state: &State,
    requested_leaf: LeafQueryData<SeqTypes>,
    finalized: usize,
    fetch_timeout: Duration,
    chain_limit: usize,
) -> Result<LeafProof, Error>
where
    State: AvailabilityDataSource<SeqTypes>,
{
    let requested = requested_leaf.height() as usize;
    // If we have a known-finalized block, we will not need a final 2-chain of QCs to prove
    // the last leaf in the result finalized, since we will either terminate with a 3-chain
    // of leaves or at the `finalized` leaf.
    if finalized <= requested {
        return Err(Error::Custom {
            message: format!(
                "finalized leaf height ({finalized}) must be greater than requested ({requested})"
            ),
            status: StatusCode::BAD_REQUEST,
        });
    }
    if finalized - requested > chain_limit {
        return Err(Error::Custom {
            message: format!(
                "finalized leaf height ({finalized}) is more than {chain_limit} blocks past the \
                 requested leaf ({requested}); request a proof without the finalized parameter \
                 instead"
            ),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let mut leaves = state.get_leaf_range(requested + 1..finalized).await;
    let mut proof = LeafProof::default();
    proof.push(requested_leaf);

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found("missing leaves"))?;

        if proof.push(leaf) {
            return Ok(proof);
        }
    }

    Ok(proof)
}

pub(crate) async fn get_header_proof<State>(
    state: &State,
    root: u64,
    requested: BlockId<SeqTypes>,
    fetch_timeout: Duration,
) -> Result<HeaderProof, Error>
where
    State: AvailabilityDataSource<SeqTypes>
        + MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence
        + VersionedDataSource,
{
    let header = state
        .get_header(requested)
        .await
        .with_timeout(fetch_timeout)
        .await
        .ok_or_else(|| not_found(format!("unknown header {requested}")))?;
    if header.height() >= root {
        return Err(Error::Custom {
            message: format!(
                "height ({}) must be less than root ({root})",
                header.height()
            ),
            status: StatusCode::BAD_REQUEST,
        });
    }
    let path = MerklizedStateDataSource::<SeqTypes, BlockMerkleTree, _>::get_path(
        state,
        Snapshot::Index(root),
        header.height(),
    )
    .await;
    let path = match path {
        Ok(path) => path,
        Err(err) if matches!(err.status(), StatusCode::NOT_FOUND | StatusCode::GONE) => {
            // No usable snapshot at `root` (pruned or missing): rebuild from the newest one.
            let head = state.get_last_state_height().await.map_err(merkle_error)? as u64;
            if head <= root {
                return Err(merkle_error(err));
            }
            return rebuild_header_path(state, root, header, head, fetch_timeout).await;
        },
        Err(err) => return Err(merkle_error(err)),
    };

    Ok(HeaderProof::new(header, path))
}

fn merkle_error(source: QueryError) -> Error {
    Error::MerklizedState {
        source: source.into(),
    }
}

/// Prove `header` against `root` using the snapshot at `head`, when the state at `root` is pruned.
///
/// The result is verified against the commitment in header `root`.
async fn rebuild_header_path<State>(
    state: &State,
    root: u64,
    header: Header,
    head: u64,
    fetch_timeout: Duration,
) -> Result<HeaderProof, Error>
where
    State: AvailabilityDataSource<SeqTypes>
        + MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>,
{
    let get_path = |height| {
        MerklizedStateDataSource::<SeqTypes, BlockMerkleTree, _>::get_path(
            state,
            Snapshot::Index(head),
            height,
        )
    };
    let frontier = get_path(root - 1).await.map_err(merkle_error)?;
    let target = get_path(header.height()).await.map_err(merkle_error)?;
    let root_header = state
        .get_header(BlockId::Number(root as usize))
        .await
        .with_timeout(fetch_timeout)
        .await
        .ok_or_else(|| not_found(format!("unknown header {root}")))?;

    let proof = rebuild_block_proof(root, &frontier, &target).map_err(internal)?;
    let proof = HeaderProof::new(header, proof);
    proof
        .verify_ref(root_header.block_merkle_tree_root())
        .map_err(|err| {
            internal(format!(
                "rebuilt block merkle path does not match root {root}: {err:#}"
            ))
        })?;
    Ok(proof)
}

type BlockMerkleProof = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;
type BlockMerkleNode = MerkleNode<Commitment<Header>, u64, Sha3Node>;

/// The path to `target.pos` in the block tree of the first `num_leaves` leaves.
///
/// Both proofs come from one snapshot with more than `num_leaves` leaves; `frontier` is for leaf
/// `num_leaves - 1`. Subtrees left of the frontier are full and unchanged since, subtrees right of
/// it are empty, and the frontier nodes are recomputed.
pub(crate) fn rebuild_block_proof(
    num_leaves: u64,
    frontier: &BlockMerkleProof,
    target: &BlockMerkleProof,
) -> Result<BlockMerkleProof> {
    ensure!(
        frontier.pos + 1 == num_leaves,
        "frontier proof is for leaf {}, expected {}",
        frontier.pos,
        num_leaves.wrapping_sub(1),
    );
    ensure!(target.pos < num_leaves, "target leaf is beyond the tree");
    ensure!(
        frontier.proof.len() == target.proof.len(),
        "proofs have different heights"
    );

    // Value of the frontier node at each level, starting with the leaf.
    let mut edge_values = vec![frontier.proof[0].value()];
    for (level, node) in frontier.proof.iter().enumerate().skip(1) {
        let edge = frontier.pos / ARITY.pow(level as u32 - 1);
        let data = child_values(node, edge / ARITY, edge, edge_values[level - 1])?;
        edge_values.push(<Sha3Digest as DigestAlgorithm<
            Commitment<Header>,
            u64,
            _,
        >>::digest(&data)?);
    }

    let mut proof = vec![target.proof[0].clone()];
    for (level, node) in target.proof.iter().enumerate().skip(1) {
        let divisor = ARITY.pow(level as u32 - 1);
        let edge = frontier.pos / divisor;
        let data = child_values(
            node,
            target.pos / divisor / ARITY,
            edge,
            edge_values[level - 1],
        )?;
        let children = data
            .iter()
            .enumerate()
            .map(|(n, value)| {
                let index = target.pos / divisor / ARITY * ARITY + n as u64;
                Arc::new(match index.cmp(&edge) {
                    Ordering::Greater => BlockMerkleNode::Empty,
                    _ => BlockMerkleNode::ForgettenSubtree { value: *value },
                })
            })
            .collect();
        proof.push(BlockMerkleNode::Branch {
            value: Sha3Node::default(),
            children,
        });
    }
    Ok(BlockMerkleProof {
        pos: target.pos,
        proof,
    })
}

const ARITY: u64 = BlockMerkleTree::ARITY as u64;

/// Child values of `node`, whose children have indices `parent * ARITY + n` at their level, in the
/// tree where the rightmost non-empty node of that level is `edge` with value `edge_value`.
fn child_values(
    node: &BlockMerkleNode,
    parent: u64,
    edge: u64,
    edge_value: Sha3Node,
) -> Result<Vec<Sha3Node>> {
    let BlockMerkleNode::Branch { children, .. } = node else {
        bail!("expected a branch node in the proof");
    };
    Ok(children
        .iter()
        .enumerate()
        .map(|(n, child)| match (parent * ARITY + n as u64).cmp(&edge) {
            Ordering::Less => child.value(),
            Ordering::Equal => edge_value,
            Ordering::Greater => Sha3Node::default(),
        })
        .collect())
}

pub(crate) async fn fetch_block_data_range<State>(
    state: &State,
    start: usize,
    end: usize,
    fetch_timeout: Duration,
    large_object_range_limit: usize,
) -> Result<
    Vec<(
        Header,
        PayloadQueryData<SeqTypes>,
        VidCommonQueryData<SeqTypes>,
    )>,
    Error,
>
where
    State: AvailabilityDataSource<SeqTypes>,
{
    if end <= start {
        return Err(Error::Custom {
            message: format!("requested empty interval [{start}, {end})"),
            status: StatusCode::BAD_REQUEST,
        });
    }
    if end - start > large_object_range_limit {
        return Err(Error::Custom {
            message: format!(
                "requested range [{start}, {end}) exceeds maximum size {large_object_range_limit}"
            ),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let fetch_headers = async move {
        state
            .get_header_range(start..end)
            .await
            .enumerate()
            .then(|(i, fetch)| async move {
                fetch
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| Error::Custom {
                        message: format!("missing header {}", start + i),
                        status: StatusCode::NOT_FOUND,
                    })
            })
            .try_collect::<Vec<_>>()
            .await
    };
    let fetch_payloads = async move {
        state
            .get_payload_range(start..end)
            .await
            .enumerate()
            .then(|(i, fetch)| async move {
                fetch
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| Error::Custom {
                        message: format!("missing payload {}", start + i),
                        status: StatusCode::NOT_FOUND,
                    })
            })
            .try_collect::<Vec<_>>()
            .await
    };
    let fetch_vid_commons = async move {
        state
            .get_vid_common_range(start..end)
            .await
            .enumerate()
            .then(|(i, fetch)| async move {
                fetch
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| Error::Custom {
                        message: format!("missing VID common {}", start + i),
                        status: StatusCode::NOT_FOUND,
                    })
            })
            .try_collect::<Vec<_>>()
            .await
    };
    let (headers, (payloads, vid_commons)) =
        try_join(fetch_headers, try_join(fetch_payloads, fetch_vid_commons)).await?;

    Ok(izip!(headers, payloads, vid_commons).collect())
}

/// Single-namespace version of [`get_namespaces_proof_range`].
pub(crate) async fn get_namespace_proof_range<State>(
    state: &State,
    start: usize,
    end: usize,
    namespace: u64,
    fetch_timeout: Duration,
    large_object_range_limit: usize,
) -> Result<Vec<NamespaceProof>, Error>
where
    State: AvailabilityDataSource<SeqTypes>,
{
    let blocks = get_namespaces_proof_range(
        state,
        start,
        end,
        &[namespace],
        fetch_timeout,
        large_object_range_limit,
    )
    .await?;
    Ok(blocks
        .into_iter()
        .map(|mut block| {
            block
                .remove(&namespace)
                .unwrap_or_else(NamespaceProof::not_present)
        })
        .collect())
}

pub(crate) async fn get_namespaces_proof_range<State>(
    state: &State,
    start: usize,
    end: usize,
    namespaces: &[u64],
    fetch_timeout: Duration,
    large_object_range_limit: usize,
) -> Result<Vec<HashMap<u64, NamespaceProof>>, Error>
where
    State: AvailabilityDataSource<SeqTypes>,
{
    fetch_block_data_range(state, start, end, fetch_timeout, large_object_range_limit)
        .await?
        .into_iter()
        .map(|(header, payload, vid_common)| {
            namespaces
                .iter()
                .filter_map(|&namespace| {
                    let ns_index = header.ns_table().find_ns_id(&namespace.into())?;
                    Some(
                        build_namespace_proof(&payload, &ns_index, &vid_common)
                            .map(|proof| (namespace, proof)),
                    )
                })
                .collect::<Result<HashMap<_, _>, _>>()
        })
        .collect()
}

/// Decode the `namespaces` path segment: a `TaggedBase64` string tagged `NS` wrapping a
/// JSON-encoded `Vec<u64>`.
pub(crate) fn parse_namespaces_str(encoded: &str) -> anyhow::Result<Vec<u64>> {
    let encoded: TaggedBase64 = encoded
        .parse()
        .map_err(|err| anyhow::anyhow!("invalid namespaces parameter: {err}"))?;
    if encoded.tag() != NAMESPACES_PARAM_TAG {
        anyhow::bail!(
            "invalid namespaces parameter tag: expected {NAMESPACES_PARAM_TAG}, got {}",
            encoded.tag()
        );
    }
    serde_json::from_slice(&encoded.value())
        .map_err(|err| anyhow::anyhow!("invalid namespaces parameter: {err}"))
}

/// Construct a [`NamespaceProof`] for the namespace at `ns_index` of the given block.
fn build_namespace_proof(
    payload: &PayloadQueryData<SeqTypes>,
    ns_index: &NsIndex,
    vid_common: &VidCommonQueryData<SeqTypes>,
) -> Result<NamespaceProof, Error> {
    let ns_proof =
        NsProof::new(payload.data(), ns_index, vid_common.common()).ok_or_else(|| {
            Error::Custom {
                message: "failed to construct namespace proof".into(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
    Ok(NamespaceProof::new(ns_proof, vid_common.common().clone()))
}

fn internal(err: impl Display) -> Error {
    Error::Custom {
        message: err.to_string(),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn not_found(msg: impl Into<String>) -> Error {
    Error::Custom {
        message: msg.into(),
        status: StatusCode::NOT_FOUND,
    }
}

fn chain_too_long(requested: usize, chain_limit: usize) -> Error {
    not_found(format!(
        "no finality proof found within {chain_limit} leaves of requested leaf {requested}"
    ))
}

#[cfg(test)]
mod test {
    use espresso_types::BLOCK_MERKLE_TREE_HEIGHT;
    use jf_merkle_tree_compat::prelude::SHA3MerkleTree;

    use super::*;

    // The light-weight block tree forgets all but its last leaf.
    type FullTree = SHA3MerkleTree<Commitment<Header>>;

    fn fake_tree(num_leaves: u64) -> FullTree {
        FullTree::from_elems(
            Some(BLOCK_MERKLE_TREE_HEIGHT),
            (0..num_leaves).map(|i| Commitment::<Header>::from_raw([i as u8 + 1; 32])),
        )
        .unwrap()
    }

    fn lookup(tree: &FullTree, pos: u64) -> BlockMerkleProof {
        tree.lookup(pos).expect_ok().unwrap().1
    }

    /// Rebuilding from `head_leaves` leaves matches the tree of `r` leaves for every `r` in `rs`.
    fn check_rebuild(head_leaves: u64, rs: impl IntoIterator<Item = u64>) {
        let head = fake_tree(head_leaves);
        for r in rs {
            let tree = fake_tree(r);
            let frontier = lookup(&head, r - 1);
            for i in 0..r {
                let proof = rebuild_block_proof(r, &frontier, &lookup(&head, i)).unwrap();
                assert_eq!(proof, lookup(&tree, i), "r={r} i={i}");
                assert!(
                    BlockMerkleTree::verify(tree.commitment(), i, &proof)
                        .unwrap()
                        .is_ok()
                );
            }
        }
    }

    #[test]
    fn test_rebuild_block_proof_matches_snapshot() {
        check_rebuild(100, [2, 4, 5, 10, 26, 28, 50, 99]);
    }

    #[test]
    fn test_rebuild_block_proof_last_leaf() {
        let head = fake_tree(40);
        let tree = fake_tree(13);
        let proof = rebuild_block_proof(13, &lookup(&head, 12), &lookup(&head, 12)).unwrap();
        assert_eq!(proof, lookup(&tree, 12));
    }

    #[test]
    fn test_rebuild_block_proof_full_subtree() {
        check_rebuild(100, [3, 9, 27, 81]);
    }

    #[test]
    fn test_rebuild_block_proof_single_leaf() {
        check_rebuild(5, [1]);
    }

    #[test]
    fn test_rebuild_block_proof_wrong_frontier() {
        let head = fake_tree(40);
        let err = rebuild_block_proof(13, &lookup(&head, 11), &lookup(&head, 3)).unwrap_err();
        assert!(err.to_string().contains("frontier"), "{err:#}");
    }
}

#[cfg(test)]
mod postgres;
#[cfg(test)]
mod sqlite;
