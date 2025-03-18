use std::{collections::HashMap, io::Write};

use alloy::{
    contract::RawCallBuilder,
    hex::{FromHex, ToHexExt},
    network::TransactionBuilder,
    primitives::Bytes,
    providers::Provider,
};
use alloy::{
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::{anyhow, Result};
use clap::{builder::OsStr, Parser};
use derive_more::Display;
use hotshot_contract_adapter::sol_types::*;

// We pass this during `forge bind --libraries` as a placeholder for the actual deployed library address
const LIBRARY_PLACEHOLDER_ADDRESS: &str = "ffffffffffffffffffffffffffffffffffffffff";
/// `stateHistoryRetentionPeriod` in LightClient.sol as the maximum retention period in seconds
pub const MAX_HISTORY_RETENTION_SECONDS: u32 = 864000;

/// Set of predeployed contracts.
#[derive(Clone, Debug, Parser)]
pub struct DeployedContracts {
    /// Use an already-deployed PlonkVerifier.sol instead of deploying a new one.
    #[clap(long, env = Contract::PlonkVerifier)]
    plonk_verifier: Option<Address>,

    /// Use an already-deployed LightClient.sol instead of deploying a new one.
    #[clap(long, env = Contract::LightClient)]
    light_client: Option<Address>,

    /// Use an already-deployed LightClient.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::LightClientProxy)]
    light_client_proxy: Option<Address>,

    /// Use an already-deployed FeeContract.sol instead of deploying a new one.
    #[clap(long, env = Contract::FeeContract)]
    fee_contract: Option<Address>,

    /// Use an already-deployed FeeContract.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::FeeContractProxy)]
    fee_contract_proxy: Option<Address>,

    /// Use an already-deployed PermissonedStakeTable.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::PermissonedStakeTable)]
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

    pub fn address(&self, contract: Contract) -> Option<Address> {
        self.0.get(&contract).copied()
    }

    /// Deploy a contract (with logging and cached deployments)
    ///
    /// The deployment `tx` will be sent only if contract `name` is not already deployed;
    /// otherwise this function will just return the predeployed address.
    pub async fn deploy<T, P>(
        &mut self,
        name: Contract,
        tx: RawCallBuilder<T, P>,
    ) -> Result<Address>
    where
        P: Provider,
    {
        if let Some(addr) = self.0.get(&name) {
            tracing::info!("skipping deployment of {name}, already deployed at {addr:#x}");
            return Ok(*addr);
        }
        tracing::info!("deploying {name}");
        let addr = tx.deploy().await?;
        tracing::info!("deployed {name} at {addr:#x}");

        self.0.insert(name, addr);
        Ok(addr)
    }

    /// Write a .env file.
    pub fn write(&self, mut w: impl Write) -> Result<()> {
        for (contract, address) in &self.0 {
            writeln!(w, "{contract}={address:#x}")?;
        }
        Ok(())
    }
}

/// Default deployment function `LightClient.sol` or `LightClientMock.sol` with `mock: true`.
///
/// # NOTE:
/// In most cases, you only need to use [`deploy_light_client_proxy()`]
///
/// # NOTE:
/// currently, `LightClient.sol` follows upgradable contract, thus a follow-up
/// call to `.initialize()` with proper genesis block (and other constructor args)
/// are expected to be *delegatecall-ed through the proxy contract*.
pub(crate) async fn deploy_light_client_contract(
    provider: impl Provider,
    contracts: &mut Contracts,
    mock: bool,
) -> Result<Address> {
    // Deploy library contracts.
    let plonk_verifier_addr = contracts
        .deploy(
            Contract::PlonkVerifier,
            PlonkVerifier::deploy_builder(&provider),
        )
        .await?;

    // when generate alloy's bindings, we supply a placeholder address, now we modify the actual
    // bytecode with deployed address of the library.
    let target_lc_bytecode = if mock {
        LightClient::BYTECODE.encode_hex()
    } else {
        LightClientMock::BYTECODE.encode_hex()
    };
    let lc_linked_bytecode = Bytes::from_hex(target_lc_bytecode.replace(
        LIBRARY_PLACEHOLDER_ADDRESS,
        &plonk_verifier_addr.encode_hex(),
    ))?;

    // Deploy the light client
    let light_client_addr = if mock {
        // for mock, we don't populate the `contracts` since it only track production-ready deployments
        LightClientMock::deploy_builder(&provider)
            .map(|req| req.with_deploy_code(lc_linked_bytecode))
            .deploy()
            .await?
    } else {
        contracts
            .deploy(
                Contract::LightClient,
                LightClient::deploy_builder(&provider)
                    .map(|req| req.with_deploy_code(lc_linked_bytecode)),
            )
            .await?
    };
    Ok(light_client_addr)
}

/// The primary logic for deploying and initializing an upgradable light client contract.
///
/// Deploy the upgradable proxy contract, point to an already deployed light client contract as its implementation, and invoke `initialize()` on it.
/// This is run after `deploy_light_client_contract()`, returns the proxy address.
/// This works for both mock and production light client proxy.
pub async fn deploy_light_client_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    mock: bool,
    genesis_state: LightClientStateSol,
    genesis_stake: StakeTableStateSol,
    admin: Address,
    prover: Option<Address>,
) -> Result<Address> {
    // deploy the light client implementation contract
    let impl_addr = deploy_light_client_contract(&provider, contracts, mock).await?;
    let lc = LightClient::new(impl_addr, &provider);

    // prepare the input arg for `initialize()`
    let init_data = lc
        .initialize(
            genesis_state,
            genesis_stake,
            MAX_HISTORY_RETENTION_SECONDS,
            admin,
        )
        .calldata()
        .to_owned();
    // deploy proxy and initalize
    let lc_proxy_addr = contracts
        .deploy(
            Contract::LightClientProxy,
            ERC1967Proxy::deploy_builder(&provider, impl_addr, init_data),
        )
        .await?;

    // sanity check
    if !is_proxy_contract(&provider, lc_proxy_addr).await? {
        panic!("LightClientProxy detected not as a proxy, report error!");
    }

    // instantiate a proxy instance, cast as LightClient's ABI interface
    let lc_proxy = LightClient::new(lc_proxy_addr, &provider);

    // set permissioned prover
    if let Some(prover) = prover {
        tracing::info!(%lc_proxy_addr, %prover, "Set permissioned prover ");
        lc_proxy
            .setPermissionedProver(prover)
            .send()
            .await?
            .get_receipt()
            .await?;
    }

    Ok(lc_proxy_addr)
}

