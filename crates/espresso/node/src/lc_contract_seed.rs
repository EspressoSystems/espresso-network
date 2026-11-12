//! Seeds the post-start epoch catchup from the LC contract.

use std::{
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};

use anyhow::{Context, anyhow, bail};
use espresso_types::{
    ChainConfig, Header, LcContractFinalizedState, SeqTypes, traits::SequencerPersistence,
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream};
use hotshot_types::{data::EpochNumber, epoch_membership::EpochMembershipCoordinator};

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Bound on the peer requests of one seed: with no answering peer the attempt fails within it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The LC-contract seed of the catchup, behind `CatchupSeedFn`; steps in
/// `doc/agents/architecture.md`.
///
/// `F` and `blockCommRoot` come from the SNARK-verified LC contract. Every installed table comes
/// from a header proven against the block Merkle root of a header at `F` that `verify_header`
/// accepted. Proven heights are below `F`, installed epochs above `start`, and every table has a
/// block reward.
#[derive(Clone)]
pub struct LcContractSeed<P: SequencerPersistence> {
    /// Set by `bind_requests` once consensus exists; weak, so the seed owns no path to the
    /// coordinator that the protocol holds.
    requests: Arc<OnceLock<Weak<RequestResponseProtocol<P>>>>,
}

impl<P: SequencerPersistence> LcContractSeed<P> {
    /// A seed without a request-response protocol, `None` when `chain_config` names no stake
    /// table contract to read the LC contract from.
    pub fn new(chain_config: &ChainConfig) -> Option<Self> {
        chain_config.stake_table_contract.map(|_| Self {
            requests: Arc::default(),
        })
    }

    /// Binds the request-response protocol, which exists only after consensus is constructed.
    pub fn bind_requests(&self, requests: &Arc<RequestResponseProtocol<P>>) {
        self.requests
            .set(Arc::downgrade(requests))
            .expect("the request-response protocol is bound once");
    }

    /// Sets the catchup hook of `coordinator` to this seed. An attempt before `bind_requests`
    /// fails and the next catchup retries it.
    pub fn install(self, coordinator: &EpochMembershipCoordinator<SeqTypes>) {
        let seed = Arc::new(self);
        coordinator.set_catchup_seed(Arc::new(move |target, start, coordinator| {
            let seed = Arc::clone(&seed);
            async move { seed.run(target, start, coordinator).await }.boxed()
        }));
    }

    /// One seed attempt for `catchup(target)` with held pair top `start`: the walk start, `start`
    /// when not needed, `E* + 1` when seeded. The L1 read, the peer requests and each install are
    /// bounded.
    async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<EpochNumber> {
        let started = tokio::time::Instant::now();
        let lc = coordinator
            .membership()
            .fetcher()
            .lc_contract_finalized_state()
            .await?;
        let epoch_height = *coordinator.epoch_height();
        let seed_epoch = lc.seed_epoch(epoch_height);
        let log = |outcome: &str| {
            tracing::info!(
                target: "announce::catchup",
                %target,
                held_start = %start,
                lc_contract_height = lc.height,
                %seed_epoch,
                outcome,
                duration_ms = started.elapsed().as_millis() as u64,
                "lc contract seed"
            );
        };
        if !lc.seeds_target(epoch_height, target, start) {
            log("not needed");
            return Ok(start);
        }

        let (preceding, roots) = tokio::time::timeout(
            REQUEST_TIMEOUT,
            self.fetch_headers(&lc, epoch_height, start),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "seed requests for the LC contract header and proofs exceeded {REQUEST_TIMEOUT:?}"
            )
        })??;
        coordinator
            .membership()
            .install_epoch_roots(preceding, roots, &coordinator)
            .await?;

        log("seeded");
        Ok(seed_epoch + 1)
    }

    /// The header at `F`, accepted by `lc.verify_header`, then the headers it proves: the
    /// preceding root (`lc.preceding_root_height`, the block time source of the lowest installed
    /// reward) and `lc.proven_root_heights`, ascending. A rejected answer is skipped and the request
    /// keeps waiting (`request_indefinitely`).
    async fn fetch_headers(
        &self,
        lc: &LcContractFinalizedState,
        epoch_height: u64,
        start: EpochNumber,
    ) -> anyhow::Result<(Option<Header>, Vec<Header>)> {
        let protocol = self
            .requests
            .get()
            .and_then(Weak::upgrade)
            .context("the request-response protocol is not available")?;
        let checked_lc = lc.clone();
        let lc_header = protocol
            .request_indefinitely(
                Request::Leaf(lc.height),
                move |_: &Request, response: Response| {
                    let lc = checked_lc.clone();
                    async move {
                        let Response::Leaf(leaves) = response else {
                            bail!("expected leaf response");
                        };
                        let header = leaves
                            .first()
                            .context("empty leaf chain")?
                            .block_header()
                            .clone();
                        lc.verify_header(&header)?;
                        Ok(header)
                    }
                    .map(|result| {
                        result.inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                    })
                },
            )
            .await?;

        let root = lc_header.block_merkle_tree_root();
        let preceding = lc.preceding_root_height(epoch_height, start);
        let heights = preceding
            .into_iter()
            .chain(lc.proven_root_heights(epoch_height, start));
        let mut headers: Vec<Header> = stream::iter(heights)
            .map(|height| {
                let protocol = Arc::clone(&protocol);
                async move {
                    protocol
                        .request_indefinitely(
                            Request::HeaderProof {
                                root: lc.height,
                                height,
                            },
                            move |_: &Request, response: Response| {
                                async move {
                                    let Response::HeaderProof(proof) = response else {
                                        bail!("expected header proof response");
                                    };
                                    proof.verify_at_height(root, height)
                                }
                                .map(|result| {
                                    result.inspect_err(|err| {
                                        tracing::debug!("header proof rejected: {err:#}")
                                    })
                                })
                            },
                        )
                        .await
                        .map_err(anyhow::Error::from)
                }
            })
            .buffered(MAX_CONCURRENT_REQUESTS)
            .try_collect()
            .await?;
        let preceding = preceding.map(|_| headers.remove(0));
        Ok((preceding, headers))
    }
}
