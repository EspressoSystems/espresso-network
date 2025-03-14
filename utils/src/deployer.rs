use std::{collections::HashMap, io::Write, ops::Deref, sync::Arc, time::Duration};

use alloy::{
    contract::RawCallBuilder,
    network::{Ethereum, EthereumWallet},
    providers::ProviderBuilder,
    signers::Signer as _,
    transports::Transport,
};
use anyhow::{ensure, Context};
use clap::{builder::OsStr, Parser, ValueEnum};
use contract_bindings_alloy::feecontract::FeeContract;
use contract_bindings_ethers::{
    erc1967_proxy::ERC1967Proxy,
    light_client::{LightClient, LIGHTCLIENT_ABI},
    light_client_mock::LIGHTCLIENTMOCK_ABI,
    permissioned_stake_table::{NodeInfo, PermissionedStakeTable},
    plonk_verifier::PlonkVerifier,
};
use derive_more::Display;
use ethers::{
    prelude::*, signers::coins_bip39::English, solc::artifacts::BytecodeObject, utils::hex,
};
use ethers_conv::{ToAlloy as _, ToEthers};
use futures::future::{BoxFuture, FutureExt};
use hotshot_contract_adapter::light_client::{
    LightClientConstructorArgs, ParsedLightClientState, ParsedStakeTableState,
};
use url::Url;

/// Set of predeployed contracts.
#[derive(Clone, Debug, Parser)]
pub struct DeployedContracts {
    /// Use an already-deployed PlonkVerifier.sol instead of deploying a new one.
    #[arg(long, env = "PLONK_VERIFIER")]
    plonk_verifier: Option<Address>,

    /// Use an already-deployed LightClient.sol instead of deploying a new one.
    #[arg(long, env = "LIGHT_CLIENT")]
    light_client: Option<Address>,

    /// Use an already-deployed LightClient.sol proxy instead of deploying a new one.
    #[arg(long, env = "LIGHT_CLIENT_PROXY")]
    light_client_proxy: Option<Address>,

    /// Use an already-deployed FeeContract.sol instead of deploying a new one.
    #[arg(long, env = "FEE_CONTRACT")]
    fee_contract: Option<Address>,

    /// Use an already-deployed FeeContract.sol proxy instead of deploying a new one.
    #[arg(long, env = "FEE_CONTRACT_PROXY")]
    fee_contract_proxy: Option<Address>,

    /// Use an already-deployed PermissonedStakeTable.sol proxy instead of deploying a new one.
    #[arg(long, env = "PERMISSIONED_STAKE_TABLE")]
    permissioned_stake_table: Option<Address>,
}

/// An identifier for a particular contract.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash)]
pub enum Contract {
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_ADDRESS")]
    PlonkVerifier,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_ADDRESS")]
    LightClient,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")]
    LightClientProxy,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS")]
    FeeContract,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS")]
    FeeContractProxy,
    #[display("ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_ADDRESS")]
    PermissonedStakeTable,
}

impl From<Contract> for OsStr {
    fn from(c: Contract) -> OsStr {
        c.to_string().into()
    }
}

/// Cache of contracts predeployed or deployed during this current run.
#[derive(Debug, Clone, Default)]
pub struct Contracts(HashMap<Contract, Address>);

impl From<DeployedContracts> for Contracts {
    fn from(deployed: DeployedContracts) -> Self {
        let mut m = HashMap::new();
        if let Some(addr) = deployed.plonk_verifier {
            m.insert(Contract::PlonkVerifier, addr);
        }
        if let Some(addr) = deployed.light_client {
            m.insert(Contract::LightClient, addr);
        }
        if let Some(addr) = deployed.light_client_proxy {
            m.insert(Contract::LightClientProxy, addr);
        }
        if let Some(addr) = deployed.fee_contract {
            m.insert(Contract::FeeContract, addr);
        }
        if let Some(addr) = deployed.fee_contract_proxy {
            m.insert(Contract::FeeContractProxy, addr);
        }
        if let Some(addr) = deployed.permissioned_stake_table {
            m.insert(Contract::PermissonedStakeTable, addr);
        }
        Self(m)
    }
}

