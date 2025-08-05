//! A light client prover service

// TODO(Chengyu): this service is still under development.

use std::{collections::HashMap, sync::Arc, time::Instant};

use alloy::{
    network::EthereumWallet,
    primitives::{utils::format_units, Address, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
};
use anyhow::{anyhow, Context, Result};
use espresso_types::SeqTypes;
use futures::FutureExt;
use hotshot_contract_adapter::{
    field_to_u256,
    sol_types::{LightClientStateSol, LightClientV2, StakeTableStateSol},
};
use hotshot_query_service::availability::StateCertQueryData;
use hotshot_task_impls::helpers::derive_signed_state_digest;
use hotshot_types::{
    data::EpochNumber,
    light_client::{
        CircuitField, LightClientState, StakeTableState, StateSignature, StateSignaturesBundle,
        StateVerKey,
    },
    simple_certificate::LightClientStateUpdateCertificate,
    traits::{
        node_implementation::{ConsensusTime, NodeType},
        signature_key::LCV2StateSignatureKey,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, is_ge_epoch_root, option_epoch_from_block_number,
    },
};
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_relation::Circuit as _;
use surf_disco::Client;
use tide_disco::{error::ServerError, Api};
use time::ext::InstantExt;
use tokio::{io, spawn, task::spawn_blocking, time::sleep};
use url::Url;
use vbs::version::{StaticVersion, StaticVersionType};

use crate::{
    v3::snark::{Proof, ProvingKey, PublicInput},
    ProverError, ProverServiceState, StateProverConfig,
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
        let srs = ark_srs::kzg10::aztec20::setup(num_gates + 2)
            .expect("Error loading proving key: Aztec SRS fail to load.");
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
    let (pk, _) = super::snark::preprocess(&srs, stake_table_capacity)
        .expect("Error loading proving key: failed to preprocess state prover circuit.");
    let key_gen_elapsed = Instant::now().signed_duration_since(key_gen_timer);
    tracing::info!("Done in {key_gen_elapsed:.3}");
    pk
}

#[inline(always)]
/// Get the latest LightClientState and signature bundle from Sequencer network
pub async fn fetch_latest_state<ApiVer: StaticVersionType>(
    client: &Client<ServerError, ApiVer>,
) -> Result<StateSignaturesBundle, ProverError> {
    tracing::info!("Fetching the latest state signatures bundle from relay server.");
    client
        .get::<StateSignaturesBundle>("/api/state")
        .send()
        .await
        .map_err(ProverError::RelayServerError)
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
    _provider: impl Provider,
    _address: Address,
    _proof: Proof,
    _public_input: PublicInput,
) -> Result<TransactionReceipt, ProverError> {
    todo!("Waiting for light client V3 contract")
}

async fn fetch_epoch_state_from_sequencer(
    sequencer_url: &Url,
    epoch: u64,
) -> Result<LightClientStateUpdateCertificate<SeqTypes>, ProverError> {
    let state_cert =
        surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
            sequencer_url.clone(),
        )
        .get::<StateCertQueryData<SeqTypes>>(&format!("availability/state-cert/{epoch}"))
        .send()
        .await
        .map_err(|err| {
            ProverError::SequencerCommunicationError(
                sequencer_url
                    .join(&format!("availability/state-cert/{epoch}"))
                    .unwrap(),
                err,
            )
        })?;
    Ok(state_cert.0)
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
            if <StateVerKey as LCV2StateSignatureKey>::verify_state_sig(
                key,
                sig,
                &light_client_state,
                &next_stake_table_state,
            ) {
                signer_bit_vec[i] = true;
                signatures[i] = sig.clone();
                accumulated_weight += *stake;
            } else {
                tracing::warn!("Invalid signature from key: {}", key);
            }
        }
    });
    tracing::debug!(
        "Collected signatures with accumulated weight: {}",
        accumulated_weight
    );

    if accumulated_weight < field_to_u256(current_stake_table_state.threshold) {
        return Err(ProverError::InvalidState(
            "The signers' total weight doesn't reach the threshold.".to_string(),
        ));
    }

    tracing::info!("Collected latest state and signatures. Start generating SNARK proof.");
    let proof_gen_start = Instant::now();
    let proving_key_clone = proving_key.clone();
    let stake_table_capacity = state.config.stake_table_capacity;
    let auth_root = FixedBytes::from([0; 32]); // TODO(Chengyu): replace with actual auth_root
    let signed_state_digest =
        derive_signed_state_digest(&light_client_state, &next_stake_table_state, &auth_root);
    let (proof, public_input) = spawn_blocking(move || {
        super::snark::generate_state_update_proof(
            &mut ark_std::rand::thread_rng(),
            &proving_key_clone,
            entries,
            signer_bit_vec,
            signatures,
            &current_stake_table_state,
            stake_table_capacity,
            &signed_state_digest,
        )
    })
    .await
    .with_context(|| "Failed to join the proof generation task")
    .map_err(ProverError::Internal)??;

    let proof_gen_elapsed = Instant::now().signed_duration_since(proof_gen_start);
    tracing::info!("Proof generation completed. Elapsed: {proof_gen_elapsed:.3}");

    Ok((proof, public_input))
}

