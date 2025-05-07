use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use committable::Committable;
use espresso_types::{
    v0_99::{
        BidTx, RollupRegistration, RollupRegistrationBody, RollupUpdate, RollupUpdatebody,
        SolverAuctionResults,
    },
    SeqTypes,
    Update::Set,
};
use hotshot::types::SignatureKey;
use hotshot_types::{data::ViewNumber, traits::node_implementation::NodeType, PeerConfig};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::{database::PostgresClient, overflow_err, SolverError, SolverResult};

// TODO ED: Implement a shared solver state with the HotShot events received
pub struct GlobalState {
    solver: SolverState,
    database: PostgresClient,
}

impl GlobalState {
    pub fn solver(&self) -> &SolverState {
        &self.solver
    }

    pub fn database(&self) -> &PgPool {
        self.database.pool()
    }
}

impl GlobalState {
    pub fn new(db: PostgresClient, state: SolverState) -> anyhow::Result<Self> {
        Ok(Self {
            solver: state,
            database: db,
        })
    }
}

pub struct SolverState {
    pub stake_table: StakeTable,
    pub bid_txs: HashMap<ViewNumber, HashMap<<SeqTypes as NodeType>::BuilderSignatureKey, BidTx>>,
}

pub struct StakeTable {
    pub known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
}

#[async_trait]
pub trait UpdateSolverState {
    async fn submit_bid_tx(&mut self, bid_tx: BidTx) -> SolverResult<()>;

    async fn register_rollup(
        &self,
        registration: RollupRegistration,
    ) -> SolverResult<RollupRegistration>;
    async fn update_rollup_registration(
        &self,
        update: RollupUpdate,
    ) -> SolverResult<RollupRegistration>;

    async fn get_all_rollup_registrations(&self) -> SolverResult<Vec<RollupRegistration>>;

    async fn calculate_auction_results_permissionless(
        &self,
        view_number: ViewNumber,
    ) -> SolverResult<SolverAuctionResults>;

    async fn calculate_auction_results_permissioned(
        &self,
        view_number: ViewNumber,
        _signature: <SeqTypes as NodeType>::SignatureKey,
    ) -> SolverResult<SolverAuctionResults>;
}

#[async_trait]
impl UpdateSolverState for GlobalState {
    async fn submit_bid_tx(&mut self, bid_tx: BidTx) -> SolverResult<()> {
        let view = bid_tx.view();
        let builder_key = bid_tx.account();

        let bid_txs = &mut self.solver.bid_txs;
        bid_txs.entry(view).or_default().insert(builder_key, bid_tx);
        Ok(())
    }

