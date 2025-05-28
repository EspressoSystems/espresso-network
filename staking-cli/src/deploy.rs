use std::{process::Command, time::Duration};

use alloy::{
    network::{Ethereum, EthereumWallet, TransactionBuilder as _},
    primitives::{utils::parse_ether, Address, U256},
    providers::{
        ext::AnvilApi as _,
        fillers::{FillProvider, JoinFill, WalletFiller},
        layers::AnvilProvider,
        utils::JoinedRecommendedFillers,
        Provider as _, ProviderBuilder, RootProvider, WalletProvider,
    },
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use espresso_contract_deployer::{
    build_signer, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table, Contract, Contracts,
};
use hotshot_contract_adapter::{
    sol_types::{
        EspToken::{self, EspTokenInstance},
        StakeTable,
    },
    stake_table::StakeTableContractVersion,
};
use hotshot_state_prover::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;
use hotshot_types::light_client::StateKeyPair;
use rand::{rngs::StdRng, CryptoRng, Rng as _, RngCore, SeedableRng as _};
use url::Url;

use crate::{parse::Commission, registration::register_validator, BLSKeyPair, DEV_MNEMONIC};

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
        let exit_escrow_period = Duration::from_secs(1);
        let port = portpicker::pick_unused_port().unwrap();
        // Spawn anvil
        let provider = ProviderBuilder::new().on_anvil_with_wallet_and_config(|anvil| {
            anvil.port(port).arg("--accounts").arg("20")
        })?;
        let rpc_url = format!("http://localhost:{}", port).parse()?;
        let deployer_address = provider.default_signer_address();
        // I don't know how to get the signer out of the provider, by default anvil uses the dev
        // mnemonic and the default signer is the first account.
        let signer = build_signer(DEV_MNEMONIC.to_string(), 0);
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
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(blocks_per_epoch)
            .epoch_start_block(epoch_start_block)
            .exit_escrow_period(U256::from(exit_escrow_period.as_secs()))
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

        let approval_amount = parse_ether("1000000")?;
        // Approve the stake table contract so it can transfer tokens to itself
        let receipt = EspTokenInstance::new(token, &provider)
            .approve(stake_table, approval_amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let mut rng = StdRng::from_seed([42u8; 32]);
        let (_, bls_key_pair, state_key_pair) = Self::gen_keys(&mut rng);

        Ok(Self {
            provider,
            signer,
            deployer_address,
            token,
            stake_table,
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
        let receipt = register_validator(
            &self.provider,
            self.stake_table,
            self.commission,
            self.deployer_address,
            self.bls_key_pair.clone(),
            self.state_key_pair.clone(),
        )
        .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn deregister_validator(&self) -> Result<()> {
        let stake_table = StakeTable::new(self.stake_table, &self.provider);
        let receipt = stake_table
            .deregisterValidator()
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn delegate(&self, amount: U256) -> Result<()> {
        let stake_table = StakeTable::new(self.stake_table, &self.provider);
        let receipt = stake_table
            .delegate(self.deployer_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn undelegate(&self, amount: U256) -> Result<()> {
        let stake_table = StakeTable::new(self.stake_table, &self.provider);
        let receipt = stake_table
            .undelegate(self.deployer_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn transfer_eth(&self, to: Address, amount: U256) -> Result<()> {
        let tx = TransactionRequest::default().with_to(to).with_value(amount);
        let receipt = self
            .provider
            .send_transaction(tx)
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn transfer(&self, to: Address, amount: U256) -> Result<()> {
        let token = EspToken::new(self.token, &self.provider);
        token
            .transfer(to, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        Ok(())
    }

    pub async fn warp_to_unlock_time(&self) -> Result<()> {
        self.provider
            .anvil_increase_time(self.exit_escrow_period.as_secs())
            .await?;
        Ok(())
    }

    pub async fn balance(&self, address: Address) -> Result<U256> {
        let token = EspToken::new(self.token, &self.provider);
        Ok(token.balanceOf(address).call().await?._0)
    }

    pub async fn allowance(&self, owner: Address) -> Result<U256> {
        let token = EspToken::new(self.token, &self.provider);
        Ok(token.allowance(owner, self.stake_table).call().await?._0)
    }

    pub async fn approve(&self, amount: U256) -> Result<()> {
        let token = EspToken::new(self.token, &self.provider);
        token
            .approve(self.stake_table, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(self.allowance(self.deployer_address).await? == amount);
        Ok(())
    }

    /// Inject test system config into CLI command via arguments
    pub fn args(&self, cmd: &mut Command, signer: Signer) {
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--token-address")
            .arg(self.token.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string())
            .arg("--account-index")
            .arg("0");

        match signer {
            Signer::Mnemonic => cmd.arg("--mnemonic").arg(DEV_MNEMONIC),
            Signer::Ledger => cmd.arg("--ledger"),
        };
    }
}

#[derive(Clone, Copy)]
pub enum Signer {
    Ledger,
    Mnemonic,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_deploy() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let stake_table = StakeTable::new(system.stake_table, &system.provider);
        // sanity check that we can fetch the exit escrow period
        assert_eq!(
            stake_table.exitEscrowPeriod().call().await?._0,
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
