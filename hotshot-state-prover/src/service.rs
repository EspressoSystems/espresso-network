//! A light client prover service

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use anyhow::{anyhow, Context, Result};
use displaydoc::Display;
use espresso_types::config::PublicNetworkConfig;
use futures::FutureExt;
use hotshot_contract_adapter::{
    field_to_u256,
    sol_types::{LightClientStateSol, LightClientV2, PlonkProofSol, StakeTableStateSol},
};
use hotshot_stake_table::{
    utils::one_honest_threshold,
    vec_based::{config::FieldType, StakeTable},
};
use hotshot_types::{
    light_client::{
        CircuitField, LightClientState, PublicInput, StakeTableState, StateSignaturesBundle,
        StateVerKey,
    },
    signature_key::BLSPubKey,
    traits::{
        signature_key::StakeTableEntryType,
        stake_table::{SnapshotVersion, StakeTableError, StakeTableScheme as _},
    },
    utils::is_last_block,
};
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_plonk::errors::PlonkError;
use jf_relation::Circuit as _;
use jf_signature::constants::CS_ID_SCHNORR;
use sequencer_utils::deployer::is_proxy_contract;
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::ext::InstantExt;
use tokio::{io, spawn, task::spawn_blocking, time::sleep};
use url::Url;
use vbs::version::{StaticVersion, StaticVersionType};

use crate::snark::{generate_state_update_proof, Proof, ProvingKey};

/// Configuration/Parameters used for hotshot state prover
#[derive(Debug, Clone)]
pub struct StateProverConfig {
    /// Url of the state relay server (a CDN that sequencers push their Schnorr signatures to)
    pub relay_server: Url,
    /// Interval between light client state update
    pub update_interval: Duration,
    /// Interval between retries if a state update fails
    pub retry_interval: Duration,
    /// URL of the chain (layer 1  or any layer 2) JSON-RPC provider.
    pub provider_endpoint: Url,
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
    /// Epoch length in number of Hotshot blocks. If None, config will be fetched from sequencer on demand
    pub blocks_per_epoch: Option<u64>,
}

impl StateProverConfig {
    pub async fn validate_light_client_contract(&self) -> anyhow::Result<()> {
        let provider = ProviderBuilder::new().on_http(self.provider_endpoint.clone());

        if !is_proxy_contract(&provider, self.light_client_address).await? {
            anyhow::bail!(
                "Light Client contract's address {:?} is not a proxy",
                self.light_client_address
            );
        }

        Ok(())
    }

    pub async fn blocks_per_epoch(&self) -> anyhow::Result<u64> {
        match self.blocks_per_epoch {
            Some(v) => Ok(v),
            None => Ok(epoch_config(&self.sequencer_url).await?.0),
        }
    }
}

/// Get the epoch-related  from the sequencer's `PublicHotShotConfig` struct
/// return (blocks_per_epoch, epoch_start_block)
pub async fn epoch_config(sequencer_url: &Url) -> anyhow::Result<(u64, u64)> {
    let config_url = sequencer_url
        .join("/config/hotshot")
        .with_context(|| "Invalid URL")?;

    // Request the configuration until it is successful
    let blocks_per_epoch = loop {
        match surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
            config_url.clone(),
        )
        .get::<PublicNetworkConfig>(config_url.as_str())
        .send()
        .await
        {
            Ok(resp) => {
                let config = resp.hotshot_config();
                break (config.blocks_per_epoch(), config.epoch_start_block());
            },
            Err(e) => {
                tracing::error!("Failed to fetch the network config: {e}");
                sleep(Duration::from_secs(5)).await;
            },
        }
    };
    Ok(blocks_per_epoch)
}

