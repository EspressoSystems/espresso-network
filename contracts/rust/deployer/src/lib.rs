use std::{collections::HashMap, io::Write, time::Duration};

use alloy::{
    contract::RawCallBuilder,
    dyn_abi::{DynSolType, DynSolValue, JsonAbiExt},
    hex::{FromHex, ToHexExt},
    json_abi::Function,
    network::{Ethereum, EthereumWallet, TransactionBuilder},
    primitives::{Address, Bytes, B256, U256},
    providers::{
        fillers::{FillProvider, JoinFill, WalletFiller},
        utils::JoinedRecommendedFillers,
        Provider, ProviderBuilder, RootProvider,
    },
    rpc::{client::RpcClient, types::TransactionReceipt},
    signers::{
        ledger::LedgerSigner,
        local::{coins_bip39::English, MnemonicBuilder, PrivateKeySigner},
    },
    transports::http::reqwest::Url,
};
use anyhow::{anyhow, Context, Result};
use clap::{builder::OsStr, Parser};
use derive_more::{derive::Deref, Display};
use hotshot_contract_adapter::sol_types::*;

pub mod builder;
pub mod impersonate_filler;
pub mod network_config;
pub mod proposals;
pub mod provider;

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
pub fn build_provider(
    mnemonic: impl AsRef<str>,
    account_index: u32,
    url: Url,
    poll_interval: Option<Duration>,
) -> HttpProviderWithWallet {
    let signer = build_signer(mnemonic.as_ref(), account_index);
    let wallet = EthereumWallet::from(signer);

    // alloy sets the polling interval automatically. It tries to guess if an RPC is local, but this
    // guess is wrong when the RPC is running inside docker. This results to 7 second polling
    // intervals on a chain with 1s block time. Therefore, allow overriding the polling interval
    // with a custom value.
    if let Some(interval) = poll_interval {
        tracing::info!("Using custom L1 poll interval: {interval:?}");
        let client = RpcClient::new_http(url.clone()).with_poll_interval(interval);
        ProviderBuilder::new().wallet(wallet).connect_client(client)
    } else {
        tracing::info!("Using default L1 poll interval");
        ProviderBuilder::new().wallet(wallet).connect_http(url)
    }
}

// TODO: tech-debt: provider creation logic should be refactored to handle mnemonic and
// ledger signers and consolidated with similar code in staking-cli
pub fn build_provider_ledger(
    signer: LedgerSigner,
    url: Url,
    poll_interval: Option<Duration>,
) -> HttpProviderWithWallet {
    let wallet = EthereumWallet::from(signer);

    // alloy sets the polling interval automatically. It tries to guess if an RPC is local, but this
    // guess is wrong when the RPC is running inside docker. This results to 7 second polling
    // intervals on a chain with 1s block time. Therefore, allow overriding the polling interval
    // with a custom value.
    if let Some(interval) = poll_interval {
        tracing::info!("Using custom L1 poll interval: {interval:?}");
        let client = RpcClient::new_http(url.clone()).with_poll_interval(interval);
        ProviderBuilder::new().wallet(wallet).connect_client(client)
    } else {
        tracing::info!("Using default L1 poll interval");
        ProviderBuilder::new().wallet(wallet).connect_http(url)
    }
}

pub fn build_signer(mnemonic: impl AsRef<str>, account_index: u32) -> PrivateKeySigner {
    MnemonicBuilder::<English>::default()
        .phrase(mnemonic.as_ref())
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
    ProviderBuilder::new().wallet(wallet).connect_http(url)
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
    /// OpsTimelock.sol
    #[clap(long, env = Contract::OpsTimelock)]
    ops_timelock: Option<Address>,
    /// SafeExitTimelock.sol
    #[clap(long, env = Contract::SafeExitTimelock)]
    safe_exit_timelock: Option<Address>,
    /// PlonkVerifierV2.sol
    #[clap(long, env = Contract::PlonkVerifierV2)]
    plonk_verifier_v2: Option<Address>,
    /// PlonkVerifierV3.sol
    #[clap(long, env = Contract::PlonkVerifierV3)]
    plonk_verifier_v3: Option<Address>,

    /// Use an already-deployed LightClient.sol instead of deploying a new one.
    #[clap(long, env = Contract::LightClient)]
    light_client: Option<Address>,
    /// LightClientV2.sol
    #[clap(long, env = Contract::LightClientV2)]
    light_client_v2: Option<Address>,
    /// LightClientV3.sol
    #[clap(long, env = Contract::LightClientV3)]
    light_client_v3: Option<Address>,

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

    /// Use an already-deployed EspTokenV2.sol instead of deploying a new one.
    #[clap(long, env = Contract::EspTokenV2)]
    esp_token_v2: Option<Address>,

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
    /// RewardClaim.sol
    #[clap(long, env = Contract::RewardClaim)]
    reward_claim: Option<Address>,
    /// Use an already-deployed RewardClaim.sol proxy instead of deploying a new one.
    #[clap(long, env = Contract::RewardClaimProxy)]
    reward_claim_proxy: Option<Address>,
}

