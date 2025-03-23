//! A light client prover service

use std::{
    iter,
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    network::{Ethereum, IntoWallet},
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use anyhow::{anyhow, Context, Result};
use displaydoc::Display;
use futures::FutureExt;
use hotshot_contract_adapter::{
    field_to_u256,
    sol_types::{
        LightClient, LightClientMock, LightClientStateSol, PlonkProofSol, StakeTableStateSol,
    },
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
    PeerConfig,
};
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_plonk::errors::PlonkError;
use jf_relation::Circuit as _;
use jf_signature::constants::CS_ID_SCHNORR;
use sequencer_utils::deployer::is_proxy_contract;
use serde::Deserialize;
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::ext::InstantExt;
use tokio::{io, spawn, task::spawn_blocking, time::sleep};
use url::Url;
use vbs::version::StaticVersionType;

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
    /// Address of LightClient contract
    pub light_client_address: Address,
    /// Transaction signing key for Ethereum or any other layer 2
    pub signing_key: SigningKey,
    /// URL of a node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    pub sequencer_url: Url,
    /// If daemon and provided, the service will run a basic HTTP server on the given port.
    ///
    /// The server provides healthcheck and version endpoints.
    pub port: Option<u16>,
    /// Stake table capacity for the prover circuit.
    pub stake_table_capacity: usize,
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
}