/// The primary logic for deploying and initializing an upgradable fee contract.
///
/// Deploy the upgradable proxy contract, point to a deployed fee contract as its implementation, and invoke `initialize()` on it.
/// Return the proxy address.
pub async fn deploy_fee_contract_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    admin: Address,
) -> Result<Address> {
    // deploy the fee implementation contract
    let fee_addr = contracts
        .deploy(
            Contract::FeeContract,
            FeeContract::deploy_builder(&provider),
        )
        .await?;
    let fee = FeeContract::new(fee_addr, &provider);

    // prepare the input arg for `initialize()`
    let init_data = fee.initialize(admin).calldata().to_owned();
    // deploy proxy and initalize
    let fee_proxy_addr = contracts
        .deploy(
            Contract::FeeContractProxy,
            ERC1967Proxy::deploy_builder(&provider, fee_addr, init_data),
        )
        .await?;
    // sanity check
    if !is_proxy_contract(&provider, fee_proxy_addr).await? {
        panic!("FeeContractProxy detected not as a proxy, report error!");
    }

    Ok(fee_proxy_addr)
}

/// The primary logic for deploying permissioned stake table contract.
/// Return the contract address.
pub async fn deploy_permissioned_stake_table(
    provider: impl Provider,
    contracts: &mut Contracts,
    init_stake_table: Vec<NodeInfoSol>,
) -> Result<Address> {
    // deploy the permissioned stake table contract, with initStakers constructor
    let stake_table_addr = contracts
        .deploy(
            Contract::PermissonedStakeTable,
            PermissionedStakeTable::deploy_builder(&provider, init_stake_table),
        )
        .await?;
    Ok(stake_table_addr)
}

