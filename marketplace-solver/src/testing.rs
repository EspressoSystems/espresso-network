#![cfg(all(
    any(test, feature = "testing"),
    not(target_os = "windows"),
    not(feature = "embedded-db")
))]
#![allow(dead_code)]
use std::sync::Arc;

use async_lock::RwLock;
use espresso_types::MarketplaceVersion;
use hotshot_query_service::data_source::sql::testing::TmpDb;
use portpicker::pick_unused_port;
use tide_disco::{App, Url};
use tokio::{spawn, task::JoinHandle};
use vbs::version::StaticVersionType;

use crate::{
    database::{mock::setup_mock_database, PostgresClient},
    define_api, handle_events,
    mock::run_mock_event_service,
    state::{GlobalState, SolverState, StakeTable},
    EventsServiceClient, SolverError, SOLVER_API_PATH,
};

pub struct MockSolver {
    pub events_url: Url,
    /// Solver base URL, forming the path to the solver API, with `SOLVER_API_PATH`.
    pub solver_url: Url,
    pub state: Arc<RwLock<GlobalState>>,
    pub database: PostgresClient,
    pub handles: Vec<JoinHandle<()>>,
    pub tmp_db: Arc<TmpDb>,
}

impl MockSolver {
    pub fn solver_api(&self) -> Url {
        self.solver_url.clone().join(SOLVER_API_PATH).unwrap()
    }

    pub fn state(&self) -> Arc<RwLock<GlobalState>> {
        self.state.clone()
    }
}

impl Drop for MockSolver {
    fn drop(&mut self) {
        println!("canceling handles");

        while let Some(handle) = self.handles.pop() {
            handle.abort();
        }
    }
}

impl MockSolver {
    pub async fn init() -> Self {
        let (tmp_db, database) = setup_mock_database().await;
        Self::with_db((Arc::new(tmp_db), database)).await
    }

    pub async fn with_db((tmp_db, database): (Arc<TmpDb>, PostgresClient)) -> Self {
        let (events_url, event_api_handle, generate_events_handle) = run_mock_event_service();

        let client = EventsServiceClient::new(events_url.clone()).await;
        let startup_info = client.get_startup_info().await.unwrap();
        let stream = client.get_event_stream().await.unwrap();

        let solver_state = SolverState {
            stake_table: StakeTable {
                known_nodes_with_stake: startup_info.known_node_with_stake,
            },
            bid_txs: Default::default(),
        };

        let state = Arc::new(RwLock::new(
            GlobalState::new(database.clone(), solver_state).unwrap(),
        ));

        let event_handler_handle = spawn({
            let state = state.clone();
            async move {
                let _ = handle_events(stream, state).await;
            }
        });

        let mut app = App::<_, SolverError>::with_state(state.clone());
        app.with_version(env!("CARGO_PKG_VERSION").parse().unwrap());

        let mut api = define_api(Default::default()).unwrap();
        api.with_version(env!("CARGO_PKG_VERSION").parse().unwrap());

        app.register_module::<SolverError, MarketplaceVersion>(SOLVER_API_PATH, api)
            .unwrap();

        let solver_api_port = pick_unused_port().expect("no free port");
        let solver_url: Url = Url::parse(&format!("http://localhost:{solver_api_port}")).unwrap();

        let solver_api_handle = spawn({
            let solver_url = solver_url.clone();
            async move {
                let _ = app.serve(solver_url, MarketplaceVersion::instance()).await;
            }
        });

        let handles = vec![
            generate_events_handle,
            event_handler_handle,
            event_api_handle,
            solver_api_handle,
        ];

        MockSolver {
            events_url,
            solver_url,
            state,
            database,
            tmp_db,
            handles,
        }
    }
}

#[cfg(all(test, not(feature = "embedded-db")))]
mod test {
    use std::{str::FromStr, time::Duration};

    use committable::Committable;
    use espresso_types::{
        v0_99::{
            BidTx, BidTxBody, RollupRegistration, RollupRegistrationBody, RollupUpdate, RollupUpdatebody,
        },
        FeeAccount, MarketplaceVersion, SeqTypes,
        Update::{Set, Skip},
    };
    use hotshot::types::{BLSPubKey, SignatureKey};
    use hotshot_types::{data::ViewNumber, traits::node_implementation::NodeType};
    use tide_disco::Url;

    use crate::{testing::MockSolver, SolverError, state::UpdateSolverState};