/// Initialize the stake table from a sequencer node that
/// is currently providing the HotShot config.
///
/// Does not error, runs until the stake table is provided.
pub async fn init_stake_table_from_sequencer(
    sequencer_url: &Url,
    stake_table_capacity: usize,
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>> {
    tracing::info!("Initializing stake table from node at {sequencer_url}");

    // Construct the URL to fetch the network config
    let config_url = sequencer_url
        .join("/config/hotshot")
        .with_context(|| "Invalid URL")?;

    // Request the configuration until it is successful
    let hotshot_config = loop {
        match surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
            config_url.clone(),
        )
        .get::<PublicNetworkConfig>(config_url.as_str())
        .send()
        .await
        {
            Ok(resp) => break resp.hotshot_config(),
            Err(e) => {
                tracing::error!("Failed to fetch the network config: {e}");
                sleep(Duration::from_secs(5)).await;
            },
        }
    };

    // Create empty stake table
    let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(stake_table_capacity);

    // Populate the stake table
    for node in hotshot_config.known_nodes_with_stake().into_iter() {
        st.register(
            *node.stake_table_entry.key(),
            node.stake_table_entry.stake(),
            node.state_ver_key,
        )
        .expect("Key registration shouldn't fail.");
    }

    // Advance the stake table
    st.advance();
    st.advance();

    Ok(st)
}

/// Returns both genesis light client state and stake table state
pub async fn light_client_genesis(
    sequencer_url: &Url,
    stake_table_capacity: usize,
) -> anyhow::Result<(LightClientStateSol, StakeTableStateSol)> {
    let st = init_stake_table_from_sequencer(sequencer_url, stake_table_capacity)
        .await
        .with_context(|| "Failed to initialize stake table")?;
    light_client_genesis_from_stake_table(st)
}

#[inline]
pub fn light_client_genesis_from_stake_table(
    st: StakeTable<BLSPubKey, StateVerKey, CircuitField>,
) -> anyhow::Result<(LightClientStateSol, StakeTableStateSol)> {
    let (bls_comm, schnorr_comm, stake_comm) = st
        .commitment(SnapshotVersion::LastEpochStart)
        .expect("Commitment computation shouldn't fail.");
    let threshold = one_honest_threshold(st.total_stake(SnapshotVersion::LastEpochStart)?);

    Ok((
        LightClientStateSol {
            viewNum: 0,
            blockHeight: 0,
            blockCommRoot: U256::from(0u32),
        },
        StakeTableStateSol {
            blsKeyComm: field_to_u256(bls_comm),
            schnorrKeyComm: field_to_u256(schnorr_comm),
            amountComm: field_to_u256(stake_comm),
            threshold,
        },
    ))
}

pub fn load_proving_key(stake_table_capacity: usize) -> ProvingKey {
    let srs = {
        let num_gates = crate::circuit::build_for_preprocessing::<
            CircuitField,
            ark_ed_on_bn254::EdwardsConfig,
        >(stake_table_capacity)
        .unwrap()
        .0
        .num_gates();

        tracing::info!("Loading SRS from Aztec's ceremony...");
        let srs_timer = Instant::now();
        let srs = ark_srs::kzg10::aztec20::setup(num_gates + 2).expect("Aztec SRS fail to load");
        let srs_elapsed = Instant::now().signed_duration_since(srs_timer);
        tracing::info!("Done in {srs_elapsed:.3}");

        // convert to Jellyfish type
        // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
        UnivariateUniversalParams {
            powers_of_g: srs.powers_of_g,
            h: srs.h,
            beta_h: srs.beta_h,
            powers_of_h: vec![srs.h, srs.beta_h],
        }
    };

    tracing::info!("Generating proving key and verification key.");
    let key_gen_timer = Instant::now();
    let (pk, _) = crate::snark::preprocess(&srs, stake_table_capacity)
        .expect("Fail to preprocess state prover circuit");
    let key_gen_elapsed = Instant::now().signed_duration_since(key_gen_timer);
    tracing::info!("Done in {key_gen_elapsed:.3}");
    pk
}

#[inline(always)]
/// Get the latest LightClientState and signature bundle from Sequencer network
pub async fn fetch_latest_state<ApiVer: StaticVersionType>(
    client: &Client<ServerError, ApiVer>,
) -> Result<StateSignaturesBundle, ServerError> {
    tracing::info!("Fetching the latest state signatures bundle from relay server.");
    client
        .get::<StateSignaturesBundle>("/api/state")
        .send()
        .await
}

