use std::{collections::HashMap, path::Path};

use alloy::{
    primitives::Address, providers::ProviderBuilder, rpc::client::RpcClient,
    transports::layers::RetryBackoffLayer,
};
use anyhow::{Context, Result};
use espresso_contract_deployer::contract_types::{DeploymentState, FromOnchainConfig};
use url::Url;

const STAKE_TABLE_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS";
const ESP_TOKEN_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS";
const LIGHT_CLIENT_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS";
const FEE_CONTRACT_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS";
const REWARD_CLAIM_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS";
const MULTISIG_ADDRESS: &str = "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS";
const OPS_TIMELOCK_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_OPS_TIMELOCK_PROXY_ADDRESS";
const SAFE_EXIT_TIMELOCK_PROXY_ADDRESS: &str =
    "ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_PROXY_ADDRESS";

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeploymentAddresses {
    pub stake_table_proxy: Option<Address>,
    pub esp_token_proxy: Option<Address>,
    pub light_client_proxy: Option<Address>,
    pub fee_contract_proxy: Option<Address>,
    pub reward_claim_proxy: Option<Address>,
    pub multisig: Option<Address>,
    pub ops_timelock: Option<Address>,
    pub safe_exit_timelock: Option<Address>,
}

pub fn load_addresses_from_env_file(path: Option<&Path>) -> Result<DeploymentAddresses> {
    let env_map: HashMap<String, String> = if let Some(p) = path {
        dotenvy::from_path_iter(p)
            .with_context(|| format!("Failed to read env file: {:?}", p))?
            .filter_map(|item| item.ok())
            .collect()
    } else {
        dotenvy::dotenv_iter()
            .ok()
            .map(|iter| iter.filter_map(|item| item.ok()).collect())
            .unwrap_or_default()
    };

    fn parse_address(env_map: &HashMap<String, String>, key: &str) -> Option<Address> {
        env_map.get(key).and_then(|val| {
            if val.is_empty() {
                tracing::warn!("{} is set but empty", key);
                None
            } else {
                match val.parse() {
                    Ok(addr) => Some(addr),
                    Err(e) => {
                        tracing::warn!("Failed to parse {} with value '{}': {}", key, val, e);
                        None
                    },
                }
            }
        })
    }

    Ok(DeploymentAddresses {
        stake_table_proxy: parse_address(&env_map, STAKE_TABLE_PROXY_ADDRESS),
        esp_token_proxy: parse_address(&env_map, ESP_TOKEN_PROXY_ADDRESS),
        light_client_proxy: parse_address(&env_map, LIGHT_CLIENT_PROXY_ADDRESS),
        fee_contract_proxy: parse_address(&env_map, FEE_CONTRACT_PROXY_ADDRESS),
        reward_claim_proxy: parse_address(&env_map, REWARD_CLAIM_PROXY_ADDRESS),
        multisig: parse_address(&env_map, MULTISIG_ADDRESS),
        ops_timelock: parse_address(&env_map, OPS_TIMELOCK_PROXY_ADDRESS),
        safe_exit_timelock: parse_address(&env_map, SAFE_EXIT_TIMELOCK_PROXY_ADDRESS),
    })
}

pub async fn collect_deployment_info(
    rpc_url: Url,
    addresses: DeploymentAddresses,
    chain_id: u64,
) -> Result<DeploymentState> {
    let client = RpcClient::builder()
        .layer(RetryBackoffLayer::new(10, 100, 300))
        .http(rpc_url);
    let provider = ProviderBuilder::new().connect_client(client);

    let config = FromOnchainConfig {
        light_client_proxy: addresses.light_client_proxy,
        stake_table_proxy: addresses.stake_table_proxy,
        esp_token_proxy: addresses.esp_token_proxy,
        fee_contract_proxy: addresses.fee_contract_proxy,
        reward_claim_proxy: addresses.reward_claim_proxy,
        multisig: addresses.multisig,
        ops_timelock: addresses.ops_timelock,
        safe_exit_timelock: addresses.safe_exit_timelock,
    };

    DeploymentState::from_onchain(&provider, config, chain_id).await
}