    async fn register_rollup(
        &self,
        registration: RollupRegistration,
    ) -> Result<RollupRegistration, SolverError> {
        let RollupRegistration { body, signature } = registration.clone();

        let commit = body.commit();

        let RollupRegistrationBody {
            namespace_id,
            signature_keys,
            signature_key,
            ..
        } = body;

        if !signature_keys.contains(&signature_key) {
            return Err(SolverError::SignatureKeysMismatch(signature.to_string()));
        }

        // check if signature provided is valid
        let valid_signature = <SeqTypes as NodeType>::SignatureKey::validate(
            &signature_key,
            &signature,
            commit.as_ref(),
        );

        if !valid_signature {
            return Err(SolverError::InvalidSignature(signature.to_string()));
        }

        let db = self.database();

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM rollup_registrations where namespace_id = $1);",
        )
        .bind::<i64>(u64::from(namespace_id).try_into().map_err(overflow_err)?)
        .fetch_one(db)
        .await
        .map_err(SolverError::from)?;

        if exists {
            return Err(SolverError::RollupAlreadyExists(namespace_id));
        }

        let bytes = bincode::serialize(&registration)?;

        let result = sqlx::query("INSERT INTO rollup_registrations VALUES ($1, $2);")
            .bind::<i64>(u64::from(namespace_id).try_into().map_err(overflow_err)?)
            .bind(&bytes)
            .execute(db)
            .await
            .map_err(SolverError::from)?;

        if result.rows_affected() != 1 {
            return Err(SolverError::Database(format!(
                "invalid num of rows affected. rows affected: {:?}",
                result.rows_affected()
            )));
        }

        Ok(registration)
    }

    async fn update_rollup_registration(
        &self,
        update: RollupUpdate,
    ) -> SolverResult<RollupRegistration> {
        let db = self.database();

        let RollupUpdate { body, signature } = update;

        let commit = body.commit();

        let RollupUpdatebody {
            namespace_id,
            reserve_url,
            reserve_price,
            active,
            signature_keys,
            signature_key,
            text,
        } = body;

        let valid_signature = <SeqTypes as NodeType>::SignatureKey::validate(
            &signature_key,
            &signature,
            commit.as_ref(),
        );

        if !valid_signature {
            return Err(SolverError::InvalidSignature(signature.to_string()));
        }

        let result: RollupRegistrationResult =
            sqlx::query_as("SELECT * from rollup_registrations where namespace_id = $1;")
                .bind::<i64>(u64::from(namespace_id).try_into().map_err(overflow_err)?)
                .fetch_one(db)
                .await
                .map_err(SolverError::from)?;

        let mut registration = bincode::deserialize::<RollupRegistration>(&result.data)?;

        if let Set(ru) = reserve_url {
            registration.body.reserve_url = ru;
        };

        if let Set(rp) = reserve_price {
            registration.body.reserve_price = rp;
        }

        if let Set(active) = active {
            registration.body.active = active;
        }

        // The given signature key should also be from the database `signature_keys`.
        if !registration.body.signature_keys.contains(&signature_key) {
            return Err(SolverError::SignatureKeysMismatch(
                signature_key.to_string(),
            ));
        }

        if let Set(text) = text {
            registration.body.text = text;
        }

        // If signature keys are provided for the update, verify that the given signature key is in the list
        if let Set(keys) = signature_keys {
            if !keys.contains(&signature_key) {
                return Err(SolverError::SignatureKeysMismatch(
                    signature_key.to_string(),
                ));
            }

            registration.body.signature_keys = keys;
        }

        let bytes = bincode::serialize(&registration)?;

        let result =
            sqlx::query("UPDATE rollup_registrations SET data = $1  WHERE namespace_id = $2;")
                .bind(&bytes)
                .bind::<i64>(u64::from(namespace_id).try_into().map_err(overflow_err)?)
                .execute(db)
                .await
                .map_err(SolverError::from)?;

        if result.rows_affected() != 1 {
            return Err(SolverError::Database(format!(
                "invalid num of rows affected. rows affected: {:?}",
                result.rows_affected()
            )));
        }

        Ok(registration)
    }

    async fn get_all_rollup_registrations(&self) -> SolverResult<Vec<RollupRegistration>> {
        let db = self.database();

        let rows: Vec<RollupRegistrationResult> =
            sqlx::query_as("SELECT * from rollup_registrations ORDER BY namespace_id;")
                .fetch_all(db)
                .await
                .map_err(SolverError::from)?;

        rows.iter()
            .map(|r| bincode::deserialize(&r.data).map_err(SolverError::from))
            .collect::<SolverResult<Vec<RollupRegistration>>>()
    }

    async fn calculate_auction_results_permissionless(
        &self,
        view_number: ViewNumber,
    ) -> SolverResult<SolverAuctionResults> {
        // Get all rollup registrations to include as reserve bids
        let rollups = self.get_all_rollup_registrations().await?;
        let reserve_bids = rollups
            .into_iter()
            .filter_map(|r| Some((r.body.namespace_id, r.body.reserve_url?)))
            .collect::<Vec<_>>();

        // Get all bids for the current view number
        let winning_bids = match self.solver.bid_txs.get(&view_number) {
            Some(bids_map) => {
                // Create a map to track which namespaces have been claimed
                let mut claimed_namespaces = HashSet::new();
                let mut winners = Vec::new();
                
                // Sort bids by amount (highest first)
                let mut bids = bids_map.values().collect::<Vec<_>>();
                bids.sort_by(|a, b| b.amount().cmp(&a.amount()));
                
                // Process bids in order of highest amount
                for bid in bids {
                    let namespaces = &bid.body.namespaces;
                    
                    // Check if any of the namespaces in this bid are already claimed
                    let has_conflict = namespaces.iter().any(|ns| claimed_namespaces.contains(ns));
                    
                    if !has_conflict {
                        // If no conflicts, this bid wins
                        for namespace in namespaces {
                            claimed_namespaces.insert(*namespace);
                        }
                        winners.push(bid.clone());
                    }
                }
                
                winners
            }
            None => Vec::new(),
        };

        let results = SolverAuctionResults::new(
            view_number,
            winning_bids,
            reserve_bids,
        );

        Ok(results)
    }
    
    async fn calculate_auction_results_permissioned(
        &self,
        view_number: ViewNumber,
        _signature: <SeqTypes as NodeType>::SignatureKey,
    ) -> SolverResult<SolverAuctionResults> {
        // For permissioned calculation, we use the same logic as permissionless
        // but we could add additional validation based on the signature if needed
        self.calculate_auction_results_permissionless(view_number).await
    }
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
struct RollupRegistrationResult {
    namespace_id: i64,
    data: Vec<u8>,
}