/// Read the following info from the LightClient contract storage on chain
/// - latest finalized light client state
/// - stake table commitment used in currently active epoch
///
/// Returned types are of Rust struct defined in `hotshot-types`.
pub async fn read_contract_state(
    provider: impl Provider,
    address: Address,
) -> Result<(LightClientState, StakeTableState), ProverError> {
    let contract = LightClientV2::new(address, &provider);
    let state: LightClientStateSol = match contract.finalizedState().call().await {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::error!("unable to read finalized_state from contract: {}", e);
            return Err(ProverError::ContractError(e.into()));
        },
    };
    let st_state: StakeTableStateSol = match contract.votingStakeTableState().call().await {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::error!(
                "unable to read genesis_stake_table_state from contract: {}",
                e
            );
            return Err(ProverError::ContractError(e.into()));
        },
    };

    Ok((state.into(), st_state.into()))
}

/// submit the latest finalized state along with a proof to the L1 LightClient contract
pub async fn submit_state_and_proof(
    provider: impl Provider,
    address: Address,
    proof: Proof,
    public_input: PublicInput,
) -> Result<TransactionReceipt, ProverError> {
    let contract = LightClientV2::new(address, &provider);
    // prepare the input the contract call and the tx itself
    let proof: PlonkProofSol = proof.into();
    let new_state: LightClientStateSol = public_input.lc_state.into();
    let next_stake_table: StakeTableStateSol = public_input.next_st_state.into();

    let tx = contract.newFinalizedState_1(new_state.into(), next_stake_table.into(), proof.into());
    // send the tx
    let (receipt, included_block) = sequencer_utils::contract_send(&tx)
        .await
        .map_err(ProverError::ContractError)?;

    tracing::info!(
        "Submitted state and proof to L1: tx=0x{:x} block={included_block}; success={}",
        receipt.transaction_hash,
        receipt.inner.status()
    );
    if !receipt.inner.is_success() {
        return Err(ProverError::ContractError(anyhow!("{:?}", receipt)));
    }

    Ok(receipt)
}

