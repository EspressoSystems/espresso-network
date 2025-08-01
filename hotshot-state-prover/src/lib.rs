//! SNARK-assisted `HotShot` light client state update verification

use std::time::Duration;

use alloy::{
    primitives::Address,
    providers::ProviderBuilder,
    rpc::client::RpcClient,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use anyhow::{Context, Result};
use displaydoc::Display;
use espresso_contract_deployer::{
    is_proxy_contract, network_config::fetch_stake_table_from_sequencer,
};
use espresso_types::SeqTypes;
use hotshot_types::{
    light_client::StakeTableState, stake_table::HSStakeTable, traits::node_implementation::NodeType,
};
use jf_plonk::PlonkError;
use tide_disco::error::ServerError;
use url::Url;

/// The original prover for light client V1
pub mod v1;
/// Light client V2 prover, where we allow stake table update for proof of stake upgrade.
pub mod v2;
/// Light client V3 prover, where we introduce a new field `auth_root` for contract friendly state verification.
pub mod v3;

#[cfg(test)]
mod test_utils;

/// Configuration/Parameters used for hotshot state prover
#[derive(Debug, Clone)]
pub struct StateProverConfig {
    /// Url of the state relay server (a CDN that sequencers push their Schnorr signatures to)
    pub relay_server: Url,
    /// Interval between light client state update
    pub update_interval: Duration,
    /// Interval between retries if a state update fails
    pub retry_interval: Duration,
    /// RPC client to the L1 (or L2) provider
    pub l1_rpc_client: RpcClient,
    /// Address of LightClient proxy contract
    pub light_client_address: Address,
    /// Transaction signing key for Ethereum or any other layer 2
    pub signer: LocalSigner<SigningKey>,
    /// URL of a node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    pub sequencer_url: Url,
    /// If daemon and provided, the service will run a basic HTTP server on the given port.
    ///
    /// The server provides healthcheck and version endpoints.
    pub port: Option<u16>,
    /// Stake table capacity for the prover circuit.
    pub stake_table_capacity: usize,
    /// Epoch length in number of Hotshot blocks.
    pub blocks_per_epoch: u64,
    /// The epoch start block.
    pub epoch_start_block: u64,
    /// Maximum number of retires for one-shot prover
    pub max_retries: u64,
    /// optional gas price cap **in wei** to prevent prover sending updates during jammed base layer
    pub max_gas_price: Option<u128>,
}

#[derive(Debug, Clone)]
pub struct ProverServiceState {
    /// The configuration of the prover service
    pub config: StateProverConfig,
    /// The current epoch number of the stake table
    pub epoch: Option<<SeqTypes as NodeType>::Epoch>,
    /// The stake table
    pub stake_table: HSStakeTable<SeqTypes>,
    /// The current stake table state
    pub st_state: StakeTableState,
}

impl ProverServiceState {
    pub async fn new_genesis(config: StateProverConfig) -> Result<Self> {
        let stake_table = fetch_stake_table_from_sequencer(&config.sequencer_url, None)
            .await
            .with_context(|| "Failed to initialize stake table")?;
        let st_state = stake_table
            .commitment(config.stake_table_capacity)
            .with_context(|| "Failed to compute stake table commitment")?;
        Ok(Self {
            config,
            epoch: None,
            stake_table,
            st_state,
        })
    }

    pub async fn sync_with_epoch(
        &mut self,
        epoch: Option<<SeqTypes as NodeType>::Epoch>,
    ) -> Result<()> {
        if epoch != self.epoch {
            self.stake_table = fetch_stake_table_from_sequencer(&self.config.sequencer_url, epoch)
                .await
                .with_context(|| format!("Failed to update stake table for epoch: {epoch:?}"))?;
            self.st_state = self
                .stake_table
                .commitment(self.config.stake_table_capacity)
                .with_context(|| "Failed to compute stake table commitment")?;
            self.epoch = epoch;
        }
        Ok(())
    }
}

impl StateProverConfig {
    pub async fn validate_light_client_contract(&self) -> Result<(), ProverError> {
        let provider = ProviderBuilder::new().on_client(self.l1_rpc_client.clone());

        if let Err(e) = is_proxy_contract(&provider, self.light_client_address).await {
            Err(ProverError::ContractError(anyhow::anyhow!(
                "Light Client contract's address {:?} is not a proxy: {e}",
                self.light_client_address,
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Display)]
pub enum ProverError {
    /// Invalid light client state or signatures: {0}
    InvalidState(String),
    /// Error when communicating with the smart contract: {0}
    ContractError(anyhow::Error),
    /// Error when communicating with the state relay server: {0}
    RelayServerError(ServerError),
    /// Error when communicating with the sequencer. Url: {0}, Error: {1}
    SequencerCommunicationError(Url, ServerError),
    /// Internal error when generating the SNARK proof: {0}
    PlonkError(PlonkError),
    /// Internal error: {0}
    Internal(anyhow::Error),
    /// Gas price too high: current {0} gwei, max allowed: {1} gwei
    GasPriceTooHigh(String, String),
    /// Epoch has already started on block {0}, please upgrade the contract to V2.
    EpochAlreadyStarted(u64),
}

impl From<PlonkError> for ProverError {
    fn from(err: PlonkError) -> Self {
        Self::PlonkError(err)
    }
}

impl std::error::Error for ProverError {}
