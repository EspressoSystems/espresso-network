use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_lock::RwLock;
use async_trait::async_trait;
use espresso_types::v0_99::BidTxBody;
use tokio::{spawn, time::sleep};

use espresso_types::v0_99::RollupRegistration;

use espresso_types::MarketplaceVersion;
use espresso_types::SeqTypes;
use hotshot::types::LegacyEventType;

use hotshot::types::LegacyEvent;

use hotshot_types::traits::node_implementation::Versions;
use marketplace_builder_core::hooks::BuilderHooks;

use espresso_types::FeeAmount;

use espresso_types::eth_signature_key::EthKeyPair;

use espresso_types::NamespaceId;

use hotshot_types::traits::node_implementation::NodeType;

use marketplace_solver::SolverError;
use marketplace_solver::SOLVER_API_PATH;
use sequencer::SequencerApiVersion;
use surf_disco::Client;

use tide_disco::Url;
use tracing::error;
use tracing::info;

/// Configurations for bid submission.
pub struct BidConfig {
    /// Namespace IDs to filter and bid for.
    pub namespaces: Vec<NamespaceId>,
    /// Amount to bid.
    pub amount: FeeAmount,
}

pub fn connect_to_solver(solver_base_url: Url) -> Client<SolverError, MarketplaceVersion> {
    Client::<SolverError, MarketplaceVersion>::new(solver_base_url.join(SOLVER_API_PATH).unwrap())
}

/// Fetch registered namespaces from the solver and construct the list of namespaces to skip.
///
/// # Returns
/// - `Some` namespaces if the fetching succeeds, even if the list is empty.
/// - `None` if the fetching fails.
pub async fn fetch_namespaces_to_skip(solver_base_url: Url) -> Option<HashSet<NamespaceId>> {
    let solver_client = connect_to_solver(solver_base_url);
    match solver_client
        .get::<Vec<RollupRegistration>>("rollup_registrations")
        .send()
        .await
    {
        Ok(registrations) => {
            let mut namespaces_to_skip = HashSet::new();
            for registration in registrations {
                if registration.body.reserve_url.is_some() || !registration.body.active {
                    namespaces_to_skip.insert(registration.body.namespace_id);
                }
            }
            Some(namespaces_to_skip)
        }
        Err(e) => {
            error!("Failed to get the registered rollups: {:?}.", e);
            None
        }
    }
}

/// Reserve builder hooks for espresso sequencer.
///
/// Provides bidding and transaction filtering on top of base builder functionality.
pub(crate) struct EspressoReserveHooks {
    /// IDs of namespaces to filter and bid for
    pub(crate) namespaces: HashSet<NamespaceId>,
    /// Base API to contact the solver
    pub(crate) solver_base_url: Url,
    /// Builder API base to include in the bid
    pub(crate) builder_api_base_url: Url,
    /// Keys for bidding
    pub(crate) bid_key_pair: EthKeyPair,
    /// Bid amount
    pub(crate) bid_amount: FeeAmount,
}

#[async_trait]
impl BuilderHooks<SeqTypes> for EspressoReserveHooks {
    #[inline(always)]
    async fn process_transactions(
        &self,
        mut transactions: Vec<<SeqTypes as NodeType>::Transaction>,
    ) -> Vec<<SeqTypes as NodeType>::Transaction> {
        transactions.retain(|txn| self.namespaces.contains(&txn.namespace()));
        transactions
    }

    #[inline(always)]
    async fn handle_hotshot_event(&self, event: &LegacyEvent<SeqTypes>) {
        let LegacyEventType::ViewFinished { view_number } = event.event else {
            return;
        };

        let fee_account = self.bid_key_pair.fee_account();
        let bid_amount = self.bid_amount;
        let namespaces = self.namespaces.iter().cloned().collect();
        let builder_api_base_url = self.builder_api_base_url.clone();
        let bid_key_pair = self.bid_key_pair.clone();
        let solver_base_url = self.solver_base_url.clone();

        spawn(async move {
            let bid_tx = match BidTxBody::new(
                fee_account,
                bid_amount,
                view_number + 3, // We submit a bid 3 views in advance.
                namespaces,
                builder_api_base_url,
                Default::default(),
            )
            .signed(&bid_key_pair)
            {
                Ok(bid) => bid,
                Err(e) => {
                    error!("Failed to sign the bid txn: {:?}.", e);
                    return;
                }
            };

            let solver_client = connect_to_solver(solver_base_url);
            if let Err(e) = solver_client
                .post::<()>("submit_bid")
                .body_json(&bid_tx)
                .unwrap()
                .send()
                .await
            {
                error!("Failed to submit the bid: {:?}.", e);
                return;
            }

            info!("Submitted bid for view {}", *view_number);
        });
    }
}

/// Fallback builder hooks for espresso sequencer.
///
/// Provides transaction filtering on top of base builder functionality for unregistered rollups.
pub(crate) struct EspressoFallbackHooks {
    /// Base URL to contact the solver.
    pub(crate) solver_base_url: Url,
    pub(crate) namespaces_to_skip: Arc<RwLock<Option<HashSet<NamespaceId>>>>,
}

#[async_trait]
impl BuilderHooks<SeqTypes> for EspressoFallbackHooks {
    #[inline(always)]
    async fn process_transactions(
        &self,
        mut transactions: Vec<<SeqTypes as NodeType>::Transaction>,
    ) -> Vec<<SeqTypes as NodeType>::Transaction> {
        let namespaces_to_skip = self.namespaces_to_skip.read().await;

        match namespaces_to_skip.as_ref() {
            Some(namespaces_to_skip) => {
                transactions.retain(|txn| !namespaces_to_skip.contains(&txn.namespace()));
                transactions
            }
            // Solver connection has failed and we don't have up-to-date information on this
            None => {
                error!("Not accepting transactions due to outdated information");
                Vec::new()
            }
        }
    }

    #[inline(always)]
    async fn handle_hotshot_event(&self, event: &LegacyEvent<SeqTypes>) {
        let LegacyEventType::ViewFinished { view_number } = event.event else {
            return;
        };

        // Re-query the solver every 20 views
        if view_number.rem_euclid(20) != 0 {
            return;
        }

        let solver_base_url = self.solver_base_url.clone();
        let namespaces_to_skip_lock = Arc::clone(&self.namespaces_to_skip);

        spawn(async move {
            let namespaces_to_skip = fetch_namespaces_to_skip(solver_base_url).await;
            *namespaces_to_skip_lock.write().await = namespaces_to_skip;
        });
    }
}