pub async fn sync_state<ApiVer: StaticVersionType>(
    st: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    proving_key: Arc<ProvingKey>,
    relay_server_client: &Client<ServerError, ApiVer>,
    config: &StateProverConfig,
) -> Result<(), ProverError> {
    let light_client_address = config.light_client_address;
    let wallet = EthereumWallet::from(config.signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(config.provider_endpoint.clone());

    tracing::info!(
        ?light_client_address,
        "Start syncing light client state for provider: {}",
        config.provider_endpoint,
    );

    let bundle = fetch_latest_state(relay_server_client).await?;
    tracing::info!("Bundle accumulated weight: {}", bundle.accumulated_weight);
    tracing::info!("Latest HotShot block height: {}", bundle.state.block_height);

    let (old_state, st_state) = read_contract_state(&provider, light_client_address).await?;
    tracing::info!(
        "Current HotShot block height on contract: {}",
        old_state.block_height
    );
    if old_state.block_height >= bundle.state.block_height {
        tracing::info!("No update needed.");
        return Ok(());
    }
    tracing::debug!("Old state: {old_state:?}");
    tracing::debug!("New state: {:?}", bundle.state);

    let blocks_per_epoch = config
        .blocks_per_epoch()
        .await
        .map_err(ProverError::NetworkError)?;
    let next_stake = if is_last_block(bundle.state.block_height as u64, blocks_per_epoch) {
        st.next_voting_state()?
    } else {
        st_state
    };

    let entries = st
        .try_iter(SnapshotVersion::LastEpochStart)
        .unwrap()
        .map(|(_, stake_amount, state_key)| (state_key, stake_amount))
        .collect::<Vec<_>>();
    let mut signer_bit_vec = vec![false; entries.len()];
    let mut signatures = vec![Default::default(); entries.len()];
    let mut accumulated_weight = U256::ZERO;
    entries.iter().enumerate().for_each(|(i, (key, stake))| {
        if let Some(sig) = bundle.signatures.get(key) {
            // Check if the signature is valid
            let mut msg = Vec::with_capacity(7);
            let state_msg: [FieldType; 3] = (&bundle.state).into();
            msg.extend_from_slice(&state_msg);
            let next_stake_msg: [FieldType; 4] = next_stake.into();
            msg.extend_from_slice(&next_stake_msg);

            if key.verify(&msg, sig, CS_ID_SCHNORR).is_ok() {
                signer_bit_vec[i] = true;
                signatures[i] = sig.clone();
                accumulated_weight += *stake;
            }
        }
    });

    if accumulated_weight < field_to_u256(st_state.threshold) {
        return Err(ProverError::InvalidState(
            "The signers' total weight doesn't reach the threshold.".to_string(),
        ));
    }

    tracing::info!("Collected latest state and signatures. Start generating SNARK proof.");
    let proof_gen_start = Instant::now();
    let proving_key_clone = proving_key.clone();
    let stake_table_capacity = config.stake_table_capacity;
    let (proof, public_input) = spawn_blocking(move || {
        generate_state_update_proof(
            &mut ark_std::rand::thread_rng(),
            &proving_key_clone,
            &entries,
            signer_bit_vec,
            signatures,
            &bundle.state,
            &st_state,
            stake_table_capacity,
            &next_stake,
        )
    })
    .await
    .map_err(|e| ProverError::Internal(format!("failed to join task: {e}")))??;

    let proof_gen_elapsed = Instant::now().signed_duration_since(proof_gen_start);
    tracing::info!("Proof generation completed. Elapsed: {proof_gen_elapsed:.3}");

    submit_state_and_proof(&provider, light_client_address, proof, public_input).await?;

    tracing::info!("Successfully synced light client state.");
    Ok(())
}

fn start_http_server<ApiVer: StaticVersionType + 'static>(
    port: u16,
    light_client_address: Address,
    bind_version: ApiVer,
) -> io::Result<()> {
    let mut app = tide_disco::App::<_, ServerError>::with_state(());
    let toml = toml::from_str::<toml::value::Value>(include_str!("../api/prover-service.toml"))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let mut api = Api::<_, ServerError, ApiVer>::new(toml)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    api.get("getlightclientcontract", move |_, _| {
        async move { Ok(light_client_address) }.boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    app.register_module("api", api)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    spawn(app.serve(format!("0.0.0.0:{port}"), bind_version));
    Ok(())
}

pub async fn run_prover_service<ApiVer: StaticVersionType + 'static>(
    config: StateProverConfig,
    bind_version: ApiVer,
) -> Result<()> {
    let stake_table_capacity = config.stake_table_capacity;
    tracing::info!("Stake table capacity: {}", stake_table_capacity);
    let st = Arc::new(
        init_stake_table_from_sequencer(&config.sequencer_url, stake_table_capacity)
            .await
            .with_context(|| "Failed to initialize stake table")?,
    );
    run_prover_service_with_stake_table(config, bind_version, st).await
}

pub async fn run_prover_service_with_stake_table<ApiVer: StaticVersionType + 'static>(
    config: StateProverConfig,
    bind_version: ApiVer,
    st: Arc<StakeTable<BLSPubKey, StateVerKey, CircuitField>>,
) -> Result<()> {
    tracing::info!("Light client address: {:?}", config.light_client_address);

    let relay_server_client = Arc::new(Client::<ServerError, ApiVer>::new(
        config.relay_server.clone(),
    ));

    // Start the HTTP server to get a functioning healthcheck before any heavy computations.
    if let Some(port) = config.port {
        if let Err(err) = start_http_server(port, config.light_client_address, bind_version) {
            tracing::error!("Error starting http server: {}", err);
        }
    }

    let proving_key =
        spawn_blocking(move || Arc::new(load_proving_key(config.stake_table_capacity))).await?;

    let update_interval = config.update_interval;
    let retry_interval = config.retry_interval;
    loop {
        if let Err(err) = sync_state(&st, proving_key.clone(), &relay_server_client, &config).await
        {
            tracing::error!("Cannot sync the light client state, will retry: {}", err);
            sleep(retry_interval).await;
        } else {
            tracing::info!("Sleeping for {:?}", update_interval);
            sleep(update_interval).await;
        }
    }
}

