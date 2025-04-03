//! A light client prover service

use std::{
    iter,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use contract_bindings_ethers::light_client::{LightClient, LightClientErrors};
use displaydoc::Display;
use espresso_types::SeqTypes;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::{
        gas_oracle::{GasCategory, GasOracle},
        signer::SignerMiddlewareError,
        SignerMiddleware,
    },
    providers::{Http, Middleware, Provider, ProviderError},
    signers::{LocalWallet, Signer, Wallet},
    types::{transaction::eip2718::TypedTransaction, Address, U256},
};
use futures::FutureExt;
use hotshot_contract_adapter::{
    jellyfish::{field_to_u256, ParsedPlonkProof},
    light_client::{ParsedLightClientState, ParsedStakeTableState},
};
use hotshot_stake_table::vec_based::{config::FieldType, StakeTable};
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
use sequencer_utils::{blocknative::BlockNative, deployer::is_proxy_contract};
use serde::Deserialize;
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::ext::InstantExt;
use tokio::{io, spawn, task::spawn_blocking, time::sleep};
use url::Url;
use vbs::version::StaticVersionType;

use crate::snark::{generate_state_update_proof, Proof, ProvingKey};

/// A wallet with local signer and connected to network via http
pub type SignerWallet = SignerMiddleware<Provider<Http>, LocalWallet>;

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
    pub provider: Url,
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
        let provider = Provider::<Http>::try_from(self.provider.to_string())?;

        if !is_proxy_contract(&provider, self.light_client_address).await? {
            anyhow::bail!(
                "Light Client contract's address {:?} is not a proxy",
                self.light_client_address
            );
        }

        Ok(())
    }
}

#[inline]
/// A helper function to compute the quorum threshold given a total amount of stake.
pub fn one_honest_threshold(total_stake: U256) -> U256 {
    total_stake / 3 + 1
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
        iter::repeat_n(U256::one(), bls_keys.len()),
        state_keys.iter().cloned(),
    )?;
    st.advance();
    st.advance();
    Ok(st)
}

#[derive(Debug, Deserialize)]
struct PublicHotShotConfig {
    known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
}

#[derive(Debug, Deserialize)]
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
) -> anyhow::Result<(ParsedLightClientState, ParsedStakeTableState)> {
    let st = init_stake_table_from_sequencer(sequencer_url, stake_table_capacity)
        .await
        .with_context(|| "Failed to initialize stake table")?;
    light_client_genesis_from_stake_table(st)
}

