use std::{process::Command, time::Duration};

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{utils::parse_ether, Address, U256},
    providers::{
        ext::AnvilApi as _,
        fillers::{FillProvider, JoinFill, WalletFiller},
        layers::AnvilProvider,
        utils::JoinedRecommendedFillers,
        ProviderBuilder, RootProvider, WalletProvider,
    },
};
use anyhow::Result;
use hotshot_contract_adapter::sol_types::{ERC1967Proxy, EspToken, StakeTable};
use rand::{rngs::StdRng, SeedableRng as _};
use url::Url;

use crate::{parse::Commission, registration::register_validator, BLSKeyPair, DEV_MNEMONIC};

type TestProvider = FillProvider<
    JoinFill<JoinedRecommendedFillers, WalletFiller<EthereumWallet>>,
    AnvilProvider<RootProvider>,
    Ethereum,
>;

type SchnorrKeyPair = jf_signature::schnorr::KeyPair<ark_ed_on_bn254::EdwardsConfig>;

#[derive(Debug, Clone)]
pub struct TestSystem {
    pub provider: TestProvider,
    pub deployer_address: Address,
    pub token: Address,
    pub stake_table: Address,
    pub exit_escrow_period: Duration,
    pub rpc_url: Url,
    pub bls_key_pair: BLSKeyPair,
    pub schnorr_key_pair: SchnorrKeyPair,
    pub commission: Commission,
    pub approval_amount: U256,
}

impl TestSystem {
    pub async fn deploy() -> Result<Self> {
        let exit_escrow_period = Duration::from_secs(1);
        let port = portpicker::pick_unused_port().unwrap();
        // Spawn anvil
        let provider = ProviderBuilder::new().on_anvil_with_wallet_and_config(|anvil| {
            anvil.port(port).arg("--accounts").arg("20")
        })?;
        let rpc_url = format!("http://localhost:{}", port).parse()?;
        let deployer_address = provider.default_signer_address();

        // `EspToken.sol`
        let token_impl = EspToken::deploy(provider.clone()).await?;
        let data = token_impl
            .initialize(deployer_address, deployer_address)
            .calldata()
            .clone();

        let token_proxy =
            ERC1967Proxy::deploy(provider.clone(), *token_impl.address(), data).await?;
        let token = EspToken::new(*token_proxy.address(), provider.clone());

        // `StakeTable.sol`
        let stake_table_impl = StakeTable::deploy(provider.clone()).await?;
        let data = stake_table_impl
            .initialize(
                *token_proxy.address(),
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".parse()?, // fake LC address
                U256::from(exit_escrow_period.as_secs()),
                deployer_address,
            )
            .calldata()
            .clone();

        let st_proxy =
            ERC1967Proxy::deploy(provider.clone(), *stake_table_impl.address(), data).await?;

        let approval_amount = parse_ether("1000000")?;
        // Approve the stake table contract so it can transfer tokens to itself
        let receipt = token
            .approve(*st_proxy.address(), approval_amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let mut rng = StdRng::from_seed([42u8; 32]);
        let bls_key_pair = BLSKeyPair::generate(&mut rng);
        let schnorr_key_pair = SchnorrKeyPair::generate(&mut rng);
        Ok(Self {
            provider,
            deployer_address,
            token: *token_proxy.address(),
            stake_table: *st_proxy.address(),
            exit_escrow_period,
            rpc_url,
            bls_key_pair,
            schnorr_key_pair,
            commission: Commission::try_from("12.34")?,
            approval_amount,
        })
    }

    pub async fn register_validator(&self) -> Result<()> {
        let receipt = register_validator(
            &self.provider,
            self.stake_table,
            self.commission,
            self.deployer_address,
            self.bls_key_pair.clone(),
            self.schnorr_key_pair.ver_key(),
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

    pub fn cmd(&self) -> Command {
        let mut cmd = escargot::CargoBuild::new()
            .bin("staking-cli")
            .current_release()
            .current_target()
            .run()
            .unwrap()
            .command();
        cmd.arg("--rpc-url")
            .arg(self.rpc_url.to_string())
            .arg("--mnemonic")
            .arg(DEV_MNEMONIC)
            .arg("--token-address")
            .arg(self.token.to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.to_string());
        cmd
    }
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
