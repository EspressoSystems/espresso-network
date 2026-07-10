use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use ::light_client::{
    consensus::{
        header::HeaderProof,
        leaf::{LeafProof, LeafProofHint},
        payload::PayloadProof,
    },
    testing::{EpochChangeQuorum, LEGACY_VERSION},
};
use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
};
use async_lock::Mutex;
use committable::{Commitment, Committable};
use espresso_contract_deployer::{
    Contract, Contracts, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table, upgrade_stake_table_v2,
    upgrade_stake_table_v3,
};
use espresso_types::{
    ADVZNamespaceProofQueryData, FeeAmount, Header, L1Client, L1ClientOptions,
    MOCK_SEQUENCER_VERSIONS, NamespaceId, NamespaceProofQueryData, NsProof, RegisteredValidatorMap,
    RewardDistributor, StakeTableState, StateCertQueryDataV1, StateCertQueryDataV2, ValidatedState,
    ValidatorLeaderCounts,
    config::PublicHotShotConfig,
    traits::{MembershipPersistence, NullEventConsumer, PersistenceOptions},
    v0_3::{COMMISSION_BASIS_POINTS, Fetcher, RewardAmount, RewardMerkleProofV1},
    v0_4::{RewardAccountV2, RewardMerkleProofV2},
    validators_from_l1_events,
};
use futures::{
    future::{self, join_all, try_join_all},
    stream::{StreamExt, TryStreamExt},
    try_join,
};
use hotshot::types::{Event, EventType};
use hotshot_contract_adapter::{
    reward::RewardClaimInput,
    sol_types::{EspToken, StakeTableV3},
    stake_table::StakeTableContractVersion,
};
use hotshot_query_service::{
    availability::{
        BlockQueryData, BlockSummaryQueryData, LeafQueryData, TransactionQueryData,
        VidCommonQueryData,
    },
    data_source::{
        VersionedDataSource,
        sql::Config,
        storage::{SqlStorage, StorageConnectionType},
    },
    explorer::TransactionSummariesResponse,
    types::HeightIndexed,
};
use hotshot_types::{
    ValidatorConfig,
    addr::NetAddr,
    data::EpochNumber,
    event::LeafInfo,
    new_protocol::CoordinatorEvent,
    traits::{block_contents::BlockHeader, election::Membership, metrics::NoMetrics},
    utils::epoch_from_block_number,
    x25519,
};
use jf_merkle_tree_compat::{
    MerkleTreeScheme,
    prelude::{MerkleProof, Sha3Node},
};
use pretty_assertions::assert_matches;
use rand::seq::SliceRandom;
use rstest::rstest;
use staking_cli::{
    demo::DelegationConfig, fetch_commission, update_commission, update_network_config,
};
use surf_disco::Client;
use test_helpers::{
    TestNetwork, TestNetworkConfigBuilder, catchup_test_helper, state_signature_test_helper,
    status_test_helper, submit_test_helper,
};
use test_utils::reserve_tcp_port;
use tide_disco::{
    Error, StatusCode, Url, app::AppHealth, error::ServerError, healthcheck::HealthStatus,
};
use tokio::time::sleep;
use vbs::version::StaticVersion;
use versions::{
    DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, EPOCH_VERSION, FEE_VERSION,
    NEW_PROTOCOL_VERSION, Upgrade, version,
};

use self::{
    data_source::testing::TestableSequencerDataSource, options::HotshotEvents,
    sql::DataSource as SqlDataSource,
};
use super::*;

async fn wait_until_block_height(
    client: &Client<ServerError, StaticVersion<0, 1>>,
    endpoint: &str,
    height: u64,
) {
    for _retry in 0.. {
        let bh = client
            .get::<u64>(endpoint)
            .send()
            .await
            .expect("block height not found");

        if bh >= height {
            return;
        }
        sleep(Duration::from_secs(3)).await;
    }
}
use crate::{
    api::{
        options::Query,
        sql::{impl_testable_data_source::tmp_options, reconstruct_state},
        test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
    },
    catchup::{NullStateCatchup, StatePeers},
    persistence,
    persistence::no_storage,
    testing::{TestConfig, TestConfigBuilder, wait_for_decide_on_handle, wait_for_epochs},
};

const POS_V3: Upgrade = Upgrade::trivial(version(0, 3));
const POS_V4: Upgrade = Upgrade::trivial(version(0, 4));

mod catchup;
mod endpoints;
mod light_client;
mod namespace;
mod rewards;
mod stake_table;
mod upgrades;
