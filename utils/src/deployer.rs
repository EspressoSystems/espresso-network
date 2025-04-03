use std::{collections::HashMap, io::Write};

use alloy::{
    contract::RawCallBuilder,
    hex::{FromHex, ToHexExt},
    network::TransactionBuilder,
    primitives::{Address, Bytes, U256},
    providers::Provider,
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
    /// PlonkVerifierV2.sol
    #[clap(long, env = Contract::PlonkVerifierV2)]
    plonk_verifier_v2: Option<Address>,

    /// Use an already-deployed LightClient.sol instead of deploying a new one.
    #[clap(long, env = Contract::LightClient)]
    light_client: Option<Address>,
    /// LightClientV2.sol
    #[clap(long, env = Contract::LightClientV2)]
    light_client_v2: Option<Address>,

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

    /// Use an already-deployed EspToken.sol instead of deploying a new one.
    #[clap(long, env = Contract::EspToken)]
    esp_token: Option<Address>,

    /// Use an already-deployed EspToken.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::EspTokenProxy)]
    esp_token_proxy: Option<Address>,

    /// Use an already-deployed StakeTable.sol instead of deploying a new one.
    #[clap(long, env = Contract::StakeTable)]
    stake_table: Option<Address>,

    /// Use an already-deployed StakeTable.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::StakeTableProxy)]
    stake_table_proxy: Option<Address>,
}

