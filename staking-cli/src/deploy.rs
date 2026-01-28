use std::{path::PathBuf, time::Duration};

use alloy::{
    network::{Ethereum, EthereumWallet, TransactionBuilder as _},
    primitives::{utils::parse_ether, Address, Bytes, B256, U256},
    providers::{
        ext::AnvilApi,
        fillers::{FillProvider, JoinFill, WalletFiller},
        utils::JoinedRecommendedFillers,
        Provider, ProviderBuilder, RootProvider, WalletProvider,
    },
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol_types::SolValue as _,
};
use anyhow::Result;
use espresso_contract_deployer::{
    build_provider, build_signer, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table, Contract, Contracts,
    DEFAULT_EXIT_ESCROW_PERIOD_SECONDS,
};
use espresso_types::{
    v0::v0_4::{
        RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2,
    },
    v0_3::RewardAmount,
    v0_4::REWARD_MERKLE_TREE_V2_HEIGHT,
};
use hotshot_contract_adapter::{
    sol_types::{
        EspToken::{self, EspTokenInstance},
        LightClientV3Mock, StakeTableV2,
    },
    stake_table::StakeTableContractVersion,
};
use hotshot_state_prover::v3::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;
use hotshot_types::light_client::StateKeyPair;
use jf_merkle_tree_compat::{MerkleCommitment, MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::{rngs::StdRng, CryptoRng, Rng as _, RngCore, SeedableRng as _};
use url::Url;
use warp::{http::StatusCode, Filter};

use crate::{
    parse::Commission, receipt::ReceiptExt as _, registration::fetch_commission,
    signature::NodeSignatures, transaction::Transaction, BLSKeyPair, DEV_MNEMONIC,
};

#[derive(Debug, Clone)]
pub struct DeployedContracts {
    pub token: Address,
    pub stake_table: Address,
    pub reward_claim: Option<Address>,
}

impl DeployedContracts {
    pub fn write_env(&self, path: &PathBuf) -> Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "# Deployed contract addresses")?;
        writeln!(
            file,
            "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS={}",
            self.stake_table
        )?;
        writeln!(
            file,
            "ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS={}",
            self.token
        )?;
        if let Some(reward_claim) = self.reward_claim {
            writeln!(
                file,
                "ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS={}",
                reward_claim
            )?;
        }
        Ok(())
    }
}

pub async fn deploy_to_rpc<P>(
    provider: P,
    rpc_url: Url,
    stake_table_contract_version: StakeTableContractVersion,
    exit_escrow_period: Duration,
) -> Result<DeployedContracts>
where
    P: WalletProvider + Provider + Clone,
{
    let deployer_address = provider.default_signer_address();

    let blocks_per_epoch = 100;
    let epoch_start_block = 1;
    let (genesis_state, genesis_stake) =
        light_client_genesis_from_stake_table(&Default::default(), STAKE_TABLE_CAPACITY_FOR_TEST)
            .map_err(|e| anyhow::anyhow!("failed to create genesis state: {e}"))?;

    let mut contracts = Contracts::new();
    let args = DeployerArgsBuilder::default()
        .deployer(provider)
        .rpc_url(rpc_url)
        .mock_light_client(true)
        .genesis_lc_state(genesis_state)
        .genesis_st_state(genesis_stake)
        .blocks_per_epoch(blocks_per_epoch)
        .epoch_start_block(epoch_start_block)
        .multisig_pauser(deployer_address)
        .exit_escrow_period(U256::from(exit_escrow_period.as_secs()))
        .token_name("Espresso".to_string())
        .token_symbol("ESP".to_string())
        .initial_token_supply(U256::from(3590000000u64))
        .ops_timelock_delay(U256::from(0))
        .ops_timelock_admin(deployer_address)
        .ops_timelock_proposers(vec![deployer_address])
        .ops_timelock_executors(vec![deployer_address])
        .safe_exit_timelock_delay(U256::from(10))
        .safe_exit_timelock_admin(deployer_address)
        .safe_exit_timelock_proposers(vec![deployer_address])
        .safe_exit_timelock_executors(vec![deployer_address])
        .use_timelock_owner(false)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build deployer args: {e}"))?;

    match stake_table_contract_version {
        StakeTableContractVersion::V1 => args.deploy_to_stake_table_v1(&mut contracts).await?,
        StakeTableContractVersion::V2 => args.deploy_all(&mut contracts).await?,
    };

    let stake_table = contracts
        .address(Contract::StakeTableProxy)
        .ok_or_else(|| anyhow::anyhow!("StakeTableProxy not deployed"))?;
    let token = contracts
        .address(Contract::EspTokenProxy)
        .ok_or_else(|| anyhow::anyhow!("EspTokenProxy not deployed"))?;
    let reward_claim = match stake_table_contract_version {
        StakeTableContractVersion::V1 => None,
        StakeTableContractVersion::V2 => contracts.address(Contract::RewardClaimProxy),
    };

    Ok(DeployedContracts {
        token,
        stake_table,
        reward_claim,
    })
}

