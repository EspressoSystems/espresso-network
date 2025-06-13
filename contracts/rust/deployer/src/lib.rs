use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use alloy::{
    contract::RawCallBuilder,
    hex::{FromHex, ToHexExt},
    network::{Ethereum, EthereumWallet, TransactionBuilder},
    primitives::{Address, Bytes, B256, U256},
    providers::{
        fillers::{FillProvider, JoinFill, WalletFiller},
        utils::JoinedRecommendedFillers,
        Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    signers::local::{coins_bip39::English, MnemonicBuilder, PrivateKeySigner},
    transports::http::reqwest::Url,
};
use anyhow::{anyhow, Context, Result};
use clap::{builder::OsStr, Parser};
use derive_more::{derive::Deref, Display};
use hotshot_contract_adapter::sol_types::*;

pub mod builder;
pub mod network_config;

/// Type alias that connects to providers with recommended fillers and wallet
/// use `<HttpProviderWithWallet as WalletProvider>::wallet()` to access internal wallet
/// use `<HttpProviderWithWallet as WalletProvider>::default_signer_address(&provider)` to get wallet address
pub type HttpProviderWithWallet = FillProvider<
    JoinFill<JoinedRecommendedFillers, WalletFiller<EthereumWallet>>,
    RootProvider,
    Ethereum,
>;

/// a handy thin wrapper around wallet builder and provider builder that directly
/// returns an instantiated `Provider` with default fillers with wallet, ready to send tx
pub fn build_provider(mnemonic: String, account_index: u32, url: Url) -> HttpProviderWithWallet {
    let signer = build_signer(mnemonic, account_index);
    let wallet = EthereumWallet::from(signer);
    ProviderBuilder::new().wallet(wallet).on_http(url)
}

pub fn build_signer(mnemonic: String, account_index: u32) -> PrivateKeySigner {
    MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(account_index)
        .expect("wrong mnemonic or index")
        .build()
        .expect("fail to build signer")
}

/// similar to [`build_provider()`] but using a random wallet
pub fn build_random_provider(url: Url) -> HttpProviderWithWallet {
    let signer = MnemonicBuilder::<English>::default()
        .build_random()
        .expect("fail to build signer");
    let wallet = EthereumWallet::from(signer);
    ProviderBuilder::new().wallet(wallet).on_http(url)
}

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
    /// Timelock.sol
    #[clap(long, env = Contract::Timelock)]
    timelock: Option<Address>,
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

    /// Use an already-deployed EspToken.sol instead of deploying a new one.
    #[clap(long, env = Contract::EspToken)]
    esp_token: Option<Address>,

    /// Use an already-deployed EspToken.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::EspTokenProxy)]
    esp_token_proxy: Option<Address>,

    /// Use an already-deployed StakeTable.sol instead of deploying a new one.
    #[clap(long, env = Contract::StakeTable)]
    stake_table: Option<Address>,

    /// Use an already-deployed StakeTableV2.sol instead of deploying a new one.
    #[clap(long, env = Contract::StakeTableV2)]
    stake_table_v2: Option<Address>,

    /// Use an already-deployed StakeTable.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::StakeTableProxy)]
    stake_table_proxy: Option<Address>,
}

/// An identifier for a particular contract.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash)]
pub enum Contract {
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_ADDRESS")]
    PlonkVerifier,
    #[display("ESPRESSO_SEQUENCER_TIMELOCK_CONTROLLER_ADDRESS")]
    Timelock,
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
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_ADDRESS")]
    EspToken,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS")]
    EspTokenProxy,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_ADDRESS")]
    StakeTable,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_V2_ADDRESS")]
    StakeTableV2,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS")]
    StakeTableProxy,
}

impl From<Contract> for OsStr {
    fn from(c: Contract) -> OsStr {
        c.to_string().into()
    }
}

