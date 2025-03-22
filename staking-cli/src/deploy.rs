use std::{process::Command, time::Duration};

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{utils::parse_ether, Address, U256},
    providers::{
        ext::AnvilApi as _,
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        layers::AnvilProvider,
        Identity, ProviderBuilder, RootProvider, WalletProvider,
    },
    transports::BoxTransport,
};
use anyhow::Result;
use contract_bindings_alloy::{
    erc1967proxy::ERC1967Proxy,
    esptoken::EspToken::{self, EspTokenInstance},
    staketable::StakeTable::{self, StakeTableInstance},
};
use url::Url;

use crate::{parse::Commission, registration::register_validator, BLSKeyPair, DEV_MNEMONIC};

type TestProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    AnvilProvider<RootProvider<BoxTransport>, BoxTransport>,
    BoxTransport,
    Ethereum,
>;

type SchnorrKeyPair = jf_signature::schnorr::KeyPair<ark_ed_on_bn254::EdwardsConfig>;

#[derive(Debug, Clone)]
pub struct TestSystem {
    pub provider: TestProvider,
    pub deployer_address: Address,
    pub token: EspTokenInstance<BoxTransport, TestProvider>,
    pub stake_table: StakeTableInstance<BoxTransport, TestProvider>,
    pub exit_escrow_period: Duration,
    pub rpc_url: Url,
    pub bls_key_pair: BLSKeyPair,
    pub schnorr_key_pair: SchnorrKeyPair,
    pub commission: Commission,
}

impl TestSystem {
    pub async fn deploy() -> Result<Self> {
        let exit_escrow_period = Duration::from_secs(1);
        let port = portpicker::pick_unused_port().unwrap();
        // Spawn anvil
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_anvil_with_wallet_and_config(|anvil| anvil.port(port).arg("--accounts").arg("20"));
        let rpc_url = format!("http://localhost:{}", port).parse()?;
        let deployer_address = provider.default_signer_address();

        // `EspToken.sol`
        let token_impl = EspToken::deploy(provider.clone()).await?;
        let data = token_impl
            .initialize(deployer_address, deployer_address)
            .calldata()
            .clone();

        let proxy = ERC1967Proxy::deploy(provider.clone(), *token_impl.address(), data).await?;
        let token = EspToken::new(*proxy.address(), provider.clone());

        // `StakeTable.sol`
        let stake_table_impl = StakeTable::deploy(provider.clone()).await?;
        let data = stake_table_impl
            .initialize(
                *token.address(),
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".parse()?, // fake LC address
                U256::from(exit_escrow_period.as_secs()),
                deployer_address,
            )
            .calldata()
            .clone();

        let proxy =
            ERC1967Proxy::deploy(provider.clone(), *stake_table_impl.address(), data).await?;
        let stake_table = StakeTable::new(*proxy.address(), provider.clone());

        // Approve the stake table contract so it can transfer tokens to itself
        let receipt = token
            .approve(*stake_table.address(), parse_ether("1000000")?)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let bls_key_pair = BLSKeyPair::generate(&mut rand::thread_rng());
        let schnorr_key_pair = SchnorrKeyPair::generate(&mut rand::thread_rng());
        Ok(Self {
            provider,
            deployer_address,
            token,
            stake_table,
            exit_escrow_period,
            rpc_url,
            bls_key_pair,
            schnorr_key_pair,
            commission: Commission::try_from("12.34")?,
        })
    }

    pub async fn register_validator(&self) -> Result<()> {
        let receipt = register_validator(
            self.stake_table.clone(),
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
        let receipt = self
            .stake_table
            .deregisterValidator()
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn delegate(&self, amount: U256) -> Result<()> {
        let receipt = self
            .stake_table
            .delegate(self.deployer_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn undelegate(&self, amount: U256) -> Result<()> {
        let receipt = self
            .stake_table
            .undelegate(self.deployer_address, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());
        Ok(())
    }

    pub async fn transfer(&self, to: Address, amount: U256) -> Result<()> {
        self.token
            .transfer(to, amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        Ok(())
    }

    pub async fn warp_to_unlock_time(&self) -> Result<()> {
        self.provider
            .anvil_increase_time(U256::from(self.exit_escrow_period.as_secs()))
            .await?;
        Ok(())
    }

    pub async fn balance(&self, address: Address) -> Result<U256> {
        Ok(self.token.balanceOf(address).call().await?._0)
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
            .arg(self.token.address().to_string())
            .arg("--stake-table-address")
            .arg(self.stake_table.address().to_string());
        cmd
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_deploy() -> Result<()> {
        let system = TestSystem::deploy().await?;
        // sanity check that we can fetch the exit escrow period
        assert_eq!(
            system.stake_table.exitEscrowPeriod().call().await?._0,
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