/// This function will fetch the cross epoch state update information from the sequencer query node
/// and update the light client state in the contract to the `target_epoch`.
/// In the end, both the locally stored stake table and the contract light client state will correspond
/// to the `target_epoch`.
/// It returns the final stake table state at the target epoch.
async fn advance_epoch(
    state: &mut ProverServiceState,
    provider: impl Provider,
    light_client_address: Address,
    mut cur_st_state: StakeTableState,
    proving_key: &ProvingKey,
    contract_epoch: Option<<SeqTypes as NodeType>::Epoch>,
    target_epoch: Option<<SeqTypes as NodeType>::Epoch>,
) -> Result<StakeTableState, ProverError> {
    let Some(target_epoch) = target_epoch else {
        return Err(ProverError::Internal(anyhow!(
            "Epoch related function called without a target epoch"
        )));
    };
    // First sync the local stake table if necessary.
    if state.epoch != contract_epoch {
        state
            .sync_with_epoch(contract_epoch)
            .await
            .with_context(|| format!("Failed to sync with epoch {contract_epoch:?}"))
            .map_err(ProverError::Internal)?;
    }
    let base_epoch = contract_epoch
        .map(|en| en.u64())
        .unwrap_or(0)
        .max(epoch_from_block_number(
            state.config.epoch_start_block,
            state.config.blocks_per_epoch,
        ));
    let target_epoch = target_epoch.u64();
    for epoch in base_epoch..target_epoch {
        tracing::info!("Performing epoch root state update for epoch {epoch}...");
        let state_cert =
            fetch_epoch_state_from_sequencer(&state.config.sequencer_url, epoch).await?;
        let signature_map = state_cert
            .signatures
            .into_iter()
            .collect::<HashMap<StateVerKey, StateSignature>>();

        let (proof, public_input) = generate_proof(
            state,
            state_cert.light_client_state,
            cur_st_state,
            state_cert.next_stake_table_state,
            signature_map,
            proving_key,
        )
        .await?;

        submit_state_and_proof(&provider, light_client_address, proof, public_input).await?;
        tracing::info!("Epoch root state update successfully for epoch {epoch}.");

        state
            .sync_with_epoch(Some(EpochNumber::new(epoch + 1)))
            .await
            .with_context(|| format!("Failed to sync with epoch {}", epoch + 1))
            .map_err(ProverError::Internal)?;
        cur_st_state = state_cert.next_stake_table_state;
    }
    Ok(cur_st_state)
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
        .on_client(state.config.l1_rpc_client.clone());

    // only sync light client state when gas price is sane
    if let Some(max_gas_price) = state.config.max_gas_price {
        let cur_gas_price = provider
            .get_gas_price()
            .await
            .with_context(|| "Error checking gas price")
            .map_err(ProverError::ContractError)?;
        if cur_gas_price > max_gas_price {
            let cur_gwei =
                format_units(cur_gas_price, "gwei").map_err(|e| ProverError::Internal(e.into()))?;
            let max_gwei =
                format_units(max_gas_price, "gwei").map_err(|e| ProverError::Internal(e.into()))?;
            return Err(ProverError::GasPriceTooHigh(cur_gwei, max_gwei));
        }
    }

    let blocks_per_epoch = state.config.blocks_per_epoch;
    let epoch_start_block = state.config.epoch_start_block;

    let (contract_state, mut contract_st_state) =
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
    tracing::debug!("Old light client state: {contract_state}");
    tracing::debug!("New light client state: {}", bundle.state);

    tracing::debug!("Contract stake table state: {contract_st_state}");
    tracing::debug!("Bundle stake table state: {}", bundle.next_stake);

    let contract_state_epoch_enabled = contract_state.block_height >= epoch_start_block;
    let epoch_enabled = bundle.state.block_height >= epoch_start_block;

    if !epoch_enabled {
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
    } else {
        // After the epoch is enabled
        let contract_epoch = option_epoch_from_block_number::<SeqTypes>(
            contract_state_epoch_enabled,
            contract_state.block_height,
            blocks_per_epoch,
        );
        // If the last contract update was on an epoch root, it's already on the next epoch.
        let contract_epoch = if contract_state_epoch_enabled
            && is_epoch_root(contract_state.block_height, blocks_per_epoch)
        {
            contract_epoch.map(|en| en + 1)
        } else {
            contract_epoch
        };

        let bundle_epoch = option_epoch_from_block_number::<SeqTypes>(
            epoch_enabled,
            bundle.state.block_height,
            blocks_per_epoch,
        );
        let bundle_next_epoch = bundle_epoch.map(|en| en + 1);

        // Update the local stake table if necessary
        if contract_epoch != state.epoch {
            state
                .sync_with_epoch(contract_epoch)
                .await
                .map_err(ProverError::Internal)?;
        }

        // A catchup is needed if the contract epoch is behind.
        if bundle_epoch > state.epoch {
            tracing::info!(
                "Catching up from epoch {contract_epoch:?} to epoch {bundle_epoch:?}..."
            );
            contract_st_state = advance_epoch(
                state,
                &provider,
                light_client_address,
                contract_st_state,
                proving_key,
                contract_epoch,
                bundle_epoch,
            )
            .await?;
        }

        // Now that the contract epoch should be equal to the bundle epoch.

        if is_ge_epoch_root(bundle.state.block_height as u64, blocks_per_epoch) {
            // If we reached the epoch root, proceed to the next epoch directly
            // In theory this should never happen because the node won't sign them.
            tracing::info!("Epoch reaching an end, proceed to the next epoch...");
            advance_epoch(
                state,
                &provider,
                light_client_address,
                contract_st_state,
                proving_key,
                bundle_epoch,
                bundle_next_epoch,
            )
            .await?;
        } else {
            // Otherwise process the bundle update information as usual
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
        }
    }
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
            tracing::error!(
                "Cannot sync the light client state, will retry in {:.1}s: {}",
                retry_interval.as_secs_f32(),
                err
            );
            sleep(retry_interval).await;
        } else {
            tracing::info!("Sleeping for {:.1}s", update_interval.as_secs_f32());
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
            Err(err) => {
                tracing::error!(
                    "Cannot sync the light client state, will retry in {:.1}s: {}",
                    state.config.retry_interval.as_secs_f32(),
                    err
                );
                sleep(state.config.retry_interval).await;
            },
        }
    }
    Err(anyhow::anyhow!("State update failed"))
}