    async fn register_rollup_helper(
        namespace_id: u64,
        reserve_url: Option<&str>,
        reserve_price: u64,
        active: bool,
        text: &str,
    ) -> (
        RollupRegistration,
        <BLSPubKey as SignatureKey>::PrivateKey,
        Vec<BLSPubKey>,
    ) {
        let private_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
        let signature_key = BLSPubKey::from_private(&private_key);

        // Create a list of signature keys for rollup registration data
        let mut signature_keys = vec![signature_key];

        for _ in 0..10 {
            let private_key =
                <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
            signature_keys.push(BLSPubKey::from_private(&private_key));
        }

        // Initialize a rollup registration with the provided namespace id
        let reg_body = RollupRegistrationBody {
            namespace_id: namespace_id.into(),
            reserve_url: reserve_url.map(|url| Url::from_str(url).unwrap()),
            reserve_price: reserve_price.into(),
            active,
            signature_keys: signature_keys.clone(),
            text: text.to_string(),
            signature_key,
        };

        // Sign the registration body
        let signature =
            <SeqTypes as NodeType>::SignatureKey::sign(&private_key, reg_body.commit().as_ref())
                .expect("failed to sign");

        let reg = RollupRegistration {
            body: reg_body,
            signature,
        };

        (reg, private_key, signature_keys)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_duplicate_rollup_registration() {
        let mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let (reg_ns_1, ..) =
            register_rollup_helper(1, Some("http://localhost"), 200, true, "test").await;

        // registering a rollup
        let result: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Ensure the registration result matches the initial registration data
        assert_eq!(reg_ns_1, result);

        // Attempt to re-register the same rollup (should fail)
        let err = client
            .post::<RollupRegistration>("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        // Ensure the error indicates the rollup with namespace id  = 1 already exists
        match err {
            SolverError::RollupAlreadyExists(id) if reg_ns_1.body.namespace_id == id => (),
            _ => panic!("err {err:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rollup_registration_invalid_signature() {
        let mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let (reg_ns_1, ..) =
            register_rollup_helper(1, Some("http://localhost"), 200, true, "test").await;

        // registering a rollup
        let _: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Attempt to register a new rollup with namespace id = 2 using an invalid signature
        let mut reg_ns_2 = reg_ns_1.clone();
        reg_ns_2.body.namespace_id = 2_u64.into();
        // Generating an invalid signature by signing the body of namespace id = 1
        let new_priv_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());

        let new_signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &new_priv_key,
            reg_ns_1.body.clone().commit().as_ref(),
        )
        .expect("failed to sign");
        reg_ns_2.signature = new_signature;

        // This should fail due to an invalid signature
        let err: SolverError = client
            .post::<RollupRegistration>("register_rollup")
            .body_json(&reg_ns_2)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        // Ensure the error indicates an invalid signature
        match err {
            SolverError::InvalidSignature(signature)
                if reg_ns_2.signature.to_string() == signature => {},
            _ => panic!("err {err:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_registration() {
        let mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let (mut reg_ns_1, privkey, _) =
            register_rollup_helper(1, Some("http://localhost"), 200, true, "test").await;

        // registering a rollup
        let _: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Test the update rollup endpoint
        // We will use the existing rollup registration with namespace id = 1
        // and update it by setting the `active`` status to false
        let update_body = RollupUpdatebody {
            namespace_id: 1_u64.into(),
            reserve_url: Skip,
            reserve_price: Skip,
            active: Set(false),
            signature_keys: Skip,
            signature_key: BLSPubKey::from_private(&privkey),
            text: Skip,
        };

        let signature =
            <SeqTypes as NodeType>::SignatureKey::sign(&privkey, update_body.commit().as_ref())
                .expect("failed to sign");

        // Sign the above body
        let update_rolup = RollupUpdate {
            body: update_body,
            signature,
        };

        // The result should contain the updated rollup registration data
        let result: RollupRegistration = client
            .post("update_rollup")
            .body_json(&update_rolup)
            .unwrap()
            .send()
            .await
            .unwrap();

        reg_ns_1.body.active = false;

        // Ensure the update result matches the modified registration data
        assert_eq!(reg_ns_1, result);

        // Test `rollup_registrations` endpoint to get all the registered rollups
        // The result should contain the updated rollup registration data
        let result: Vec<RollupRegistration> =
            client.get("rollup_registrations").send().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], reg_ns_1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_rollup_not_registered() {
        let mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let private_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
        let pubkey = BLSPubKey::from_private(&private_key);

        let update_body = RollupUpdatebody {
            namespace_id: 1_u64.into(),
            reserve_url: Skip,
            reserve_price: Skip,
            active: Set(false),
            signature_keys: Skip,
            text: Skip,
            signature_key: pubkey,
        };
        let signature =
            <SeqTypes as NodeType>::SignatureKey::sign(&private_key, update_body.commit().as_ref())
                .expect("failed to sign");

        let update_rollup = RollupUpdate {
            body: update_body,
            signature,
        };

        let err: SolverError = client
            .post::<RollupUpdate>("update_rollup")
            .body_json(&update_rollup)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        match err {
            SolverError::Database(_) => {},
            _ => panic!("err {err:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_signature_mismatch() {
        // In this test, a rollup is registered.
        // Next, we attempt to update the rollup with different conditions:
        // - the `signature_key` is not from the signature keys list in update rollup body, which should result in a failure.
        // - use a different key to generate the signature than the one provided in the update body (`signature_key` field),
        // which should also result in a failure.
        // - Finally, provide new signature keys and signature in the update body,
        // but the signature key is not present in the database
        // (i.e., the signature keys list from when the rollup was initially registered)

        let mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let (reg_ns_1, private_key, mut signature_keys) =
            register_rollup_helper(1, Some("http://localhost"), 200, true, "test").await;
        // registering a rollup
        let result: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Ensure the registration result matches the initial registration data
        assert_eq!(reg_ns_1, result);

        let signature_key = BLSPubKey::from_private(&private_key);
        signature_keys.retain(|&key| key != signature_key);

        // We update the rollup but the signature key in the body is not from the signature keys list so this should fail
        let update_body = RollupUpdatebody {
            namespace_id: 1_u64.into(),
            reserve_url: Skip,
            reserve_price: Skip,
            active: Set(false),
            signature_keys: Set(signature_keys.clone()),
            signature_key,
            text: Skip,
        };

        let signature =
            <SeqTypes as NodeType>::SignatureKey::sign(&private_key, update_body.commit().as_ref())
                .expect("failed to sign");

        // Sign the above body
        let mut update_rollup = RollupUpdate {
            body: update_body,
            signature: signature.clone(),
        };

        client
            .post::<RollupUpdate>("update_rollup")
            .body_json(&update_rollup)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        // add the signature back
        signature_keys.push(signature_key);
        update_rollup.body.signature_keys = Set(signature_keys.clone());

        let signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &private_key,
            update_rollup.body.clone().commit().as_ref(),
        )
        .expect("failed to sign");

        update_rollup.signature = signature.clone();

        // this should succeed
        client
            .post::<RollupRegistration>("update_rollup")
            .body_json(&update_rollup)
            .unwrap()
            .send()
            .await
            .unwrap();

        // use an invalid signature so this should fail
        let new_priv_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());

        let new_signature_key = BLSPubKey::from_private(&new_priv_key);

        let new_signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &new_priv_key,
            update_rollup.body.clone().commit().as_ref(),
        )
        .expect("failed to sign");
        update_rollup.signature = new_signature;

        // this should fail as the signature is invalid
        client
            .post::<RollupUpdate>("update_rollup")
            .body_json(&update_rollup)
            .unwrap()
            .send()
            .await
            .unwrap_err();

        // test signature key not present in database
        update_rollup.body.signature_key = new_signature_key;
        signature_keys.push(new_signature_key);
        update_rollup.body.signature_keys = Set(signature_keys);

        client
            .post::<RollupUpdate>("update_rollup")
            .body_json(&update_rollup)
            .unwrap()
            .send()
            .await
            .unwrap_err();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_bid_submission() {
        let mock_solver = MockSolver::init().await;

        let solver_api = mock_solver.solver_api();

        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(None).await;

        let key = FeeAccount::test_key_pair();
        let tx = BidTx::mock(key);

        client
            .post::<()>("submit_bid")
            .body_json(&tx)
            .unwrap()
            .send()
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_state() {
        // Initialize a mock solver and register two rollups
        // Drop the first solver tasks handles,
        // but reuse the same database for a new solver instance to test state recovery
        let mut mock_solver = MockSolver::init().await;
        let solver_api = mock_solver.solver_api();

        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api.clone());
        client.connect(Some(Duration::from_secs(5))).await;

        // Register the first rollup (ns = 1)
        let (reg_ns_1, ..) =
            register_rollup_helper(1, Some("http://localhost"), 200, true, "test").await;
        let _: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Register the second rollup (ns = 2)
        let (reg_ns_2, ..) =
            register_rollup_helper(2, Some("http://localhost"), 200, true, "test").await;
        let _: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_2)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Retrieve all registered rollups and verify that both have been registered successfully
        let result: Vec<RollupRegistration> =
            client.get("rollup_registrations").send().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], reg_ns_1);
        assert_eq!(result[1], reg_ns_2);

        // Crash solver by cancelling all handles of the mock solver
        while let Some(handle) = mock_solver.handles.pop() {
            handle.abort();
        }

        let db = mock_solver.database.clone();
        let tmp_db = mock_solver.tmp_db.clone();

        // connection should fail here
        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api.clone());
        assert!(!client.connect(Some(Duration::from_secs(10))).await);

        // Start a new solver instance using the same database
        // The new solver should return the two previously registered rollups
        let (tmp, db) = (tmp_db, db);
        let mock_solver = MockSolver::with_db((tmp, db)).await;
        let solver_api = mock_solver.solver_api();

        let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(solver_api);
        client.connect(Some(Duration::from_secs(5))).await;

        let result: Vec<RollupRegistration> =
            client.get("rollup_registrations").send().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], reg_ns_1);
        assert_eq!(result[1], reg_ns_2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_calculate_auction_results_no_bids() {
        let mock_solver = MockSolver::init().await;
        let state = mock_solver.state();
        let state = state.read().await;
        let view_number = ViewNumber::new(1);
        
        let results = state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();
        
        assert!(results.winning_bids.is_empty());
        assert!(results.reserve_bids.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_calculate_auction_results_with_bids() {
        let mock_solver = MockSolver::init().await;
        let state = mock_solver.state();
        let mut state = state.write().await;
        let view_number = ViewNumber::new(1);
        
        // Create test namespace and bid
        let namespace_id = 1u64.into();
        let bid_amount = 1000;
        
        let bid_tx = BidTx {
            body: BidTxBody {
                view: view_number,
                namespaces: vec![namespace_id],
                bid_amount,
                ..Default::default()
            },
            signature: Default::default(),
        };
        
        // Submit bid
        state.submit_bid_tx(bid_tx.clone()).await.unwrap();
        
        // Register rollup with reserve price
        let (registration, ..) = register_rollup_helper(
            1, // namespace_id
            Some("http://example.com"),
            500, // reserve_price (lower than bid amount)
            true, // active
            "test rollup"
        ).await;
        
        state.register_rollup(registration).await.unwrap();
        
        let results = state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();
        
        assert_eq!(results.winning_bids.len(), 1);
        assert_eq!(results.winning_bids[0], bid_tx);
        assert!(results.reserve_bids.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_calculate_auction_results_bid_below_reserve() {
        let mock_solver = MockSolver::init().await;
        let state = mock_solver.state();
        let mut state = state.write().await;
        let view_number = ViewNumber::new(1);
        
        // Create test namespace and bid
        let namespace_id = 1u64.into();
        let bid_amount = 100;
        
        let bid_tx = BidTx {
            body: BidTxBody {
                view: view_number,
                namespaces: vec![namespace_id],
                bid_amount,
                ..Default::default()
            },
            signature: Default::default(),
        };
        
        // Submit bid
        state.submit_bid_tx(bid_tx).await.unwrap();
        
        // Register rollup with higher reserve price
        let (registration, ..) = register_rollup_helper(
            1, // namespace_id
            Some("http://example.com"),
            500, // reserve_price (higher than bid amount)
            true, // active
            "test rollup"
        ).await;
        
        state.register_rollup(registration).await.unwrap();
        
        let results = state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();
        
        // Bid should not win as it's below reserve price
        assert!(results.winning_bids.is_empty());
        // Reserve URL should be included
        assert_eq!(results.reserve_bids.len(), 1);
        assert_eq!(
            results.reserve_bids[0],
            (namespace_id, Url::parse("http://example.com").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_calculate_auction_results_inactive_rollup() {
        let mock_solver = MockSolver::init().await;
        let state = mock_solver.state();
        let mut state = state.write().await;
        let view_number = ViewNumber::new(1);
        
        // Register inactive rollup
        let (registration, ..) = register_rollup_helper(
            1, // namespace_id
            Some("http://example.com"),
            500, // reserve_price
            false, // inactive
            "test rollup"
        ).await;
        
        state.register_rollup(registration).await.unwrap();
        
        let results = state
            .calculate_auction_results_permissionless(view_number)
            .await
            .unwrap();
        
        // No winning bids
        assert!(results.winning_bids.is_empty());
        // No reserve URLs for inactive rollup
        assert!(results.reserve_bids.is_empty());
    }
}