impl Contracts {
    pub fn new() -> Self {
        Contracts(HashMap::new())
    }

    pub fn get_contract_address(&self, contract: Contract) -> Option<Address> {
        self.0.get(&contract).copied()
    }
    /// Deploy a contract by calling a function.
    ///
    /// The `deploy` function will be called only if contract `name` is not already deployed;
    /// otherwise this function will just return the predeployed address. The `deploy` function may
    /// access this [`Contracts`] object, so this can be used to deploy contracts recursively in
    /// dependency order.
    pub async fn deploy_fn(
        &mut self,
        name: Contract,
        deploy: impl FnOnce(&mut Self) -> BoxFuture<'_, anyhow::Result<Address>>,
    ) -> anyhow::Result<Address> {
        if let Some(addr) = self.0.get(&name) {
            tracing::info!("skipping deployment of {name}, already deployed at {addr:#x}");
            return Ok(*addr);
        }
        tracing::info!("deploying {name}");
        let addr = deploy(self).await?;
        tracing::info!("deployed {name} at {addr:#x}");

        self.0.insert(name, addr);
        Ok(addr)
    }

    /// Deploy a contract by calling a function.
    ///
    /// The `deploy` function will be called only if contract `name` is not already deployed;
    /// otherwise this function will just return the predeployed address. The `deploy` function may
    /// access this [`Contracts`] object, so this can be used to deploy contracts recursively in
    /// dependency order.
    pub async fn deploy_fn_alloy(
        &mut self,
        name: Contract,
        deploy: impl FnOnce(&mut Self) -> BoxFuture<'_, anyhow::Result<alloy::primitives::Address>>,
    ) -> anyhow::Result<alloy::primitives::Address> {
        if let Some(addr) = self.0.get(&name) {
            tracing::info!("skipping deployment of {name}, already deployed at {addr:#x}");
            return Ok((*addr).to_alloy());
        }
        tracing::info!("deploying {name}");
        let addr = deploy(self).await?;
        tracing::info!("deployed {name} at {addr:#x}");

        self.0.insert(name, addr.to_ethers());
        Ok(addr)
    }

    /// Deploy a contract by executing its deploy transaction.
    ///
    /// The transaction will only be broadcast if contract `name` is not already deployed.
    pub async fn deploy_tx<M, C>(
        &mut self,
        name: Contract,
        tx: ContractDeployer<M, C>,
    ) -> anyhow::Result<Address>
    where
        M: Middleware + 'static,
        C: Deref<Target = ethers::contract::Contract<M>>
            + From<ethers::contract::ContractInstance<Arc<M>, M>>
            + Send
            + 'static,
    {
        self.deploy_fn(name, |_| {
            async {
                let contract = tx.send().await?;
                Ok(contract.address())
            }
            .boxed()
        })
        .await
    }

    /// Deploy a contract by executing its deploy transaction.
    ///
    /// The transaction will only be broadcast if contract `name` is not already deployed.
    pub async fn deploy_tx_alloy<T, P>(
        &mut self,
        name: Contract,
        tx: RawCallBuilder<T, P>,
    ) -> anyhow::Result<alloy::primitives::Address>
    where
        T: Transport + Clone,
        P: alloy::providers::Provider<T, Ethereum> + 'static,
    {
        self.deploy_fn_alloy(name, |_| {
            async move {
                let address = tx.deploy().await?;
                Ok(address)
            }
            .boxed()
        })
        .await
    }

    /// Write a .env file.
    pub fn write(&self, mut w: impl Write) -> anyhow::Result<()> {
        for (contract, address) in &self.0 {
            writeln!(w, "{contract}={address:#x}")?;
        }
        Ok(())
    }
}

