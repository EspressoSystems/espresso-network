use std::{collections::HashSet, str::FromStr};

use alloy::{
    primitives::{Address, B256, U256},
    providers::Provider,
    rpc::types::Filter,
    sol,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hotshot_contract_adapter::sol_types::{
    EspToken as EspTokenBinding, FeeContract as FeeContractBinding, ISafe, IVersioned,
    LightClient as LightClientBinding, LightClientStateSol, LightClientV2 as LightClientV2Binding,
    LightClientV3 as LightClientV3Binding, OpsTimelock as OpsTimelockBinding,
    RewardClaim as RewardClaimBinding, SafeExitTimelock as SafeExitTimelockBinding,
    StakeTable as StakeTableBinding, StakeTableStateSol, StakeTableV2 as StakeTableV2Binding,
};
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    interface IAccessControl {
        function hasRole(bytes32 role, address account) external view returns (bool);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightClientV1 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
    pub genesis_state: LightClientStateSol,
    pub genesis_stake_table_state: StakeTableStateSol,
    pub state_history_retention_period: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightClientV2 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
    pub genesis_state: LightClientStateSol,
    pub genesis_stake_table_state: StakeTableStateSol,
    pub state_history_retention_period: u32,
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightClientV3 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
    pub genesis_state: LightClientStateSol,
    pub genesis_stake_table_state: StakeTableStateSol,
    pub state_history_retention_period: u32,
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "version")]
pub enum LightClient {
    #[serde(rename = "1")]
    V1(LightClientV1),
    #[serde(rename = "2")]
    V2(LightClientV2),
    #[serde(rename = "3")]
    V3(LightClientV3),
}

impl LightClient {
    pub fn proxy_address(&self) -> Address {
        match self {
            Self::V1(v) => v.proxy_address,
            Self::V2(v) => v.proxy_address,
            Self::V3(v) => v.proxy_address,
        }
    }

    pub fn implementation_address(&self) -> Address {
        match self {
            Self::V1(v) => v.implementation_address,
            Self::V2(v) => v.implementation_address,
            Self::V3(v) => v.implementation_address,
        }
    }

    pub fn owner(&self) -> Address {
        match self {
            Self::V1(v) => v.owner,
            Self::V2(v) => v.owner,
            Self::V3(v) => v.owner,
        }
    }

    pub fn version_string(&self) -> &'static str {
        match self {
            Self::V1(_) => "1",
            Self::V2(_) => "2",
            Self::V3(_) => "3",
        }
    }
}

impl LightClientV1 {
    pub fn upgrade(
        self,
        new_implementation: Address,
        blocks_per_epoch: u64,
        epoch_start_block: u64,
    ) -> LightClientV2 {
        LightClientV2 {
            proxy_address: self.proxy_address,
            implementation_address: new_implementation,
            owner: self.owner,
            genesis_state: self.genesis_state,
            genesis_stake_table_state: self.genesis_stake_table_state,
            state_history_retention_period: self.state_history_retention_period,
            blocks_per_epoch,
            epoch_start_block,
        }
    }
}

impl LightClientV2 {
    pub fn upgrade(self, new_implementation: Address) -> LightClientV3 {
        LightClientV3 {
            proxy_address: self.proxy_address,
            implementation_address: new_implementation,
            owner: self.owner,
            genesis_state: self.genesis_state,
            genesis_stake_table_state: self.genesis_stake_table_state,
            state_history_retention_period: self.state_history_retention_period,
            blocks_per_epoch: self.blocks_per_epoch,
            epoch_start_block: self.epoch_start_block,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StakeTableV1 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
    pub token_address: Address,
    pub exit_escrow_period: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StakeTableV2 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
    pub token_address: Address,
    pub exit_escrow_period: u64,
    pub pauser_role: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "version")]
pub enum StakeTable {
    #[serde(rename = "1")]
    V1(StakeTableV1),
    #[serde(rename = "2")]
    V2(StakeTableV2),
}

impl StakeTable {
    pub fn proxy_address(&self) -> Address {
        match self {
            Self::V1(v) => v.proxy_address,
            Self::V2(v) => v.proxy_address,
        }
    }

