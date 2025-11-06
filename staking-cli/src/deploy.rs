use std::time::Duration;

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{utils::parse_ether, Address, B256, U256},
    providers::{
        ext::AnvilApi as _,
        fillers::{FillProvider, JoinFill, WalletFiller},
        layers::AnvilProvider,
        utils::JoinedRecommendedFillers,
        ProviderBuilder, RootProvider, WalletProvider,
    },
    signers::local::PrivateKeySigner,
    sol_types::SolValue as _,
};
use anyhow::Result;
use espresso_contract_deployer::{
    build_signer, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table, Contract, Contracts,
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
use warp::Filter;

use crate::{
    delegation::{approve, delegate, undelegate},
    funding::{send_esp, send_eth},
    parse::Commission,
    receipt::ReceiptExt as _,
    registration::{deregister_validator, fetch_commission, register_validator},
    signature::NodeSignatures,
    BLSKeyPair, DEV_MNEMONIC,
};

type TestProvider = FillProvider<
    JoinFill<JoinedRecommendedFillers, WalletFiller<EthereumWallet>>,
    AnvilProvider<RootProvider>,
    Ethereum,
>;

#[derive(Debug, Clone)]
pub struct TestSystem {
    pub provider: TestProvider,
    pub signer: PrivateKeySigner,
    pub deployer_address: Address,
    pub token: Address,
    pub stake_table: Address,
    pub reward_claim: Option<Address>,
    pub exit_escrow_period: Duration,
    pub rpc_url: Url,
    pub bls_key_pair: BLSKeyPair,
    pub state_key_pair: StateKeyPair,
    pub commission: Commission,
    pub approval_amount: U256,
}

impl TestSystem {
    pub async fn deploy() -> Result<Self> {
        Self::deploy_version(StakeTableContractVersion::V2).await
    }

    pub async fn deploy_version(
        stake_table_contract_version: StakeTableContractVersion,
    ) -> Result<Self> {
        let exit_escrow_period = Duration::from_secs(250);
        // Sporadically the provider builder fails with a timeout inside alloy.
        // Retry a few times.
        let mut attempts = 0;
        let (port, provider) = loop {
            let port = portpicker::pick_unused_port().unwrap();
            match ProviderBuilder::new().connect_anvil_with_wallet_and_config(|anvil| {
                anvil.port(port).arg("--accounts").arg("20")
            }) {
                Ok(provider) => break (port, provider),
                Err(e) => {
                    attempts += 1;
                    if attempts >= 5 {
                        anyhow::bail!("Failed to spawn anvil after 5 attempts: {e}");
                    }
                    tracing::warn!("Anvil spawn failed, retrying: {e}");
                },
            }
        };

        let rpc_url: Url = format!("http://localhost:{port}").parse()?;
        let deployer_address = provider.default_signer_address();
        // I don't know how to get the signer out of the provider, by default anvil uses the dev
        // mnemonic and the default signer is the first account.
        let signer = build_signer(DEV_MNEMONIC, 0);
        assert_eq!(
            signer.address(),
            deployer_address,
            "Signer address mismatch"
        );

        // Create a fake stake table to create a genesis state. This is fine because we don't
        // currently use the light client contract. Will need to be updated once we implement
        // slashing and call the light client contract from the stake table contract.
        let blocks_per_epoch = 100;
        let epoch_start_block = 1;
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
            .blocks_per_epoch(blocks_per_epoch)
            .epoch_start_block(epoch_start_block)
            .multisig_pauser(deployer_address)
            .exit_escrow_period(U256::from(exit_escrow_period.as_secs()))
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(U256::from(3590000000u64))
            .ops_timelock_delay(U256::from(0))
            .ops_timelock_admin(signer.address())
            .ops_timelock_proposers(vec![signer.address()])
            .ops_timelock_executors(vec![signer.address()])
            .safe_exit_timelock_delay(U256::from(10))
            .safe_exit_timelock_admin(signer.address())
            .safe_exit_timelock_proposers(vec![signer.address()])
            .safe_exit_timelock_executors(vec![signer.address()])
            .use_timelock_owner(false)
            .build()
            .unwrap();

        match stake_table_contract_version {
            StakeTableContractVersion::V1 => args.deploy_to_stake_table_v1(&mut contracts).await?,
            StakeTableContractVersion::V2 => args.deploy_all(&mut contracts).await?,
        };

        let stake_table = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy deployed");
        let token = contracts
            .address(Contract::EspTokenProxy)
            .expect("EspTokenProxy deployed");
        let reward_claim = match stake_table_contract_version {
            StakeTableContractVersion::V1 => None,
            StakeTableContractVersion::V2 => contracts.address(Contract::RewardClaimProxy),
        };

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
            bls_key_pair,
            state_key_pair,
            commission: Commission::try_from("12.34")?,
            approval_amount,
        })
    }

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
        register_validator(&self.provider, self.stake_table, self.commission, payload)
            .await?
            .assert_success()
            .await?;
        Ok(())
    }

    pub async fn deregister_validator(&self) -> Result<()> {
        deregister_validator(&self.provider, self.stake_table)
            .await?
            .assert_success()
            .await?;
        Ok(())
    }

    pub async fn delegate(&self, amount: U256) -> Result<()> {
        delegate(
            &self.provider,
            self.stake_table,
            self.deployer_address,
            amount,
        )
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn undelegate(&self, amount: U256) -> Result<()> {
        undelegate(
            &self.provider,
            self.stake_table,
            self.deployer_address,
            amount,
        )
        .await?
        .assert_success()
        .await?;
        Ok(())
    }

    pub async fn transfer_eth(&self, to: Address, amount: U256) -> Result<()> {
        send_eth(&self.provider, to, amount)
            .await?
            .assert_success()
            .await?;
        Ok(())
    }

    pub async fn transfer(&self, to: Address, amount: U256) -> Result<()> {
        send_esp(&self.provider, self.token, to, amount)
            .await?
            .assert_success()
            .await?;
        Ok(())
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
        approve(&self.provider, self.token, self.stake_table, amount)
            .await?
            .assert_success()
            .await?;
        assert!(self.allowance(self.deployer_address).await? == amount);
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

        Ok(format!("http://localhost:{}/", port).parse()?)
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