/// Default deployment function `LightClient.sol` in production
///
/// # NOTE:
/// currently, `LightClient.sol` follows upgradable contract, thus a follow-up
/// call to `.initialize()` with proper genesis block (and other constructor args)
/// are expected to be *delegatecall-ed through the proxy contract*.
pub async fn deploy_light_client_contract<M: Middleware + 'static>(
    l1: Arc<M>,
    contracts: &mut Contracts,
) -> anyhow::Result<Address> {
    // Deploy library contracts.
    let plonk_verifier = contracts
        .deploy_tx(
            Contract::PlonkVerifier,
            PlonkVerifier::deploy(l1.clone(), ())?,
        )
        .await?;

    let bytecode = link_light_client_contract(plonk_verifier)?;

    // Deploy light client.
    let light_client_factory = ContractFactory::new(
        LIGHTCLIENT_ABI.clone(),
        bytecode
            .as_bytes()
            .context("error parsing bytecode for linked LightClient contract")?
            .clone(),
        l1,
    );
    let contract = light_client_factory.deploy(())?.send().await?;
    Ok(contract.address())
}

/// Default deployment function `LightClientMock.sol` for testing
///
/// # NOTE
/// unlike [`deploy_light_client_contract()`], the `LightClientMock` doesn't
/// use upgradable contract for simplicity, thus there's no follow-up `.initialize()`
/// necessary, as we have already call its un-disabled constructor.
pub async fn deploy_mock_light_client_contract<M: Middleware + 'static>(
    l1: Arc<M>,
    contracts: &mut Contracts,
    constructor_args: Option<LightClientConstructorArgs>,
) -> anyhow::Result<Address> {
    // Deploy library contracts.
    let plonk_verifier = contracts
        .deploy_tx(
            Contract::PlonkVerifier,
            PlonkVerifier::deploy(l1.clone(), ())?,
        )
        .await?;

    let mut bytecode: BytecodeObject = serde_json::from_str(include_str!(
        "../../contract-bindings/artifacts/LightClientMock_bytecode.json",
    ))?;
    ensure!(
        bytecode.is_unlinked(),
        "LightClientMock contract bytecode is linked, but should have an external library"
    );
    bytecode
        .link_fully_qualified(
            "contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier",
            plonk_verifier,
        )
        .resolve()
        .context("error linking PlonkVerifier lib")?;
    ensure!(
        !bytecode.is_unlinked(),
        "failed to link LightClientMock.sol"
    );

    // Deploy light client.
    let light_client_factory = ContractFactory::new(
        LIGHTCLIENTMOCK_ABI.clone(),
        bytecode
            .as_bytes()
            .context("error parsing bytecode for linked LightClientMock contract")?
            .clone(),
        l1,
    );
    let constructor_args: LightClientConstructorArgs = match constructor_args {
        Some(args) => args,
        None => LightClientConstructorArgs::dummy_genesis(),
    };
    let contract = light_client_factory
        .deploy(constructor_args)?
        .send()
        .await?;

    Ok(contract.address())
}