/// Common logic for any Ownable contract to tranfer ownership
pub async fn transfer_ownership(
    provider: impl Provider,
    target: Contract,
    addr: Address,
    new_owner: Address,
) -> Result<TransactionReceipt> {
    let receipt = match target {
        Contract::LightClient | Contract::LightClientProxy => {
            tracing::info!(%addr, %new_owner, "Transfer LightClient ownership");
            let lc = LightClient::new(addr, &provider);
            lc.transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::FeeContract | Contract::FeeContractProxy => {
            tracing::info!(%addr, %new_owner, "Transfer FeeContract ownership");
            let fee = FeeContract::new(addr, &provider);
            fee.transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::PermissonedStakeTable => {
            tracing::info!(%addr, %new_owner, "Transfer PermissionedStakeTable ownership");
            let st = PermissionedStakeTable::new(addr, &provider);
            st.transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        _ => return Err(anyhow!("Not Ownable, can't transfer ownership!")),
    };
    Ok(receipt)
}

/// helper function to decide if the contract at given address `addr` is a proxy contract
pub async fn is_proxy_contract(provider: impl Provider, addr: Address) -> Result<bool> {
    // confirm that the proxy_address is a proxy
    // using the implementation slot, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc, which is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1
    let impl_slot = U256::from_str_radix(
        "360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc",
        16,
    )?;
    let storage = provider.get_storage_at(addr, impl_slot).await?;
    let impl_address = Address::from_slice(&storage.to_be_bytes_vec()[12..]);

    // when the implementation address is not equal to zero, it's a proxy
    Ok(impl_address != Address::default())
}

#[cfg(test)]
mod tests {
    use alloy::providers::ProviderBuilder;
    use hotshot::rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[tokio::test]
    async fn test_is_proxy_contract() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let deployer = provider.get_accounts().await?[0];

        let fee_contract = FeeContract::deploy(&provider).await?;
        let init_data = fee_contract.initialize(deployer).calldata().clone();
        let proxy = ERC1967Proxy::deploy(&provider, *fee_contract.address(), init_data).await?;

        assert!(is_proxy_contract(&provider, *proxy.address()).await?);
        assert!(!is_proxy_contract(&provider, *fee_contract.address()).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_light_client() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // first test if LightClientMock can be deployed
        let mock_lc_addr = deploy_light_client_contract(&provider, &mut contracts, true).await?;
        let pv_addr = contracts.address(Contract::PlonkVerifier).unwrap();

        // then deploy the actual LightClient
        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;
        assert_ne!(mock_lc_addr, lc_addr);
        // check that we didn't redeploy PlonkVerifier again, instead use existing ones
        assert_eq!(contracts.address(Contract::PlonkVerifier).unwrap(), pv_addr);
        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_light_client_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // prepare `initialize()` input
        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();
        let admin = provider.get_accounts().await?[0];
        let prover = Address::random();

        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            false,
            genesis_state.clone(),
            genesis_stake.clone(),
            admin,
            Some(prover),
        )
        .await?;

        // check initialization is correct
        let lc = LightClient::new(lc_proxy_addr, &provider);
        let finalized_state = lc.finalizedState().call().await?;
        assert_eq!(finalized_state.viewNum, genesis_state.viewNum);
        assert_eq!(finalized_state.blockHeight, genesis_state.blockHeight);
        assert_eq!(&finalized_state.blockCommRoot, &genesis_state.blockCommRoot);

        let fetched_stake = lc.genesisStakeTableState().call().await?;
        assert_eq!(fetched_stake.blsKeyComm, genesis_stake.blsKeyComm);
        assert_eq!(fetched_stake.schnorrKeyComm, genesis_stake.schnorrKeyComm);
        assert_eq!(fetched_stake.amountComm, genesis_stake.amountComm);
        assert_eq!(fetched_stake.threshold, genesis_stake.threshold);

        let fetched_prover = lc.permissionedProver().call().await?;
        assert_eq!(fetched_prover._0, prover);

        // test transfer ownership to multisig
        let multisig = Address::random();
        let _receipt = transfer_ownership(
            &provider,
            Contract::LightClientProxy,
            lc_proxy_addr,
            multisig,
        )
        .await?;
        assert_eq!(lc.owner().call().await?._0, multisig);

        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_fee_contract_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let admin = provider.get_accounts().await?[0];
        let alice = Address::random();

        let fee_proxy_addr = deploy_fee_contract_proxy(&provider, &mut contracts, alice).await?;

        // check initialization is correct
        let fee = FeeContract::new(fee_proxy_addr, &provider);
        let fetched_owner = fee.owner().call().await?._0;
        assert_eq!(fetched_owner, alice);

        // redeploy new fee with admin being the owner
        contracts = Contracts::new();
        let fee_proxy_addr = deploy_fee_contract_proxy(&provider, &mut contracts, admin).await?;
        let fee = FeeContract::new(fee_proxy_addr, &provider);

        // test transfer ownership to multisig
        let multisig = Address::random();
        let _receipt = transfer_ownership(
            &provider,
            Contract::FeeContractProxy,
            fee_proxy_addr,
            multisig,
        )
        .await?;
        assert_eq!(fee.owner().call().await?._0, multisig);

        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_permissioned_stake_table() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let mut rng = StdRng::from_seed([42u8; 32]);

        let mut init_stake_table = vec![];
        for _ in 0..5 {
            init_stake_table.push(NodeInfoSol::rand(&mut rng));
        }
        let st_addr =
            deploy_permissioned_stake_table(&provider, &mut contracts, init_stake_table.clone())
                .await?;

        // check initialization is correct
        let st = PermissionedStakeTable::new(st_addr, &provider);
        for staker in init_stake_table {
            assert!(st.isStaker(staker.blsVK).call().await?._0);
        }

        // test transfer ownership to multisig
        let multisig = Address::random();
        let _receipt = transfer_ownership(
            &provider,
            Contract::PermissonedStakeTable,
            st_addr,
            multisig,
        )
        .await?;
        assert_eq!(st.owner().call().await?._0, multisig);

        Ok(())
    }
}