/// An identifier for a particular contract.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash)]
pub enum Contract {
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_ADDRESS")]
    PlonkVerifier,
    #[display("ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS")]
    OpsTimelock,
    #[display("ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS")]
    SafeExitTimelock,
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_V2_ADDRESS")]
    PlonkVerifierV2,
    #[display("ESPRESSO_SEQUENCER_PLONK_VERIFIER_V3_ADDRESS")]
    PlonkVerifierV3,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_ADDRESS")]
    LightClient,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_V2_ADDRESS")]
    LightClientV2,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_V3_ADDRESS")]
    LightClientV3,
    #[display("ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")]
    LightClientProxy,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_ADDRESS")]
    FeeContract,
    #[display("ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS")]
    FeeContractProxy,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_ADDRESS")]
    EspToken,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_V2_ADDRESS")]
    EspTokenV2,
    #[display("ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS")]
    EspTokenProxy,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_ADDRESS")]
    StakeTable,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_V2_ADDRESS")]
    StakeTableV2,
    #[display("ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS")]
    StakeTableProxy,
    #[display("ESPRESSO_SEQUENCER_REWARD_CLAIM_ADDRESS")]
    RewardClaim,
    #[display("ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS")]
    RewardClaimProxy,
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
        if let Some(addr) = deployed.plonk_verifier_v3 {
            m.insert(Contract::PlonkVerifierV3, addr);
        }
        if let Some(addr) = deployed.safe_exit_timelock {
            m.insert(Contract::SafeExitTimelock, addr);
        }
        if let Some(addr) = deployed.ops_timelock {
            m.insert(Contract::OpsTimelock, addr);
        }
        if let Some(addr) = deployed.light_client {
            m.insert(Contract::LightClient, addr);
        }
        if let Some(addr) = deployed.light_client_v2 {
            m.insert(Contract::LightClientV2, addr);
        }
        if let Some(addr) = deployed.light_client_v3 {
            m.insert(Contract::LightClientV3, addr);
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
        if let Some(addr) = deployed.esp_token_v2 {
            m.insert(Contract::EspTokenV2, addr);
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
        if let Some(addr) = deployed.reward_claim {
            m.insert(Contract::RewardClaim, addr);
        }
        if let Some(addr) = deployed.reward_claim_proxy {
            m.insert(Contract::RewardClaimProxy, addr);
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
    pub async fn deploy<P>(&mut self, name: Contract, tx: RawCallBuilder<P>) -> Result<Address>
    where
        P: Provider,
    {
        if let Some(addr) = self.0.get(&name) {
            tracing::info!("skipping deployment of {name}, already deployed at {addr:#x}");
            return Ok(*addr);
        }
        tracing::info!("deploying {name}");
        let pending_tx = tx.send().await?;
        let tx_hash = *pending_tx.tx_hash();
        tracing::info!(%tx_hash, "waiting for tx to be mined");
        let receipt = pending_tx.get_receipt().await?;
        tracing::info!(%receipt.gas_used, %tx_hash, "tx mined");
        let addr = receipt
            .contract_address
            .ok_or(alloy::contract::Error::ContractNotDeployed)?;

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
    assert_eq!(lc_proxy.owner().call().await?, admin);
    if let Some(prover) = prover {
        assert_eq!(lc_proxy.permissionedProver().call().await?, prover);
    }
    assert_eq!(lc_proxy.stateHistoryRetentionPeriod().call().await?, 864000);
    assert_eq!(
        lc_proxy.currentBlockNumber().call().await?,
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
///
/// Returns the receipt of the upgrade transaction.
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
            let state_history_retention_period = proxy.stateHistoryRetentionPeriod().call().await?;
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

            // get owner of proxy
            let owner = proxy.owner().call().await?;
            let owner_addr = owner;
            tracing::info!("Proxy owner: {owner_addr:#x}");

            // prepare initial function call calldata (checks if already initialized)
            // you cannot initialize a proxy that is already initialized
            // so if one wanted to use this function to upgrade a proxy to v2, that's already v2
            // then we shouldn't call the initialize function
            let lcv2 = LightClientV2::new(lcv2_addr, &provider);
            let init_data = if already_initialized(&provider, proxy_addr, 2).await? {
                vec![].into()
            } else {
                lcv2.initializeV2(blocks_per_epoch, epoch_start_block)
                    .calldata()
                    .to_owned()
            };
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
                assert_eq!(proxy_as_v2.blocksPerEpoch().call().await?, blocks_per_epoch);
                assert_eq!(
                    proxy_as_v2.epochStartBlock().call().await?,
                    epoch_start_block
                );
                assert_eq!(
                    proxy_as_v2.stateHistoryRetentionPeriod().call().await?,
                    state_history_retention_period
                );
                assert_eq!(
                    proxy_as_v2.currentBlockNumber().call().await?,
                    U256::from(provider.get_block_number().await?)
                );

                tracing::info!(%lcv2_addr, "LightClientProxy successfully upgrade to: ");
                tracing::info!(
                    "blocksPerEpoch: {}",
                    proxy_as_v2.blocksPerEpoch().call().await?
                );
                tracing::info!(
                    "epochStartBlock: {}",
                    proxy_as_v2.epochStartBlock().call().await?
                );
            } else {
                tracing::error!("LightClientProxy upgrade failed: {:?}", receipt);
            }

            Ok(receipt)
        },
    }
}

/// Upgrade the light client proxy to use LightClientV3.
/// Internally, first detect existence of proxy, then deploy LCV3, then upgrade and initializeV3.
/// Internal to "deploy LCV3", we deploy PlonkVerifierV3 whose address will be used at LCV3 init time.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a regular EOA, not a multisig.
/// - the proxy is not yet initialized for V3
pub async fn upgrade_light_client_v3(
    provider: impl Provider,
    contracts: &mut Contracts,
    is_mock: bool,
) -> Result<TransactionReceipt> {
    match contracts.address(Contract::LightClientProxy) {
        // check if proxy already exists
        None => Err(anyhow!("LightClientProxy not found, can't upgrade")),
        Some(proxy_addr) => {
            let proxy = LightClient::new(proxy_addr, &provider);

            // Check proxy version, V3 requires at least V2 as a prerequisite
            // This ensures we don't try to upgrade from V1 directly to V3
            // V1 -> V2 -> V3 is the correct upgrade path
            let version = proxy.getVersion().call().await?;
            if version.majorVersion < 2 {
                anyhow::bail!(
                    "LightClientProxy is V{}, can't upgrade to V3. Must upgrade to V2 first.",
                    version.majorVersion
                );
            }

            // first deploy PlonkVerifierV3.sol
            let pv3_addr = contracts
                .deploy(
                    Contract::PlonkVerifierV3,
                    PlonkVerifierV3::deploy_builder(&provider),
                )
                .await?;
            assert!(is_contract(&provider, pv3_addr).await?);

            // then deploy LightClientV3.sol
            let target_lcv3_bytecode = if is_mock {
                LightClientV3Mock::BYTECODE.encode_hex()
            } else {
                LightClientV3::BYTECODE.encode_hex()
            };
            let lcv3_linked_bytecode = {
                match target_lcv3_bytecode
                    .matches(LIBRARY_PLACEHOLDER_ADDRESS)
                    .count()
                {
                    0 => return Err(anyhow!("lib placeholder not found")),
                    1 => Bytes::from_hex(target_lcv3_bytecode.replacen(
                        LIBRARY_PLACEHOLDER_ADDRESS,
                        &pv3_addr.encode_hex(),
                        1,
                    ))?,
                    _ => {
                        return Err(anyhow!(
                            "more than one lib placeholder found, consider using a different value"
                        ))
                    },
                }
            };
            let lcv3_addr = if is_mock {
                let addr = LightClientV3Mock::deploy_builder(&provider)
                    .map(|req| req.with_deploy_code(lcv3_linked_bytecode))
                    .deploy()
                    .await?;
                tracing::info!("deployed LightClientV3Mock at {addr:#x}");
                addr
            } else {
                contracts
                    .deploy(
                        Contract::LightClientV3,
                        LightClientV3::deploy_builder(&provider)
                            .map(|req| req.with_deploy_code(lcv3_linked_bytecode)),
                    )
                    .await?
            };

            // get owner of proxy
            let owner = proxy.owner().call().await?;
            let owner_addr = owner;
            tracing::info!("Proxy owner: {owner_addr:#x}");

            let lcv3 = LightClientV3::new(lcv3_addr, &provider);

            // prepare initial function call calldata (checks if already initialized)
            // you cannot initialize a proxy that is already initialized
            // so if one wanted to use this function to upgrade a proxy to v3, that's already v3
            // then we shouldn't call the initialize function
            let init_data = if already_initialized(&provider, proxy_addr, 3).await? {
                vec![].into()
            } else {
                lcv3.initializeV3().calldata().to_owned()
            };

            // invoke upgrade on proxy
            let receipt = proxy
                .upgradeToAndCall(lcv3_addr, init_data)
                .send()
                .await?
                .get_receipt()
                .await?;
            if receipt.inner.is_success() {
                // post deploy verification checks
                let proxy_as_v3 = LightClientV3::new(proxy_addr, &provider);
                assert_eq!(proxy_as_v3.getVersion().call().await?.majorVersion, 3);
                tracing::info!(%lcv3_addr, "LightClientProxy successfully upgrade to: ")
            } else {
                tracing::error!("LightClientProxy upgrade failed: {:?}", receipt);
            }

            Ok(receipt)
        },
    }
}

async fn already_initialized(
    provider: impl Provider,
    proxy_addr: Address,
    expected_major_version: u8,
) -> Result<bool> {
    let initialized = get_proxy_initialized_version(&provider, proxy_addr).await?;
    tracing::info!("Initialized version: {}", initialized);

    // since all upgradable contracts have a getVersion() function, we can use it to get the major version
    let contract_proxy = LightClientV2::new(proxy_addr, &provider);
    let contract_major_version = contract_proxy.getVersion().call().await?.majorVersion;

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
    assert_eq!(fee_proxy.owner().call().await?, admin);

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
    assert_eq!(token_proxy.owner().call().await?, owner);
    assert_eq!(token_proxy.symbol().call().await?, symbol);
    assert_eq!(token_proxy.decimals().call().await?, 18);
    assert_eq!(token_proxy.name().call().await?, name);
    let total_supply = token_proxy.totalSupply().call().await?;
    assert_eq!(
        token_proxy.balanceOf(init_grant_recipient).call().await?,
        total_supply
    );

    Ok(token_proxy_addr)
}

/// Upgrade the esp token proxy to use EspTokenV2.
pub async fn upgrade_esp_token_v2(
    provider: impl Provider,
    contracts: &mut Contracts,
) -> Result<TransactionReceipt> {
    let Some(proxy_addr) = contracts.address(Contract::EspTokenProxy) else {
        anyhow::bail!("EspTokenProxy not found, can't upgrade")
    };

    let proxy = EspToken::new(proxy_addr, &provider);
    // Deploy the new implementation
    let v2_addr = contracts
        .deploy(Contract::EspTokenV2, EspTokenV2::deploy_builder(&provider))
        .await?;

    assert!(is_contract(&provider, v2_addr).await?);

    // prepare init calldata for V2
    let reward_claim_addr = contracts
        .address(Contract::RewardClaimProxy)
        .ok_or_else(|| anyhow!("RewardClaimProxy not found"))?;
    let proxy_as_v2 = EspTokenV2::new(proxy_addr, &provider);
    let init_data = proxy_as_v2
        .initializeV2(reward_claim_addr)
        .calldata()
        .to_owned();

    // invoke upgrade on proxy with initializeV2 call
    let receipt = proxy
        .upgradeToAndCall(v2_addr, init_data)
        .send()
        .await?
        .get_receipt()
        .await?;

    if receipt.inner.is_success() {
        // post deploy verification checks
        let proxy_as_v2 = EspTokenV2::new(proxy_addr, &provider);
        assert_eq!(proxy_as_v2.getVersion().call().await?.majorVersion, 2);
        assert_eq!(proxy_as_v2.name().call().await?, "Espresso");
        assert_eq!(proxy_as_v2.rewardClaim().call().await?, reward_claim_addr);
        tracing::info!(%v2_addr, "EspToken successfully upgraded to");
    } else {
        anyhow::bail!("EspToken upgrade failed: {:?}", receipt);
    }

    Ok(receipt)
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
    if !is_contract(&provider, token_addr).await? {
        anyhow::bail!("Token address is not a contract, can't deploy StakeTableProxy");
    }

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
    assert_eq!(st_proxy.owner().call().await?, owner);
    assert_eq!(st_proxy.token().call().await?, token_addr);
    assert_eq!(st_proxy.lightClient().call().await?, light_client_addr);
    assert_eq!(
        st_proxy.exitEscrowPeriod().call().await?,
        exit_escrow_period
    );

    Ok(st_proxy_addr)
}

/// Deploy and initialize the RewardClaim contract behind a proxy
pub async fn deploy_reward_claim_proxy(
    provider: impl Provider,
    contracts: &mut Contracts,
    esp_token_addr: Address,
    light_client_addr: Address,
    owner: Address,
) -> Result<Address> {
    let reward_claim_addr = contracts
        .deploy(
            Contract::RewardClaim,
            RewardClaim::deploy_builder(&provider),
        )
        .await?;
    let reward_claim = RewardClaim::new(reward_claim_addr, &provider);

    // verify the esp token address contains a contract
    if !is_contract(&provider, esp_token_addr).await? {
        anyhow::bail!("EspToken address is not a contract, can't deploy RewardClaimProxy");
    }

    // verify the light client address contains a contract
    if !is_contract(&provider, light_client_addr).await? {
        anyhow::bail!("LightClient address is not a contract, can't deploy RewardClaimProxy");
    }

    let init_data = reward_claim
        .initialize(owner, esp_token_addr, light_client_addr)
        .calldata()
        .to_owned();
    let reward_claim_proxy_addr = contracts
        .deploy(
            Contract::RewardClaimProxy,
            ERC1967Proxy::deploy_builder(&provider, reward_claim_addr, init_data),
        )
        .await?;

    if !is_proxy_contract(&provider, reward_claim_proxy_addr).await? {
        panic!("RewardClaimProxy detected not as a proxy, report error!");
    }

    let reward_claim_proxy = RewardClaim::new(reward_claim_proxy_addr, &provider);
    assert_eq!(
        reward_claim_proxy.getVersion().call().await?,
        (1, 0, 0).into()
    );
    assert_eq!(reward_claim_proxy.owner().call().await?, owner);
    assert_eq!(reward_claim_proxy.espToken().call().await?, esp_token_addr);
    assert_eq!(
        reward_claim_proxy.lightClient().call().await?,
        light_client_addr
    );

    Ok(reward_claim_proxy_addr)
}

/// Read commission values from L1 StakeTable V1 ValidatorRegistered events for V2 migration
///
/// Assumes an infura RPC is used, otherwise it may hit other rate limits.
pub async fn fetch_commissions_for_stake_table_storage_migration(
    l1_provider: impl Provider,
    stake_table_address: Address,
) -> Result<Vec<StakeTableV2::InitialCommission>> {
    let stake_table = StakeTable::new(stake_table_address, &l1_provider);

    // Verify this is a V1 contract
    let version = stake_table.getVersion().call().await?;
    if version.majorVersion != 1 {
        anyhow::bail!(
            "Expected StakeTable V1 for commission migration, found V{}",
            version.majorVersion
        );
    }

    let start_block = stake_table.initializedAtBlock().call().await?.to::<u64>();

    tracing::info!(
        "Reading ValidatorRegistered events from L1 StakeTable V1 starting at block {}",
        start_block
    );

    // Query ValidatorRegistered events (V1) to get initial commission values
    let registration_events = stake_table
        .ValidatorRegistered_filter()
        .from_block(start_block)
        .query()
        .await
        .context("Failed to query ValidatorRegistered events from V1 contract")?;

    // Create a vec to store commissions in chronological order
    // Note: V1 events only have initial registration, no updates
    let mut initial_commissions = Vec::<StakeTableV2::InitialCommission>::new();

    for (event, _log) in registration_events {
        tracing::debug!(
            "ValidatorRegistered: validator={:?}, commission={}",
            event.account,
            event.commission
        );
        initial_commissions.push(event.into());
    }

    tracing::info!(
        "Found {} validators with commissions to migrate from V1",
        initial_commissions.len()
    );

    Ok(initial_commissions)
}

/// Prepare the upgrade data for StakeTable V2, checking version and fetching commissions if needed.
///
/// Returns:
/// - The initialization commissions (maybe used for post deployment verification)
/// - The initialization calldata (if initialization is needed)
pub async fn prepare_stake_table_v2_upgrade(
    provider: impl Provider,
    proxy_addr: Address,
    pauser: Address,
    admin: Address,
) -> Result<(Option<Vec<StakeTableV2::InitialCommission>>, Option<Bytes>)> {
    let proxy = StakeTable::new(proxy_addr, &provider);

    let current_version = proxy.getVersion().call().await?;
    let target_version = 2;
    if current_version.majorVersion > target_version {
        anyhow::bail!(
            "Expected StakeTable V1 or V2, found V{}",
            current_version.majorVersion
        );
    }

    // For a non-major version upgrade the proxy storage must already be initialized.
    let needs_initialization = !already_initialized(&provider, proxy_addr, target_version).await?;
    assert_eq!(
        needs_initialization,
        current_version.majorVersion < target_version,
        "unexpected version initialized"
    );

    if needs_initialization {
        tracing::info!("Fetching commissions from V1 contract for migration");
        let commissions =
            fetch_commissions_for_stake_table_storage_migration(&provider, proxy_addr).await?;
        tracing::info!("Fetched {} commissions from V1 contract", commissions.len());

        tracing::info!(
            %pauser,
            %admin,
            commission_count = commissions.len(),
            "Init Data to be signed. Function: initializeV2",
        );

        // We can use any address here since we're just building calldata
        let data = StakeTableV2::new(Address::ZERO, &provider)
            .initializeV2(pauser, admin, commissions.clone())
            .calldata()
            .to_owned();

        Ok((Some(commissions), Some(data)))
    } else {
        tracing::info!(
            "Proxy was already initialized for version {}",
            target_version
        );
        Ok((None, None))
    }
}

/// Upgrade the stake table proxy from V1 to V2, or patch V2
pub async fn upgrade_stake_table_v2(
    provider: impl Provider,
    contracts: &mut Contracts,
    pauser: Address,
    admin: Address,
) -> Result<TransactionReceipt> {
    tracing::info!("Upgrading StakeTableProxy to StakeTableV2 with EOA admin");
    let Some(proxy_addr) = contracts.address(Contract::StakeTableProxy) else {
        anyhow::bail!("StakeTableProxy not found, can't upgrade")
    };

    // First prepare upgrade data (including fetching commissions if needed)
    let (init_commissions, init_data) =
        prepare_stake_table_v2_upgrade(&provider, proxy_addr, pauser, admin).await?;

    // Then deploy the new implementation
    let v2_addr = contracts
        .deploy(
            Contract::StakeTableV2,
            StakeTableV2::deploy_builder(&provider),
        )
        .await?;

    let proxy = StakeTable::new(proxy_addr, &provider);

    let receipt = proxy
        .upgradeToAndCall(v2_addr, init_data.unwrap_or_default())
        .send()
        .await?
        .get_receipt()
        .await?;

    if receipt.inner.is_success() {
        // post deploy verification checks
        let proxy_as_v2 = StakeTableV2::new(proxy_addr, &provider);
        assert_eq!(proxy_as_v2.getVersion().call().await?.majorVersion, 2);

        let pauser_role = proxy_as_v2.PAUSER_ROLE().call().await?;
        assert!(proxy_as_v2.hasRole(pauser_role, pauser).call().await?,);

        let admin_role = proxy_as_v2.DEFAULT_ADMIN_ROLE().call().await?;
        assert!(proxy_as_v2.hasRole(admin_role, admin).call().await?,);

        if let Some(migrated) = init_commissions {
            tracing::info!("Verifying migrated commissions, may take a minute");
            for init_comm in migrated {
                let tracking = proxy_as_v2
                    .commissionTracking(init_comm.validator)
                    .call()
                    .await?;
                assert_eq!(tracking.commission, init_comm.commission);
            }
        }

        tracing::info!(%v2_addr, "StakeTable successfully upgraded to");
    } else {
        anyhow::bail!("StakeTable upgrade failed: {:?}", receipt);
    }

    Ok(receipt)
}

/// Common logic for any Ownable contract to transfer ownership
pub async fn transfer_ownership(
    provider: impl Provider,
    target_contract: Contract,
    target_address: Address,
    new_owner: Address,
) -> Result<TransactionReceipt> {
    let receipt = match target_contract {
        Contract::LightClient | Contract::LightClientProxy => {
            tracing::info!(%target_address, %new_owner, "Transfer LightClient ownership");
            let lc = LightClient::new(target_address, &provider);
            lc.transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::FeeContract | Contract::FeeContractProxy => {
            tracing::info!(%target_address, %new_owner, "Transfer FeeContract ownership");
            let fee = FeeContract::new(target_address, &provider);
            fee.transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::EspToken | Contract::EspTokenProxy => {
            tracing::info!(%target_address, %new_owner, "Transfer EspToken ownership");
            let token = EspToken::new(target_address, &provider);
            token
                .transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        Contract::StakeTable | Contract::StakeTableProxy | Contract::StakeTableV2 => {
            tracing::info!(%target_address, %new_owner, "Transfer StakeTable ownership");
            let stake_table = StakeTable::new(target_address, &provider);
            stake_table
                .transferOwnership(new_owner)
                .send()
                .await?
                .get_receipt()
                .await?
        },
        _ => return Err(anyhow!("Not Ownable, can't transfer ownership!")),
    };
    let tx_hash = receipt.transaction_hash;
    tracing::info!(%receipt.gas_used, %tx_hash, "ownership transferred");
    Ok(receipt)
}

/// helper function to decide if the contract at given address `addr` is a proxy contract
pub async fn is_proxy_contract(provider: impl Provider, addr: Address) -> Result<bool> {
    // when the implementation address is not equal to zero, it's a proxy
    Ok(read_proxy_impl(provider, addr).await? != Address::default())
}

pub async fn read_proxy_impl(provider: impl Provider, addr: Address) -> Result<Address> {
    // confirm that the proxy_address is a proxy
    // using the implementation slot, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc, which is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1
    let impl_slot = U256::from_str_radix(
        "360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc",
        16,
    )?;
    let storage = provider.get_storage_at(addr, impl_slot).await?;
    Ok(Address::from_slice(&storage.to_be_bytes_vec()[12..]))
}

pub async fn is_contract(provider: impl Provider, address: Address) -> Result<bool> {
    if address == Address::ZERO {
        return Ok(false);
    }
    Ok(!provider.get_code_at(address).await?.is_empty())
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

/// Deploy and initialize the Ops Timelock contract
///
/// Parameters:
/// - `min_delay`: The minimum delay for operations
/// - `proposers`: The list of addresses that can propose
/// - `executors`: The list of addresses that can execute
/// - `admin`: The address that can perform admin actions
pub async fn deploy_ops_timelock(
    provider: impl Provider,
    contracts: &mut Contracts,
    min_delay: U256,
    proposers: Vec<Address>,
    executors: Vec<Address>,
    admin: Address,
) -> Result<Address> {
    tracing::info!(
        "OpsTimelock will be deployed with the following parameters: min_delay: {:?}, proposers: \
         {:?}, executors: {:?}, admin: {:?}",
        min_delay,
        proposers,
        executors,
        admin
    );
    let timelock_addr = contracts
        .deploy(
            Contract::OpsTimelock,
            OpsTimelock::deploy_builder(
                &provider,
                min_delay,
                proposers.clone(),
                executors.clone(),
                admin,
            ),
        )
        .await?;

    // Verify deployment
    let timelock = OpsTimelock::new(timelock_addr, &provider);

    // Verify initialization parameters
    assert_eq!(timelock.getMinDelay().call().await?, min_delay);
    assert!(
        timelock
            .hasRole(timelock.PROPOSER_ROLE().call().await?, proposers[0])
            .call()
            .await?
    );
    assert!(
        timelock
            .hasRole(timelock.EXECUTOR_ROLE().call().await?, executors[0])
            .call()
            .await?
    );

    // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
    let default_admin_role = U256::ZERO;
    assert!(
        timelock
            .hasRole(default_admin_role.into(), admin)
            .call()
            .await?
    );

    Ok(timelock_addr)
}

/// Deploy and initialize the Safe Exit Timelock contract
///
/// Parameters:
/// - `min_delay`: The minimum delay for operations
/// - `proposers`: The list of addresses that can propose
/// - `executors`: The list of addresses that can execute
/// - `admin`: The address that can perform admin actions
pub async fn deploy_safe_exit_timelock(
    provider: impl Provider,
    contracts: &mut Contracts,
    min_delay: U256,
    proposers: Vec<Address>,
    executors: Vec<Address>,
    admin: Address,
) -> Result<Address> {
    tracing::info!(
        "SafeExitTimelock will be deployed with the following parameters: min_delay: {:?}, \
         proposers: {:?}, executors: {:?}, admin: {:?}",
        min_delay,
        proposers,
        executors,
        admin
    );
    let timelock_addr = contracts
        .deploy(
            Contract::SafeExitTimelock,
            SafeExitTimelock::deploy_builder(
                &provider,
                min_delay,
                proposers.clone(),
                executors.clone(),
                admin,
            ),
        )
        .await?;

    // Verify deployment
    let timelock = SafeExitTimelock::new(timelock_addr, &provider);

    // Verify initialization parameters
    assert_eq!(timelock.getMinDelay().call().await?, min_delay);
    assert!(
        timelock
            .hasRole(timelock.PROPOSER_ROLE().call().await?, proposers[0])
            .call()
            .await?
    );
    assert!(
        timelock
            .hasRole(timelock.EXECUTOR_ROLE().call().await?, executors[0])
            .call()
            .await?
    );

    // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
    let default_admin_role = U256::ZERO;
    assert!(
        timelock
            .hasRole(default_admin_role.into(), admin)
            .call()
            .await?
    );

    Ok(timelock_addr)
}

/// Encode a function call with the given signature and arguments
///
/// Parameters:
/// - `signature`: e.g. `"transfer(address,uint256)"`
/// - `args`: Solidity typed arguments as `Vec<&str>`
///
/// Returns:
/// - Full calldata: selector + encoded arguments
pub fn encode_function_call(signature: &str, args: Vec<String>) -> Result<Bytes> {
    let func = Function::parse(signature)?;

    // Check if argument count matches the function signature
    if args.len() != func.inputs.len() {
        anyhow::bail!(
            "Mismatch between argument count ({}) and parameter count ({})",
            args.len(),
            func.inputs.len()
        );
    }

    // Parse argument values using the function's parameter types directly
    let arg_values: Vec<DynSolValue> =
        func.inputs
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let arg_str = &args[i];
                let dyn_type: DynSolType =
                    param.ty.to_string().parse().map_err(|e| {
                        anyhow!("Failed to parse parameter type '{}': {}", param.ty, e)
                    })?;
                dyn_type.coerce_str(arg_str).map_err(|e| {
                    anyhow!(
                        "Failed to coerce argument '{}' to type '{}': {}",
                        arg_str,
                        param.ty,
                        e
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

    let encoded_input = func.abi_encode_input(&arg_values)?;
    let data = Bytes::from(encoded_input);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use alloy::{
        node_bindings::Anvil,
        primitives::utils::parse_ether,
        providers::{ext::AnvilApi, layers::AnvilProvider, ProviderBuilder},
        sol_types::SolValue,
    };
    use hotshot_contract_adapter::{
        sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol, StakeTableV2},
        stake_table::sign_address_bls,
    };
    use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use super::*;
    use crate::{
        impersonate_filler::ImpersonateFiller,
        proposals::{
            multisig::{
                transfer_ownership_from_multisig_to_timelock, upgrade_esp_token_v2_multisig_owner,
                upgrade_light_client_v2_multisig_owner, upgrade_stake_table_v2_multisig_owner,
                LightClientV2UpgradeParams, TransferOwnershipParams,
            },
            timelock::{
                cancel_timelock_operation, execute_timelock_operation, schedule_timelock_operation,
                TimelockOperationData,
            },
        },
    };

    #[test_log::test(tokio::test)]
    async fn test_is_contract() -> Result<(), anyhow::Error> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

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

    #[test_log::test(tokio::test)]
    async fn test_is_proxy_contract() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let deployer = provider.get_accounts().await?[0];

        let fee_contract = FeeContract::deploy(&provider).await?;
        let init_data = fee_contract.initialize(deployer).calldata().clone();
        let proxy = ERC1967Proxy::deploy(&provider, *fee_contract.address(), init_data).await?;

        assert!(is_proxy_contract(&provider, *proxy.address()).await?);
        assert!(!is_proxy_contract(&provider, *fee_contract.address()).await?);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_light_client() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

    #[test_log::test(tokio::test)]
    async fn test_deploy_mock_light_client_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

    #[test_log::test(tokio::test)]
    async fn test_deploy_light_client_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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
        assert_eq!(fetched_prover, prover);

        // test transfer ownership to multisig
        let multisig = Address::random();
        let _receipt = transfer_ownership(
            &provider,
            Contract::LightClientProxy,
            lc_proxy_addr,
            multisig,
        )
        .await?;
        assert_eq!(lc.owner().call().await?, multisig);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_fee_contract_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let admin = provider.get_accounts().await?[0];
        let alice = Address::random();

        let fee_proxy_addr = deploy_fee_contract_proxy(&provider, &mut contracts, alice).await?;

        // check initialization is correct
        let fee = FeeContract::new(fee_proxy_addr, &provider);
        let fetched_owner = fee.owner().call().await?;
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
        assert_eq!(fee.owner().call().await?, multisig);

        Ok(())
    }

    async fn test_upgrade_light_client_to_v2_helper(is_mock: bool) -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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
            .await?;

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
        assert_eq!(lc.blocksPerEpoch().call().await?, blocks_per_epoch);
        assert_eq!(lc.epochStartBlock().call().await?, epoch_start_block);
        assert_eq!(
            lc.stateHistoryRetentionPeriod().call().await?,
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
            assert_eq!(new_blocks_per_epoch, lc_mock.blocksPerEpoch().call().await?);
        }
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_light_client_to_v2() -> Result<()> {
        test_upgrade_light_client_to_v2_helper(false).await
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_mock_light_client_v2() -> Result<()> {
        test_upgrade_light_client_to_v2_helper(true).await
    }

    #[test_log::test(tokio::test)]
    async fn test_fetch_commissions_for_stake_table_storage_migration() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let owner = provider.get_accounts().await?[0];

        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            owner,
            owner,
            U256::from(10_000_000u64),
            "Test Token",
            "TEST",
        )
        .await?;
        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;
        let exit_escrow_period = U256::from(1000);

        let stake_table_proxy_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            exit_escrow_period,
            owner,
        )
        .await?;

        // Use V2 interface even for V1 contract (V2 ABI is a superset of V1)
        let stake_table = StakeTableV2::new(stake_table_proxy_addr, &provider);

        let accounts = provider.get_accounts().await?;
        let test_validators = [
            (accounts[0], 0u16),
            (accounts[1], 100u16),
            (accounts[2], 10_000u16),
        ];

        let mut rng = StdRng::from_seed([42u8; 32]);
        for (validator_addr, commission) in test_validators.iter() {
            let bls_key_pair = BLSKeyPair::generate(&mut rng);
            let state_key_pair = StateKeyPair::generate_from_seed(rng.gen());
            let bls_vk_sol: G2PointSol = bls_key_pair.ver_key().to_affine().into();
            let bls_sig_sol: G1PointSol = sign_address_bls(&bls_key_pair, *validator_addr).into();
            let schnorr_vk_sol: EdOnBN254PointSol = state_key_pair.ver_key().to_affine().into();

            let receipt = stake_table
                .registerValidator(bls_vk_sol, schnorr_vk_sol, bls_sig_sol.into(), *commission)
                .from(*validator_addr)
                .send()
                .await?
                .get_receipt()
                .await?;
            assert!(receipt.status());
        }

        let fetched_commissions =
            fetch_commissions_for_stake_table_storage_migration(&provider, stake_table_proxy_addr)
                .await?;

        assert_eq!(fetched_commissions.len(), test_validators.len(),);

        for ((validator, commission), fetched) in test_validators.iter().zip(&fetched_commissions) {
            assert_eq!(fetched.validator, *validator);
            assert_eq!(fetched.commission, *commission);
        }

        // Migration only applies to V1 contract
        let stake_table_v2 = StakeTableV2::deploy(&provider).await?;
        let err = fetch_commissions_for_stake_table_storage_migration(
            &provider,
            *stake_table_v2.address(),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("Expected StakeTable V1"));

        Ok(())
    }

    /// Check that we can fetch the commissions on sepolia where we will do the
    /// commission migration.
    ///
    /// Assumes an infura RPC is used, otherwise fetching commissions may hit other rate limits.
    ///
    /// env RPC_URL=... cargo test -p espresso-contract-deployer -- --ignored test_fetch_commissions_sepolia
    #[ignore]
    #[test_log::test(tokio::test)]
    async fn test_fetch_commissions_sepolia() -> Result<()> {
        let rpc_url = std::env::var("RPC_URL")
            .expect("RPC_URL environment variable not set")
            .parse()?;
        let provider = ProviderBuilder::new().connect_http(rpc_url);

        // Decaf / sepolia stake table address
        let stake_table_address: Address = "0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037".parse()?;
        let fetched_commissions =
            fetch_commissions_for_stake_table_storage_migration(&provider, stake_table_address)
                .await?;
        assert!(!fetched_commissions.is_empty());

        println!(
            "Fetched {} commissions from Sepolia StakeTable",
            fetched_commissions.len()
        );
        for commission in &fetched_commissions {
            println!(
                "  Validator: {}, Commission: {}",
                commission.validator, commission.commission
            );
        }

        let pauser = Address::random();
        let admin = Address::random();
        let init_v2_calldata = StakeTableV2::new(stake_table_address, &provider)
            .initializeV2(pauser, admin, fetched_commissions)
            .calldata()
            .clone();
        println!("Calldata size: {} bytes", init_v2_calldata.len());

        // The max calldata size is 128 kB per tx, but at the time of writing we
        // only need about 7 kB therefore applying a stricter limit of 32 kB
        assert!(init_v2_calldata.len() < 32 * 1024);
        Ok(())
    }

    impl Contracts {
        fn insert(&mut self, name: Contract, address: Address) -> Option<Address> {
            self.0.insert(name, address)
        }
    }

    /// Fork test to test if we can upgrade the decaf stake table from V1 to V2
    /// This test forks Sepolia (where decaf runs) using anvil, fetches existing commissions,
    /// impersonates the proxy owner, and performs the upgrade.
    ///
    /// Assumes an infura RPC is used, otherwise fetching commissions may hit other rate limits.
    ///
    /// env RPC_URL=... cargo test -p espresso-contract-deployer -- --ignored test_upgrade_decaf_stake_table_fork
    #[ignore]
    #[test_log::test(tokio::test)]
    async fn test_upgrade_decaf_stake_table_fork() -> Result<()> {
        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL environment variable not set");

        // Decaf / sepolia stake table address
        let stake_table_address: Address = "0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037".parse()?;
        let anvil = Anvil::new()
            .fork(rpc_url)
            .arg("--retries")
            .arg("20")
            .spawn();

        let provider = ProviderBuilder::new().connect_http(anvil.endpoint().parse()?);
        let proxy = StakeTable::new(stake_table_address, &provider);
        let proxy_owner = proxy.owner().call().await?;
        tracing::info!("Proxy owner address: {proxy_owner:#x}");

        // Enable impersonation for the proxy owner
        let provider = ProviderBuilder::new()
            .filler(ImpersonateFiller::new(proxy_owner))
            .connect_http(anvil.endpoint().parse()?);
        let anvil_provider = AnvilProvider::new(provider.clone(), Arc::new(anvil));
        anvil_provider.anvil_auto_impersonate_account(true).await?;
        anvil_provider
            .anvil_set_balance(proxy_owner, parse_ether("100")?)
            .await?;

        // We need a Contracts instance with proxy deployed
        let mut contracts = Contracts::new();
        contracts.insert(Contract::StakeTableProxy, stake_table_address);
        let pauser = Address::random();
        let admin = proxy_owner;

        upgrade_stake_table_v2(&provider, &mut contracts, pauser, admin).await?;
        Ok(())
    }

    async fn test_upgrade_light_client_to_v3_helper(options: UpgradeTestOptions) -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

        // first upgrade to v2
        upgrade_light_client_v2(
            &provider,
            &mut contracts,
            options.is_mock,
            blocks_per_epoch,
            epoch_start_block,
        )
        .await?;

        // then upgrade to v3
        upgrade_light_client_v3(&provider, &mut contracts, options.is_mock).await?;

        // test correct v1 and v2 state persistence
        let lc = LightClientV3::new(lc_proxy_addr, &provider);
        let finalized_state: LightClientStateSol = lc.finalizedState().call().await?.into();
        assert_eq!(
            genesis_state.abi_encode_params(),
            finalized_state.abi_encode_params()
        );

        // test v2 state persistence
        let next_stake: StakeTableStateSol = lc.votingStakeTableState().call().await?.into();
        assert_eq!(
            genesis_stake.abi_encode_params(),
            next_stake.abi_encode_params()
        );

        // test v3 specific properties
        assert_eq!(lc.getVersion().call().await?.majorVersion, 3);

        // V3 inherits blocks_per_epoch and epoch_start_block from V2
        let lc_as_v2 = LightClientV2::new(lc_proxy_addr, &provider);
        assert_eq!(lc_as_v2.blocksPerEpoch().call().await?, blocks_per_epoch);
        assert_eq!(lc_as_v2.epochStartBlock().call().await?, epoch_start_block);

        // test mock-specific functions
        if options.is_mock {
            // recast to mock
            let lc_mock = LightClientV3Mock::new(lc_proxy_addr, &provider);
            // Test that mock-specific functions work
            let new_blocks_per_epoch = blocks_per_epoch + 10;
            lc_mock
                .setBlocksPerEpoch(new_blocks_per_epoch)
                .send()
                .await?
                .watch()
                .await?;
            assert_eq!(new_blocks_per_epoch, lc_mock.blocksPerEpoch().call().await?);
        }
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_light_client_to_v3() -> Result<()> {
        test_upgrade_light_client_to_v3_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::RealRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_mock_light_client_v3() -> Result<()> {
        test_upgrade_light_client_to_v3_helper(UpgradeTestOptions {
            is_mock: true,
            run_mode: RunMode::RealRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
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
                panic!(
                    "ESPRESSO_SEQUENCER_L1_PROVIDER and ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
                     must be set in .env.deployer.rs.test"
                );
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
        assert_eq!(lc.owner().call().await?, multisig_admin);

        // then send upgrade proposal to the multisig wallet
        let result = upgrade_light_client_v2_multisig_owner(
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

    #[test_log::test(tokio::test)]
    async fn test_upgrade_light_client_to_v2_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::DryRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    // We expect no init data for the second upgrade because the proxy was already initialized
    #[test_log::test(tokio::test)]
    async fn test_upgrade_light_client_to_v2_twice_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::DryRun,
            upgrade_count: UpgradeCount::Twice,
        })
        .await
    }

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn test_upgrade_light_client_to_v2_multisig_owner_live_eth_network() -> Result<()> {
        test_upgrade_light_client_to_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::RealRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_token_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

        assert_eq!(token.owner().call().await?, rand_owner);
        let total_supply = token.totalSupply().call().await?;
        assert_eq!(
            total_supply,
            parse_ether(&initial_supply.to_string()).unwrap()
        );
        assert_eq!(token.balanceOf(init_recipient).call().await?, total_supply);
        assert_eq!(token.name().call().await?, name);
        assert_eq!(token.symbol().call().await?, symbol);

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_stake_table_proxy() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

        // deploy light client proxy
        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            false,
            LightClientStateSol::dummy_genesis(),
            StakeTableStateSol::dummy_genesis(),
            init_recipient,
            Some(init_recipient),
        )
        .await?;

        // deploy stake table
        let exit_escrow_period = U256::from(250);
        let owner = init_recipient;
        let stake_table_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_proxy_addr,
            exit_escrow_period,
            owner,
        )
        .await?;
        let stake_table = StakeTable::new(stake_table_addr, &provider);

        assert_eq!(
            stake_table.exitEscrowPeriod().call().await?,
            exit_escrow_period
        );
        assert_eq!(stake_table.owner().call().await?, owner);
        assert_eq!(stake_table.token().call().await?, token_addr);
        assert_eq!(stake_table.lightClient().call().await?, lc_proxy_addr);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_stake_table_v2() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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

        // deploy light client proxy
        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            false,
            LightClientStateSol::dummy_genesis(),
            StakeTableStateSol::dummy_genesis(),
            init_recipient,
            Some(init_recipient),
        )
        .await?;

        // deploy stake table
        let exit_escrow_period = U256::from(250);
        let owner = init_recipient;
        let stake_table_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_proxy_addr,
            exit_escrow_period,
            owner,
        )
        .await?;
        let stake_table_v1 = StakeTable::new(stake_table_addr, &provider);
        assert_eq!(stake_table_v1.getVersion().call().await?, (1, 0, 0).into());

        // snapshot for later patch upgrade, the upgrade will skip the v2
        // deployment if contracts already contains a v2
        let mut contracts_v1 = contracts.clone();

        // upgrade to v2
        let pauser = Address::random();
        upgrade_stake_table_v2(&provider, &mut contracts, pauser, owner).await?;

        let stake_table_v2 = StakeTableV2::new(stake_table_addr, &provider);

        assert_eq!(stake_table_v2.getVersion().call().await?, (2, 0, 0).into());
        assert_eq!(stake_table_v2.owner().call().await?, owner);
        assert_eq!(stake_table_v2.token().call().await?, token_addr);
        assert_eq!(stake_table_v2.lightClient().call().await?, lc_proxy_addr);

        // get pauser role
        let pauser_role = stake_table_v2.PAUSER_ROLE().call().await?;
        assert!(stake_table_v2.hasRole(pauser_role, pauser).call().await?,);

        // get admin role
        let admin_role = stake_table_v2.DEFAULT_ADMIN_ROLE().call().await?;
        assert!(stake_table_v2.hasRole(admin_role, owner).call().await?,);

        // ensure we can upgrade (again) to a V2 patch version
        let current_impl = read_proxy_impl(&provider, stake_table_addr).await?;
        upgrade_stake_table_v2(&provider, &mut contracts_v1, pauser, owner).await?;
        assert_ne!(
            read_proxy_impl(&provider, stake_table_addr).await?,
            current_impl
        );

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
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
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
        // deploy light client proxy
        let lc_proxy_addr = deploy_light_client_proxy(
            &provider,
            &mut contracts,
            false,
            LightClientStateSol::dummy_genesis(),
            StakeTableStateSol::dummy_genesis(),
            init_recipient,
            Some(init_recipient),
        )
        .await?;

        let exit_escrow_period = U256::from(250);
        let owner = init_recipient;
        let stake_table_proxy_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_proxy_addr,
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
        assert_eq!(stake_table.owner().call().await?, multisig_admin);
        // then send upgrade proposal to the multisig wallet
        let pauser = Address::random();
        upgrade_stake_table_v2_multisig_owner(
            &provider,
            &mut contracts,
            sepolia_rpc_url,
            multisig_admin,
            pauser,
            Some(dry_run),
        )
        .await?;

        // v1 state persistence cannot be tested here because the upgrade proposal is not yet executed
        // One has to test that the upgrade proposal is available via the Safe UI
        // and then test that the v1 state is persisted
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_stake_table_to_v2_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_stake_table_to_v2_multisig_owner_helper(true).await
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_ops_timelock() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // Setup test parameters
        let min_delay = U256::from(86400); // 1 day in seconds
        let admin = provider.get_accounts().await?[0];
        let proposers = vec![Address::random()];
        let executors = vec![Address::random()];

        let timelock_addr = deploy_ops_timelock(
            &provider,
            &mut contracts,
            min_delay,
            proposers.clone(),
            executors.clone(),
            admin,
        )
        .await?;

        // Verify deployment
        let timelock = OpsTimelock::new(timelock_addr, &provider);
        assert_eq!(timelock.getMinDelay().call().await?, min_delay);

        // Verify initialization parameters
        assert_eq!(timelock.getMinDelay().call().await?, min_delay);
        assert!(
            timelock
                .hasRole(timelock.PROPOSER_ROLE().call().await?, proposers[0])
                .call()
                .await?
        );
        assert!(
            timelock
                .hasRole(timelock.EXECUTOR_ROLE().call().await?, executors[0])
                .call()
                .await?
        );

        // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
        let default_admin_role = U256::ZERO;
        assert!(
            timelock
                .hasRole(default_admin_role.into(), admin)
                .call()
                .await?
        );
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_deploy_safe_exit_timelock() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // Setup test parameters
        let min_delay = U256::from(86400); // 1 day in seconds
        let admin = provider.get_accounts().await?[0];
        let proposers = vec![Address::random()];
        let executors = vec![Address::random()];

        let timelock_addr = deploy_safe_exit_timelock(
            &provider,
            &mut contracts,
            min_delay,
            proposers.clone(),
            executors.clone(),
            admin,
        )
        .await?;

        // Verify deployment
        let timelock = SafeExitTimelock::new(timelock_addr, &provider);
        assert_eq!(timelock.getMinDelay().call().await?, min_delay);

        // Verify initialization parameters
        assert_eq!(timelock.getMinDelay().call().await?, min_delay);
        assert!(
            timelock
                .hasRole(timelock.PROPOSER_ROLE().call().await?, proposers[0])
                .call()
                .await?
        );
        assert!(
            timelock
                .hasRole(timelock.EXECUTOR_ROLE().call().await?, executors[0])
                .call()
                .await?
        );

        // test that the admin is in the default admin role where DEFAULT_ADMIN_ROLE = 0x00
        let default_admin_role = U256::ZERO;
        assert!(
            timelock
                .hasRole(default_admin_role.into(), admin)
                .call()
                .await?
        );
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_upgrade_esp_token_v2() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();

        // deploy token
        let init_recipient = provider.get_accounts().await?[1];
        let token_owner = provider.get_accounts().await?[0];
        let token_name = "Espresso";
        let token_symbol = "ESP";
        let initial_supply = U256::from(3590000000u64);
        let token_proxy_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            token_owner,
            init_recipient,
            initial_supply,
            token_name,
            token_symbol,
        )
        .await?;
        let esp_token = EspToken::new(token_proxy_addr, &provider);
        assert_eq!(esp_token.name().call().await?, token_name);

        let fake_reward_claim = Address::random();
        contracts.insert(Contract::RewardClaimProxy, fake_reward_claim);

        // upgrade to v2
        upgrade_esp_token_v2(&provider, &mut contracts).await?;

        let esp_token_v2 = EspTokenV2::new(token_proxy_addr, &provider);

        assert_eq!(esp_token_v2.getVersion().call().await?, (2, 0, 0).into());
        assert_eq!(esp_token_v2.owner().call().await?, token_owner);

        // name is hardcoded in the EspTokenV2 contract
        assert_eq!(esp_token_v2.name().call().await?, "Espresso");
        assert_eq!(esp_token_v2.symbol().call().await?, "ESP");
        assert_eq!(esp_token_v2.decimals().call().await?, 18);

        let initial_supply_in_wei = parse_ether(&initial_supply.to_string()).unwrap();
        assert_eq!(
            esp_token_v2.totalSupply().call().await?,
            initial_supply_in_wei
        );
        assert_eq!(
            esp_token_v2.balanceOf(init_recipient).call().await?,
            initial_supply_in_wei
        );
        assert_eq!(
            esp_token_v2.balanceOf(token_owner).call().await?,
            U256::ZERO
        );

        assert_eq!(esp_token_v2.rewardClaim().call().await?, fake_reward_claim);

        Ok(())
    }

    // We expect no init data for the upgrade because there is no reinitializer for v2
    #[test_log::test(tokio::test)]
    async fn test_upgrade_esp_token_v2_multisig_owner_dry_run() -> Result<()> {
        test_upgrade_esp_token_v2_multisig_owner_helper(UpgradeTestOptions {
            is_mock: false,
            run_mode: RunMode::DryRun,
            upgrade_count: UpgradeCount::Once,
        })
        .await
    }

    // This test is used to test the upgrade of the EspTokenProxy via the multisig wallet
    // It only tests the upgrade proposal via the typescript script and thus requires the upgrade proposal to be sent to a real network
    // However, the contracts are deployed on anvil, so the test will pass even if the upgrade proposal is not executed
    // The test assumes that there is a file .env.deployer.rs.test in the root directory:
    // Ensure that the private key has proposal rights on the Safe Multisig Wallet and the SDK supports the network
    async fn test_upgrade_esp_token_v2_multisig_owner_helper(
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
                panic!(
                    "ESPRESSO_SEQUENCER_L1_PROVIDER and ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS \
                     must be set in .env.deployer.rs.test"
                );
            }
        }

        let mut contracts = Contracts::new();
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
        let init_recipient = provider.get_accounts().await?[0];
        let initial_supply = U256::from(3590000000u64);
        let token_name = "Espresso";
        let token_symbol = "ESP";

        // deploy proxy and V1
        let esp_token_proxy_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            admin,
            init_recipient,
            initial_supply,
            token_name,
            token_symbol,
        )
        .await?;
        if matches!(options.upgrade_count, UpgradeCount::Twice) {
            // upgrade to v2
            upgrade_esp_token_v2(&provider, &mut contracts).await?;
        }