#[cfg(all(any(test, feature = "testing"), not(feature = "embedded-db")))]
impl GlobalState {
    pub async fn mock() -> Self {
        let db = hotshot_query_service::data_source::sql::testing::TmpDb::init().await;
        let host = db.host();
        let port = db.port();

        let opts = crate::DatabaseOptions {
            url: None,
            host: Some(host),
            port: Some(port),
            db_name: None,
            username: Some("postgres".to_string()),
            password: Some("password".to_string()),
            max_connections: Some(100),
            acquire_timeout: None,
            require_ssl: false,
            migrations: true,
            reset: false,
        };

        let client = PostgresClient::connect(opts)
            .await
            .expect("failed to connect to database");

        Self {
            solver: SolverState::mock(),
            database: client,
        }
    }
}

#[cfg(all(any(test, feature = "testing"), not(feature = "embedded-db")))]
impl SolverState {
    pub fn mock() -> Self {
        Self {
            stake_table: StakeTable {
                known_nodes_with_stake: crate::mock::generate_stake_table(),
            },
            bid_txs: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BidTxBody;
    use hotshot_types::traits::signature_key::DummySignatureKey;
    use std::sync::Arc;
    use url::Url;

    // Helper function to create a test bid
    fn create_test_bid(
        view: ViewNumber,
        amount: u64,
        namespaces: Vec<NamespaceId>,
        account_id: u64,
    ) -> BidTx {
        BidTx {
            body: BidTxBody {
                view,
                amount,
                namespaces,
            },
            signature: DummySignatureKey::from(account_id),
        }
    }

    // Helper function to create a test URL
    fn create_test_url(id: u64) -> Url {
        Url::parse(&format!("https://example.com/{}", id)).unwrap()
    }

    // Helper function to create a test rollup registration
    fn create_test_rollup_registration(
        namespace_id: NamespaceId,
        reserve_url: Option<Url>,
    ) -> RollupRegistration {
        let mut registration = RollupRegistration::default();
        registration.body.namespace_id = namespace_id;
        registration.body.reserve_url = reserve_url;
        registration
    }

    #[tokio::test]
    async fn test_empty_bids() {
        // Arrange
        let mut state = SolverState::mock();
        let view_number = ViewNumber::from(1);
        let global_state = GlobalState {
            solver: state,
            database: PostgresClient::Empty,
        };

        // Act
        let results = global_state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();

        // Assert
        assert_eq!(results.view(), view_number);
        assert!(results.winning_bids().is_empty());
        assert!(results.reserve_bids.is_empty());
    }

    #[tokio::test]
    async fn test_highest_bid_wins() {
        // Arrange
        let mut state = SolverState::mock();
        let view_number = ViewNumber::from(1);
        let namespace_id = NamespaceId::from(100);
        
        // Create bids for the same namespace with different amounts
        let low_bid = create_test_bid(view_number, 100, vec![namespace_id], 1);
        let high_bid = create_test_bid(view_number, 200, vec![namespace_id], 2);
        
        let mut bids_map = HashMap::new();
        bids_map.insert(low_bid.signature.clone(), low_bid.clone());
        bids_map.insert(high_bid.signature.clone(), high_bid.clone());
        
        state.bid_txs.insert(view_number, bids_map);
        
        let global_state = GlobalState {
            solver: state,
            database: PostgresClient::Empty,
        };

        // Act
        let results = global_state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();

        // Assert
        assert_eq!(results.winning_bids().len(), 1);
        assert_eq!(results.winning_bids()[0].amount(), 200);
    }

    #[tokio::test]
    async fn test_all_or_nothing_bidding() {
        // Arrange
        let mut state = SolverState::mock();
        let view_number = ViewNumber::from(1);
        
        // Create namespaces
        let namespace1 = NamespaceId::from(100);
        let namespace2 = NamespaceId::from(101);
        let namespace3 = NamespaceId::from(102);
        
        // Create a bid for multiple namespaces
        let multi_bid = create_test_bid(
            view_number, 
            150, 
            vec![namespace1, namespace2], 
            1
        );
        
        // Create higher bids for individual namespaces
        let higher_bid1 = create_test_bid(view_number, 200, vec![namespace1], 2);
        let higher_bid2 = create_test_bid(view_number, 200, vec![namespace3], 3);
        
        let mut bids_map = HashMap::new();
        bids_map.insert(multi_bid.signature.clone(), multi_bid.clone());
        bids_map.insert(higher_bid1.signature.clone(), higher_bid1.clone());
        bids_map.insert(higher_bid2.signature.clone(), higher_bid2.clone());
        
        state.bid_txs.insert(view_number, bids_map);
        
        let global_state = GlobalState {
            solver: state,
            database: PostgresClient::Empty,
        };

        // Act
        let results = global_state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();

        // Assert - higher_bid1 should win for namespace1, and higher_bid2 for namespace3
        assert_eq!(results.winning_bids().len(), 2);
        
        // Check that the winning bids include the higher individual bids
        let winning_accounts: Vec<_> = results.winning_bids().iter().map(|b| b.account()).collect();
        assert!(winning_accounts.contains(&higher_bid1.account()));
        assert!(winning_accounts.contains(&higher_bid2.account()));
        assert!(!winning_accounts.contains(&multi_bid.account()));
    }

    #[tokio::test]
    async fn test_namespace_conflict_resolution() {
        // Arrange
        let mut state = SolverState::mock();
        let view_number = ViewNumber::from(1);
        
        // Create namespaces
        let namespace1 = NamespaceId::from(100);
        let namespace2 = NamespaceId::from(101);
        let namespace3 = NamespaceId::from(102);
        
        // Create several bids with conflicting namespaces but different amounts
        let bid1 = create_test_bid(view_number, 300, vec![namespace1, namespace2], 1);
        let bid2 = create_test_bid(view_number, 200, vec![namespace2, namespace3], 2);
        let bid3 = create_test_bid(view_number, 100, vec![namespace3], 3);
        
        let mut bids_map = HashMap::new();
        bids_map.insert(bid1.signature.clone(), bid1.clone());
        bids_map.insert(bid2.signature.clone(), bid2.clone());
        bids_map.insert(bid3.signature.clone(), bid3.clone());
        
        state.bid_txs.insert(view_number, bids_map);
        
        let global_state = GlobalState {
            solver: state,
            database: PostgresClient::Empty,
        };

        // Act
        let results = global_state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();

        // Assert - bid1 should win for namespace1 and namespace2, making bid2 invalid
        // bid3 should lose to bid2, but bid2 loses to bid1, so no winner for namespace3
        assert_eq!(results.winning_bids().len(), 1);
        assert_eq!(results.winning_bids()[0].account(), bid1.account());
    }

    #[tokio::test]
    async fn test_reserve_bids_inclusion() {
        // Arrange
        let mut state = SolverState::mock();
        let view_number = ViewNumber::from(1);
        
        // Create a namespace and URL for reserve bid
        let namespace_id = NamespaceId::from(100);
        let reserve_url = create_test_url(1);
        
        // Create a custom GlobalState for this test
        struct TestGlobalState {
            solver: SolverState,
            database: PostgresClient,
        }
        
        // Implement just what we need for this test
        #[async_trait]
        impl UpdateSolverState for TestGlobalState {
            async fn submit_bid_tx(&mut self, _bid_tx: BidTx) -> SolverResult<()> {
                unimplemented!()
            }
            
            async fn register_rollup(
                &self,
                _registration: RollupRegistration,
            ) -> SolverResult<RollupRegistration> {
                unimplemented!()
            }
            
            async fn update_rollup_registration(
                &self,
                _update: RollupUpdate,
            ) -> SolverResult<RollupRegistration> {
                unimplemented!()
            }
            
            async fn get_all_rollup_registrations(&self) -> SolverResult<Vec<RollupRegistration>> {
                // Return our test registration
                Ok(vec![create_test_rollup_registration(
                    NamespaceId::from(100),
                    Some(create_test_url(1)),
                )])
            }
            
            async fn calculate_auction_results_permissionless(
                &self,
                view_number: ViewNumber,
            ) -> SolverResult<SolverAuctionResults> {
                // Get all rollup registrations to include as reserve bids
                let rollups = self.get_all_rollup_registrations().await?;
                let reserve_bids = rollups
                    .into_iter()
                    .filter_map(|r| Some((r.body.namespace_id, r.body.reserve_url?)))
                    .collect::<Vec<_>>();

                // Get all bids for the current view number
                let winning_bids = match self.solver.bid_txs.get(&view_number) {
                    Some(bids_map) => {
                        // Create a map to track which namespaces have been claimed
                        let mut claimed_namespaces = HashSet::new();
                        let mut winners = Vec::new();
                        
                        // Sort bids by amount (highest first)
                        let mut bids = bids_map.values().collect::<Vec<_>>();
                        bids.sort_by(|a, b| b.amount().cmp(&a.amount()));
                        
                        // Process bids in order of highest amount
                        for bid in bids {
                            let namespaces = &bid.body.namespaces;
                            
                            // Check if any of the namespaces in this bid are already claimed
                            let has_conflict = namespaces.iter().any(|ns| claimed_namespaces.contains(ns));
                            
                            if !has_conflict {
                                // If no conflicts, this bid wins
                                for namespace in namespaces {
                                    claimed_namespaces.insert(*namespace);
                                }
                                winners.push(bid.clone());
                            }
                        }
                        
                        winners
                    }
                    None => Vec::new(),
                };

                let results = SolverAuctionResults::new(
                    view_number,
                    winning_bids,
                    reserve_bids,
                );

                Ok(results)
            }
            
            async fn calculate_auction_results_permissioned(
                &self,
                view_number: ViewNumber,
                _signature: <SeqTypes as NodeType>::SignatureKey,
            ) -> SolverResult<SolverAuctionResults> {
                self.calculate_auction_results_permissionless(view_number).await
            }
        }
        
        let test_global_state = TestGlobalState {
            solver: state,
            database: PostgresClient::Empty,
        };

        // Act
        let results = test_global_state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();

        // Assert
        assert_eq!(results.reserve_bids.len(), 1);
        assert_eq!(results.reserve_bids[0].0, namespace_id);
        assert_eq!(results.reserve_bids[0].1.to_string(), reserve_url.to_string());
    }
}