#[allow(clippy::too_many_arguments)]
pub async fn deploy(
    l1url: Url,
    l1_interval: Duration,
    mnemonic: String,
    account_index: u32,
    multisig_address: Option<H160>,
    use_mock_contract: bool,
    only: Option<Vec<ContractGroup>>,
    genesis: BoxFuture<'_, anyhow::Result<(ParsedLightClientState, ParsedStakeTableState)>>,
    permissioned_prover: Option<Address>,
    mut contracts: Contracts,
    initial_stake_table: Option<Vec<NodeInfo>>,
) -> anyhow::Result<Contracts> {
    let provider = Provider::<Http>::try_from(l1url.to_string())?.interval(l1_interval);
    let chain_id = provider.get_chainid().await?.as_u64();
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(mnemonic.as_str())
        .index(account_index)?
        .build()?
        .with_chain_id(chain_id);
    let deployer = wallet.address();
    let l1 = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

    let signer_alloy = alloy::signers::local::MnemonicBuilder::<
        alloy::signers::local::coins_bip39::English,
    >::default()
    .phrase(mnemonic.as_str())
    .index(account_index)?
    .build()?
    .with_chain_id(Some(chain_id));
    let wallet_alloy = EthereumWallet::from(signer_alloy);
    let l1_alloy = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet_alloy)
        .on_http(l1url);

    // As a sanity check, check that the deployer address has some balance of ETH it can use to pay
    // gas.
    let balance = l1.get_balance(deployer, None).await?;
    ensure!(
        balance > 0.into(),
        "deployer account {deployer:#x} is not funded!"
    );
    tracing::info!(%balance, "deploying from address {deployer:#x}");

    // `LightClient.sol`
    if should_deploy(ContractGroup::LightClient, &only) {
        // Deploy the upgradable light client contract first, then initialize it through a proxy contract
        let lc_address = if use_mock_contract {
            contracts
                .deploy_fn(Contract::LightClient, |contracts| {
                    deploy_mock_light_client_contract(l1.clone(), contracts, None).boxed()
                })
                .await?
        } else {
            contracts
                .deploy_fn(Contract::LightClient, |contracts| {
                    deploy_light_client_contract(l1.clone(), contracts).boxed()
                })
                .await?
        };
        let light_client = LightClient::new(lc_address, l1.clone());

        let (genesis_lc, genesis_stake) = genesis.await?.clone();

        let data = light_client
            .initialize(genesis_lc.into(), genesis_stake.into(), 864000, deployer)
            .calldata()
            .context("calldata for initialize transaction not available")?;
        let light_client_proxy_address = contracts
            .deploy_tx(
                Contract::LightClientProxy,
                ERC1967Proxy::deploy(l1.clone(), (lc_address, data))?,
            )
            .await?;

        // confirm that the implementation address is the address of the light client contract deployed above
        if !is_proxy_contract(&provider, light_client_proxy_address)
            .await
            .expect("Failed to determine if light contract is a proxy")
        {
            panic!("Light Client contract's address is not a proxy");
        }

        // Instantiate a wrapper with the proxy address and light client ABI.
        let proxy = LightClient::new(light_client_proxy_address, l1.clone());

        // Perission the light client prover.
        if let Some(prover) = permissioned_prover {
            tracing::info!(%light_client_proxy_address, %prover, "setting permissioned prover");
            proxy.set_permissioned_prover(prover).send().await?.await?;
        }

        // Transfer ownership to the multisig wallet if provided.
        if let Some(owner) = multisig_address {
            tracing::info!(
                ?light_client_proxy_address,
                ?owner,
                "transferring light client proxy ownership to multisig",
            );
            proxy.transfer_ownership(owner).send().await?.await?;
        }
    }

    // `FeeContract.sol`
    if should_deploy(ContractGroup::FeeContract, &only) {
        let fee_contract_address = contracts
            .deploy_tx_alloy(
                Contract::FeeContract,
                FeeContract::deploy_builder(l1_alloy.clone()),
            )
            .await?;
        let fee_contract = FeeContract::new(fee_contract_address, l1_alloy.clone());
        let data = fee_contract
            .initialize(deployer.to_alloy())
            .calldata()
            .clone();
        let fee_contract_proxy_address = contracts
            .deploy_tx(
                Contract::FeeContractProxy,
                ERC1967Proxy::deploy(
                    l1.clone(),
                    (fee_contract.address().to_ethers(), data.to_ethers()),
                )?,
            )
            .await?;

        // confirm that the implementation address is the address of the fee contract deployed above
        if !is_proxy_contract(&provider, fee_contract_proxy_address)
            .await
            .expect("Failed to determine if fee contract is a proxy")
        {
            panic!("Fee contract's address is not a proxy");
        }

        // Instantiate a wrapper with the proxy address and fee contract ABI.
        let proxy = FeeContract::new(fee_contract_proxy_address.to_alloy(), l1_alloy.clone());

        // Transfer ownership to the multisig wallet if provided.
        if let Some(owner) = multisig_address {
            tracing::info!(
                ?fee_contract_proxy_address,
                ?owner,
                "transferring fee contract proxy ownership to multisig",
            );
            let _ = proxy.transferOwnership(owner.to_alloy()).send().await?;
        }
    }

    // `PermissionedStakeTable.sol`
    if should_deploy(ContractGroup::PermissionedStakeTable, &only) {
        let initial_stake_table: Vec<_> = initial_stake_table.unwrap_or_default();
        let stake_table_address = contracts
            .deploy_tx(
                Contract::PermissonedStakeTable,
                PermissionedStakeTable::deploy(l1.clone(), initial_stake_table)?,
            )
            .await?;
        let stake_table = PermissionedStakeTable::new(stake_table_address, l1.clone());

        // Transfer ownership to the multisig wallet if provided.
        if let Some(owner) = multisig_address {
            tracing::info!(
                ?stake_table_address,
                ?owner,
                "transferring PermissionedStakeTable ownership to multisig",
            );
            stake_table.transfer_ownership(owner).send().await?.await?;
        }
    }

    Ok(contracts)
}