/// Run light client state prover once
pub async fn run_prover_once<ApiVer: StaticVersionType>(
    config: StateProverConfig,
    _: ApiVer,
) -> Result<()> {
    // TODO: (alex) use `/node/stake-table/:epoch` to reconstruct the `st: StakeTable` locally before passing in.
    let st = init_stake_table_from_sequencer(&config.sequencer_url, config.stake_table_capacity)
        .await
        .with_context(|| "Failed to initialize stake table")?;
    let stake_table_capacity = config.stake_table_capacity;
    let proving_key =
        spawn_blocking(move || Arc::new(load_proving_key(stake_table_capacity))).await?;
    let relay_server_client = Client::<ServerError, ApiVer>::new(config.relay_server.clone());

    sync_state(&st, proving_key, &relay_server_client, &config)
        .await
        .expect("Error syncing the light client state.");

    Ok(())
}

#[derive(Debug, Display)]
pub enum ProverError {
    /// Invalid light client state or signatures
    InvalidState(String),
    /// Error when communicating with the smart contract: {0}
    ContractError(anyhow::Error),
    /// Error when communicating with the state relay server: {0}
    RelayServerError(ServerError),
    /// Internal error with the stake table
    StakeTableError(StakeTableError),
    /// Internal error when generating the SNARK proof: {0}
    PlonkError(PlonkError),
    /// Internal error
    Internal(String),
    /// General network issue: {0}
    NetworkError(anyhow::Error),
}

impl From<ServerError> for ProverError {
    fn from(err: ServerError) -> Self {
        Self::RelayServerError(err)
    }
}

impl From<PlonkError> for ProverError {
    fn from(err: PlonkError) -> Self {
        Self::PlonkError(err)
    }
}

impl From<StakeTableError> for ProverError {
    fn from(err: StakeTableError) -> Self {
        Self::StakeTableError(err)
    }
}

impl std::error::Error for ProverError {}

#[cfg(test)]
mod test {

    use alloy::{node_bindings::Anvil, providers::layers::AnvilProvider, sol_types::SolValue};
    use anyhow::Result;
    use hotshot_contract_adapter::sol_types::LightClientV2Mock;
    use jf_utils::test_rng;
    use sequencer_utils::{
        deployer::{deploy_light_client_proxy, upgrade_light_client_v2, Contracts},
        test_utils::setup_test,
    };

    use super::*;
    use crate::mock_ledger::{
        MockLedger, MockSystemParam, EPOCH_HEIGHT_FOR_TEST, STAKE_TABLE_CAPACITY_FOR_TEST,
    };

    // const MAX_HISTORY_SECONDS: u32 = 864000;
    const NUM_INIT_VALIDATORS: usize = STAKE_TABLE_CAPACITY_FOR_TEST / 2;

    /// This helper function deploy LightClient V1, and its Proxy, then deploy V2 and upgrade the proxy.
    /// Returns the address of the proxy, caller can cast the address to be `LightClientV2` or `LightClientV2Mock`
    async fn deploy_and_upgrade(
        provider: impl Provider,
        contracts: &mut Contracts,
        is_mock_v2: bool,
        genesis_state: LightClientStateSol,
        genesis_stake: StakeTableStateSol,
    ) -> Result<Address> {
        // prepare for V1 deployment
        let admin = provider.get_accounts().await?[0];
        let prover = admin;

        // deploy V1 and proxy (and initialize V1)
        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            contracts,
            false,
            genesis_state,
            genesis_stake,
            admin,
            Some(prover),
        )
        .await?;

        // upgrade to V2
        upgrade_light_client_v2(&provider, contracts, is_mock_v2, EPOCH_HEIGHT_FOR_TEST).await?;