/// An identifier for a particular contract.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash)]
pub enum Contract {
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_ADDRESS")]
    PlonkVerifier,
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_V2_ADDRESS")]
    PlonkVerifierV2,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_ADDRESS")]
    LightClient,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_V2_ADDRESS")]
    LightClientV2,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")]
    LightClientProxy,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS")]
    FeeContract,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS")]
    FeeContractProxy,
    #[display("ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_ADDRESS")]
    PermissonedStakeTable,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_ADDRESS")]
    EspToken,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS")]
    EspTokenProxy,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_ADDRESS")]
    StakeTable,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS")]
    StakeTableProxy,
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
        if let Some(addr) = deployed.plonk_verifier_v2 {
            m.insert(Contract::PlonkVerifierV2, addr);
        }
        if let Some(addr) = deployed.light_client {
            m.insert(Contract::LightClient, addr);
        }
        if let Some(addr) = deployed.light_client_v2 {
            m.insert(Contract::LightClientV2, addr);
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
        if let Some(addr) = deployed.esp_token {
            m.insert(Contract::EspToken, addr);
        }
        if let Some(addr) = deployed.esp_token_proxy {
            m.insert(Contract::EspTokenProxy, addr);
        }
        if let Some(addr) = deployed.stake_table {
            m.insert(Contract::StakeTable, addr);
        }
        if let Some(addr) = deployed.stake_table_proxy {
            m.insert(Contract::StakeTableProxy, addr);
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
        LightClientMock::BYTECODE.encode_hex()
    } else {
        LightClient::BYTECODE.encode_hex()
    };
    let lc_linked_bytecode = Bytes::from_hex(target_lc_bytecode.replace(
        LIBRARY_PLACEHOLDER_ADDRESS,
        &plonk_verifier_addr.encode_hex(),
    ))?;

    // Deploy the light client
    let light_client_addr = if mock {
        // for mock, we don't populate the `contracts` since it only track production-ready deployments
        let addr = LightClientMock::deploy_builder(&provider)
            .map(|req| req.with_deploy_code(lc_linked_bytecode))
            .deploy()
            .await?;
        tracing::info!("deployed LightClientMock at {addr:#x}");
        addr
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
    // deploy proxy and initialize
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

/// Upgrade the light client proxy to use LightClientV2.
/// Internally, first detect existence of proxy, then deploy LCV2, then upgrade and initializeV2.
/// Internal to "deploy LCV2", we deploy PlonkVerifierV2 whose address will be used at LCV2 init time.
pub async fn upgrade_light_client_v2(
    provider: impl Provider,
    contracts: &mut Contracts,
    is_mock: bool,
    blocks_per_epoch: u64,
) -> Result<TransactionReceipt> {
    match contracts.address(Contract::LightClientProxy) {
        // check if proxy already exists
        None => Err(anyhow!("LightClientProxy not found, can't upgrade")),
        Some(proxy_addr) => {
            let proxy = LightClient::new(proxy_addr, &provider);
            // first deploy PlonkVerifierV2.sol
            let pv2_addr = contracts
                .deploy(
                    Contract::PlonkVerifierV2,
                    PlonkVerifierV2::deploy_builder(&provider),
                )
                .await?;
            // then deploy LightClientV2.sol
            let target_lcv2_bytecode = if is_mock {
                LightClientV2Mock::BYTECODE.encode_hex()
            } else {
                LightClientV2::BYTECODE.encode_hex()
            };
            let lcv2_linked_bytecode = Bytes::from_hex(
                target_lcv2_bytecode.replace(LIBRARY_PLACEHOLDER_ADDRESS, &pv2_addr.encode_hex()),
            )?;

            let lcv2_addr = if is_mock {
                let addr = LightClientV2Mock::deploy_builder(&provider)
                    .map(|req| req.with_deploy_code(lcv2_linked_bytecode))
                    .deploy()
                    .await?;
                tracing::info!("deployed LightClientV2Mock at {addr:#x}");
                addr
            } else {
                contracts
                    .deploy(
                        Contract::LightClientV2,
                        LightClientV2::deploy_builder(&provider)
                            .map(|req| req.with_deploy_code(lcv2_linked_bytecode)),
                    )
                    .await?
            };

            // prepare init calldata
            let lcv2 = LightClientV2::new(lcv2_addr, &provider);
            let init_data = lcv2.initializeV2(blocks_per_epoch).calldata().to_owned();
            // invoke upgrade on proxy
            let receipt = proxy
                .upgradeToAndCall(lcv2_addr, init_data)
                .send()
                .await?
                .get_receipt()
                .await?;
            if receipt.inner.is_success() {
                tracing::info!(%lcv2_addr, "LightClientProxy successfully upgrade to: ")
            } else {
                tracing::error!("LightClientProxy upgrade failed: {:?}", receipt);
            }

            Ok(receipt)
        },
    }
}

/// The primary logic for deploying and initializing an upgradable fee contract.
///
/// Deploy the upgradable proxy contract, point to a deployed fee contract as its implementation, and invoke `initialize()` on it.
/// - `admin`: is the new owner (e.g. a multisig address) of the proxy contract
///
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
    // deploy proxy and initialize
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

/// The primary logic for deploying and initializing an upgradable Espresso Token contract.
pub async fn deploy_token_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    owner: Address,
    init_grant_recipient: Address,
) -> Result<Address> {
    let token_addr = contracts
        .deploy(Contract::EspToken, EspToken::deploy_builder(&provider))
        .await?;
    let token = EspToken::new(token_addr, &provider);

    let init_data = token
        .initialize(owner, init_grant_recipient)
        .calldata()
        .to_owned();
    let token_proxy_addr = contracts
        .deploy(
            Contract::EspTokenProxy,
            ERC1967Proxy::deploy_builder(&provider, token_addr, init_data),
        )
        .await?;

    if !is_proxy_contract(&provider, token_proxy_addr).await? {
        panic!("EspTokenProxy detected not as a proxy, report error!");
    }
    Ok(token_proxy_addr)
}

/// The primary logic for deploying and initializing an upgradable permissionless StakeTable contract.
pub async fn deploy_stake_table_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    token_addr: Address,
    light_client_addr: Address,
    exit_escrow_period: U256,
    owner: Address,
) -> Result<Address> {
    let stake_table_addr = contracts
        .deploy(Contract::StakeTable, StakeTable::deploy_builder(&provider))
        .await?;
    let stake_table = StakeTable::new(stake_table_addr, &provider);

    let init_data = stake_table
        .initialize(token_addr, light_client_addr, exit_escrow_period, owner)
        .calldata()
        .to_owned();
    let st_proxy_addr = contracts
        .deploy(
            Contract::StakeTableProxy,
            ERC1967Proxy::deploy_builder(&provider, stake_table_addr, init_data),
        )
        .await?;

    if !is_proxy_contract(&provider, st_proxy_addr).await? {
        panic!("StakeTableProxy detected not as a proxy, report error!");
    }

    Ok(st_proxy_addr)
}

/// Common logic for any Ownable contract to transfer ownership
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
        Contract::EspToken | Contract::EspTokenProxy => {
            tracing::info!(%addr, %new_owner, "Transfer EspToken ownership");
            let token = EspToken::new(addr, &provider);
            token
                .transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::StakeTable | Contract::StakeTableProxy => {
            tracing::info!(%addr, %new_owner, "Transfer StakeTable ownership");
            let stake_table = StakeTable::new(addr, &provider);
            stake_table
                .transferOwnership(new_owner)
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
    use alloy::{primitives::utils::parse_units, providers::ProviderBuilder, sol_types::SolValue};
    use hotshot::rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::test_utils::setup_test;

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
    async fn test_deploy_mock_light_client_proxy() -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // prepare `initialize()` input
        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();
        let admin = provider.get_accounts().await?[0];
        let prover = admin;

        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            true, // is_mock = true
            genesis_state.clone(),
            genesis_stake.clone(),
            admin,
            Some(prover),
        )
        .await?;

        // check initialization is correct
        let lc = LightClientMock::new(lc_proxy_addr, &provider);
        let finalized_state: LightClientStateSol = lc.finalizedState().call().await?.into();
        assert_eq!(
            genesis_state.abi_encode_params(),
            finalized_state.abi_encode_params()
        );
        // mock set the state
        let new_state = LightClientStateSol {
            viewNum: 10,
            blockHeight: 10,
            blockCommRoot: U256::from(42),
        };
        lc.setFinalizedState(new_state.clone().into())
            .send()
            .await?
            .watch()
            .await?;
        let finalized_state: LightClientStateSol = lc.finalizedState().call().await?.into();
        assert_eq!(
            new_state.abi_encode_params(),
            finalized_state.abi_encode_params()
        );

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

    async fn test_upgrade_light_client_to_v2_helper(is_mock: bool) -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let blocks_per_epoch = 3; // for test

        // prepare `initialize()` input
        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();
        let admin = provider.get_accounts().await?[0];
        let prover = Address::random();

        // deploy proxy and V1
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

        // then upgrade to v2
        upgrade_light_client_v2(&provider, &mut contracts, is_mock, blocks_per_epoch).await?;

        // test correct v1 state persistence
        let lc = LightClientV2::new(lc_proxy_addr, &provider);
        let finalized_state: LightClientStateSol = lc.finalizedState().call().await?.into();
        assert_eq!(
            genesis_state.abi_encode_params(),
            finalized_state.abi_encode_params()
        );
        // test new v2 state
        let next_stake: StakeTableStateSol = lc.votingStakeTableState().call().await?.into();
        assert_eq!(
            genesis_stake.abi_encode_params(),
            next_stake.abi_encode_params()
        );
        assert_eq!(lc.getVersion().call().await?.majorVersion, 2);
        assert_eq!(lc._blocksPerEpoch().call().await?._0, blocks_per_epoch);

        // test mock-specific functions
        if is_mock {
            // recast to mock
            let lc_mock = LightClientV2Mock::new(lc_proxy_addr, &provider);
            let new_blocks_per_epoch = blocks_per_epoch + 10;
            lc_mock
                .setBlocksPerEpoch(new_blocks_per_epoch)
                .send()
                .await?
                .watch()
                .await?;
            assert_eq!(
                new_blocks_per_epoch,
                lc_mock._blocksPerEpoch().call().await?._0
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_upgrade_light_client_to_v2() -> Result<()> {
        test_upgrade_light_client_to_v2_helper(false).await
    }

    #[tokio::test]
    async fn test_upgrade_mock_light_client_v2() -> Result<()> {
        test_upgrade_light_client_to_v2_helper(true).await
    }

    #[tokio::test]
    async fn test_deploy_token_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        let init_recipient = provider.get_accounts().await?[0];
        let rand_owner = Address::random();

        let addr =
            deploy_token_proxy(&provider, &mut contracts, rand_owner, init_recipient).await?;
        let token = EspToken::new(addr, &provider);

        assert_eq!(token.owner().call().await?._0, rand_owner);
        assert_eq!(
            token.balanceOf(init_recipient).call().await?._0,
            parse_units("1000000000", "ether").unwrap().into(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_stake_table_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // deploy token
        let init_recipient = provider.get_accounts().await?[0];
        let token_owner = Address::random();
        let token_addr =
            deploy_token_proxy(&provider, &mut contracts, token_owner, init_recipient).await?;

        // deploy light client
        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;

        // deploy stake table
        let exit_escrow_period = U256::from(1000);
        let owner = init_recipient;
        let stake_table_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            exit_escrow_period,
            owner,
        )
        .await?;
        let stake_table = StakeTable::new(stake_table_addr, &provider);

        assert_eq!(
            stake_table.exitEscrowPeriod().call().await?._0,
            exit_escrow_period
        );
        assert_eq!(stake_table.owner().call().await?._0, owner);
        assert_eq!(stake_table.token().call().await?._0, token_addr);
        assert_eq!(stake_table.lightClient().call().await?._0, lc_addr);
        Ok(())
    }
}
