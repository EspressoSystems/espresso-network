//! A light client prover service

use std::{collections::HashMap, sync::Arc, time::Instant};

use alloy::{
    network::EthereumWallet,
    primitives::{utils::format_units, Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
};
use anyhow::{anyhow, Result};
use futures::FutureExt;
use hotshot_contract_adapter::{
    field_to_u256,
    sol_types::{LightClient, LightClientStateSol, PlonkProofSol, StakeTableStateSol},
};
use hotshot_types::{
    light_client::{CircuitField, LightClientState, StakeTableState, StateSignature, StateVerKey},
    traits::signature_key::StateSignatureKey,
};
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_relation::Circuit as _;
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::ext::InstantExt;
use tokio::{io, spawn, task::spawn_blocking, time::sleep};
use vbs::version::StaticVersionType;

use crate::{
    legacy::snark::{Proof, ProvingKey, PublicInput},
    service::{fetch_latest_state, ProverError, ProverServiceState, StateProverConfig},
};

pub fn load_proving_key(stake_table_capacity: usize) -> ProvingKey {
    let srs = {
        let num_gates = super::circuit::build_for_preprocessing::<
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

/// Read the following info from the LightClient contract storage on chain
/// - latest finalized light client state
/// - stake table commitment used in currently active epoch
///
/// Returned types are of Rust struct defined in `hotshot-types`.
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

    let tx = contract.newFinalizedState(new_state, proof);
    tracing::debug!(
        "Sending newFinalizedState tx: address={}, new_state={}\n full tx={:?}",
        address,
        public_input.lc_state,
        tx
    );
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

async fn generate_proof(
    state: &mut ProverServiceState,
    light_client_state: LightClientState,
    current_stake_table_state: StakeTableState,
    next_stake_table_state: StakeTableState,
    signature_map: HashMap<StateVerKey, StateSignature>,
    proving_key: &ProvingKey,
) -> Result<(Proof, PublicInput), ProverError> {
    // Stake table update is already handled in the epoch catchup
    let entries = state
        .stake_table
        .iter()
        .map(|entry| {
            (
                entry.state_ver_key.clone(),
                entry.stake_table_entry.stake_amount,
            )
        })
        .collect::<Vec<_>>();
    let mut signer_bit_vec = vec![false; entries.len()];
    let mut signatures = vec![Default::default(); entries.len()];
    let mut accumulated_weight = U256::ZERO;
    entries.iter().enumerate().for_each(|(i, (key, stake))| {
        if let Some(sig) = signature_map.get(key) {
            // Check if the signature is valid
            if key.verify_state_sig(sig, &light_client_state, &next_stake_table_state) {
                signer_bit_vec[i] = true;
                signatures[i] = sig.clone();
                accumulated_weight += *stake;
            } else {
                tracing::info!("Invalid signature for key: {:?}", key);
            }
        }
    });

    if accumulated_weight < field_to_u256(current_stake_table_state.threshold) {
        return Err(ProverError::InvalidState(
            "The signers' total weight doesn't reach the threshold.".to_string(),
        ));
    }

    tracing::info!("Collected latest state and signatures. Start generating SNARK proof.");
    let proof_gen_start = Instant::now();
    let proving_key_clone = proving_key.clone();
    let stake_table_capacity = state.config.stake_table_capacity;
    let (proof, public_input) = spawn_blocking(move || {
        super::snark::generate_state_update_proof(
            &mut ark_std::rand::thread_rng(),
            &proving_key_clone,
            entries,
            signer_bit_vec,
            signatures,
            &light_client_state,
            &current_stake_table_state,
            stake_table_capacity,
        )
    })
    .await
    .map_err(|e| ProverError::Internal(format!("failed to join task: {e}")))??;

    let proof_gen_elapsed = Instant::now().signed_duration_since(proof_gen_start);
    tracing::info!("Proof generation completed. Elapsed: {proof_gen_elapsed:.3}");

    Ok((proof, public_input))
}

/// Sync the light client state from the relay server and submit the proof to the L1 LightClient contract
pub async fn sync_state<ApiVer: StaticVersionType>(
    state: &mut ProverServiceState,
    proving_key: &ProvingKey,
    relay_server_client: &Client<ServerError, ApiVer>,
) -> Result<(), ProverError> {
    let light_client_address = state.config.light_client_address;
    let wallet = EthereumWallet::from(state.config.signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(state.config.provider_endpoint.clone());

    // only sync light client state when gas price is sane
    if let Some(max_gas_price) = state.config.max_gas_price {
        let cur_gas_price = provider
            .get_gas_price()
            .await
            .map_err(|e| ProverError::NetworkError(anyhow!("{e}")))?;
        if cur_gas_price > max_gas_price {
            let cur_gwei = format_units(cur_gas_price, "gwei")
                .map_err(|e| ProverError::Internal(format!("{e}")))?;
            let max_gwei = format_units(max_gas_price, "gwei")
                .map_err(|e| ProverError::Internal(format!("{e}")))?;
            tracing::warn!(
                "Current gas price too high: cur={} gwei, max={} gwei",
                cur_gwei,
                max_gwei,
            );
            return Err(ProverError::GasPriceTooHigh(cur_gwei, max_gwei));
        }
    }

    tracing::info!(
        ?light_client_address,
        "Start syncing light client state for provider: {}",
        state.config.provider_endpoint,
    );

    let (contract_state, contract_st_state) =
        read_contract_state(&provider, light_client_address).await?;
    tracing::info!(
        "Current HotShot block height on contract: {}",
        contract_state.block_height
    );

    let bundle = fetch_latest_state(relay_server_client).await?;
    tracing::debug!("Bundle accumulated weight: {}", bundle.accumulated_weight);
    tracing::info!("Latest HotShot block height: {}", bundle.state.block_height);

    if contract_state.block_height >= bundle.state.block_height {
        tracing::info!("No update needed.");
        return Ok(());
    }
    tracing::debug!("Old state: {contract_state:?}");
    tracing::debug!("New state: {:?}", bundle.state);

    tracing::debug!("Contract st state: {contract_st_state}");
    tracing::debug!("Bundle st state: {}", bundle.next_stake);

    // If epoch hasn't been enabled, directly update the contract.
    let (proof, public_input) = generate_proof(
        state,
        bundle.state,
        contract_st_state,
        contract_st_state,
        bundle.signatures,
        proving_key,
    )
    .await?;

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
    let toml = toml::from_str::<toml::value::Value>(include_str!("../../api/prover-service.toml"))
        .map_err(io::Error::other)?;

    let mut api = Api::<_, ServerError, ApiVer>::new(toml).map_err(io::Error::other)?;

    api.get("getlightclientcontract", move |_, _| {
        async move { Ok(light_client_address) }.boxed()
    })
    .map_err(io::Error::other)?;
    app.register_module("api", api).map_err(io::Error::other)?;

    spawn(app.serve(format!("0.0.0.0:{port}"), bind_version));
    Ok(())
}

/// Run prover in daemon mode
pub async fn run_prover_service<ApiVer: StaticVersionType + 'static>(
    config: StateProverConfig,
    bind_version: ApiVer,
) -> Result<()> {
    let mut state = ProverServiceState::new_genesis(config).await?;

    let stake_table_capacity = state.config.stake_table_capacity;
    tracing::info!("Stake table capacity: {}", stake_table_capacity);

    tracing::info!(
        "Light client address: {:?}",
        state.config.light_client_address
    );

    let relay_server_client = Arc::new(Client::<ServerError, ApiVer>::new(
        state.config.relay_server.clone(),
    ));

    // Start the HTTP server to get a functioning healthcheck before any heavy computations.
    if let Some(port) = state.config.port {
        if let Err(err) = start_http_server(port, state.config.light_client_address, bind_version) {
            tracing::error!("Error starting http server: {}", err);
        }
    }

    let proving_key =
        spawn_blocking(move || Arc::new(load_proving_key(state.config.stake_table_capacity)))
            .await?;

    let update_interval = state.config.update_interval;
    let retry_interval = state.config.retry_interval;
    loop {
        if let Err(err) = sync_state(&mut state, &proving_key, &relay_server_client).await {
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
    let mut state = ProverServiceState::new_genesis(config).await?;

    let stake_table_capacity = state.config.stake_table_capacity;
    let proving_key =
        spawn_blocking(move || Arc::new(load_proving_key(stake_table_capacity))).await?;
    let relay_server_client = Client::<ServerError, ApiVer>::new(state.config.relay_server.clone());

    for _ in 0..state.config.max_retries {
        match sync_state(&mut state, &proving_key, &relay_server_client).await {
            Ok(_) => return Ok(()),
            Err(ProverError::GasPriceTooHigh(..)) => {
                // static ERROR message for easier observability and alert
                tracing::error!("Gas price too high, sync later");
            },
            Err(err) => {
                tracing::error!("Cannot sync the light client state, will retry: {}", err);
                sleep(state.config.retry_interval).await;
            },
        }
    }
    Err(anyhow::anyhow!("State update failed"))
}

#[cfg(test)]
mod test {

    use alloy::{node_bindings::Anvil, providers::layers::AnvilProvider, sol_types::SolValue};
    use anyhow::Result;
    use espresso_contract_deployer::{deploy_light_client_proxy, Contracts};
    use hotshot_contract_adapter::sol_types::LightClientMock;
    use jf_utils::test_rng;
    use sequencer_utils::test_utils::setup_test;

    use super::*;
    use crate::legacy::mock_ledger::{MockLedger, MockSystemParam, STAKE_TABLE_CAPACITY_FOR_TEST};

    // const MAX_HISTORY_SECONDS: u32 = 864000;
    const NUM_INIT_VALIDATORS: usize = STAKE_TABLE_CAPACITY_FOR_TEST / 2;

    /// This helper function deploy LightClient V1, and its Proxy, then deploy V2 and upgrade the proxy.
    /// Returns the address of the proxy, caller can cast the address to be `LightClientV2` or `LightClientV2Mock`
    async fn deploy(
        provider: impl Provider,
        contracts: &mut Contracts,
        is_mock: bool,
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
            is_mock,
            genesis_state.clone(),
            genesis_stake.clone(),
            admin,
            Some(prover),
        )
        .await?;

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

        println!("genesis_state: {:?}", genesis_state);
        let lc_proxy_addr = deploy(
            &provider,
            &mut contracts,
            true,
            genesis_state.clone(),
            genesis_stake.clone(),
        )
        .await?;
        let (state, st_state) = read_contract_state(&provider, lc_proxy_addr).await?;

        // first test the default storage
        assert_eq!(state, genesis_state.into());
        assert_eq!(st_state, genesis_stake.clone().into());

        // then manually set the `finalizedState` (via mocked methods)
        let lc = LightClientMock::new(lc_proxy_addr, &provider);
        let new_state = LightClientStateSol::rand(rng);
        println!("new_state: {:?}", new_state);
        lc.setFinalizedState(new_state.clone().into())
            .send()
            .await?
            .watch()
            .await?;

        // now query again, the states read should reflect the changes
        let (state, st_state) = read_contract_state(&provider, lc_proxy_addr).await?;
        assert_eq!(state, new_state.into());
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

        let anvil = Anvil::new().spawn();
        let wallet = anvil.wallet().unwrap();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        // a provider that holds both anvil (to avoid accidental drop) and wallet-enabled L1 provider
        let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));
        let mut contracts = Contracts::new();

        let lc_proxy_addr = deploy(
            &provider,
            &mut contracts,
            true,
            genesis_state,
            genesis_stake.clone(),
        )
        .await?;
        let lc = LightClientMock::new(lc_proxy_addr, &provider);

        // get 10 blocks
        for _ in 0..10 {
            ledger.elapse_with_block();
        }
        let (pi, proof) = ledger.gen_state_proof();
        tracing::info!("Successfully generated proof for new state.");

        submit_state_and_proof(&provider, lc_proxy_addr, proof, pi).await?;
        tracing::info!("Successfully submitted new finalized state to L1.");

        // test if new state is updated in l1
        let finalized_l1: LightClientStateSol = lc.finalizedState().call().await?.into();
        let expected: LightClientStateSol = ledger.light_client_state().into();
        assert_eq!(
            finalized_l1.abi_encode_params(),
            expected.abi_encode_params(),
            "finalizedState not updated"
        );

        Ok(())
    }
}