        Ok(lc_proxy_addr)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_contract_state() -> Result<()> {
        setup_test();

        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let rng = &mut test_rng();
        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();

        let lc_proxy_addr = deploy_and_upgrade(
            &provider,
            &mut contracts,
            true,
            genesis_state.clone(),
            genesis_stake.clone(),
        )
        .await?;
        let (state, st_state) = super::read_contract_state(&provider, lc_proxy_addr).await?;

        // first test the default storage
        assert_eq!(state, genesis_state.into());
        assert_eq!(st_state, genesis_stake.into());

        // then manually set the `finalizedState` and `votingStakeTableState` (via mocked methods)
        let lc_v2 = LightClientV2Mock::new(lc_proxy_addr, &provider);
        let new_state = LightClientStateSol::rand(rng);
        let new_stake = StakeTableStateSol::rand(rng);
        lc_v2
            .setFinalizedState(new_state.clone().into())
            .send()
            .await?
            .watch()
            .await?;
        lc_v2
            .setVotingStakeTableState(new_stake.clone().into())
            .send()
            .await?
            .watch()
            .await?;

        // now query again, the states read should reflect the changes
        let (state, st_state) = super::read_contract_state(&provider, lc_proxy_addr).await?;
        assert_eq!(state, new_state.into());
        assert_eq!(st_state, new_stake.into());

        Ok(())
    }

    // This test is temporarily ignored. We are unifying the contract deployment in #1071.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_submit_state_and_proof() -> Result<()> {
        setup_test();

        let pp = MockSystemParam::init();
        let mut ledger = MockLedger::init(pp, NUM_INIT_VALIDATORS);
        let genesis_state: LightClientStateSol = ledger.light_client_state().into();
        let genesis_stake: StakeTableStateSol = ledger.voting_stake_table_state().into();

        let anvil = Anvil::new().spawn();
        let wallet = anvil.wallet().unwrap();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        // a provider that holds both anvil (to avoid accidental drop) and wallet-enabled L1 provider
        let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));
        let mut contracts = Contracts::new();

        let lc_proxy_addr = deploy_and_upgrade(
            &provider,
            &mut contracts,
            true,
            genesis_state,
            genesis_stake.clone(),
        )
        .await?;
        let lc_v2 = LightClientV2Mock::new(lc_proxy_addr, &provider);

        // simulate some block elapsing
        for _ in 0..EPOCH_HEIGHT_FOR_TEST - 1 {
            ledger.elapse_with_block();
        }
        ledger.sync_stake_table(5, 2); // update the stake table, some register, some exit
        ledger.elapse_with_block(); // the last block in the first epoch, thus updating the `next_stake_table`
        assert_eq!(ledger.state.block_height, EPOCH_HEIGHT_FOR_TEST);

        let (pi, proof) = ledger.gen_state_proof();
        tracing::info!("Successfully generated proof for new state.");

        super::submit_state_and_proof(&provider, lc_proxy_addr, proof, pi).await?;
        tracing::info!("Successfully submitted new finalized state to L1.");

        // test if new state is updated in l1
        let finalized_l1: LightClientStateSol = lc_v2.finalizedState().call().await?.into();
        let expected: LightClientStateSol = ledger.light_client_state().into();
        assert_eq!(
            finalized_l1.abi_encode_params(),
            expected.abi_encode_params(),
            "finalizedState not updated"
        );

        let expected_new_stake: StakeTableStateSol = ledger.next_stake_table_state().into();
        // make sure it's different from the genesis, i.e. use a new stake table for the next epoch
        assert_ne!(
            expected_new_stake.abi_encode_params(),
            genesis_stake.abi_encode_params()
        );
        let voting_stake_l1: StakeTableStateSol =
            lc_v2.votingStakeTableState().call().await?.into();
        assert_eq!(
            voting_stake_l1.abi_encode_params(),
            expected_new_stake.abi_encode_params(),
            "votingStakeTableState not updated"
        );

        Ok(())
    }
}
