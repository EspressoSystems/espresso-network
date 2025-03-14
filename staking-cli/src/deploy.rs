use std::time::Duration;

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, U256},
    providers::{
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
/// Utility for testing the CLI the code is the same as in
use contract_bindings_alloy::{
    erc1967proxy::ERC1967Proxy,
    esptoken::EspToken::{self, EspTokenInstance},
    staketable::StakeTable::{self, StakeTableInstance},
};

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

pub struct TestSystem {
    pub provider: TestProvider,
    pub token: EspTokenInstance<BoxTransport, TestProvider>,
    pub stake_table: StakeTableInstance<BoxTransport, TestProvider>,
    pub exit_escrow_period: Duration,
    // pub rpc_url: Url,
}

impl TestSystem {
    pub async fn deploy(exit_escrow_period: Duration) -> Result<Self> {
        // Spawn anvil
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_anvil_with_wallet();

        // `EspToken.sol`
        let token = EspToken::deploy(provider.clone()).await?;
        let data = token
            .initialize(
                provider.default_signer_address(),
                provider.default_signer_address(),
            )
            .calldata()
            .clone();

        let proxy = ERC1967Proxy::deploy(provider.clone(), *token.address(), data).await?;
        let token = EspToken::new(proxy.address().clone(), provider.clone());

        // `StakeTable.sol`
        let stake_table = StakeTable::deploy(provider.clone()).await?;
        let data = stake_table
            .initialize(
                *token.address(),
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".parse()?, // fake LC address
                U256::from(exit_escrow_period.as_secs()),
                provider.default_signer_address(),
            )
            .calldata()
            .clone();

        let proxy = ERC1967Proxy::deploy(provider.clone(), *stake_table.address(), data).await?;
        let stake_table = StakeTable::new(*proxy.address(), provider.clone());

        Ok(Self {
            provider,
            token,
            stake_table,
            exit_escrow_period,
        })
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

    pub async fn balance(&self, address: Address) -> Result<U256> {
        Ok(self.token.balanceOf(address).call().await?._0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_deploy() -> Result<()> {
        let exit_escrow_period = Duration::from_secs(60);
        let system = TestSystem::deploy(exit_escrow_period).await?;
        // sanity check that we can fetch the exit escrow period
        assert_eq!(
            system.stake_table.exitEscrowPeriod().call().await?._0,
            U256::from(exit_escrow_period.as_secs())
        );

        let to = "0x1111111111111111111111111111111111111111".parse()?;

        // sanity check that we can transfer tokens
        system.transfer(to, U256::from(123)).await?;

        // sanity check that we can fetch the balance
        assert_eq!(system.balance(to).await?, U256::from(123));

        Ok(())
    }
}