#[cfg(test)]
mod test {

    use alloy::providers::ProviderBuilder;
    use anyhow::Result;
    use espresso_contract_deployer::{
        deploy_light_client_proxy, upgrade_light_client_v2, Contracts,
    };
    use hotshot_contract_adapter::sol_types::LightClientV2Mock;
    use jf_utils::test_rng;

    use super::*;
    use crate::v3::mock_ledger::{EPOCH_HEIGHT_FOR_TEST, EPOCH_START_BLOCK_FOR_TEST};

    // const MAX_HISTORY_SECONDS: u32 = 864000;
    // const NUM_INIT_VALIDATORS: usize = STAKE_TABLE_CAPACITY_FOR_TEST / 2;

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
        upgrade_light_client_v2(
            &provider,
            contracts,
            is_mock_v2,
            EPOCH_HEIGHT_FOR_TEST,
            EPOCH_START_BLOCK_FOR_TEST,
        )
        .await?;

        Ok(lc_proxy_addr)
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_read_contract_state() -> Result<()> {
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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_submit_state_and_proof() -> Result<()> {
        // TODO(Chengyu): disabled because it's under development

        // let pp = MockSystemParam::init();
        // let mut ledger = MockLedger::init(pp, NUM_INIT_VALIDATORS);
        // let genesis_state: LightClientStateSol = ledger.light_client_state().into();
        // let genesis_stake: StakeTableStateSol = ledger.voting_stake_table_state().into();

        // let anvil = Anvil::new().spawn();
        // let wallet = anvil.wallet().unwrap();
        // let inner_provider = ProviderBuilder::new()
        //     .wallet(wallet)
        //     .on_http(anvil.endpoint_url());
        // // a provider that holds both anvil (to avoid accidental drop) and wallet-enabled L1 provider
        // let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));
        // let mut contracts = Contracts::new();

        // let lc_proxy_addr = deploy_and_upgrade(
        //     &provider,
        //     &mut contracts,
        //     true,
        //     genesis_state,
        //     genesis_stake.clone(),
        // )
        // .await?;
        // let lc_v2 = LightClientV2Mock::new(lc_proxy_addr, &provider);

        // // update first epoch root (in numerical 2nd epoch)
        // // there will be new key registration but the effect only take place on the second epoch root update
        // while ledger.light_client_state().block_height < 2 * EPOCH_HEIGHT_FOR_TEST - 5 {
        //     ledger.elapse_with_block();
        // }

        // let (pi, proof) = ledger.gen_state_proof();
        // tracing::info!("Successfully generated proof for new state.");

        // super::submit_state_and_proof(&provider, lc_proxy_addr, proof, pi).await?;
        // tracing::info!("Successfully submitted new finalized state to L1.");

        // // second epoch root update
        // while ledger.light_client_state().block_height < 3 * EPOCH_HEIGHT_FOR_TEST - 5 {
        //     ledger.elapse_with_block();
        // }
        // let (pi, proof) = ledger.gen_state_proof();
        // tracing::info!("Successfully generated proof for new state.");

        // super::submit_state_and_proof(&provider, lc_proxy_addr, proof, pi).await?;
        // tracing::info!("Successfully submitted new finalized state to L1.");

        // // test if new state is updated in l1
        // let finalized_l1: LightClientStateSol = lc_v2.finalizedState().call().await?.into();
        // let expected: LightClientStateSol = ledger.light_client_state().into();
        // assert_eq!(
        //     finalized_l1.abi_encode_params(),
        //     expected.abi_encode_params(),
        //     "finalizedState not updated"
        // );

        // let expected_new_stake: StakeTableStateSol = ledger.next_stake_table_state().into();
        // // make sure it's different from the genesis, i.e. use a new stake table for the next epoch
        // assert_ne!(
        //     expected_new_stake.abi_encode_params(),
        //     genesis_stake.abi_encode_params()
        // );
        // let voting_stake_l1: StakeTableStateSol =
        //     lc_v2.votingStakeTableState().call().await?.into();
        // assert_eq!(
        //     voting_stake_l1.abi_encode_params(),
        //     expected_new_stake.abi_encode_params(),
        //     "votingStakeTableState not updated"
        // );

        Ok(())
    }
}