        // transfer ownership to multisig
        let _receipt = transfer_ownership(
            &provider,
            Contract::EspTokenProxy,
            esp_token_proxy_addr,
            multisig_admin,
        )
        .await?;
        let esp_token = EspToken::new(esp_token_proxy_addr, &provider);
        assert_eq!(esp_token.owner().call().await?, multisig_admin);

        let fake_reward_claim = Address::random();
        contracts.insert(Contract::RewardClaimProxy, fake_reward_claim);

        let init_data = EspTokenV2::new(Address::ZERO, &provider)
            .initializeV2(fake_reward_claim)
            .calldata()
            .to_owned();

        // then send upgrade proposal to the multisig wallet
        let result = upgrade_esp_token_v2_multisig_owner(
            &provider,
            &mut contracts,
            sepolia_rpc_url.clone(),
            Some(dry_run),
        )
        .await?;
        tracing::info!(
            "Result when trying to upgrade EspTokenProxy via the multisig wallet: {:?}",
            result
        );
        if dry_run {
            let data: serde_json::Value = serde_json::from_str(&result)?;
            assert_eq!(data["rpcUrl"], sepolia_rpc_url);
            assert_eq!(data["safeAddress"], multisig_admin.to_string());
            assert_eq!(data["proxyAddress"], esp_token_proxy_addr.to_string());
            assert_eq!(data["initData"], init_data.to_string());
            assert_eq!(data["useHardwareWallet"], false);
        }
        // v1 state persistence cannot be tested here because the upgrade proposal is not yet executed
        // One has to test that the upgrade proposal is available via the Safe UI
        // and then test that the v1 state is persisted
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_schedule_and_execute_timelock_operation() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let delay = U256::from(0);