pub fn init_stake_table(
    bls_keys: &[BLSPubKey],
    state_keys: &[StateVerKey],
    stake_table_capacity: usize,
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>, StakeTableError> {
    // We now initialize a static stake table as what hotshot orchestrator does.
    // In the future we should get the stake table from the contract.
    let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(stake_table_capacity);
    st.batch_register(
        bls_keys.iter().cloned(),
        iter::repeat(U256::from(1)).take(bls_keys.len()),
        state_keys.iter().cloned(),
    )?;
    st.advance();
    st.advance();
    Ok(st)
}

#[derive(Debug, Deserialize)]
/// Part of the full `PublicHotShotConfig` needed for our state-prover purposes
struct PublicHotShotConfig {
    known_nodes_with_stake: Vec<PeerConfig<BLSPubKey>>,
}

#[derive(Debug, Deserialize)]
/// Part of the full `PublicNetworkConfig` needed for our state-prover purposes
struct PublicNetworkConfig {
    config: PublicHotShotConfig,
}

/// Initialize the stake table from a sequencer node that
/// is currently providing the HotShot config.
///
/// Does not error, runs until the stake table is provided.
async fn init_stake_table_from_sequencer(
    sequencer_url: &Url,
    stake_table_capacity: usize,
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>> {
    tracing::info!("Initializing stake table from node at {sequencer_url}");

    // Construct the URL to fetch the network config
    let config_url = sequencer_url
        .join("/v0/config/hotshot")
        .with_context(|| "Invalid URL")?;

    // Request the configuration until it is successful
    let network_config: PublicHotShotConfig = loop {
        match reqwest::get(config_url.clone()).await {
            Ok(resp) => match resp.json::<PublicNetworkConfig>().await {
                Ok(config) => break config.config,
                Err(e) => {
                    tracing::error!("Failed to parse the network config: {e}");
                    sleep(Duration::from_secs(5)).await;
                },
            },
            Err(e) => {
                tracing::error!("Failed to fetch the network config: {e}");
                sleep(Duration::from_secs(5)).await;
            },
        }
    };

    // Create empty stake table
    let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(stake_table_capacity);

    // Populate the stake table
    for node in network_config.known_nodes_with_stake.into_iter() {
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

        std::println!("Loading SRS from Aztec's ceremony...");
        let srs_timer = Instant::now();
        let srs = ark_srs::kzg10::aztec20::setup(num_gates + 2).expect("Aztec SRS fail to load");
        let srs_elapsed = Instant::now().signed_duration_since(srs_timer);
        std::println!("Done in {srs_elapsed:.3}");

        // convert to Jellyfish type
        // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
        UnivariateUniversalParams {
            powers_of_g: srs.powers_of_g,
            h: srs.h,
            beta_h: srs.beta_h,
            powers_of_h: vec![srs.h, srs.beta_h],
        }
    };

    std::println!("Generating proving key and verification key.");
    let key_gen_timer = Instant::now();
    let (pk, _) = crate::snark::preprocess(&srs, stake_table_capacity)
        .expect("Fail to preprocess state prover circuit");
    let key_gen_elapsed = Instant::now().signed_duration_since(key_gen_timer);
    std::println!("Done in {key_gen_elapsed:.3}");
    pk
}

pub async fn fetch_latest_state<ApiVer: StaticVersionType>(
    client: &Client<ServerError, ApiVer>,
) -> Result<StateSignaturesBundle, ServerError> {
    tracing::info!("Fetching the latest state signatures bundle from relay server.");
    client
        .get::<StateSignaturesBundle>("/api/state")
        .send()
        .await
}

/// get the `finalizedState` from the LightClient contract storage on L1
pub async fn read_contract_state(
    provider: impl Provider,
    address: Address,
) -> Result<(LightClientState, StakeTableState), ProverError> {
    let contract = LightClient::new(address, &provider);
    let state: LightClientStateSol = match contract.finalizedState().call().await {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::error!("unable to read finalized_state from contract: {}", e);
            return Err(ProverError::ContractError(e.into()));
        },
    };
    let st_state: StakeTableStateSol = match contract.genesisStakeTableState().call().await {
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
    let contract = LightClient::new(address, &provider);
    // prepare the input the contract call and the tx itself
    let proof: PlonkProofSol = proof.into();
    let new_state: LightClientStateSol = public_input.lc_state.into();
    let _next_stake_table: StakeTableStateSol = public_input.next_st_state.into();

    let tx = contract.newFinalizedState(new_state, proof);
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
    let provider = ProviderBuilder::new()
        .wallet(IntoWallet::<Ethereum>::into_wallet(
            LocalSigner::from_signing_key(config.signing_key.clone()),
        ))
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
            let state_msg: [FieldType; 3] = (&bundle.state).into();
            if key.verify(&state_msg, sig, CS_ID_SCHNORR).is_ok() {
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
        generate_state_update_proof::<_, _, _, _>(
            &mut ark_std::rand::thread_rng(),
            &proving_key_clone,
            &entries,
            signer_bit_vec,
            signatures,
            &bundle.state,
            &st_state,
            stake_table_capacity,
            &st_state, // FIXME: use next_st_state later!
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
    // TODO(#1022): maintain the following stake table
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
    /// Internal error when generating the SNARK proof
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

    use alloy::{
        node_bindings::{Anvil, AnvilInstance},
        providers::layers::AnvilProvider,
        sol_types::SolValue,
    };
    use anyhow::Result;
    use hotshot_contract_adapter::sol_types::LightClientMock;
    use jf_utils::test_rng;
    use sequencer_utils::{
        deployer::{deploy_light_client_proxy, Contracts},
        test_utils::setup_test,
    };

    use super::*;
    use crate::mock_ledger::{
        MockLedger, MockSystemParam, EPOCH_HEIGHT_FOR_TEST, STAKE_TABLE_CAPACITY_FOR_TEST,
    };

    // const MAX_HISTORY_SECONDS: u32 = 864000;
    const NUM_INIT_VALIDATORS: usize = STAKE_TABLE_CAPACITY_FOR_TEST / 2;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_contract_state() -> Result<()> {
        setup_test();

        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let admin = provider.get_accounts().await?[0];
        let prover = admin;
        let is_mock = true;

        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();

        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            is_mock,
            genesis_state.clone(),
            genesis_stake.clone(),
            admin,
            Some(prover),
        )
        .await?;

        let (state, st_state) = super::read_contract_state(&provider, lc_proxy_addr).await?;

        assert_eq!(state, genesis_state.into());
        assert_eq!(st_state, genesis_stake.into());

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
        let is_mock = true;

        let anvil = Anvil::new().spawn();
        let wallet = anvil.wallet().unwrap();
        let admin = wallet.default_signer().address();
        let prover = admin;
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        // a provider that holds both anvil (to avoid accidental drop) and wallet-enabled L1 provider
        let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));
        let mut contracts = Contracts::new();

        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            is_mock,
            genesis_state.clone(),
            genesis_stake.clone(),
            admin,
            Some(prover),
        )
        .await?;
        let contract = LightClientMock::new(lc_proxy_addr, &provider);

        let genesis_l1: LightClientStateSol = contract.genesisState().call().await?.into();
        assert_eq!(
            genesis_l1.abi_encode_params(),
            genesis_state.abi_encode_params(),
            "mismatched genesis, aborting tests"
        );

        // simulate some block elapsing
        for _ in 0..EPOCH_HEIGHT_FOR_TEST - 1 {
            ledger.elapse_with_block();
        }

        let (pi, proof) = ledger.gen_state_proof();
        tracing::info!("Successfully generated proof for new state.");

        super::submit_state_and_proof(&provider, lc_proxy_addr, proof, pi).await?;
        tracing::info!("Successfully submitted new finalized state to L1.");

        // test if new state is updated in l1
        let finalized_l1: LightClientStateSol = contract.finalizedState().call().await?.into();
        let expected: LightClientStateSol = ledger.light_client_state().into();
        assert_eq!(
            finalized_l1.abi_encode_params(),
            expected.abi_encode_params(),
            "finalizedState not updated"
        );

        Ok(())
    }
}