    pub fn implementation_address(&self) -> Address {
        match self {
            Self::V1(v) => v.implementation_address,
            Self::V2(v) => v.implementation_address,
        }
    }

    pub fn owner(&self) -> Address {
        match self {
            Self::V1(v) => v.owner,
            Self::V2(v) => v.owner,
        }
    }

    pub fn version_string(&self) -> &'static str {
        match self {
            Self::V1(_) => "1",
            Self::V2(_) => "2",
        }
    }
}

impl StakeTableV1 {
    pub fn upgrade(self, new_implementation: Address, pauser_role: Address) -> StakeTableV2 {
        StakeTableV2 {
            proxy_address: self.proxy_address,
            implementation_address: new_implementation,
            owner: self.owner,
            token_address: self.token_address,
            exit_escrow_period: self.exit_escrow_period,
            pauser_role,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EspTokenV1 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EspTokenV2 {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "version")]
pub enum EspToken {
    #[serde(rename = "1")]
    V1(EspTokenV1),
    #[serde(rename = "2")]
    V2(EspTokenV2),
}

impl EspToken {
    pub fn proxy_address(&self) -> Address {
        match self {
            Self::V1(v) => v.proxy_address,
            Self::V2(v) => v.proxy_address,
        }
    }

    pub fn implementation_address(&self) -> Address {
        match self {
            Self::V1(v) => v.implementation_address,
            Self::V2(v) => v.implementation_address,
        }
    }

    pub fn owner(&self) -> Address {
        match self {
            Self::V1(v) => v.owner,
            Self::V2(v) => v.owner,
        }
    }

    pub fn version_string(&self) -> &'static str {
        match self {
            Self::V1(_) => "1",
            Self::V2(_) => "2",
        }
    }
}

impl EspTokenV1 {
    pub fn upgrade(self, new_implementation: Address) -> EspTokenV2 {
        EspTokenV2 {
            proxy_address: self.proxy_address,
            implementation_address: new_implementation,
            owner: self.owner,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeContract {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub owner: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewardClaim {
    pub proxy_address: Address,
    pub implementation_address: Address,
    pub admin: Address,
    pub pauser_role: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Multisig {
    pub address: Address,
    pub owners: Vec<Address>,
    pub threshold: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpsTimelock {
    pub address: Address,
    pub min_delay: u64,
    pub admin: Address,
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SafeExitTimelock {
    pub address: Address,
    pub min_delay: u64,
    pub admin: Address,
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentState {
    pub light_client: Option<LightClient>,
    pub stake_table: Option<StakeTable>,
    pub esp_token: Option<EspToken>,
    pub fee_contract: Option<FeeContract>,
    pub reward_claim: Option<RewardClaim>,
    pub multisig: Option<Multisig>,
    pub ops_timelock: Option<OpsTimelock>,
    pub safe_exit_timelock: Option<SafeExitTimelock>,
    pub chain_id: u64,
}

pub struct FromOnchainConfig {
    pub light_client_proxy: Option<Address>,
    pub stake_table_proxy: Option<Address>,
    pub esp_token_proxy: Option<Address>,
    pub fee_contract_proxy: Option<Address>,
    pub reward_claim_proxy: Option<Address>,
    pub multisig: Option<Address>,
    pub ops_timelock: Option<Address>,
    pub safe_exit_timelock: Option<Address>,
}

pub enum ContractId {
    LightClient,
    StakeTable,
    EspToken,
    FeeContract,
    RewardClaim,
}

pub enum Action {
    DeployEspToken(DeployEspTokenParams),
    DeployStakeTable(DeployStakeTableParams),
    DeployLightClient(DeployLightClientParams),
    DeployFeeContract(DeployFeeContractParams),
    DeployRewardClaim(DeployRewardClaimParams),
    DeployOpsTimelock(DeployOpsTimelockParams),
    DeploySafeExitTimelock(DeploySafeExitTimelockParams),
    SetMultisig(SetMultisigParams),

    UpgradeLightClientV1ToV2(UpgradeLightClientV1ToV2Params),
    UpgradeLightClientV2ToV3(UpgradeLightClientV2ToV3Params),
    UpgradeStakeTableV1ToV2(UpgradeStakeTableV1ToV2Params),
    UpgradeEspTokenV1ToV2(UpgradeEspTokenV1ToV2Params),
    UpgradeFeeContract(UpgradeFeeContractParams),
    UpgradeRewardClaim(UpgradeRewardClaimParams),

    TransferOwnership(TransferOwnershipParams),
    SetStateHistoryRetentionPeriod(SetRetentionParams),
    UpdateEpochStartBlock(UpdateEpochParams),
    SetPermissionedProver(SetPermissionedProverParams),
    DisablePermissionedProverMode(DisablePermissionedProverParams),
}

pub struct DeployEspTokenParams {
    pub owner: Address,
    pub name: String,
    pub symbol: String,
    pub initial_supply: U256,
}

pub struct DeployStakeTableParams {
    pub owner: Address,
    pub token_address: Address,
    pub exit_escrow_period: u64,
    pub pauser_role: Address,
}

pub struct DeployLightClientParams {
    pub owner: Address,
    pub genesis_state: LightClientStateSol,
    pub genesis_stake_table_state: StakeTableStateSol,
    pub state_history_retention_period: u32,
    pub permissioned_prover: Option<Address>,
}

pub struct DeployFeeContractParams {
    pub owner: Address,
}

pub struct DeployRewardClaimParams {
    pub owner: Address,
}

pub struct DeployOpsTimelockParams {
    pub min_delay: U256,
    pub admin: Address,
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

pub struct DeploySafeExitTimelockParams {
    pub min_delay: U256,
    pub admin: Address,
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

pub struct SetMultisigParams {
    pub address: Address,
}

pub struct UpgradeLightClientV1ToV2Params {
    pub new_implementation: Address,
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

pub struct UpgradeLightClientV2ToV3Params {
    pub new_implementation: Address,
}

pub struct UpgradeStakeTableV1ToV2Params {
    pub new_implementation: Address,
    pub pauser_role: Address,
}

pub struct UpgradeEspTokenV1ToV2Params {
    pub new_implementation: Address,
}

pub struct UpgradeFeeContractParams {
    pub new_implementation: Address,
}

pub struct UpgradeRewardClaimParams {
    pub new_implementation: Address,
}

pub struct TransferOwnershipParams {
    pub contract: ContractId,
    pub new_owner: Address,
}

pub struct SetRetentionParams {
    pub new_period: u32,
}

pub struct UpdateEpochParams {
    pub new_epoch_start_block: u64,
}

pub struct SetPermissionedProverParams {
    pub prover: Address,
}

pub struct DisablePermissionedProverParams {}

#[async_trait]
pub trait FromOnchain: Sized {
    type Config;

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        address: Address,
        config: Self::Config,
    ) -> Result<Self>;
}

async fn read_erc1967_implementation<P: Provider + Sync>(
    provider: &P,
    proxy_address: Address,
) -> Result<Address> {
    let slot =
        B256::from_str("0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc")?;
    let value = provider.get_storage_at(proxy_address, slot.into()).await?;
    let bytes = value.to_be_bytes::<32>();
    Ok(Address::from_slice(&bytes[12..]))
}

async fn query_timelock_roles<P: Provider + Sync>(
    provider: &P,
    address: Address,
) -> Result<(Address, Vec<Address>, Vec<Address>)> {
    let role_granted_sig =
        B256::from_str("0x2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d")?;
    let role_revoked_sig =
        B256::from_str("0xf6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b")?;
    let proposer_role =
        B256::from_str("0xb09aa5aeb3702cfd50b6b62bc4532604938f21248a27a1d5ca736082b6819cc1")?;
    let executor_role =
        B256::from_str("0xd8aa0f3194971a2a116679f7c2090f6939c8d4e01a2a8d7e41d55e5351469e63")?;
    let admin_role = B256::ZERO;

    let granted_filter = Filter::new()
        .address(address)
        .event_signature(role_granted_sig)
        .from_block(0);
    let granted_logs = provider.get_logs(&granted_filter).await?;

    let revoked_filter = Filter::new()
        .address(address)
        .event_signature(role_revoked_sig)
        .from_block(0);
    let revoked_logs = provider.get_logs(&revoked_filter).await?;

    let mut proposers: HashSet<Address> = HashSet::new();
    let mut executors: HashSet<Address> = HashSet::new();
    let mut admins: HashSet<Address> = HashSet::new();

    for log in granted_logs {
        let role = log.topics()[1];
        let account = Address::from_slice(&log.topics()[2].as_slice()[12..]);
        if role == proposer_role {
            proposers.insert(account);
        } else if role == executor_role {
            executors.insert(account);
        } else if role == admin_role {
            admins.insert(account);
        }
    }

    for log in revoked_logs {
        let role = log.topics()[1];
        let account = Address::from_slice(&log.topics()[2].as_slice()[12..]);
        if role == proposer_role {
            proposers.remove(&account);
        } else if role == executor_role {
            executors.remove(&account);
        } else if role == admin_role {
            admins.remove(&account);
        }
    }

    // TimelockController grants DEFAULT_ADMIN_ROLE to itself for self-administration.
    // Filter it out to get the external admin.
    admins.remove(&address);

    let admin = if admins.len() == 1 {
        *admins.iter().next().unwrap()
    } else if admins.is_empty() {
        // Self-administered timelock (no external admin)
        address
    } else {
        return Err(anyhow!(
            "Expected at most one external admin, found {}",
            admins.len()
        ));
    };

    Ok((
        admin,
        proposers.into_iter().collect(),
        executors.into_iter().collect(),
    ))
}

/// HACK: RewardClaim stores _currentAdmin as private at slot 7.
/// TODO: Make _currentAdmin public in RewardClaim.sol so we can use a getter instead.
async fn read_reward_claim_admin<P: Provider + Sync>(
    provider: &P,
    address: Address,
) -> Result<Address> {
    let slot = U256::from(7);
    let value = provider.get_storage_at(address, slot).await?;
    let bytes = value.to_be_bytes::<32>();
    Ok(Address::from_slice(&bytes[12..]))
}

#[async_trait]
impl FromOnchain for LightClient {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        proxy_address: Address,
        _config: (),
    ) -> Result<Self> {
        let version = IVersioned::new(proxy_address, provider)
            .getVersion()
            .call()
            .await?;
        let implementation_address = read_erc1967_implementation(provider, proxy_address).await?;

        let contract = LightClientBinding::new(proxy_address, provider);
        let owner = contract.owner().call().await?;
        let genesis_state = contract.genesisState().call().await?.into();
        let genesis_stake_table_state = contract.genesisStakeTableState().call().await?.into();
        let state_history_retention_period = contract.stateHistoryRetentionPeriod().call().await?;

        match (version._0, version._1, version._2) {
            (1, ..) => Ok(Self::V1(LightClientV1 {
                proxy_address,
                implementation_address,
                owner,
                genesis_state,
                genesis_stake_table_state,
                state_history_retention_period,
            })),
            (2, ..) => {
                let v2_contract = LightClientV2Binding::new(proxy_address, provider);
                let blocks_per_epoch = v2_contract.blocksPerEpoch().call().await?;
                let epoch_start_block = v2_contract.epochStartBlock().call().await?;

                Ok(Self::V2(LightClientV2 {
                    proxy_address,
                    implementation_address,
                    owner,
                    genesis_state,
                    genesis_stake_table_state,
                    state_history_retention_period,
                    blocks_per_epoch,
                    epoch_start_block,
                }))
            },
            (3, ..) => {
                let v3_contract = LightClientV3Binding::new(proxy_address, provider);
                let blocks_per_epoch = v3_contract.blocksPerEpoch().call().await?;
                let epoch_start_block = v3_contract.epochStartBlock().call().await?;

                Ok(Self::V3(LightClientV3 {
                    proxy_address,
                    implementation_address,
                    owner,
                    genesis_state,
                    genesis_stake_table_state,
                    state_history_retention_period,
                    blocks_per_epoch,
                    epoch_start_block,
                }))
            },
            _ => Err(anyhow!(
                "Unknown LightClient version: {}.{}.{}",
                version._0,
                version._1,
                version._2
            )),
        }
    }
}

#[async_trait]
impl FromOnchain for StakeTable {
    type Config = Option<Address>;

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        proxy_address: Address,
        multisig_address: Option<Address>,
    ) -> Result<Self> {
        let version = IVersioned::new(proxy_address, provider)
            .getVersion()
            .call()
            .await?;
        let implementation_address = read_erc1967_implementation(provider, proxy_address).await?;

        let contract = StakeTableBinding::new(proxy_address, provider);
        let owner = contract.owner().call().await?;
        let token_address = contract.token().call().await?;
        let exit_escrow_period = contract.exitEscrowPeriod().call().await?.to::<u64>();

        match (version._0, version._1, version._2) {
            (1, ..) => Ok(Self::V1(StakeTableV1 {
                proxy_address,
                implementation_address,
                owner,
                token_address,
                exit_escrow_period,
            })),
            (2, ..) => {
                let pauser_role = if let Some(multisig) = multisig_address {
                    let v2_contract = StakeTableV2Binding::new(proxy_address, provider);
                    let pauser_role_hash = v2_contract.PAUSER_ROLE().call().await?;
                    let has_pauser_role = v2_contract
                        .hasRole(pauser_role_hash, multisig)
                        .call()
                        .await?;

                    if !has_pauser_role {
                        return Err(anyhow!(
                            "Multisig {} does not have PAUSER_ROLE on StakeTable",
                            multisig
                        ));
                    }

                    multisig
                } else {
                    Address::ZERO
                };

                Ok(Self::V2(StakeTableV2 {
                    proxy_address,
                    implementation_address,
                    owner,
                    token_address,
                    exit_escrow_period,
                    pauser_role,
                }))
            },
            _ => Err(anyhow!(
                "Unknown StakeTable version: {}.{}.{}",
                version._0,
                version._1,
                version._2
            )),
        }
    }
}

#[async_trait]
impl FromOnchain for EspToken {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        proxy_address: Address,
        _config: (),
    ) -> Result<Self> {
        let version = IVersioned::new(proxy_address, provider)
            .getVersion()
            .call()
            .await?;
        let implementation_address = read_erc1967_implementation(provider, proxy_address).await?;

        let contract = EspTokenBinding::new(proxy_address, provider);
        let owner = contract.owner().call().await?;

        match (version._0, version._1, version._2) {
            (1, ..) => Ok(Self::V1(EspTokenV1 {
                proxy_address,
                implementation_address,
                owner,
            })),
            (2, ..) => Ok(Self::V2(EspTokenV2 {
                proxy_address,
                implementation_address,
                owner,
            })),
            _ => Err(anyhow!(
                "Unknown EspToken version: {}.{}.{}",
                version._0,
                version._1,
                version._2
            )),
        }
    }
}

#[async_trait]
impl FromOnchain for FeeContract {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        proxy_address: Address,
        _config: (),
    ) -> Result<Self> {
        let implementation_address = read_erc1967_implementation(provider, proxy_address).await?;
        let contract = FeeContractBinding::new(proxy_address, provider);
        let owner = contract.owner().call().await?;

        Ok(Self {
            proxy_address,
            implementation_address,
            owner,
        })
    }
}

#[async_trait]
impl FromOnchain for Multisig {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        address: Address,
        _config: (),
    ) -> Result<Self> {
        let contract = ISafe::new(address, provider);
        let owners = contract.getOwners().call().await?;
        let threshold = contract.getThreshold().call().await?.to::<u64>();

        Ok(Self {
            address,
            owners,
            threshold,
        })
    }
}

#[async_trait]
impl FromOnchain for RewardClaim {
    type Config = Option<Address>;

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        proxy_address: Address,
        multisig_address: Option<Address>,
    ) -> Result<Self> {
        let implementation_address = read_erc1967_implementation(provider, proxy_address).await?;
        let admin = read_reward_claim_admin(provider, proxy_address).await?;

        let pauser_role = if let Some(multisig) = multisig_address {
            let contract = RewardClaimBinding::new(proxy_address, provider);
            let pauser_role_hash = contract.PAUSER_ROLE().call().await?;
            let has_pauser_role = contract.hasRole(pauser_role_hash, multisig).call().await?;

            if !has_pauser_role {
                return Err(anyhow!(
                    "Multisig {} does not have PAUSER_ROLE on RewardClaim",
                    multisig
                ));
            }

            multisig
        } else {
            Address::ZERO
        };

        Ok(Self {
            proxy_address,
            implementation_address,
            admin,
            pauser_role,
        })
    }
}

#[async_trait]
impl FromOnchain for OpsTimelock {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        address: Address,
        _config: (),
    ) -> Result<Self> {
        let contract = OpsTimelockBinding::new(address, provider);
        let min_delay = contract.getMinDelay().call().await?.to::<u64>();

        let (admin, proposers, executors) = query_timelock_roles(provider, address).await?;

        Ok(Self {
            address,
            min_delay,
            admin,
            proposers,
            executors,
        })
    }
}

#[async_trait]
impl FromOnchain for SafeExitTimelock {
    type Config = ();

    async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        address: Address,
        _config: (),
    ) -> Result<Self> {
        let contract = SafeExitTimelockBinding::new(address, provider);
        let min_delay = contract.getMinDelay().call().await?.to::<u64>();

        let (admin, proposers, executors) = query_timelock_roles(provider, address).await?;

        Ok(Self {
            address,
            min_delay,
            admin,
            proposers,
            executors,
        })
    }
}

impl DeploymentState {
    pub async fn from_onchain<P: Provider + Sync>(
        provider: &P,
        config: FromOnchainConfig,
        chain_id: u64,
    ) -> Result<Self> {
        let multisig = match config.multisig {
            Some(addr) => Some(Multisig::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        let multisig_address = multisig.as_ref().map(|m| m.address);

        let light_client = match config.light_client_proxy {
            Some(addr) => Some(LightClient::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        let stake_table = match config.stake_table_proxy {
            Some(addr) => Some(StakeTable::from_onchain(provider, addr, multisig_address).await?),
            None => None,
        };

        let esp_token = match config.esp_token_proxy {
            Some(addr) => Some(EspToken::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        let fee_contract = match config.fee_contract_proxy {
            Some(addr) => Some(FeeContract::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        let reward_claim = match config.reward_claim_proxy {
            Some(addr) => Some(RewardClaim::from_onchain(provider, addr, multisig_address).await?),
            None => None,
        };

        let ops_timelock = match config.ops_timelock {
            Some(addr) => Some(OpsTimelock::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        let safe_exit_timelock = match config.safe_exit_timelock {
            Some(addr) => Some(SafeExitTimelock::from_onchain(provider, addr, ()).await?),
            None => None,
        };

        Ok(Self {
            light_client,
            stake_table,
            esp_token,
            fee_contract,
            reward_claim,
            multisig,
            ops_timelock,
            safe_exit_timelock,
            chain_id,
        })
    }
}