        // Get the provider's wallet address (the one actually sending transactions)
        let provider_wallet = provider.get_accounts().await?[0];

        let proposers = vec![provider_wallet];
        let executors = vec![provider_wallet];

        let timelock_addr = deploy_ops_timelock(
            &provider,
            &mut contracts,
            delay,
            proposers,
            executors,
            provider_wallet, // Use provider wallet as admin too
        )
        .await?;

        // deploy fee contract and set the timelock as the admin
        let fee_contract_proxy_addr =
            deploy_fee_contract_proxy(&provider, &mut contracts, timelock_addr).await?;

        let proxy = FeeContract::new(fee_contract_proxy_addr, &provider);
        let upgrade_data = proxy
            .transferOwnership(provider_wallet)
            .calldata()
            .to_owned();

        // propose a timelock operation
        let mut operation = TimelockOperationData {
            target: fee_contract_proxy_addr,
            value: U256::ZERO,
            data: upgrade_data,
            predecessor: B256::ZERO,
            salt: B256::ZERO,
            delay,
        };
        let operation_id =
            schedule_timelock_operation(&provider, Contract::FeeContractProxy, operation.clone())
                .await?;

        // check that the tx is scheduled
        let timelock = OpsTimelock::new(timelock_addr, &provider);
        assert!(timelock.isOperationPending(operation_id).call().await?);
        assert!(timelock.isOperationReady(operation_id).call().await?);
        assert!(!timelock.isOperationDone(operation_id).call().await?);
        assert!(timelock.getTimestamp(operation_id).call().await? > U256::ZERO);