/// Cache of contracts predeployed or deployed during this current run.
#[derive(Deref, Debug, Clone, Default)]
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
        if let Some(addr) = deployed.timelock {
            m.insert(Contract::Timelock, addr);
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
        if let Some(addr) = deployed.esp_token {
            m.insert(Contract::EspToken, addr);
        }
        if let Some(addr) = deployed.esp_token_proxy {
            m.insert(Contract::EspTokenProxy, addr);
        }
        if let Some(addr) = deployed.stake_table {
            m.insert(Contract::StakeTable, addr);
        }
        if let Some(addr) = deployed.stake_table_v2 {
            m.insert(Contract::StakeTableV2, addr);
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

/// Verify the node js files are present and can be executed.
///
/// It calls the upgrade proxy script with a dummy address and a dummy rpc url in dry run mode
pub async fn verify_node_js_files() -> Result<()> {
    call_upgrade_proxy_script(
        Address::random(),
        Address::random(),
        String::from("0x"),
        String::from("https://sepolia.infura.io/v3/"),
        Address::random(),
        Some(true),
    )
    .await?;
    Ok(())
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

    assert!(is_contract(&provider, plonk_verifier_addr).await?);

    // when generate alloy's bindings, we supply a placeholder address, now we modify the actual
    // bytecode with deployed address of the library.
    let target_lc_bytecode = if mock {
        LightClientMock::BYTECODE.encode_hex()
    } else {
        LightClient::BYTECODE.encode_hex()
    };
    let lc_linked_bytecode = {
        match target_lc_bytecode
            .matches(LIBRARY_PLACEHOLDER_ADDRESS)
            .count()
        {
            0 => return Err(anyhow!("lib placeholder not found")),
            1 => Bytes::from_hex(target_lc_bytecode.replacen(
                LIBRARY_PLACEHOLDER_ADDRESS,
                &plonk_verifier_addr.encode_hex(),
                1,
            ))?,
            _ => {
                return Err(anyhow!(
                    "more than one lib placeholder found, consider using a different value"
                ))
            },
        }
    };

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

    // post deploy verification checks
    assert_eq!(lc_proxy.getVersion().call().await?.majorVersion, 1);
    assert_eq!(lc_proxy.owner().call().await?._0, admin);
    if let Some(prover) = prover {
        assert_eq!(lc_proxy.permissionedProver().call().await?._0, prover);
    }
    assert_eq!(
        lc_proxy.stateHistoryRetentionPeriod().call().await?._0,
        864000
    );
    assert_eq!(
        lc_proxy.currentBlockNumber().call().await?._0,
        U256::from(provider.get_block_number().await?)
    );

    Ok(lc_proxy_addr)
}

/// Upgrade the light client proxy to use LightClientV2.
/// Internally, first detect existence of proxy, then deploy LCV2, then upgrade and initializeV2.
/// Internal to "deploy LCV2", we deploy PlonkVerifierV2 whose address will be used at LCV2 init time.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a regular EOA, not a multisig.
/// - the proxy is not yet initialized for V2
pub async fn upgrade_light_client_v2(
    provider: impl Provider,
    contracts: &mut Contracts,
    is_mock: bool,
    blocks_per_epoch: u64,
    epoch_start_block: u64,
) -> Result<TransactionReceipt> {
    match contracts.address(Contract::LightClientProxy) {
        // check if proxy already exists
        None => Err(anyhow!("LightClientProxy not found, can't upgrade")),
        Some(proxy_addr) => {
            let proxy = LightClient::new(proxy_addr, &provider);
            let state_history_retention_period =
                proxy.stateHistoryRetentionPeriod().call().await?._0;
            // first deploy PlonkVerifierV2.sol
            let pv2_addr = contracts
                .deploy(
                    Contract::PlonkVerifierV2,
                    PlonkVerifierV2::deploy_builder(&provider),
                )
                .await?;

            assert!(is_contract(&provider, pv2_addr).await?);

            // then deploy LightClientV2.sol
            let target_lcv2_bytecode = if is_mock {
                LightClientV2Mock::BYTECODE.encode_hex()
            } else {
                LightClientV2::BYTECODE.encode_hex()
            };
            let lcv2_linked_bytecode = {
                match target_lcv2_bytecode
                    .matches(LIBRARY_PLACEHOLDER_ADDRESS)
                    .count()
                {
                    0 => return Err(anyhow!("lib placeholder not found")),
                    1 => Bytes::from_hex(target_lcv2_bytecode.replacen(
                        LIBRARY_PLACEHOLDER_ADDRESS,
                        &pv2_addr.encode_hex(),
                        1,
                    ))?,
                    _ => {
                        return Err(anyhow!(
                            "more than one lib placeholder found, consider using a different value"
                        ))
                    },
                }
            };
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
            let init_data = lcv2
                .initializeV2(blocks_per_epoch, epoch_start_block)
                .calldata()
                .to_owned();
            // invoke upgrade on proxy
            let receipt = proxy
                .upgradeToAndCall(lcv2_addr, init_data)
                .send()
                .await?
                .get_receipt()
                .await?;
            if receipt.inner.is_success() {
                // post deploy verification checks
                let proxy_as_v2 = LightClientV2::new(proxy_addr, &provider);
                assert_eq!(proxy_as_v2.getVersion().call().await?.majorVersion, 2);
                assert_eq!(
                    proxy_as_v2.blocksPerEpoch().call().await?._0,
                    blocks_per_epoch
                );
                assert_eq!(
                    proxy_as_v2.epochStartBlock().call().await?._0,
                    epoch_start_block
                );
                assert_eq!(
                    proxy_as_v2.stateHistoryRetentionPeriod().call().await?._0,
                    state_history_retention_period
                );
                assert_eq!(
                    proxy_as_v2.currentBlockNumber().call().await?._0,
                    U256::from(provider.get_block_number().await?)
                );

                tracing::info!(%lcv2_addr, "LightClientProxy successfully upgrade to: ")
            } else {
                tracing::error!("LightClientProxy upgrade failed: {:?}", receipt);
            }

            Ok(receipt)
        },
    }
}

pub struct LightClientV2UpgradeParams {
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

/// Upgrade the light client proxy to use LightClientV2.
/// Internally, first detect existence of proxy, then deploy LCV2, then upgrade and initializeV2.
/// Internal to "deploy LCV2", we deploy PlonkVerifierV2 whose address will be used at LCV2 init time.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
/// - the proxy is not yet initialized for V2
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_light_client_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    params: LightClientV2UpgradeParams,
    is_mock: bool,
    rpc_url: String,
    dry_run: Option<bool>,
) -> Result<(String, bool)> {
    let expected_major_version: u8 = 2;
    let dry_run = dry_run.unwrap_or_else(|| {
        tracing::warn!("Dry run not specified, defaulting to false");
        false
    });

    let proxy_addr = contracts
        .address(Contract::LightClientProxy)
        .ok_or_else(|| anyhow!("LightClientProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("LightClientProxy found at {proxy_addr:#x}");
    let proxy = LightClient::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?._0;

    if !dry_run {
        tracing::info!("Checking if owner is a contract");
        assert!(
            is_contract(&provider, owner_addr).await?,
            "Owner is not a contract so not a multisig wallet"
        );
    }

    // Prepare addresses
    let (_pv2_addr, lcv2_addr) = if !dry_run {
        // Deploy PlonkVerifierV2.sol (if not already deployed)
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
        let lcv2_linked_bytecode = {
            match target_lcv2_bytecode
                .matches(LIBRARY_PLACEHOLDER_ADDRESS)
                .count()
            {
                0 => return Err(anyhow!("lib placeholder not found")),
                1 => Bytes::from_hex(target_lcv2_bytecode.replacen(
                    LIBRARY_PLACEHOLDER_ADDRESS,
                    &pv2_addr.encode_hex(),
                    1,
                ))?,
                _ => {
                    return Err(anyhow!(
                        "more than one lib placeholder found, consider using a different value"
                    ))
                },
            }
        };
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
        (pv2_addr, lcv2_addr)
    } else {
        // Use dummy addresses for dry run
        (Address::random(), Address::random())
    };

    // Prepare init data
    let init_data = if already_initialized(
        &provider,
        proxy_addr,
        Contract::LightClientV2,
        expected_major_version,
    )
    .await?
    {
        tracing::info!(
            "Proxy was already initialized for version {}",
            expected_major_version
        );
        vec![].into()
    } else {
        tracing::info!("Init Data to be signed.\n Function: initializeV2\n Arguments:\n blocks_per_epoch: {:?}\n epoch_start_block: {:?}", params.blocks_per_epoch, params.epoch_start_block);
        LightClientV2::new(lcv2_addr, &provider)
            .initializeV2(params.blocks_per_epoch, params.epoch_start_block)
            .calldata()
            .to_owned()
    };

    // invoke upgrade on proxy via the safeSDK
    let result = call_upgrade_proxy_script(
        proxy_addr,
        lcv2_addr,
        init_data.to_string(),
        rpc_url,
        owner_addr,
        Some(dry_run),
    )
    .await?;

    tracing::info!("Init data: {:?}", init_data);
    if init_data.to_string() != "0x" {
        tracing::info!("Data to be signed:\n Function: initializeV2\n Arguments:\n blocks_per_epoch: {:?}\n epoch_start_block: {:?}", params.blocks_per_epoch, params.epoch_start_block);
    }
    if !dry_run {
        tracing::info!(
            "LightClientProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}",
            owner_addr
        );
    }
    // IDEA: add a function to wait for the proposal to be executed

    Ok(result)
}

async fn already_initialized(
    provider: impl Provider,
    proxy_addr: Address,
    contract: Contract,
    expected_major_version: u8,
) -> Result<bool> {
    let initialized = get_proxy_initialized_version(&provider, proxy_addr).await?;
    tracing::info!("Initialized version: {}", initialized);

    let contract_major_version = match contract {
        Contract::LightClientV2 => {
            let contract_proxy = LightClientV2::new(proxy_addr, &provider);
            contract_proxy.getVersion().call().await?.majorVersion
        },
        Contract::StakeTableV2 => {
            let contract_proxy = StakeTableV2::new(proxy_addr, &provider);
            contract_proxy.getVersion().call().await?.majorVersion
        },
        _ => anyhow::bail!("Unsupported contract type for already_initialized"),
    };

    Ok(initialized == contract_major_version && contract_major_version == expected_major_version)
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

    // post deploy verification checks
    let fee_proxy = FeeContract::new(fee_proxy_addr, &provider);
    assert_eq!(fee_proxy.getVersion().call().await?.majorVersion, 1);
    assert_eq!(fee_proxy.owner().call().await?._0, admin);

    Ok(fee_proxy_addr)
}

/// The primary logic for deploying and initializing an upgradable Espresso Token contract.
pub async fn deploy_token_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    owner: Address,
    init_grant_recipient: Address,
    initial_supply: U256,
    name: &str,
    symbol: &str,
) -> Result<Address> {
    let token_addr = contracts
        .deploy(Contract::EspToken, EspToken::deploy_builder(&provider))
        .await?;
    let token = EspToken::new(token_addr, &provider);

    let init_data = token
        .initialize(
            owner,
            init_grant_recipient,
            initial_supply,
            name.to_string(),
            symbol.to_string(),
        )
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

    // post deploy verification checks
    let token_proxy = EspToken::new(token_proxy_addr, &provider);
    assert_eq!(token_proxy.getVersion().call().await?.majorVersion, 1);
    assert_eq!(token_proxy.owner().call().await?._0, owner);
    assert_eq!(token_proxy.symbol().call().await?._0, symbol);
    assert_eq!(token_proxy.decimals().call().await?._0, 18);
    assert_eq!(token_proxy.name().call().await?._0, name);
    let total_supply = token_proxy.totalSupply().call().await?._0;
    assert_eq!(
        token_proxy.balanceOf(init_grant_recipient).call().await?._0,
        total_supply
    );

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

    // TODO: verify the light client address contains a contract
    // See #3163, it's a cyclic dependency in the demo environment
    // assert!(is_contract(&provider, light_client_addr).await?);

    // verify the token address contains a contract
    assert!(is_contract(&provider, token_addr).await?);

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

    let st_proxy = StakeTable::new(st_proxy_addr, &provider);
    assert_eq!(st_proxy.getVersion().call().await?.majorVersion, 1);
    assert_eq!(st_proxy.owner().call().await?._0, owner);
    assert_eq!(st_proxy.token().call().await?._0, token_addr);
    assert_eq!(st_proxy.lightClient().call().await?._0, light_client_addr);
    assert_eq!(
        st_proxy.exitEscrowPeriod().call().await?._0,
        exit_escrow_period
    );

    Ok(st_proxy_addr)
}

/// Upgrade the stake table proxy to use StakeTableV2.
async fn upgrade_stake_table_v2(
    provider: impl Provider,
    contracts: &mut Contracts,
    pauser: Address,
    admin: Address,
) -> Result<TransactionReceipt> {
    let Some(proxy_addr) = contracts.address(Contract::StakeTableProxy) else {
        anyhow::bail!("StakeTableProxy not found, can't upgrade")
    };

    let proxy = StakeTable::new(proxy_addr, &provider);
    // Deploy the new implementation
    let v2_addr = contracts
        .deploy(
            Contract::StakeTableV2,
            StakeTableV2::deploy_builder(&provider),
        )
        .await?;

    assert!(is_contract(&provider, v2_addr).await?);

    // Prepare init data
    let expected_major_version = 2;
    let init_data = if already_initialized(
        &provider,
        proxy_addr,
        Contract::StakeTableV2,
        expected_major_version,
    )
    .await?
    {
        tracing::info!(
            "Proxy was already initialized for version {}",
            expected_major_version
        );
        vec![].into()
    } else {
        tracing::info!("Init Data to be signed.\n Function: initializeV2\n Arguments:\n pauser: {:?}\n admin: {:?}", pauser, admin);
        StakeTableV2::new(v2_addr, &provider)
            .initializeV2(pauser, admin)
            .calldata()
            .to_owned()
    };

    // invoke upgrade on proxy
    let receipt = proxy
        .upgradeToAndCall(v2_addr, init_data)
        .send()
        .await?
        .get_receipt()
        .await?;

    if receipt.inner.is_success() {
        // post deploy verification checks
        let proxy_as_v2 = StakeTableV2::new(proxy_addr, &provider);
        assert_eq!(proxy_as_v2.getVersion().call().await?.majorVersion, 2);
        // get pauser role
        let pauser_role = proxy_as_v2.PAUSER_ROLE().call().await?._0;
        assert!(proxy_as_v2.hasRole(pauser_role, pauser).call().await?._0,);
        // get admin role
        let admin_role = proxy_as_v2.DEFAULT_ADMIN_ROLE().call().await?._0;
        assert!(proxy_as_v2.hasRole(admin_role, admin).call().await?._0,);
        tracing::info!(%v2_addr, "StakeTable successfully upgraded to")
    } else {
        anyhow::bail!("StakeTable upgrade failed: {:?}", receipt);
    }

    Ok(receipt)
}

/// Upgrade the stake table proxy to use StakeTableV2.
/// Internally, first detect existence of proxy, then deploy StakeTableV2
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_stake_table_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    rpc_url: String,
    multisig_address: Address,
    pauser: Address,
    dry_run: Option<bool>,
) -> Result<(String, bool)> {
    let dry_run = dry_run.unwrap_or(false);
    match contracts.address(Contract::StakeTableProxy) {
        // check if proxy already exists
        None => Err(anyhow!("StakeTableProxy not found, can't upgrade")),
        Some(proxy_addr) => {
            let proxy = StakeTable::new(proxy_addr, &provider);
            let owner = proxy.owner().call().await?;
            let owner_addr = owner._0;
            assert_eq!(
                owner_addr, multisig_address,
                "Proxy is not owned by the multisig"
            );
            // TODO: check if owner is a multisig
            // first deploy StakeTableV2.sol implementation
            let stake_table_v2_addr = contracts
                .deploy(
                    Contract::StakeTableV2,
                    StakeTableV2::deploy_builder(&provider),
                )
                .await?;

            // prepare init data
            let expected_major_version = 2;
            let init_data = if already_initialized(
                &provider,
                proxy_addr,
                Contract::StakeTableV2,
                expected_major_version,
            )
            .await?
            {
                tracing::info!(
                    "Proxy was already initialized for version {}",
                    expected_major_version
                );
                vec![].into()
            } else {
                tracing::info!("Init Data to be signed.\n Function: initializeV2\n Arguments:\n pauser: {:?}\n admin: {:?}", pauser, owner_addr);
                StakeTableV2::new(stake_table_v2_addr, &provider)
                    .initializeV2(pauser, owner_addr)
                    .calldata()
                    .to_owned()
            };

            // invoke upgrade on proxy via the safeSDK
            let result = call_upgrade_proxy_script(
                proxy_addr,
                stake_table_v2_addr,
                init_data.to_string(),
                rpc_url,
                owner_addr,
                Some(dry_run),
            )
            .await;

            // Check the result directly
            if let Err(ref err) = result {
                tracing::error!("StakeTableProxy upgrade failed: {:?}", err);
            } else {
                tracing::info!("StakeTableProxy upgrade proposal sent");
                // IDEA: add a function to wait for the proposal to be executed
            }
            Ok(result.context("Upgrade proposal failed")?)
        },
    }
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

pub async fn is_contract(provider: impl Provider, address: Address) -> Result<bool> {
    if address == Address::ZERO {
        return Ok(false);
    }

    let code = provider.get_code_at(address).await?;
    if code.is_empty() {
        return Ok(false);
    }

    Ok(true)
}

pub async fn get_proxy_initialized_version(
    provider: impl Provider,
    proxy_addr: Address,
) -> Result<u8> {
    // From openzeppelin Initializable.sol, the initialized version slot is keccak256("openzeppelin.storage.Initializable");
    let slot: B256 = "0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00"
        .parse()
        .unwrap();
    let value = provider.get_storage_at(proxy_addr, slot.into()).await?;
    let initialized = value.as_le_bytes()[0]; // `_initialized` is u8 stored in the last byte
    Ok(initialized)
}

/// this depends on upgradeProxy.ts which has to be ran on a real network supported by the safeSDK
pub async fn call_upgrade_proxy_script(
    proxy_addr: Address,
    new_impl_addr: Address,
    init_data: String,
    rpc_url: String,
    safe_addr: Address,
    dry_run: Option<bool>,
) -> Result<(String, bool), anyhow::Error> {
    let dry_run = dry_run.unwrap_or(false);
    tracing::info!("Dry run: {}", dry_run);
    tracing::info!(
        "Attempting to send the upgrade proposal to multisig: {}",
        safe_addr
    );

    let script_path = find_script_path()?;

    let output = Command::new(script_path)
        .arg("--from-rust")
        .arg("--proxy")
        .arg(proxy_addr.to_string())
        .arg("--impl")
        .arg(new_impl_addr.to_string())
        .arg("--init-data")
        .arg(init_data)
        .arg("--rpc-url")
        .arg(rpc_url)
        .arg("--safe-address")
        .arg(safe_addr.to_string())
        .arg("--dry-run")
        .arg(dry_run.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = output.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // if stderr is not empty, return the stderr
    if !output.status.success() {
        return Err(anyhow!("Upgrade script failed: {}", stderr));
    }
    Ok((stdout.to_string(), true))
}

fn find_script_path() -> Result<PathBuf> {
    let mut path_options = Vec::new();
    if let Ok(cargo_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        path_options.push(
            PathBuf::from(cargo_manifest_dir.clone())
                .join("../../../scripts/multisig-upgrade-entrypoint"),
        );
        path_options
            .push(PathBuf::from(cargo_manifest_dir).join("../scripts/multisig-upgrade-entrypoint"));
    }
    path_options.push(PathBuf::from("/bin/multisig-upgrade-entrypoint"));
    for path in path_options {
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Upgrade entrypoint script, multisig-upgrade-entrypoint, not found in any of the possible locations");
}

/// Deploy and initialize a Timelock contract
///
/// Parameters:
/// - `min_delay`: The minimum delay for operations
/// - `proposers`: The list of addresses that can propose
/// - `executors`: The list of addresses that can execute
/// - `admin`: The address that can perform admin actions
pub async fn deploy_timelock(
    provider: impl Provider,
    contracts: &mut Contracts,
    min_delay: U256,
    proposers: Vec<Address>,
    executors: Vec<Address>,
    admin: Address,
) -> Result<Address> {
    let timelock_addr = contracts
        .deploy(
            Contract::Timelock,
            Timelock::deploy_builder(
                &provider,
                min_delay,
                proposers.clone(),
                executors.clone(),
                admin,
            ),
        )
        .await?;

    // Verify deployment
    let timelock = Timelock::new(timelock_addr, &provider);

    // Verify initialization parameters
    assert_eq!(timelock.getMinDelay().call().await?._0, min_delay);
    assert!(
        timelock
            .hasRole(timelock.PROPOSER_ROLE().call().await?._0, proposers[0])
            .call()
            .await?
            ._0
    );
    assert!(
        timelock
            .hasRole(timelock.EXECUTOR_ROLE().call().await?._0, executors[0])
            .call()
            .await?
            ._0
    );

    // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
    let default_admin_role = U256::ZERO;
    assert!(
        timelock
            .hasRole(default_admin_role.into(), admin)
            .call()
            .await?
            ._0
    );

    Ok(timelock_addr)
}

#[cfg(test)]
mod tests {
    use alloy::{
        node_bindings::Anvil, primitives::utils::parse_ether, providers::ProviderBuilder,
        sol_types::SolValue,
    };
    use sequencer_utils::test_utils::setup_test;

    use super::*;

    #[tokio::test]
    async fn test_is_contract() -> Result<(), anyhow::Error> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();

        // test with zero address returns false
        let zero_address = Address::ZERO;
        assert!(!is_contract(&provider, zero_address).await?);

        // Test with a non-contract address (e.g., a random address)
        let random_address = Address::random();
        assert!(!is_contract(&provider, random_address).await?);

        // Deploy a contract and test with its address
        let fee_contract = FeeContract::deploy(&provider).await?;
        let contract_address = *fee_contract.address();
        assert!(is_contract(&provider, contract_address).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_is_proxy_contract() -> Result<()> {
        setup_test();
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
        setup_test();
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
        setup_test();
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
        setup_test();
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

    async fn test_upgrade_light_client_to_v2_helper(is_mock: bool) -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let blocks_per_epoch = 10; // for test
        let epoch_start_block = 22;

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

        let state_history_retention_period = LightClient::new(lc_proxy_addr, &provider)
            .stateHistoryRetentionPeriod()
            .call()
            .await?
            ._0;

        // then upgrade to v2
        upgrade_light_client_v2(
            &provider,
            &mut contracts,
            is_mock,
            blocks_per_epoch,
            epoch_start_block,
        )
        .await?;

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
        assert_eq!(lc.blocksPerEpoch().call().await?._0, blocks_per_epoch);
        assert_eq!(lc.epochStartBlock().call().await?._0, epoch_start_block);
        assert_eq!(
            lc.stateHistoryRetentionPeriod().call().await?._0,
            state_history_retention_period
        );

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
                lc_mock.blocksPerEpoch().call().await?._0
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_upgrade_light_client_to_v2() -> Result<()> {
        setup_test();
        test_upgrade_light_client_to_v2_helper(false).await
    }

    #[tokio::test]
    async fn test_upgrade_mock_light_client_v2() -> Result<()> {
        setup_test();
        test_upgrade_light_client_to_v2_helper(true).await
    }
    #[derive(Debug, Clone, Copy)]
    pub enum RunMode {
        DryRun,
        RealRun,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum UpgradeCount {
        Once,
        Twice,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct UpgradeTestOptions {
        pub is_mock: bool,
        pub run_mode: RunMode,
        pub upgrade_count: UpgradeCount,
    }
    // This test is used to test the upgrade of the LightClientProxy via the multisig wallet
    // It only tests the upgrade proposal via the typescript script and thus requires the upgrade proposal to be sent to a real network
    // However, the contracts are deployed on anvil, so the test will pass even if the upgrade proposal is not executed
    // The test assumes that there is a file .env.deployer.rs.test in the root directory:
    // Ensure that the private key has proposal rights on the Safe Multisig Wallet and the SDK supports the network
    async fn test_upgrade_light_client_to_v2_multisig_owner_helper(
        options: UpgradeTestOptions,
    ) -> Result<()> {
        let mut sepolia_rpc_url = "http://localhost:8545".to_string();
        let mut multisig_admin = Address::random();
        let mut mnemonic =
            "test test test test test test test test test test test junk".to_string();
        let mut account_index = 0;
        let anvil = Anvil::default().spawn();
        let dry_run = matches!(options.run_mode, RunMode::DryRun);
        if !dry_run {
            dotenvy::from_filename_override(".env.deployer.rs.test").ok();

            for item in dotenvy::from_filename_iter(".env.deployer.rs.test")
                .expect("Failed to read .env.deployer.rs.test")
            {
                let (key, val) = item?;
                if key == "ESPRESSO_SEQUENCER_L1_PROVIDER" {
                    sepolia_rpc_url = val.to_string();
                } else if key == "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS" {
                    multisig_admin = val.parse::<Address>()?;
                } else if key == "ESPRESSO_SEQUENCER_ETH_MNEMONIC" {
                    mnemonic = val.to_string();
                } else if key == "ESPRESSO_DEPLOYER_ACCOUNT_INDEX" {
                    account_index = val.parse::<u32>()?;
                }
            }

            if sepolia_rpc_url.is_empty() || multisig_admin.is_zero() {
                panic!("ESPRESSO_SEQUENCER_L1_PROVIDER and ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS must be set in .env.deployer.rs.test");
            }
        }

        let mut contracts = Contracts::new();
        let blocks_per_epoch = 10; // for test
        let epoch_start_block = 22;
        let admin_signer = MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .index(account_index)
            .expect("wrong mnemonic or index")
            .build()?;
        let admin = admin_signer.address();
        let provider = if !dry_run {
            ProviderBuilder::new()
                .wallet(admin_signer)
                .connect(&sepolia_rpc_url)
                .await?
        } else {
            ProviderBuilder::new()
                .wallet(admin_signer)
                .connect(&anvil.endpoint())
                .await?
        };

        // prepare `initialize()` input
        let genesis_state = LightClientStateSol::dummy_genesis();
        let genesis_stake = StakeTableStateSol::dummy_genesis();

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
        if matches!(options.upgrade_count, UpgradeCount::Twice) {
            // upgrade to v2
            upgrade_light_client_v2(
                &provider,
                &mut contracts,
                options.is_mock,
                blocks_per_epoch,
                epoch_start_block,
            )
            .await?;
        }

        // transfer ownership to multisig
        let _receipt = transfer_ownership(
            &provider,
            Contract::LightClientProxy,
            lc_proxy_addr,
            multisig_admin,
        )
        .await?;
        let lc = LightClient::new(lc_proxy_addr, &provider);
        assert_eq!(lc.owner().call().await?._0, multisig_admin);

        // then send upgrade proposal to the multisig wallet
        let (result, success) = upgrade_light_client_v2_multisig_owner(
            &provider,
            &mut contracts,
            LightClientV2UpgradeParams {
                blocks_per_epoch,
                epoch_start_block,
            },
            options.is_mock,
            sepolia_rpc_url.clone(),
            Some(dry_run),
        )
        .await?;
        tracing::info!(
            "Result when trying to upgrade LightClientProxy via the multisig wallet: {:?}",
            result
        );
        assert!(success);
        if dry_run {
            let data: serde_json::Value = serde_json::from_str(&result)?;
            assert_eq!(data["rpcUrl"], sepolia_rpc_url);
            assert_eq!(data["safeAddress"], multisig_admin.to_string());
            assert_eq!(data["proxyAddress"], lc_proxy_addr.to_string());

            let expected_init_data = if matches!(options.upgrade_count, UpgradeCount::Twice) {
                "0x" // no init data for the second upgrade because the proxy was already initialized
            } else {
                &LightClientV2::new(lc_proxy_addr, &provider)
                    .initializeV2(blocks_per_epoch, epoch_start_block)
                    .calldata()
                    .to_owned()
                    .to_string()
            };

            assert_eq!(data["initData"], expected_init_data.to_string());
            assert_eq!(data["useHardwareWallet"], false);
        }
        // v1 state persistence cannot be tested here because the upgrade proposal is not yet executed
        // One has to test that the upgrade proposal is available via the Safe UI
        // and then test that the v1 state is persisted
        Ok(())
    }

    #[tokio::test]
    async fn test_upgrade_light_client_to_v2_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::DryRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    // We expect no init data for the second upgrade because the proxy was already initialized
    #[tokio::test]
    async fn test_upgrade_light_client_to_v2_twice_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::DryRun,
            upgrade_count: UpgradeCount::Twice,
        })
        .await
    }

    #[tokio::test]
    #[ignore]
    async fn test_upgrade_light_client_to_v2_multisig_owner_live_eth_network() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::RealRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    #[tokio::test]
    async fn test_deploy_token_proxy() -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        let init_recipient = provider.get_accounts().await?[0];
        let rand_owner = Address::random();
        let initial_supply = U256::from(3590000000u64);
        let name = "Espresso";
        let symbol = "ESP";

        let addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            rand_owner,
            init_recipient,
            initial_supply,
            name,
            symbol,
        )
        .await?;
        let token = EspToken::new(addr, &provider);

        assert_eq!(token.owner().call().await?._0, rand_owner);
        let total_supply = token.totalSupply().call().await?._0;
        assert_eq!(
            total_supply,
            parse_ether(&initial_supply.to_string()).unwrap()
        );
        assert_eq!(
            token.balanceOf(init_recipient).call().await?._0,
            total_supply,
        );
        assert_eq!(token.name().call().await?._0, name);
        assert_eq!(token.symbol().call().await?._0, symbol);

        Ok(())
    }

    #[tokio::test]
    async fn test_deploy_stake_table_proxy() -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // deploy token
        let init_recipient = provider.get_accounts().await?[0];
        let token_owner = Address::random();
        let token_name = "Espresso";
        let token_symbol = "ESP";
        let initial_supply = U256::from(3590000000u64);
        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            token_owner,
            init_recipient,
            initial_supply,
            token_name,
            token_symbol,
        )
        .await?;

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

    #[tokio::test]
    async fn test_upgrade_stake_table_v2() -> Result<()> {
        setup_test();
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // deploy token
        let init_recipient = provider.get_accounts().await?[0];
        let token_owner = Address::random();
        let token_name = "Espresso";
        let token_symbol = "ESP";
        let initial_supply = U256::from(3590000000u64);
        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            token_owner,
            init_recipient,
            initial_supply,
            token_name,
            token_symbol,
        )
        .await?;

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
        let stake_table_v1 = StakeTable::new(stake_table_addr, &provider);
        assert_eq!(stake_table_v1.getVersion().call().await?, (1, 0, 0).into());

        // upgrade to v2
        let pauser = Address::random();
        upgrade_stake_table_v2(&provider, &mut contracts, pauser, owner).await?;

        let stake_table_v2 = StakeTableV2::new(stake_table_addr, &provider);

        assert_eq!(stake_table_v2.getVersion().call().await?, (2, 0, 0).into());
        assert_eq!(stake_table_v2.owner().call().await?._0, owner);
        assert_eq!(stake_table_v2.token().call().await?._0, token_addr);
        assert_eq!(stake_table_v2.lightClient().call().await?._0, lc_addr);

        // get pauser role
        let pauser_role = stake_table_v2.PAUSER_ROLE().call().await?._0;
        assert!(stake_table_v2.hasRole(pauser_role, pauser).call().await?._0,);

        // get admin role
        let admin_role = stake_table_v2.DEFAULT_ADMIN_ROLE().call().await?._0;
        assert!(stake_table_v2.hasRole(admin_role, owner).call().await?._0,);
        Ok(())
    }

    // This test is used to test the upgrade of the StakeTableProxy via the multisig wallet
    // It only tests the upgrade proposal via the typescript script and thus requires the upgrade proposal to be sent to a real network
    // However, the contracts are deployed on anvil, so the test will pass even if the upgrade proposal is not executed
    // The test assumes that there is a file .env.deployer.rs.test in the root directory with the following variables:
    // RPC_URL=
    // SAFE_MULTISIG_ADDRESS=0x0000000000000000000000000000000000000000
    // SAFE_ORCHESTRATOR_PRIVATE_KEY=0x0000000000000000000000000000000000000000000000000000000000000000
    // Ensure that the private key has proposal rights on the Safe Multisig Wallet and the SDK supports the network
    async fn test_upgrade_stake_table_to_v2_multisig_owner_helper(dry_run: bool) -> Result<()> {
        let mut sepolia_rpc_url = "http://localhost:8545".to_string();
        let mut multisig_admin = Address::random();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let init_recipient = provider.get_accounts().await?[0];
        let token_owner = Address::random();
        let initial_supply = U256::from(3590000000u64);

        if !dry_run {
            dotenvy::from_filename_override(".env.deployer.rs.test").ok();

            for item in dotenvy::from_filename_iter(".env.deployer.rs.test")
                .expect("Failed to read .env.deployer.rs.test")
            {
                let (key, val) = item?;
                if key == "RPC_URL" {
                    sepolia_rpc_url = val.to_string();
                } else if key == "SAFE_MULTISIG_ADDRESS" {
                    multisig_admin = val.parse::<Address>()?;
                }
            }

            if sepolia_rpc_url.is_empty() || multisig_admin.is_zero() {
                panic!("RPC_URL and SAFE_MULTISIG_ADDRESS must be set in .env.deployer.rs.test");
            }
        }

        // deploy proxy and V1
        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            token_owner,
            init_recipient,
            initial_supply,
            "Espresso",
            "ESP",
        )
        .await?;
        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;
        let exit_escrow_period = U256::from(1000);
        let owner = init_recipient;
        let stake_table_proxy_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            exit_escrow_period,
            owner,
        )
        .await?;
        // transfer ownership to multisig
        let _receipt = transfer_ownership(
            &provider,
            Contract::StakeTableProxy,
            stake_table_proxy_addr,
            multisig_admin,
        )
        .await?;
        let stake_table = StakeTable::new(stake_table_proxy_addr, &provider);
        assert_eq!(stake_table.owner().call().await?._0, multisig_admin);
        // then send upgrade proposal to the multisig wallet
        let pauser = Address::random();
        let (result, success) = upgrade_stake_table_v2_multisig_owner(
            &provider,
            &mut contracts,
            sepolia_rpc_url,
            multisig_admin,
            pauser,
            Some(dry_run),
        )
        .await?;
        tracing::info!(
            "Result when trying to upgrade StakeTableProxy via the multisig wallet: {:?}",
            result
        );
        assert!(success);

        // v1 state persistence cannot be tested here because the upgrade proposal is not yet executed
        // One has to test that the upgrade proposal is available via the Safe UI
        // and then test that the v1 state is persisted
        Ok(())
    }

    #[tokio::test]
    async fn test_upgrade_stake_table_to_v2_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_stake_table_to_v2_multisig_owner_helper(true).await
    }

    #[tokio::test]
    async fn test_deploy_timelock() -> Result<()> {
        setup_test();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // Setup test parameters
        let min_delay = U256::from(86400); // 1 day in seconds
        let admin = provider.get_accounts().await?[0];
        let proposers = vec![Address::random()];
        let executors = vec![Address::random()];

        let timelock_addr = deploy_timelock(
            &provider,
            &mut contracts,
            min_delay,
            proposers.clone(),
            executors.clone(),
            admin,
        )
        .await?;

        // Verify deployment
        let timelock = Timelock::new(timelock_addr, &provider);
        assert_eq!(timelock.getMinDelay().call().await?._0, min_delay);

        // Verify initialization parameters
        assert_eq!(timelock.getMinDelay().call().await?._0, min_delay);
        assert!(
            timelock
                .hasRole(timelock.PROPOSER_ROLE().call().await?._0, proposers[0])
                .call()
                .await?
                ._0
        );
        assert!(
            timelock
                .hasRole(timelock.EXECUTOR_ROLE().call().await?._0, executors[0])
                .call()
                .await?
                ._0
        );

        // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
        let default_admin_role = U256::ZERO;
        assert!(
            timelock
                .hasRole(default_admin_role.into(), admin)
                .call()
                .await?
                ._0
        );
        Ok(())
    }
}
