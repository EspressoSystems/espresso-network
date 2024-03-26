//! A light client prover service

use crate::snark::{generate_state_update_proof, Proof, ProvingKey};
use anyhow::anyhow;
use async_std::{
    io,
    sync::Arc,
    task::{sleep, spawn},
};
use contract_bindings::light_client::LightClient;
use displaydoc::Display;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::Http,
    providers::{Middleware, Provider, ProviderError},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, U256},
};
use futures::FutureExt;
use hotshot_contract_adapter::jellyfish::{u256_to_field, ParsedPlonkProof};
use hotshot_contract_adapter::light_client::ParsedLightClientState;
use hotshot_orchestrator::OrchestratorVersion;
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use hotshot_stake_table::vec_based::config::FieldType;
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::signature_key::BLSPubKey;
use hotshot_types::traits::stake_table::{SnapshotVersion, StakeTableError, StakeTableScheme as _};
use hotshot_types::{
    light_client::{
        CircuitField, GenericPublicInput, LightClientState, PublicInput, StateSignaturesBundle,
        StateVerKey,
    },
    traits::signature_key::StakeTableEntryType,
};

use jf_plonk::errors::PlonkError;
use jf_primitives::constants::CS_ID_SCHNORR;
use jf_primitives::pcs::prelude::UnivariateUniversalParams;
use jf_relation::Circuit as _;
use std::{iter, time::Duration};
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::Instant;
use url::Url;
use versioned_binary_serialization::version::StaticVersionType;

type F = ark_ed_on_bn254::Fq;

/// A wallet with local signer and connected to network via http
pub type L1Wallet = SignerMiddleware<Provider<Http>, LocalWallet>;

type NetworkConfig = hotshot_orchestrator::config::NetworkConfig<
    BLSPubKey,
    hotshot::traits::election::static_committee::StaticElectionConfig,
>;

/// Configuration/Parameters used for hotshot state prover
#[derive(Debug, Clone)]
pub struct StateProverConfig {
    /// Url of the state relay server (a CDN that sequencers push their Schnorr signatures to)
    pub relay_server: Url,
    /// Interval between light client state update
    pub update_interval: Duration,
    /// URL of layer 1 Ethereum JSON-RPC provider.
    pub l1_provider: Url,
    /// Address of LightClient contract on layer 1.
    pub light_client_address: Address,
    /// Transaction signing key for Ethereum
    pub eth_signing_key: SigningKey,
    /// Address of the hotshot orchestrator, used for stake table initialization.
    pub orchestrator_url: Url,
    /// If daemon and provided, the service will run a basic HTTP server on the given port.
    ///
    /// The server provides healthcheck and version endpoints.
    pub port: Option<u16>,
}

