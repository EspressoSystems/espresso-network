use std::collections::HashMap;

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
        // todo (ab): actual logic needs to implemented
        // for we, we just return some default results

        let rollups = self.get_all_rollup_registrations().await?;

        let results = SolverAuctionResults::new(
            view_number,
            Vec::new(),
            rollups
                .into_iter()
                .filter_map(|r| Some((r.body.namespace_id, r.body.reserve_url?)))
                .collect(),
        );

        Ok(results)
    }
    async fn calculate_auction_results_permissioned(
        &self,
        view_number: ViewNumber,
        _signature: <SeqTypes as NodeType>::SignatureKey,
    ) -> SolverResult<SolverAuctionResults> {
        // todo (ab): actual logic needs to implemented
        // for we, we just return some default results

        let rollups = self.get_all_rollup_registrations().await?;

        let results = SolverAuctionResults::new(
            view_number,
            Vec::new(),
            rollups
                .into_iter()
                .filter_map(|r| Some((r.body.namespace_id, r.body.reserve_url?)))
                .collect(),
        );

        Ok(results)
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