fn should_deploy(group: ContractGroup, only: &Option<Vec<ContractGroup>>) -> bool {
    match only {
        Some(groups) => groups.contains(&group),
        None => true,
    }
}

pub async fn is_proxy_contract(
    provider: &impl Middleware<Error: 'static>,
    proxy_address: H160,
) -> anyhow::Result<bool> {
    // confirm that the proxy_address is a proxy
    // using the implementation slot, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc, which is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1
    let hex_bytes = hex::decode("360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc")
        .expect("Failed to decode hex string");
    let implementation_slot = ethers::types::H256::from_slice(&hex_bytes);
    let storage = provider
        .get_storage_at(proxy_address, implementation_slot, None)
        .await?;

    let implementation_address = H160::from_slice(&storage[12..]);

    // when the implementation address is not equal to zero, it's a proxy
    Ok(implementation_address != H160::zero())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ContractGroup {
    FeeContract,
    LightClient,
    PermissionedStakeTable,
}

// Link with LightClient's bytecode artifacts. We include the unlinked bytecode for the contract
// in this binary so that the contract artifacts do not have to be distributed with the binary.
// This should be fine because if the bindings we are importing are up to date, so should be the
// contract artifacts: this is no different than foundry inlining bytecode objects in generated
// bindings, except that foundry doesn't provide the bytecode for contracts that link with
// libraries, so we have to do it ourselves.
fn link_light_client_contract(
    plonk_verifier: Address,
    // vk: Address,
) -> anyhow::Result<BytecodeObject> {
    let mut bytecode: BytecodeObject = serde_json::from_str(include_str!(
        "../../contract-bindings/artifacts/LightClient_bytecode.json",
    ))?;
    ensure!(
        bytecode.is_unlinked(),
        "LightClient contract bytecode is linked, but should have an external library"
    );
    bytecode
        .link_fully_qualified(
            "contracts/src/libraries/PlonkVerifier.sol:PlonkVerifier",
            plonk_verifier,
        )
        .resolve()
        .context("error linking PlonkVerifier lib")?;
    ensure!(!bytecode.is_unlinked(), "failed to link LightClient.sol");
    Ok(bytecode)
}

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers {

    use std::sync::Arc;

    use anyhow::Context;
    use contract_bindings_ethers::{
        erc1967_proxy::ERC1967Proxy,
        fee_contract::{FeeContract, FEECONTRACT_ABI, FEECONTRACT_BYTECODE},
        light_client::{LightClient, LIGHTCLIENT_ABI},
        plonk_verifier::PlonkVerifier,
    };
    use ethers::prelude::*;
    use hotshot_contract_adapter::light_client::LightClientConstructorArgs;

    use super::{Contract, Contracts};
    use crate::deployer::link_light_client_contract;

    /// Deployment `LightClientMock.sol` as proxy for testing
    pub async fn deploy_light_client_contract_as_proxy_for_test<M: Middleware + 'static>(
        l1: Arc<M>,
        contracts: &mut Contracts,
        constructor_args: Option<LightClientConstructorArgs>,
    ) -> anyhow::Result<Address> {
        // Deploy library contracts.
        let plonk_verifier = contracts
            .deploy_tx(
                Contract::PlonkVerifier,
                PlonkVerifier::deploy(l1.clone(), ())?,
            )
            .await?;

        let bytecode = link_light_client_contract(plonk_verifier)?;

        // Deploy light client.
        let light_client_factory = ContractFactory::new(
            LIGHTCLIENT_ABI.clone(),
            bytecode
                .as_bytes()
                .context("error parsing bytecode for linked LightClient contract")?
                .clone(),
            l1.clone(),
        );
        let contract = light_client_factory.deploy(())?.send().await?;

        let light_client_address = contract.address();

        let light_client = LightClient::new(light_client_address, l1.clone());

        let constructor_args: LightClientConstructorArgs = match constructor_args {
            Some(args) => args,
            None => LightClientConstructorArgs::dummy_genesis(),
        };

        let deployer = *(l1.clone().get_accounts().await?.first()).expect("Address not found");

        let data = light_client
            .initialize(
                constructor_args.light_client_state.into(),
                constructor_args.stake_table_state.into(),
                constructor_args.max_history_seconds,
                deployer,
            )
            .calldata()
            .context("calldata for initialize transaction not available")?;
        let light_client_proxy_address = contracts
            .deploy_tx(
                Contract::LightClientProxy,
                ERC1967Proxy::deploy(l1, (light_client_address, data))?,
            )
            .await?;

        Ok(light_client_proxy_address)
    }

    /// Default deployment function `FeeContract.sol` for testing
    ///
    pub async fn deploy_fee_contract<M: Middleware + 'static>(
        l1: Arc<M>,
    ) -> anyhow::Result<Address> {
        // Deploy fee contract
        let fee_contract_factory = ContractFactory::new(
            FEECONTRACT_ABI.clone(),
            FEECONTRACT_BYTECODE.clone(),
            l1.clone(),
        );
        let contract = fee_contract_factory.deploy(())?.send().await?;

        let fee_contract_address = contract.address();

        Ok(fee_contract_address)
    }

    /// Default deployment function `FeeContract.sol` (as proxy) for testing
    ///
    pub async fn deploy_fee_contract_as_proxy<M: Middleware + 'static>(
        l1: Arc<M>,
        contracts: &mut Contracts,
    ) -> anyhow::Result<Address> {
        // Deploy fee contract
        let fee_contract_factory = ContractFactory::new(
            FEECONTRACT_ABI.clone(),
            FEECONTRACT_BYTECODE.clone(),
            l1.clone(),
        );
        let contract = fee_contract_factory.deploy(())?.send().await?;

        let fee_contract_address = contract.address();

        let fee_contract = FeeContract::new(fee_contract_address, l1.clone());

        let deployer = *(l1.clone().get_accounts().await?.first()).expect("Address not found");

        let data = fee_contract
            .initialize(deployer)
            .calldata()
            .context("calldata for initialize transaction not available")?;

        let fee_contract_proxy_address = contracts
            .deploy_tx(
                Contract::FeeContractProxy,
                ERC1967Proxy::deploy(l1.clone(), (fee_contract_address, data))?,
            )
            .await?;

        Ok(fee_contract_proxy_address)
    }
}