pub fn init_stake_table(
    bls_keys: &[BLSPubKey],
    state_keys: &[StateVerKey],
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>, StakeTableError> {
    // We now initialize a static stake table as what hotshot orchestrator does.
    // In the future we should get the stake table from the contract.
    let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(STAKE_TABLE_CAPACITY);
    st.batch_register(
        bls_keys.iter().cloned(),
        iter::repeat(U256::one()).take(bls_keys.len()),
        state_keys.iter().cloned(),
    )?;
    st.advance();
    st.advance();
    Ok(st)
}

async fn init_stake_table_from_orchestrator(
    orchestrator_url: &Url,
) -> StakeTable<BLSPubKey, StateVerKey, CircuitField> {
    tracing::info!("Initializing stake table from HotShot orchestrator.");
    let client = Client::<ServerError, OrchestratorVersion>::new(orchestrator_url.clone());
    loop {
        match client.get::<bool>("api/peer_pub_ready").send().await {
            Ok(true) => {
                match client
                    .get::<NetworkConfig>("api/get_config_after_peer_collected")
                    .send()
                    .await
                {
                    Ok(config) => {
                        let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(
                            STAKE_TABLE_CAPACITY,
                        );
                        config
                            .config
                            .known_nodes_with_stake
                            .into_iter()
                            .for_each(|config| {
                                st.register(
                                    *config.stake_table_entry.get_key(),
                                    config.stake_table_entry.get_stake(),
                                    config.state_ver_key,
                                )
                                .expect("Key registration shouldn't fail.");
                            });
                        st.advance();
                        st.advance();
                        return st;
                    }
                    Err(e) => {
                        tracing::warn!("Orchestrator error: {e}, retrying.");
                    }
                }
            }
            Ok(false) => {
                tracing::info!("Peers' keys are not ready, retrying.");
            }
            Err(e) => {
                tracing::warn!("Orchestrator error {e}, retrying.");
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
}

pub async fn light_client_genesis(
    orchestrator_url: &Url,
) -> anyhow::Result<ParsedLightClientState> {
    let st = init_stake_table_from_orchestrator(orchestrator_url).await;
    let (bls_comm, schnorr_comm, stake_comm) = st
        .commitment(SnapshotVersion::LastEpochStart)
        .expect("Commitment computation shouldn't fail.");
    let threshold = st.total_stake(SnapshotVersion::LastEpochStart)? * 2 / 3;

    let pi = vec![
        u256_to_field(threshold),
        F::from(0_u64), // Arbitrary value for view number
        F::from(0_u64), // Arbitrary value for block height
        F::from(0_u64), // Arbitrary value for state commitment
        F::from(0_u64), // Arbitrary value for fee ledger commitment
        bls_comm,
        schnorr_comm,
        stake_comm,
    ];
    let pi: GenericPublicInput<F> = pi.into();
    Ok(pi.into())
}

pub fn load_proving_key() -> ProvingKey {
    let srs = {
        let num_gates = crate::circuit::build_for_preprocessing::<
            CircuitField,
            ark_ed_on_bn254::EdwardsConfig,
            STAKE_TABLE_CAPACITY,
        >()
        .unwrap()
        .0
        .num_gates();

        std::println!("Loading SRS from Aztec's ceremony...");
        let srs_timer = Instant::now();
        let srs = ark_srs::aztec20::kzg10_setup(num_gates + 2).expect("Aztec SRS fail to load");
        let srs_elapsed = srs_timer.elapsed();
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
    let (pk, _) = crate::snark::preprocess::<STAKE_TABLE_CAPACITY>(&srs)
        .expect("Fail to preprocess state prover circuit");
    let key_gen_elapsed = key_gen_timer.elapsed();
    std::println!("Done in {key_gen_elapsed:.3}");
    pk
}

pub async fn fetch_latest_state<Ver: StaticVersionType>(
    client: &Client<ServerError, Ver>,
) -> Result<StateSignaturesBundle, ServerError> {
    tracing::info!("Fetching the latest state signatures bundle from relay server.");
    client
        .get::<StateSignaturesBundle>("/api/state")
        .send()
        .await
}

/// prepare a contract interface ready to be read from or written to
async fn prepare_contract(
    config: &StateProverConfig,
) -> Result<LightClient<L1Wallet>, ProverError> {
    let provider = Provider::try_from(config.l1_provider.to_string())
        .expect("unable to instantiate Provider, likely wrong URL");
    let signer = Wallet::from(config.eth_signing_key.clone())
        .with_chain_id(provider.get_chainid().await?.as_u64());
    let l1_wallet = Arc::new(L1Wallet::new(provider, signer));

    let contract = LightClient::new(config.light_client_address, l1_wallet);
    Ok(contract)
}

/// get the `finalizedState` from the LightClient contract storage on L1
pub async fn read_contract_state(
    config: &StateProverConfig,
) -> Result<LightClientState, ProverError> {
    let contract = prepare_contract(config).await?;
    let state: ParsedLightClientState = match contract.get_finalized_state().call().await {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::error!("unable to read finalized_state from contract: {}", e);
            return Err(ProverError::ContractError(e.into()));
        }
    };
    let state: LightClientState = state.into();

    Ok(state)
}

/// submit the latest finalized state along with a proof to the L1 LightClient contract
pub async fn submit_state_and_proof(
    proof: Proof,
    public_input: PublicInput,
    config: &StateProverConfig,
) -> Result<(), ProverError> {
    let contract = prepare_contract(config).await?;

    // prepare the input the contract call and the tx itself
    let proof: ParsedPlonkProof = proof.into();
    let new_state: ParsedLightClientState = public_input.into();
    let tx = contract.new_finalized_state(new_state.into(), proof.into());

    // send the tx
    let (receipt, included_block) = sequencer_utils::contract_send(&tx)
        .await
        .map_err(ProverError::ContractError)?;

    tracing::info!(
        "Submitted state and proof to L1: tx={:x} block={included_block}",
        receipt.transaction_hash,
    );

    Ok(())
}

pub async fn sync_state<Ver: StaticVersionType>(
    st: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    proving_key: &ProvingKey,
    relay_server_client: &Client<ServerError, Ver>,
    config: &StateProverConfig,
) -> Result<(), ProverError> {
    tracing::info!("Start syncing light client state.");

    let bundle = fetch_latest_state(relay_server_client).await?;
    tracing::info!("Latest HotShot block height: {}", bundle.state.block_height);
    let old_state = read_contract_state(config).await?;
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

    let threshold = st.total_stake(SnapshotVersion::LastEpochStart)? * 2 / 3;
    tracing::info!("Threshold before syncing state: {}", threshold);
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
            let state_msg: [FieldType; 7] = (&bundle.state).into();
            if key.verify(&state_msg, sig, CS_ID_SCHNORR).is_ok() {
                signer_bit_vec[i] = true;
                signatures[i] = sig.clone();
                accumulated_weight += *stake;
            }
        }
    });

    if accumulated_weight < threshold {
        return Err(ProverError::InvalidState(
            "The signers' total weight doesn't reach the threshold.".to_string(),
        ));
    }

    // TODO this assert fails. See https://github.com/EspressoSystems/espresso-sequencer/issues/1161
    // assert_eq!(
    //     bundle.state.stake_table_comm,
    //     st.commitment(SnapshotVersion::LastEpochStart).unwrap()
    // );

    tracing::info!("Collected latest state and signatures. Start generating SNARK proof.");
    let proof_gen_start = time::Instant::now();
    let (proof, public_input) = generate_state_update_proof::<_, _, _, _, STAKE_TABLE_CAPACITY>(
        &mut ark_std::rand::thread_rng(),
        proving_key,
        &entries,
        signer_bit_vec,
        signatures,
        &bundle.state,
        &threshold,
    )?;
    let proof_gen_elapsed = proof_gen_start.elapsed();
    tracing::info!("Proof generation completed. Elapsed: {proof_gen_elapsed:.3}");

    submit_state_and_proof(proof, public_input, config).await?;

    tracing::info!("Successfully synced light client state.");
    Ok(())
}

fn start_http_server<Ver: StaticVersionType + 'static>(
    port: u16,
    lightclient_address: Address,
    bind_version: Ver,
) -> io::Result<()> {
    let mut app = tide_disco::App::<(), ServerError, Ver>::with_state(());
    let toml = toml::from_str::<toml::value::Value>(include_str!("../api/prover-service.toml"))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let mut api = Api::<(), ServerError, Ver>::new(toml)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    api.get("getlightclientcontract", move |_, _| {
        async move { Ok(lightclient_address) }.boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    app.register_module("api", api)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    spawn(app.serve(format!("0.0.0.0:{port}"), bind_version));
    Ok(())
}

pub async fn run_prover_service<Ver: StaticVersionType + 'static>(
    config: StateProverConfig,
    bind_version: Ver,
) {
    // TODO(#1022): maintain the following stake table
    let st = Arc::new(init_stake_table_from_orchestrator(&config.orchestrator_url).await);
    let proving_key = Arc::new(load_proving_key());
    let relay_server_client =
        Arc::new(Client::<ServerError, Ver>::new(config.relay_server.clone()));
    let config = Arc::new(config);
    let update_interval = config.update_interval;

    tracing::info!("Light client address: {:?}", config.light_client_address);

    if let Some(port) = config.port {
        if let Err(err) = start_http_server(port, config.light_client_address, bind_version) {
            tracing::error!("Error starting http server: {}", err);
        }
    }

    loop {
        let st = st.clone();
        let proving_key = proving_key.clone();
        let relay_server_client = relay_server_client.clone();
        let config = config.clone();
        spawn(async move {
            if let Err(err) = sync_state(&st, &proving_key, &relay_server_client, &config).await {
                tracing::error!("Cannot sync the light client state: {}", err);
            }
        });
        sleep(update_interval).await;
    }
}

/// Run light client state prover once
pub async fn run_prover_once<Ver: StaticVersionType>(config: StateProverConfig, _: Ver) {
    let st = init_stake_table_from_orchestrator(&config.orchestrator_url).await;
    let proving_key = load_proving_key();
    let relay_server_client = Client::<ServerError, Ver>::new(config.relay_server.clone());

    sync_state(&st, &proving_key, &relay_server_client, &config)
        .await
        .expect("Error syncing the light client state.");
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

impl std::error::Error for ProverError {}

#[cfg(test)]
mod test {

    use super::*;
    use crate::mock_ledger::{MockLedger, MockSystemParam};
    use anyhow::Result;
    use ark_ed_on_bn254::EdwardsConfig;
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use ethers::{
        abi::AbiEncode,
        providers::Middleware,
        utils::{Anvil, AnvilInstance},
    };
    use hotshot_stake_table::vec_based::StakeTable;
    use hotshot_types::light_client::StateSignKey;
    use jf_primitives::signatures::{SchnorrSignatureScheme, SignatureScheme};
    use jf_utils::test_rng;
    use std::process::Command;

    const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;
    const BLOCKS_PER_EPOCH: u32 = 10;
    const NUM_INIT_VALIDATORS: u32 = (STAKE_TABLE_CAPACITY_FOR_TEST / 2) as u32;
    const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

    /// Init a meaningful ledger state that prover can generate future valid proof.
    /// this is used for testing purposes, contract deployed to test proof verification should also be initialized with this genesis
    ///
    #[allow(clippy::type_complexity)]
    fn init_ledger_for_test() -> (
        ParsedLightClientState,
        Vec<BLSPubKey>,
        Vec<(StateSignKey, StateVerKey)>,
        StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) {
        let pp = MockSystemParam::init(BLOCKS_PER_EPOCH);
        let ledger = MockLedger::init(pp, NUM_INIT_VALIDATORS as usize);

        let genesis = ledger.get_state();
        let qc_keys = ledger.qc_keys;
        let state_keys = ledger.state_keys;
        let st = ledger.st;

        eprintln!(
            "Genesis: view_num: {}, block_height: {}, block_comm_root: {}, fee_ledger_comm: {}\
             bls_key_comm: {:x?},\
             schnorr_key_comm: {:x?},\
             amount_comm: {:x?},\
             threshold: {}",
            genesis.view_num,
            genesis.block_height,
            genesis.block_comm_root,
            genesis.fee_ledger_comm,
            genesis.bls_key_comm.encode_hex(),
            genesis.schnorr_key_comm.encode_hex(),
            genesis.amount_comm.encode_hex(),
            genesis.threshold,
        );
        (genesis, qc_keys, state_keys, st)
    }

    // everybody signs, then generate a proof
    fn gen_state_proof(
        old_state: &ParsedLightClientState,
        new_state: ParsedLightClientState,
        state_keypairs: &[(StateSignKey, StateVerKey)],
        st: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) -> (PublicInput, Proof) {
        let mut rng = test_rng();

        let new_state_msg: [CircuitField; 7] = {
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
            let srs = ark_srs::aztec20::kzg10_setup(2u64.pow(16) as usize + 2)
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
        let (pk, _) = crate::preprocess::<STAKE_TABLE_CAPACITY_FOR_TEST>(&srs)
            .expect("Fail to preprocess state prover circuit");
        let stake_table_entries = st
            .try_iter(SnapshotVersion::LastEpochStart)
            .unwrap()
            .map(|(_, stake_amount, schnorr_key)| (schnorr_key, stake_amount))
            .collect::<Vec<_>>();
        let (proof, pi) =
            crate::generate_state_update_proof::<_, _, _, _, STAKE_TABLE_CAPACITY_FOR_TEST>(
                &mut rng,
                &pk,
                &stake_table_entries,
                &bit_vec,
                &sigs,
                &new_state.into(),
                &old_state.threshold,
            )
            .expect("Fail to generate state proof");

        (pi, proof)
    }

    /// deploy LightClient.sol on local blockchain (via `anvil`) for testing
    /// return (signer-loaded wallet, contract instance)
    async fn deploy_contract_for_test(
        anvil: &AnvilInstance,
    ) -> Result<(Arc<L1Wallet>, LightClient<L1Wallet>)> {
        let provider = Provider::<Http>::try_from(anvil.endpoint())?;
        let signer = Wallet::from(anvil.keys()[0].clone());
        let l1_wallet = Arc::new(L1Wallet::new(provider.clone(), signer));

        let output = Command::new("just")
            .arg("dev-deploy")
            .arg(anvil.endpoint())
            .arg(TEST_MNEMONIC)
            .arg(BLOCKS_PER_EPOCH.to_string())
            .arg(NUM_INIT_VALIDATORS.to_string())
            .output()
            .expect("fail to deploy");

        if !output.status.success() {
            tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
            return Err(anyhow!("failed to deploy contract"));
        }

        let last_blk_num = provider.get_block_number().await?;
        // the first tx deploys PlonkVerifier.sol library, the second deploys LightClient.sol
        let address = provider
            .get_block_receipts(last_blk_num)
            .await?
            .last()
            .unwrap()
            .contract_address
            .expect("fail to get LightClient address from receipt");

        let contract = LightClient::new(address, l1_wallet.clone());
        Ok((l1_wallet, contract))
    }

    impl StateProverConfig {
        /// update only L1 related info
        fn update_l1_info(&mut self, anvil: &AnvilInstance, light_client_address: Address) {
            self.l1_provider = Url::parse(&anvil.endpoint()).unwrap();
            self.light_client_address = light_client_address;
            self.eth_signing_key = anvil.keys()[0].clone().into();
        }
    }
    // only for testing purposes
    impl Default for StateProverConfig {
        fn default() -> Self {
            Self {
                relay_server: Url::parse("http://localhost").unwrap(),
                update_interval: Duration::default(),
                l1_provider: Url::parse("http://localhost").unwrap(),
                light_client_address: Address::default(),
                eth_signing_key: SigningKey::random(&mut test_rng()),
                orchestrator_url: Url::parse("http://localhost").unwrap(),
                port: None,
            }
        }
    }

    #[async_std::test]
    async fn test_read_contract_state() -> Result<()> {
        setup_logging();
        setup_backtrace();

        let anvil = Anvil::new().spawn();
        let (_wallet, contract) = deploy_contract_for_test(&anvil).await?;

        // now test if we can read from the contract
        assert_eq!(contract.blocks_per_epoch().call().await?, BLOCKS_PER_EPOCH);
        let genesis: ParsedLightClientState = contract.get_genesis_state().await?.into();
        // NOTE: these values changes with `contracts/scripts/LightClient.s.sol`
        assert_eq!(genesis.view_num, 0);
        assert_eq!(genesis.block_height, 0);
        assert_eq!(genesis.threshold, U256::from(40));

        let mut config = StateProverConfig::default();
        config.update_l1_info(&anvil, contract.address());
        let state = super::read_contract_state(&config).await?;
        assert_eq!(state, genesis.into());
        Ok(())
    }

    // This test is temporarily ignored. We are unifying the contract deployment in #1071.
    #[async_std::test]
    async fn test_submit_state_and_proof() -> Result<()> {
        setup_logging();
        setup_backtrace();

        let (genesis, _qc_keys, state_keys, st) = init_ledger_for_test();

        let anvil = Anvil::new().spawn();
        let (_wallet, contract) = deploy_contract_for_test(&anvil).await?;
        let mut config = StateProverConfig::default();
        config.update_l1_info(&anvil, contract.address());
        // sanity check on `config`

        // sanity check to ensure the same genesis state for LightClientTest and for our tests
        let genesis_l1: ParsedLightClientState = contract.get_genesis_state().await?.into();
        assert_eq!(genesis_l1, genesis, "mismatched genesis, aborting tests");

        let mut new_state = genesis.clone();
        new_state.view_num = 5;
        new_state.block_height = 1;

        let (pi, proof) = gen_state_proof(&genesis, new_state.clone(), &state_keys, &st);
        tracing::info!("Successfully generated proof for new state.");

        super::submit_state_and_proof(proof, pi, &config).await?;
        tracing::info!("Successfully submitted new finalized state to L1.");
        // test if new state is updated in l1
        let finalized_l1: ParsedLightClientState = contract.get_finalized_state().await?.into();
        assert_eq!(finalized_l1, new_state);
        Ok(())
    }
}