pub fn write_deployment_info(state: &DeploymentState, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let json =
        serde_json::to_string_pretty(state).context("Failed to serialize deployment info")?;

    std::fs::write(output_path, json).context("Failed to write deployment info")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::{
        node_bindings::Anvil,
        primitives::U256,
        providers::{ProviderBuilder, WalletProvider},
    };
    use espresso_contract_deployer::{
        builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table,
        Contract, Contracts,
    };
    use hotshot_state_prover::v3::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;

    use super::*;

    #[test_log::test(tokio::test)]
    async fn test_collect_deployment_info_with_deployed_contracts() -> Result<()> {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new()
            .wallet(anvil.wallet().unwrap())
            .connect_http(anvil.endpoint_url());
        let rpc_url = anvil.endpoint_url();
        let deployer_address = provider.default_signer_address();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &Default::default(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(provider.clone())
            .rpc_url(rpc_url.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(100)
            .epoch_start_block(1)
            .multisig_pauser(deployer_address)
            .exit_escrow_period(U256::from(250))
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(U256::from(3590000000u64))
            .ops_timelock_delay(U256::from(100))
            .ops_timelock_admin(deployer_address)
            .ops_timelock_proposers(vec![deployer_address])
            .ops_timelock_executors(vec![deployer_address])
            .safe_exit_timelock_delay(U256::from(200))
            .safe_exit_timelock_admin(deployer_address)
            .safe_exit_timelock_proposers(vec![deployer_address])
            .safe_exit_timelock_executors(vec![deployer_address])
            .use_timelock_owner(false)
            .build()
            .unwrap();

        args.deploy_all(&mut contracts).await?;

        let stake_table_addr = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy deployed");
        let esp_token_addr = contracts
            .address(Contract::EspTokenProxy)
            .expect("EspTokenProxy deployed");
        let light_client_addr = contracts
            .address(Contract::LightClientProxy)
            .expect("LightClientProxy deployed");
        let fee_contract_addr = contracts
            .address(Contract::FeeContractProxy)
            .expect("FeeContractProxy deployed");
        let reward_claim_addr = contracts
            .address(Contract::RewardClaimProxy)
            .expect("RewardClaimProxy deployed");
        let ops_timelock_addr = contracts
            .address(Contract::OpsTimelock)
            .expect("OpsTimelock deployed");
        let safe_exit_timelock_addr = contracts
            .address(Contract::SafeExitTimelock)
            .expect("SafeExitTimelock deployed");

        let addresses = DeploymentAddresses {
            stake_table_proxy: Some(stake_table_addr),
            esp_token_proxy: Some(esp_token_addr),
            light_client_proxy: Some(light_client_addr),
            fee_contract_proxy: Some(fee_contract_addr),
            reward_claim_proxy: Some(reward_claim_addr),
            multisig: None,
            ops_timelock: Some(ops_timelock_addr),
            safe_exit_timelock: Some(safe_exit_timelock_addr),
        };

        let chain_id = anvil.chain_id();

        let state = collect_deployment_info(rpc_url, addresses, chain_id).await?;

        assert_eq!(state.chain_id, chain_id);

        assert!(state.light_client.is_some());
        let lc = state.light_client.as_ref().unwrap();
        assert_eq!(lc.proxy_address(), light_client_addr);
        assert_eq!(lc.owner(), deployer_address);
        assert_eq!(lc.version_string(), "3");

        assert!(state.stake_table.is_some());
        let st = state.stake_table.as_ref().unwrap();
        assert_eq!(st.proxy_address(), stake_table_addr);
        assert_eq!(st.owner(), deployer_address);
        assert_eq!(st.version_string(), "2");

        assert!(state.esp_token.is_some());
        let token = state.esp_token.as_ref().unwrap();
        assert_eq!(token.proxy_address(), esp_token_addr);
        assert_eq!(token.owner(), deployer_address);
        assert_eq!(token.version_string(), "2");

        assert!(state.fee_contract.is_some());
        let fee = state.fee_contract.as_ref().unwrap();
        assert_eq!(fee.proxy_address, fee_contract_addr);
        assert_eq!(fee.owner, deployer_address);

        assert!(state.reward_claim.is_some());
        let reward = state.reward_claim.as_ref().unwrap();
        assert_eq!(reward.proxy_address, reward_claim_addr);
        assert_eq!(reward.admin, deployer_address);

        assert!(state.ops_timelock.is_some());
        let ops = state.ops_timelock.as_ref().unwrap();
        assert_eq!(ops.address, ops_timelock_addr);
        assert_eq!(ops.min_delay, 100);
        assert_eq!(ops.admin, deployer_address);
        assert_eq!(ops.proposers, vec![deployer_address]);
        assert_eq!(ops.executors, vec![deployer_address]);

        assert!(state.safe_exit_timelock.is_some());
        let safe_exit = state.safe_exit_timelock.as_ref().unwrap();
        assert_eq!(safe_exit.address, safe_exit_timelock_addr);
        assert_eq!(safe_exit.min_delay, 200);
        assert_eq!(safe_exit.admin, deployer_address);
        assert_eq!(safe_exit.proposers, vec![deployer_address]);
        assert_eq!(safe_exit.executors, vec![deployer_address]);

        assert!(state.multisig.is_none());

        Ok(())
    }
}