pub async fn deploy_contracts_for_testing(
    rpc_url: Url,
    mnemonic: String,
    account_index: u32,
    output: PathBuf,
) -> Result<DeployedContracts> {
    tracing::info!("deploying staking contracts for testing");

    let provider = build_provider(mnemonic, account_index, rpc_url.clone(), None);
    tracing::info!("deployer address: {}", provider.default_signer_address());

    let exit_escrow_period = Duration::from_secs(300);
    let contracts = deploy_to_rpc(
        provider,
        rpc_url,
        StakeTableContractVersion::V2,
        exit_escrow_period,
    )
    .await?;

    tracing::info!("stake table deployed: {}", contracts.stake_table);
    tracing::info!("ESP token deployed: {}", contracts.token);

    contracts.write_env(&output)?;
    tracing::info!("contract addresses written to {}", output.display());

    println!("STAKE_TABLE_ADDRESS={}", contracts.stake_table);
    println!("ESP_TOKEN_ADDRESS={}", contracts.token);

    Ok(contracts)
}

/// Provider type for Anvil-based testing (with Anvil-specific API support).
///
/// This type can be used with both local Anvil instances and external RPC endpoints
/// (like Reth) that expose the Anvil API. The `AnvilApi` trait is implemented for
/// any `Provider`, so anvil methods will work as long as the RPC endpoint supports them.
pub type AnvilTestProvider = FillProvider<
    JoinFill<JoinedRecommendedFillers, WalletFiller<EthereumWallet>>,
    RootProvider,
    Ethereum,
>;

/// Test system for deploying and interacting with staking contracts.
#[derive(Debug)]
pub struct TestSystem {
    pub provider: AnvilTestProvider,
    pub signer: PrivateKeySigner,
    pub deployer_address: Address,
    pub token: Address,
    pub stake_table: Address,
    pub reward_claim: Option<Address>,
    pub exit_escrow_period: Duration,
    pub rpc_url: Url,
    /// Port for the local node. `None` for external providers.
    pub port: Option<u16>,
    pub bls_key_pair: BLSKeyPair,
    pub state_key_pair: StateKeyPair,
    pub commission: Commission,
    pub approval_amount: U256,
    pub version: StakeTableContractVersion,
    /// Anvil instance for local testing. Kept alive to prevent process termination.
    /// `None` for external providers.
    #[allow(dead_code)]
    anvil_instance: Option<std::sync::Arc<alloy::node_bindings::AnvilInstance>>,
}

impl TestSystem {
    /// Note: Generates random keys, the Ethereum key won't match the deployer key.
    pub fn gen_keys(
        rng: &mut (impl RngCore + CryptoRng),
    ) -> (PrivateKeySigner, BLSKeyPair, StateKeyPair) {
        (
            PrivateKeySigner::random_with(rng),
            BLSKeyPair::generate(rng),
            StateKeyPair::generate_from_seed(rng.gen()),
        )
    }

