#![allow(unused_imports)]
use std::{
    alloc::System,
    any,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    thread::Builder,
};

use async_lock::RwLock;
use async_trait::async_trait;
use espresso_types::{
    eth_signature_key::EthKeyPair,
    v0::traits::{PersistenceOptions, SequencerPersistence, StateCatchup},
    v0_99::BidTxBody,
    FeeVersion, MarketplaceVersion, SeqTypes, SequencerVersions,
};
use futures::{
    future::{join_all, Future},
    stream::{Stream, StreamExt},
};
use hotshot::{
    traits::election::static_committee::StaticCommittee,
    types::{SignatureKey, SystemContextHandle},
    HotShotInitializer, SystemContext,
};
use hotshot_builder_api::v0_99::builder::{
    BuildError, Error as BuilderApiError, Options as HotshotBuilderApiOptions,
};
use hotshot_orchestrator::client::{OrchestratorClient, ValidatorArgs};
// Should move `STAKE_TABLE_CAPACITY` in the sequencer repo when we have variate stake table support
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    event::LeafInfo,
    light_client::StateKeyPair,
    network::NetworkConfig,
    signature_key::{BLSPrivKey, BLSPubKey},
    traits::{
        block_contents::{BlockHeader, BlockPayload, EncodeBytes, GENESIS_VID_NUM_STORAGE_NODES},
        election::Membership,
        metrics::Metrics,
        node_implementation::{ConsensusTime, NodeType, Versions},
    },
    utils::BuilderCommitment,
    HotShotConfig, PeerConfig, ValidatorConfig,
};
use jf_merkle_tree::{namespaced_merkle_tree::NamespacedMerkleTreeScheme, MerkleTreeScheme};
use jf_signature::bls_over_bn254::VerKey;
use marketplace_builder_core::service::{GlobalState, ProxyGlobalState};
use sequencer::{
    catchup::StatePeers,
    context::{Consensus, SequencerContext},
    network,
    state_signature::StateSigner,
    L1Params, NetworkParams, Node, SequencerApiVersion,
};
use surf_disco::Client;
use tide_disco::{app, method::ReadState, App, Url};
use tokio::{spawn, task::JoinHandle};
use tracing::error;
use vbs::version::{StaticVersion, StaticVersionType};

pub mod builder;

pub mod hooks;