        // execute the tx since the delay is 0
        execute_timelock_operation(&provider, Contract::FeeContractProxy, operation.clone())
            .await?;

        // check that the tx is executed
        assert!(timelock.isOperationDone(operation_id).call().await?);
        assert!(!timelock.isOperationPending(operation_id).call().await?);
        assert!(!timelock.isOperationReady(operation_id).call().await?);
        // check that the new owner is the provider_wallet
        let fee_contract = FeeContract::new(operation.target, &provider);
        assert_eq!(fee_contract.owner().call().await?, provider_wallet);

        operation.value = U256::from(1);
        //transfer ownership back to the timelock
        let tx_receipt = fee_contract
            .transferOwnership(timelock_addr)
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(tx_receipt.inner.is_success());

        schedule_timelock_operation(&provider, Contract::FeeContractProxy, operation.clone())
            .await?;

        cancel_timelock_operation(&provider, Contract::FeeContractProxy, operation.clone()).await?;

        // check that the tx is cancelled
        let next_operation_id = timelock
            .hashOperation(
                operation.target,
                operation.value,
                operation.data.clone(),
                operation.predecessor,
                operation.salt,
            )
            .call()
            .await?;
        assert!(timelock.getTimestamp(next_operation_id).call().await? == U256::ZERO);
        Ok(())
    }

    async fn test_transfer_ownership_helper(
        contract_type: Contract,
        options: UpgradeTestOptions,
    ) -> Result<()> {
        assert!(
            std::path::Path::new("../../../scripts/multisig-upgrade-entrypoint").exists(),
            "Script not found!"
        );
        let mut sepolia_rpc_url = "http://127.0.0.1:8545".to_string();
        let mut multisig_admin = Address::random();
        let mut timelock = Address::random();
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
                } else if key == "ESPRESSO_SEQUENCER_TIMELOCK_ADDRESS" {
                    timelock = val.parse::<Address>()?;
                }
            }

            if sepolia_rpc_url.is_empty() || multisig_admin.is_zero() || timelock.is_zero() {
                panic!(
                    "ESPRESSO_SEQUENCER_L1_PROVIDER, ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS, \
                     ESPRESSO_SEQUENCER_TIMELOCK_ADDRESS must be set in .env.deployer.rs.test"
                );
            }
        }

        let mut contracts = Contracts::new();
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
        let proxy_addr = match contract_type {
            Contract::LightClientProxy => {
                deploy_light_client_proxy(
                    &provider,
                    &mut contracts,
                    false,
                    genesis_state.clone(),
                    genesis_stake.clone(),
                    admin,
                    Some(prover),
                )
                .await?
            },
            Contract::FeeContractProxy => {
                deploy_fee_contract_proxy(&provider, &mut contracts, admin).await?
            },
            Contract::EspTokenProxy => {
                deploy_token_proxy(
                    &provider,
                    &mut contracts,
                    admin,
                    multisig_admin,
                    U256::from(0u64),
                    "Test",
                    "TEST",
                )
                .await?
            },
            Contract::StakeTableProxy => {
                let token_addr = deploy_token_proxy(
                    &provider,
                    &mut contracts,
                    admin,
                    admin,
                    U256::from(0u64),
                    "Test",
                    "TEST",
                )
                .await?;
                let initial_admin = provider.get_accounts().await?[0];
                // deploy light client proxy
                let lc_proxy_addr = deploy_light_client_proxy(
                    &provider,
                    &mut contracts,
                    false,
                    LightClientStateSol::dummy_genesis(),
                    StakeTableStateSol::dummy_genesis(),
                    initial_admin,
                    Some(prover),
                )
                .await?;
                // upgrade to v2
                let blocks_per_epoch = 50;
                let epoch_start_block = 50;
                upgrade_light_client_v2(
                    &provider,
                    &mut contracts,
                    false,
                    blocks_per_epoch,
                    epoch_start_block,
                )
                .await?;
                let lc_v2 = LightClientV2::new(lc_proxy_addr, &provider);
                assert_eq!(lc_v2.getVersion().call().await?.majorVersion, 2);
                assert_eq!(lc_v2.blocksPerEpoch().call().await?, blocks_per_epoch);
                assert_eq!(lc_v2.epochStartBlock().call().await?, epoch_start_block);

                deploy_stake_table_proxy(
                    &provider,
                    &mut contracts,
                    token_addr,
                    lc_proxy_addr,
                    U256::from(1000u64),
                    admin,
                )
                .await?
            },
            _ => anyhow::bail!("Not a proxy contract, can't transfer ownership"),
        };

        // transfer ownership to multisig
        let _receipt =
            transfer_ownership(&provider, contract_type, proxy_addr, multisig_admin).await?;
        assert_eq!(
            OwnableUpgradeable::new(proxy_addr, &provider)
                .owner()
                .call()
                .await?,
            multisig_admin
        );

        // then send transfer ownership proposal to the multisig wallet
        let result = transfer_ownership_from_multisig_to_timelock(
            &provider,
            &mut contracts,
            contract_type,
            TransferOwnershipParams {
                new_owner: timelock,
                rpc_url: sepolia_rpc_url.clone(),
                safe_addr: multisig_admin,
                use_hardware_wallet: false,
                dry_run,
            },
        )
        .await?;
        assert!(result.status.success());
        tracing::info!("Transfer ownership output: {:?}", result);

        let stdout = String::from_utf8_lossy(&result.stdout);
        let first_line = stdout.lines().next().unwrap();
        let data: serde_json::Value = serde_json::from_str(first_line)?;
        assert_eq!(data["rpcUrl"], sepolia_rpc_url);
        assert_eq!(data["safeAddress"], multisig_admin.to_string());

        let expected_init_data = match contract_type {
            Contract::LightClientProxy => LightClient::new(proxy_addr, &provider)
                .transferOwnership(timelock)
                .calldata()
                .to_string(),
            Contract::FeeContractProxy => FeeContract::new(proxy_addr, &provider)
                .transferOwnership(timelock)
                .calldata()
                .to_string(),
            Contract::EspTokenProxy => EspToken::new(proxy_addr, &provider)
                .transferOwnership(timelock)
                .calldata()
                .to_string(),
            Contract::StakeTableProxy => StakeTable::new(proxy_addr, &provider)
                .transferOwnership(timelock)
                .calldata()
                .to_string(),
            _ => "0x".to_string(),
        };

        assert_eq!(data["initData"], expected_init_data);
        assert_eq!(data["useHardwareWallet"], false);
        // }
        // v1 state persistence cannot be tested here because the upgrade proposal is not yet executed
        // One has to test that the upgrade proposal is available via the Safe UI
        // and then test that the v1 state is persisted
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call() -> Result<()> {
        let function_signature = "transfer(address,uint256)".to_string();
        let values = vec![
            "0x000000000000000000000000000000000000dead".to_string(),
            "1000".to_string(),
        ];
        let expected = "0xa9059cbb000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000003e8".parse::<Bytes>()?;
        let encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call_upgrade_to_and_call() -> Result<()> {
        let function_signature = "upgradeToAndCall(address,bytes)".to_string();
        let values = vec![
            "0xe1f131b07550a689d6a11f21d9e9238a5c466996".to_string(),
            "0x".to_string(),
        ];
        let expected = "0x4f1ef286000000000000000000000000e1f131b07550a689d6a11f21d9e9238a5c46699600000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000".parse::<Bytes>()?;
        let encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call_with_bytes32() -> Result<()> {
        let function_signature = "setHash(bytes32)".to_string();
        let values =
            vec!["0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()];
        let expected = "0x0c4c42850123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .parse::<Bytes>()?;
        let encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call_with_bytes() -> Result<()> {
        let function_signature = "emitData(bytes)".to_string();
        let values = vec!["0xdeadbeef".to_string()];
        let expected = "0xd836083e00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000004deadbeef00000000000000000000000000000000000000000000000000000000".parse::<Bytes>()?;
        let encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call_with_bool() -> Result<()> {
        let function_signature = "setFlag(bool)".to_string();
        let mut values = vec!["true".to_string()];
        let mut expected =
            "0x3927f6af0000000000000000000000000000000000000000000000000000000000000001"
                .parse::<Bytes>()?;
        let mut encoded =
            encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);

        values = vec!["false".to_string()];
        expected = "0x3927f6af0000000000000000000000000000000000000000000000000000000000000000"
            .parse::<Bytes>()?;
        encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_encode_function_call_with_string() -> Result<()> {
        let function_signature = "logMessage(string)".to_string();
        let values = vec!["Hello, world!".to_string()];
        let expected = "0x7c9900520000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000d48656c6c6f2c20776f726c642100000000000000000000000000000000000000".parse::<Bytes>()?;
        let encoded = encode_function_call(&function_signature, values).expect("encoding failed");

        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_transfer_ownership_light_client_proxy() -> Result<()> {
        test_transfer_ownership_helper(
            Contract::LightClientProxy,
            UpgradeTestOptions {
                is_mock: false,
                run_mode: RunMode::DryRun,
                upgrade_count: UpgradeCount::Once,
            },
        )
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_transfer_ownership_fee_contract_proxy() -> Result<()> {
        test_transfer_ownership_helper(
            Contract::FeeContractProxy,
            UpgradeTestOptions {
                is_mock: false,
                run_mode: RunMode::DryRun,
                upgrade_count: UpgradeCount::Once,
            },
        )
        .await
    }

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn test_transfer_ownership_fee_contract_proxy_real_proposal() -> Result<()> {
        println!("Starting test_transfer_ownership_fee_contract_proxy_real_proposal");
        tracing::info!("Starting test_transfer_ownership_fee_contract_proxy_real_proposal");

        test_transfer_ownership_helper(
            Contract::FeeContractProxy,
            UpgradeTestOptions {
                is_mock: false,
                run_mode: RunMode::RealRun,
                upgrade_count: UpgradeCount::Once,
            },
        )
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_transfer_ownership_esp_token_proxy() -> Result<()> {
        test_transfer_ownership_helper(
            Contract::EspTokenProxy,
            UpgradeTestOptions {
                is_mock: false,
                run_mode: RunMode::DryRun,
                upgrade_count: UpgradeCount::Once,
            },
        )
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_transfer_ownership_stake_table_proxy() -> Result<()> {
        test_transfer_ownership_helper(
            Contract::StakeTableProxy,
            UpgradeTestOptions {
                is_mock: false,
                run_mode: RunMode::DryRun,
                upgrade_count: UpgradeCount::Once,
            },
        )
        .await
    }

    #[test_log::test(tokio::test)]
    async fn test_get_proxy_initialized_version_initialized() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let owner = provider.get_accounts().await?[0];

        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            owner,
            owner,
            U256::from(10_000_000u64),
            "Test Token",
            "TEST",
        )
        .await?;

        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;
        let exit_escrow_period = U256::from(1000);

        let stake_table_proxy_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            exit_escrow_period,
            owner,
        )
        .await?;

        let version = get_proxy_initialized_version(&provider, stake_table_proxy_addr).await?;
        assert_eq!(version, 1, "Initialized proxy should return version 1");

        Ok(())
    }

    #[test_log::test(tokio::test)]
    async fn test_get_proxy_initialized_version_reinitialized() -> Result<()> {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let mut contracts = Contracts::new();
        let owner = provider.get_accounts().await?[0];

        let token_addr = deploy_token_proxy(
            &provider,
            &mut contracts,
            owner,
            owner,
            U256::from(10_000_000u64),
            "Test Token",
            "TEST",
        )
        .await?;

        let lc_addr = deploy_light_client_contract(&provider, &mut contracts, false).await?;
        let exit_escrow_period = U256::from(1000);

        let stake_table_proxy_addr = deploy_stake_table_proxy(
            &provider,
            &mut contracts,
            token_addr,
            lc_addr,
            exit_escrow_period,
            owner,
        )
        .await?;

        let pauser = Address::random();
        let admin = Address::random();
        upgrade_stake_table_v2(&provider, &mut contracts, pauser, admin).await?;

        let version = get_proxy_initialized_version(&provider, stake_table_proxy_addr).await?;
        assert_eq!(version, 2, "Reinitialized proxy should return version 2");

        Ok(())
    }
}