    pub async fn register_validator(&self) -> Result<()> {
        let payload = NodeSignatures::create(
            self.deployer_address,
            &self.bls_key_pair.clone(),
            &self.state_key_pair.clone(),
        );
        let metadata_uri = "https://example.com/metadata".parse()?;
        Transaction::RegisterValidator {
            stake_table: self.stake_table,
            commission: self.commission,
            metadata_uri,
            payload,
            version: self.version,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn deregister_validator(&self) -> Result<()> {
        Transaction::DeregisterValidator {
            stake_table: self.stake_table,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn delegate(&self, amount: U256) -> Result<()> {
        Transaction::Delegate {
            stake_table: self.stake_table,
            validator: self.deployer_address,
            amount,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn undelegate(&self, amount: U256) -> Result<()> {
        Transaction::Undelegate {
            stake_table: self.stake_table,
            validator: self.deployer_address,
            amount,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn transfer_eth(&self, to: Address, amount: U256) -> Result<()> {
        let tx = TransactionRequest::default().with_to(to).with_value(amount);
        self.provider
            .send_transaction(tx)
            .await?
            .assert_success()
            .await?;
        Ok(())
    }

    pub async fn transfer(&self, to: Address, amount: U256) -> Result<()> {
        Transaction::Transfer {
            token: self.token,
            to,
            amount,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn get_min_commission_increase_interval(&self) -> Result<U256> {
        let stake_table = StakeTableV2::new(self.stake_table, &self.provider);
        let interval = stake_table.minCommissionIncreaseInterval().call().await?;
        Ok(interval)
    }

    pub async fn fetch_commission(&self) -> Result<Commission> {
        fetch_commission(&self.provider, self.stake_table, self.deployer_address).await
    }

    pub async fn balance(&self, address: Address) -> Result<U256> {
        let token = EspToken::new(self.token, &self.provider);
        Ok(token.balanceOf(address).call().await?)
    }

    pub async fn allowance(&self, owner: Address) -> Result<U256> {
        let token = EspToken::new(self.token, &self.provider);
        Ok(token.allowance(owner, self.stake_table).call().await?)
    }

    pub async fn approve(&self, amount: U256) -> Result<()> {
        Transaction::Approve {
            token: self.token,
            spender: self.stake_table,
            amount,
        }
        .send(&self.provider)
        .await?
        .assert_success()
        .await?;
        assert!(self.allowance(self.deployer_address).await? == amount);
        Ok(())
    }

    pub async fn set_min_delegate_amount(&self, amount: U256) -> Result<()> {
        let stake_table = StakeTableV2::new(self.stake_table, &self.provider);
        stake_table
            .setMinDelegateAmount(amount)
            .send()
            .await?
            .assert_success()
            .await?;
        Ok(())
    }

    pub async fn setup_reward_claim_mock(&self, balance: U256) -> Result<Url> {
        let stake_table = StakeTableV2::new(self.stake_table, &self.provider);
        let light_client_addr = stake_table.lightClient().call().await?;
        let light_client = LightClientV3Mock::new(light_client_addr, &self.provider);

        let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let account = RewardAccountV2::from(self.deployer_address);
        let amount = RewardAmount::from(balance);

        tree.update(account, amount)?;

        let commitment = tree.commitment();
        let root = commitment.digest().into();

        let auth_root_fields: [B256; 8] = [
            root,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
            B256::ZERO,
        ];
        let auth_root = alloy::primitives::keccak256(auth_root_fields.abi_encode());

        light_client
            .setAuthRoot(auth_root.into())
            .send()
            .await?
            .get_receipt()
            .await?;

        let query_data: RewardAccountQueryDataV2 = RewardAccountProofV2::prove(&tree, account.0)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate proof"))?
            .into();
        let claim_input = query_data.to_reward_claim_input()?;
        let claim_input = std::sync::Arc::new(claim_input);

        let port = portpicker::pick_unused_port().expect("No ports available");

        let route = warp::path!("reward-state-v2" / "reward-claim-input" / u64 / String).map(
            move |_block_height: u64, _address: String| warp::reply::json(&*claim_input.clone()),
        );

        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));

        Ok(format!("http://127.0.0.1:{}/", port).parse()?)
    }

    pub fn setup_reward_claim_not_found_mock(&self) -> Url {
        let port = portpicker::pick_unused_port().expect("No ports available");

        let route = warp::path!("reward-state-v2" / "reward-claim-input" / u64 / String)
            .map(|_, _| warp::reply::with_status(warp::reply(), StatusCode::NOT_FOUND));

        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));

        format!("http://127.0.0.1:{}/", port).parse().unwrap()
    }

    pub async fn warp_to_unlock_time(&self) -> Result<()> {
        self.provider
            .anvil_increase_time(self.exit_escrow_period.as_secs())
            .await?;
        Ok(())
    }

    pub async fn anvil_increase_time(&self, seconds: U256) -> Result<()> {
        self.provider
            .anvil_increase_time(seconds.to::<u64>())
            .await?;
        Ok(())
    }

    pub async fn dump_state(&self) -> Result<Bytes> {
        Ok(self.provider.anvil_dump_state().await?)
    }

    pub async fn load_state(&self, state: Bytes) -> Result<()> {
        self.provider.anvil_load_state(state).await?;
        Ok(())
    }
    pub async fn deploy() -> Result<Self> {
        Self::deploy_version(StakeTableContractVersion::V2).await
    }

    pub async fn deploy_version(
        stake_table_contract_version: StakeTableContractVersion,
    ) -> Result<Self> {
        use alloy::node_bindings::Anvil;

        let exit_escrow_period = Duration::from_secs(DEFAULT_EXIT_ESCROW_PERIOD_SECONDS);
        // Sporadically the provider builder fails with a timeout inside alloy.
        // Retry a few times.
        let mut attempts = 0;
        let (port, anvil_instance) = loop {
            let port = portpicker::pick_unused_port().unwrap();
            match Anvil::new()
                .port(port)
                .arg("--accounts")
                .arg("20")
                .try_spawn()
            {
                Ok(anvil) => break (port, anvil),
                Err(e) => {
                    attempts += 1;
                    if attempts >= 5 {
                        anyhow::bail!("Failed to spawn anvil after 5 attempts: {e}");
                    }
                    tracing::warn!("Anvil spawn failed, retrying: {e}");
                },
            }
        };

        let rpc_url: Url = format!("http://127.0.0.1:{port}").parse()?;
        // By default anvil uses the dev mnemonic and the default signer is the first account.
        let signer = build_signer(DEV_MNEMONIC, 0);
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer.clone()))
            .connect_http(rpc_url.clone());
        let deployer_address = provider.default_signer_address();
        assert_eq!(
            signer.address(),
            deployer_address,
            "Signer address mismatch"
        );

        let contracts = deploy_to_rpc(
            provider.clone(),
            rpc_url.clone(),
            stake_table_contract_version,
            exit_escrow_period,
        )
        .await?;
        let token = contracts.token;
        let stake_table = contracts.stake_table;
        let reward_claim = contracts.reward_claim;

        let approval_amount = parse_ether("1000000")?;
        // Approve the stake table contract so it can transfer tokens to itself
        EspTokenInstance::new(token, &provider)
            .approve(stake_table, approval_amount)
            .send()
            .await?
            .assert_success()
            .await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let (_, bls_key_pair, state_key_pair) = Self::gen_keys(&mut rng);

        Ok(Self {
            provider,
            signer,
            deployer_address,
            token,
            stake_table,
            reward_claim,
            exit_escrow_period,
            rpc_url,
            port: Some(port),
            bls_key_pair,
            state_key_pair,
            commission: Commission::try_from("12.34")?,
            approval_amount,
            version: stake_table_contract_version,
            anvil_instance: Some(std::sync::Arc::new(anvil_instance)),
        })
    }

    /// Deploy contracts to an external RPC endpoint.
    ///
    /// Uses the dev mnemonic account 0 for deployment and initial funding.
    /// The external RPC (e.g., Reth with `--http.api=anvil`) must expose the Anvil API
    /// for time manipulation methods to work.
    pub async fn deploy_to_external(rpc_url: Url) -> Result<Self> {
        let exit_escrow_period = Duration::from_secs(DEFAULT_EXIT_ESCROW_PERIOD_SECONDS);
        let signer = build_signer(DEV_MNEMONIC, 0);
        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer.clone()))
            .connect_http(rpc_url.clone());
        let deployer_address = provider.default_signer_address();

        let contracts = deploy_to_rpc(
            provider.clone(),
            rpc_url.clone(),
            StakeTableContractVersion::V2,
            exit_escrow_period,
        )
        .await?;
        let token = contracts.token;
        let stake_table = contracts.stake_table;
        let reward_claim = contracts.reward_claim;

        let approval_amount = parse_ether("1000000")?;
        // Approve the stake table contract so it can transfer tokens to itself
        EspTokenInstance::new(token, &provider)
            .approve(stake_table, approval_amount)
            .send()
            .await?
            .assert_success()
            .await?;

        let mut rng = StdRng::from_seed([42u8; 32]);
        let (_, bls_key_pair, state_key_pair) = Self::gen_keys(&mut rng);

        Ok(Self {
            provider,
            signer,
            deployer_address,
            token,
            stake_table,
            reward_claim,
            exit_escrow_period,
            rpc_url,
            port: None,
            bls_key_pair,
            state_key_pair,
            commission: Commission::try_from("12.34")?,
            approval_amount,
            version: StakeTableContractVersion::V2,
            anvil_instance: None,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_deploy() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let stake_table = StakeTableV2::new(system.stake_table, &system.provider);
        // sanity check that we can fetch the exit escrow period
        assert_eq!(
            stake_table.exitEscrowPeriod().call().await?,
            U256::from(system.exit_escrow_period.as_secs())
        );

        let to = "0x1111111111111111111111111111111111111111".parse()?;

        // sanity check that we can transfer tokens
        system.transfer(to, U256::from(123)).await?;

        // sanity check that we can fetch the balance
        assert_eq!(system.balance(to).await?, U256::from(123));

        Ok(())
    }
}