#[inline]
pub fn light_client_genesis_from_stake_table(
    st: StakeTable<BLSPubKey, StateVerKey, CircuitField>,
) -> anyhow::Result<(ParsedLightClientState, ParsedStakeTableState)> {
    let (bls_comm, schnorr_comm, stake_comm) = st
        .commitment(SnapshotVersion::LastEpochStart)
        .expect("Commitment computation shouldn't fail.");
    let threshold = one_honest_threshold(st.total_stake(SnapshotVersion::LastEpochStart)?);

    Ok((
        ParsedLightClientState {
            view_num: 0,
            block_height: 0,
            block_comm_root: U256::from(0u32),
        },
        ParsedStakeTableState {
            bls_key_comm: field_to_u256(bls_comm),
            schnorr_key_comm: field_to_u256(schnorr_comm),
            amount_comm: field_to_u256(stake_comm),
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

/// prepare a contract interface ready to be read from or written to
async fn prepare_contract(
    provider: Url,
    key: SigningKey,
    light_client_address: Address,
) -> Result<LightClient<SignerWallet>, ProverError> {
    let provider = Provider::try_from(provider.as_str())
        .expect("unable to instantiate Provider, likely wrong URL");
    let signer = Wallet::from(key).with_chain_id(provider.get_chainid().await?.as_u64());
    let wallet = Arc::new(SignerWallet::new(provider, signer));

    let contract = LightClient::new(light_client_address, wallet);
    Ok(contract)
}

/// get the `finalizedState` from the LightClient contract storage on L1
pub async fn read_contract_state(
    contract: &LightClient<SignerWallet>,
) -> Result<(LightClientState, StakeTableState), ProverError> {
    let state: ParsedLightClientState = match contract.finalized_state().call().await {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::error!("unable to read finalized_state from contract: {}", e);
            return Err(ProverError::ContractError(e.into()));
        },
    };
    let st_state: ParsedStakeTableState = match contract.genesis_stake_table_state().call().await {
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
    proof: Proof,
    public_input: PublicInput,
    contract: &LightClient<SignerWallet>,
) -> Result<(), ProverError> {
    // prepare the input the contract call and the tx itself
    let proof: ParsedPlonkProof = proof.into();
    let new_state: ParsedLightClientState = public_input.into();

    let mut tx = contract.new_finalized_state(new_state.into(), proof.into());

    // only use gas oracle for mainnet
    if contract.client_ref().get_chainid().await?.as_u64() == 1 {
        let gas_oracle = BlockNative::new(None).category(GasCategory::SafeLow);
        match gas_oracle.estimate_eip1559_fees().await {
            Ok((max_fee, priority_fee)) => {
                if let TypedTransaction::Eip1559(inner) = &mut tx.tx {
                    inner.max_fee_per_gas = Some(max_fee);
                    inner.max_priority_fee_per_gas = Some(priority_fee);
                    tracing::info!(
                        "Setting maxFeePerGas: {}; maxPriorityFeePerGas to: {}",
                        max_fee,
                        priority_fee
                    );
                }
            },
            Err(e) => {
                tracing::warn!("!! BlockNative Price Oracle failed: {}", e);
            },
        }
    }

    // send the tx
    let (receipt, included_block) = sequencer_utils::contract_send::<_, _, LightClientErrors>(&tx)
        .await
        .map_err(ProverError::ContractError)?;

    tracing::info!(
        "Submitted state and proof to L1: tx={:x} block={included_block}",
        receipt.transaction_hash,
    );

    Ok(())
}

pub async fn sync_state<ApiVer: StaticVersionType>(
    st: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    proving_key: Arc<ProvingKey>,
    relay_server_client: &Client<ServerError, ApiVer>,
    config: &StateProverConfig,
) -> Result<(), ProverError> {
    let light_client_address = config.light_client_address;
    let provider = config.provider.clone();
    let key = config.signing_key.clone();

    tracing::info!(
        ?light_client_address,
        "Start syncing light client state for provider: {}",
        provider,
    );

    let bundle = fetch_latest_state(relay_server_client).await?;
    tracing::info!("Bundle accumulated weight: {}", bundle.accumulated_weight);
    tracing::info!("Latest HotShot block height: {}", bundle.state.block_height);
    let contract = prepare_contract(provider.clone(), key.clone(), light_client_address).await?;
    let (old_state, st_state) = read_contract_state(&contract).await?;
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
    let mut accumulated_weight = U256::zero();
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
        )
    })
    .await
    .map_err(|e| ProverError::Internal(format!("failed to join task: {e}")))??;

    let proof_gen_elapsed = Instant::now().signed_duration_since(proof_gen_start);
    tracing::info!("Proof generation completed. Elapsed: {proof_gen_elapsed:.3}");

    submit_state_and_proof(proof, public_input, &contract).await?;

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

impl From<ProviderError> for ProverError {
    fn from(err: ProviderError) -> Self {
        Self::ContractError(anyhow!("{}", err))
    }
}
impl From<SignerMiddlewareError<Provider<Http>, LocalWallet>> for ProverError {
    fn from(err: SignerMiddlewareError<Provider<Http>, LocalWallet>) -> Self {
        Self::ContractError(anyhow!("{}", err))
    }
}

impl std::error::Error for ProverError {}

#[cfg(test)]
mod test {

    use anyhow::Result;
    use ark_ed_on_bn254::EdwardsConfig;
    use ethers::utils::{Anvil, AnvilInstance};
    use hotshot_contract_adapter::light_client::{
        LightClientConstructorArgs, ParsedStakeTableState,
    };
    use hotshot_stake_table::vec_based::StakeTable;
    use hotshot_types::light_client::StateSignKey;
    use jf_signature::{schnorr::SchnorrSignatureScheme, SignatureScheme};
    use jf_utils::test_rng;
    use sequencer_utils::{
        deployer::{self, test_helpers::deploy_light_client_contract_as_proxy_for_test},
        test_utils::setup_test,
    };

    use super::*;
    use crate::mock_ledger::{MockLedger, MockSystemParam};

    const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;
    const MAX_HISTORY_SECONDS: u32 = 864000;

    const NUM_INIT_VALIDATORS: u32 = (STAKE_TABLE_CAPACITY_FOR_TEST / 2) as u32;

    /// Init a meaningful ledger state that prover can generate future valid proof.
    /// this is used for testing purposes, contract deployed to test proof verification should also be initialized with this genesis
    ///
    #[allow(clippy::type_complexity)]
    fn init_ledger_for_test() -> (
        ParsedLightClientState,
        ParsedStakeTableState,
        Vec<BLSPubKey>,
        Vec<(StateSignKey, StateVerKey)>,
        StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) {
        let pp = MockSystemParam::init();
        let ledger = MockLedger::init(pp, NUM_INIT_VALIDATORS as usize);

        let genesis = ledger.get_state();
        let stake_genesis = ledger.get_stake_table_state();

        let qc_keys = ledger.qc_keys;
        let state_keys = ledger.state_keys;
        let st = ledger.st;

        eprintln!(
            "Genesis: view_num: {}, block_height: {}, block_comm_root: {}",
            genesis.view_num, genesis.block_height, genesis.block_comm_root,
        );
        (genesis, stake_genesis, qc_keys, state_keys, st)
    }

    // everybody signs, then generate a proof
    fn gen_state_proof(
        new_state: ParsedLightClientState,
        genesis_stake_state: &ParsedStakeTableState,
        state_keypairs: &[(StateSignKey, StateVerKey)],
        st: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) -> (PublicInput, Proof) {
        let mut rng = test_rng();

        let new_state_msg: [CircuitField; 3] = {
            // sorry for the complicated .into() conversion chain, might improve in the future
            let pi_msg: LightClientState = new_state.clone().into();
            pi_msg.into()
        };
        let bit_vec = vec![true; st.len(SnapshotVersion::LastEpochStart).unwrap()];
        let sigs = state_keypairs
            .iter()
            .map(|(sk, _)| {
                SchnorrSignatureScheme::<EdwardsConfig>::sign(&(), sk, new_state_msg, &mut rng)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let srs = {
            // load SRS from Aztec's ceremony
            let srs = ark_srs::kzg10::aztec20::setup(2u64.pow(16) as usize + 2)
                .expect("Aztec SRS fail to load");
            // convert to Jellyfish type
            // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
            UnivariateUniversalParams {
                powers_of_g: srs.powers_of_g,
                h: srs.h,
                beta_h: srs.beta_h,
                powers_of_h: vec![srs.h, srs.beta_h],
            }
        };
        let (pk, _) = crate::preprocess(&srs, STAKE_TABLE_CAPACITY_FOR_TEST)
            .expect("Fail to preprocess state prover circuit");
        let stake_table_entries = st
            .try_iter(SnapshotVersion::LastEpochStart)
            .unwrap()
            .map(|(_, stake_amount, schnorr_key)| (schnorr_key, stake_amount))
            .collect::<Vec<_>>();
        let (proof, pi) = crate::generate_state_update_proof::<_, _, _, _>(
            &mut rng,
            &pk,
            &stake_table_entries,
            &bit_vec,
            &sigs,
            &new_state.into(),
            &genesis_stake_state.clone().into(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .expect("Fail to generate state proof");

        (pi, proof)
    }

    /// deploy LightClientMock.sol on local blockchain (via `anvil`) for testing
    /// return (signer-loaded wallet, contract instance)
    async fn deploy_contract_for_test(
        anvil: &AnvilInstance,
        genesis: ParsedLightClientState,
        stake_genesis: ParsedStakeTableState,
    ) -> Result<(Arc<SignerWallet>, LightClient<SignerWallet>)> {
        let provider =
            Provider::<Http>::try_from(anvil.endpoint())?.interval(Duration::from_millis(20));
        let signer = Wallet::from(anvil.keys()[0].clone())
            .with_chain_id(provider.get_chainid().await?.as_u64());
        let l1_wallet = Arc::new(SignerWallet::new(provider.clone(), signer));

        let genesis_constructor_args: LightClientConstructorArgs = LightClientConstructorArgs {
            light_client_state: genesis,
            stake_table_state: stake_genesis,
            max_history_seconds: MAX_HISTORY_SECONDS,
        };

        let mut contracts = deployer::Contracts::default();
        let address = deployer::deploy_mock_light_client_contract(
            l1_wallet.clone(),
            &mut contracts,
            Some(genesis_constructor_args),
        )
        .await?;

        let light_client_contract = LightClient::new(address, l1_wallet.clone());

        Ok((l1_wallet, light_client_contract))
    }

    async fn deploy_contract_as_proxy_for_test(
        anvil: &AnvilInstance,
        genesis: ParsedLightClientState,
        stake_genesis: ParsedStakeTableState,
    ) -> Result<(Arc<SignerWallet>, LightClient<SignerWallet>)> {
        let provider = Provider::<Http>::try_from(anvil.endpoint())?;
        let signer = Wallet::from(anvil.keys()[0].clone())
            .with_chain_id(provider.get_chainid().await?.as_u64());
        let l1_wallet = Arc::new(SignerWallet::new(provider.clone(), signer));
        let genesis_constructor_args: LightClientConstructorArgs = LightClientConstructorArgs {
            light_client_state: genesis,
            stake_table_state: stake_genesis,
            max_history_seconds: MAX_HISTORY_SECONDS,
        };

        let mut contracts = deployer::Contracts::default();
        let proxy_address = deploy_light_client_contract_as_proxy_for_test(
            l1_wallet.clone(),
            &mut contracts,
            Some(genesis_constructor_args),
        )
        .await?;

        let light_client_contract = LightClient::new(proxy_address, l1_wallet.clone());

        Ok((l1_wallet, light_client_contract))
    }

    impl StateProverConfig {
        /// update only L1 related info
        fn update_l1_info(&mut self, anvil: &AnvilInstance, light_client_address: Address) {
            self.provider = Url::parse(&anvil.endpoint()).unwrap();
            self.light_client_address = light_client_address;
            self.signing_key = anvil.keys()[0].clone().into();
        }
    }
    // only for testing purposes
    impl Default for StateProverConfig {
        fn default() -> Self {
            Self {
                relay_server: Url::parse("http://localhost").unwrap(),
                update_interval: Duration::default(),
                retry_interval: Duration::default(),
                provider: Url::parse("http://localhost").unwrap(),
                light_client_address: Address::default(),
                signing_key: SigningKey::random(&mut test_rng()),
                sequencer_url: Url::parse("http://localhost").unwrap(),
                port: None,
                stake_table_capacity: 10,
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_light_contract_is_proxy() -> Result<()> {
        setup_test();

        let anvil = Anvil::new().spawn();
        let dummy_genesis = ParsedLightClientState::dummy_genesis();
        let dummy_stake_genesis = ParsedStakeTableState::dummy_genesis();
        let (_wallet, contract) = deploy_contract_as_proxy_for_test(
            &anvil,
            dummy_genesis.clone(),
            dummy_stake_genesis.clone(),
        )
        .await?;

        // now test if we can read from the contract
        let genesis: ParsedLightClientState = contract.genesis_state().await?.into();
        assert_eq!(genesis, dummy_genesis);

        let stake_genesis: ParsedStakeTableState =
            contract.genesis_stake_table_state().await?.into();
        assert_eq!(stake_genesis, dummy_stake_genesis);

        let config = StateProverConfig {
            provider: Url::parse(anvil.endpoint().as_str())
                .expect("Cannot parse anvil endpoint to URL."),
            light_client_address: contract.clone().address(),
            ..Default::default()
        };

        let result = config.validate_light_client_contract().await;

        // check if the result was ok
        assert!(
            result.is_ok(),
            "Expected Light Client contract to be a proxy, but it was not"
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_light_contract_is_not_a_proxy() -> Result<()> {
        setup_test();

        let anvil = Anvil::new().spawn();
        let dummy_genesis = ParsedLightClientState::dummy_genesis();
        let dummy_stake_genesis = ParsedStakeTableState::dummy_genesis();
        let (_wallet, contract) =
            deploy_contract_for_test(&anvil, dummy_genesis.clone(), dummy_stake_genesis.clone())
                .await?;

        let config = StateProverConfig {
            provider: Url::parse(anvil.endpoint().as_str())
                .expect("Cannot parse anvil endpoint to URL."),
            light_client_address: contract.clone().address(),
            ..Default::default()
        };

        assert!(config
            .validate_light_client_contract()
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("not a proxy"));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_contract_state() -> Result<()> {
        setup_test();
        let anvil = Anvil::new().spawn();
        let dummy_genesis = ParsedLightClientState::dummy_genesis();
        let dummy_stake_genesis = ParsedStakeTableState::dummy_genesis();
        let (_wallet, contract) =
            deploy_contract_for_test(&anvil, dummy_genesis.clone(), dummy_stake_genesis.clone())
                .await?;

        // now test if we can read from the contract
        let genesis: ParsedLightClientState = contract.genesis_state().await?.into();
        assert_eq!(genesis, dummy_genesis);

        let stake_genesis: ParsedStakeTableState =
            contract.genesis_stake_table_state().await?.into();
        assert_eq!(stake_genesis, dummy_stake_genesis);

        let mut config = StateProverConfig::default();
        config.update_l1_info(&anvil, contract.address());
        let contract = super::prepare_contract(
            config.provider,
            config.signing_key,
            config.light_client_address,
        )
        .await?;
        let (state, st_state) = super::read_contract_state(&contract).await?;

        assert_eq!(state, genesis.into());
        assert_eq!(st_state, stake_genesis.into());

        Ok(())
    }

    // This test is temporarily ignored. We are unifying the contract deployment in #1071.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_submit_state_and_proof() -> Result<()> {
        setup_test();

        let (genesis, stake_genesis, _qc_keys, state_keys, st) = init_ledger_for_test();

        let anvil = Anvil::new().spawn();
        let (_wallet, contract) =
            deploy_contract_for_test(&anvil, genesis.clone(), stake_genesis.clone()).await?;
        let mut config = StateProverConfig::default();
        config.update_l1_info(&anvil, contract.address());

        let genesis_l1: ParsedLightClientState = contract.genesis_state().await?.into();
        assert_eq!(genesis_l1, genesis, "mismatched genesis, aborting tests");

        let mut new_state = genesis.clone();
        new_state.view_num = 5;
        new_state.block_height = 1;

        let (pi, proof) = gen_state_proof(new_state.clone(), &stake_genesis, &state_keys, &st);
        tracing::info!("Successfully generated proof for new state.");

        let contract = super::prepare_contract(
            config.provider,
            config.signing_key,
            config.light_client_address,
        )
        .await?;
        super::submit_state_and_proof(proof, pi, &contract).await?;
        tracing::info!("Successfully submitted new finalized state to L1.");
        // test if new state is updated in l1
        let finalized_l1: ParsedLightClientState = contract.finalized_state().await?.into();
        assert_eq!(finalized_l1, new_state);
        Ok(())
    }
}
