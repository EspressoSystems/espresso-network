use alloy::{
    hex::{FromHex, ToHexExt},
    network::TransactionBuilder,
    primitives::{Address, Bytes, U256},
    providers::Provider,
};
use anyhow::{Context, Result, anyhow};
use espresso_types::v0_1::L1Client;
use hotshot_contract_adapter::sol_types::{
    EspToken, EspTokenV2, FeeContract, LightClientV2, LightClientV2Mock, LightClientV3,
    LightClientV3Mock, OwnableUpgradeable, PlonkVerifierV2, PlonkVerifierV3, StakeTable,
    StakeTableV2,
};

use crate::{
    Contract, Contracts, LIBRARY_PLACEHOLDER_ADDRESS,
    output::{CalldataInfo, FunctionInfo},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultisigOwnerCheck {
    RequireContract,
    Skip,
}

#[derive(Clone)]
pub struct TransferOwnershipParams {
    pub new_owner: Address,
}

/// Encode `upgradeToAndCall(address,bytes)` calldata for a proxy upgrade.
pub fn encode_upgrade_calldata(
    proxy_addr: Address,
    new_impl_addr: Address,
    init_data: Bytes,
) -> Result<CalldataInfo> {
    let sig = "upgradeToAndCall(address newImplementation, bytes data)";
    let args = vec![new_impl_addr.to_string(), init_data.to_string()];
    let data = crate::encode_function_call(sig, args.clone())
        .context("Failed to encode upgradeToAndCall calldata")?;
    Ok(CalldataInfo::with_method(
        proxy_addr,
        data,
        U256::ZERO,
        FunctionInfo {
            signature: sig.to_string(),
            args,
        },
    ))
}

/// Encode `transferOwnership(address)` calldata.
pub fn encode_transfer_ownership_calldata(
    proxy_addr: Address,
    new_owner: Address,
) -> Result<CalldataInfo> {
    let sig = "transferOwnership(address newOwner)";
    let args = vec![new_owner.to_string()];
    let data = crate::encode_function_call(sig, args.clone())
        .context("Failed to encode transferOwnership calldata")?;
    Ok(CalldataInfo::with_method(
        proxy_addr,
        data,
        U256::ZERO,
        FunctionInfo {
            signature: sig.to_string(),
            args,
        },
    ))
}

/// Encode calldata for any function call.
pub fn encode_generic_calldata(
    target: Address,
    function_signature: &str,
    function_args: Vec<String>,
    value: U256,
) -> Result<CalldataInfo> {
    let data = crate::encode_function_call(function_signature, function_args.clone())
        .context("Failed to encode generic calldata")?;
    Ok(CalldataInfo::with_method(
        target,
        data,
        value,
        FunctionInfo {
            signature: function_signature.to_string(),
            args: function_args,
        },
    ))
}

pub fn transfer_ownership_from_multisig_to_timelock(
    contracts: &mut Contracts,
    contract: Contract,
    params: TransferOwnershipParams,
) -> Result<CalldataInfo> {
    tracing::info!(
        "Encoding ownership transfer for {} to timelock {}",
        contract,
        params.new_owner
    );

    let proxy_addr = match contract {
        Contract::LightClientProxy
        | Contract::FeeContractProxy
        | Contract::EspTokenProxy
        | Contract::StakeTableProxy
        | Contract::RewardClaimProxy => contracts
            .address(contract)
            .ok_or_else(|| anyhow!("{contract} (multisig owner) not found, can't upgrade"))?,
        _ => anyhow::bail!("Not a proxy contract, can't transfer ownership"),
    };
    tracing::info!("{} found at {proxy_addr:#x}", contract);

    encode_transfer_ownership_calldata(proxy_addr, params.new_owner)
}

/// Parameters for upgrading LightClient to V2
pub struct LightClientV2UpgradeParams {
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

/// Upgrade the light client proxy to use LightClientV2.
/// Deploys new implementation contracts, then returns encoded upgrade calldata.
pub async fn upgrade_light_client_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    params: LightClientV2UpgradeParams,
    is_mock: bool,
    multisig_owner_check: MultisigOwnerCheck,
) -> Result<CalldataInfo> {
    let expected_major_version: u8 = 2;

    let proxy_addr = contracts
        .address(Contract::LightClientProxy)
        .ok_or_else(|| anyhow!("LightClientProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("LightClientProxy found at {proxy_addr:#x}");

    let owner_addr = OwnableUpgradeable::new(proxy_addr, &provider)
        .owner()
        .call()
        .await?;
    if multisig_owner_check == MultisigOwnerCheck::RequireContract
        && !crate::is_contract(&provider, owner_addr).await?
    {
        anyhow::bail!(
            "LightClientProxy owner {owner_addr:#x} is not a contract (expected multisig)"
        );
    }

    let pv2_addr = contracts
        .deploy(
            Contract::PlonkVerifierV2,
            PlonkVerifierV2::deploy_builder(&provider),
        )
        .await?;

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
                ));
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

    let init_data =
        if crate::already_initialized(&provider, proxy_addr, expected_major_version).await? {
            tracing::info!(
                "Proxy was already initialized for version {}",
                expected_major_version
            );
            vec![].into()
        } else {
            tracing::info!(
                "Init Data to be signed.\n Function: initializeV2\n Arguments:\n \
                 blocks_per_epoch: {:?}\n epoch_start_block: {:?}",
                params.blocks_per_epoch,
                params.epoch_start_block
            );
            LightClientV2::new(lcv2_addr, &provider)
                .initializeV2(params.blocks_per_epoch, params.epoch_start_block)
                .calldata()
                .to_owned()
        };

    encode_upgrade_calldata(proxy_addr, lcv2_addr, init_data)
}

/// Upgrade the light client proxy to use LightClientV3.
/// Deploys new implementation contracts, then returns encoded upgrade calldata.
pub async fn upgrade_light_client_v3_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    is_mock: bool,
    multisig_owner_check: MultisigOwnerCheck,
) -> Result<CalldataInfo> {
    let expected_major_version: u8 = 3;

    let proxy_addr = contracts
        .address(Contract::LightClientProxy)
        .ok_or_else(|| anyhow!("LightClientProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("LightClientProxy found at {proxy_addr:#x}");

    let owner_addr = OwnableUpgradeable::new(proxy_addr, &provider)
        .owner()
        .call()
        .await?;
    if multisig_owner_check == MultisigOwnerCheck::RequireContract
        && !crate::is_contract(&provider, owner_addr).await?
    {
        anyhow::bail!(
            "LightClientProxy owner {owner_addr:#x} is not a contract (expected multisig)"
        );
    }

    let pv3_addr = contracts
        .deploy(
            Contract::PlonkVerifierV3,
            PlonkVerifierV3::deploy_builder(&provider),
        )
        .await?;

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
                ));
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

    let init_data =
        if crate::already_initialized(&provider, proxy_addr, expected_major_version).await? {
            tracing::info!(
                "Proxy was already initialized for version {}",
                expected_major_version
            );
            vec![].into()
        } else {
            tracing::info!(
                "Init Data to be signed.\n Function: initializeV3\n Arguments: none (V3 inherits \
                 from V2)"
            );
            LightClientV3::new(lcv3_addr, &provider)
                .initializeV3()
                .calldata()
                .to_owned()
        };

    encode_upgrade_calldata(proxy_addr, lcv3_addr, init_data)
}

/// Upgrade the EspToken proxy to use EspTokenV2.
/// Deploys new implementation, then returns encoded upgrade calldata.
pub async fn upgrade_esp_token_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    multisig_owner_check: MultisigOwnerCheck,
) -> Result<CalldataInfo> {
    let proxy_addr = contracts
        .address(Contract::EspTokenProxy)
        .ok_or_else(|| anyhow!("EspTokenProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("EspTokenProxy found at {proxy_addr:#x}");
    let proxy = EspToken::new(proxy_addr, &provider);

    // Check if already on V2
    let current_version = proxy.getVersion().call().await?;
    if current_version.majorVersion >= 2 {
        anyhow::bail!(
            "EspToken is already on V{}, no upgrade needed",
            current_version.majorVersion
        )
    }

    let owner_addr = proxy.owner().call().await?;
    if multisig_owner_check == MultisigOwnerCheck::RequireContract
        && !crate::is_contract(&provider, owner_addr).await?
    {
        anyhow::bail!("EspTokenProxy owner {owner_addr:#x} is not a contract (expected multisig)");
    }

    let esp_token_v2_addr = contracts
        .deploy(Contract::EspTokenV2, EspTokenV2::deploy_builder(&provider))
        .await?;

    let reward_claim_addr = contracts
        .address(Contract::RewardClaimProxy)
        .ok_or_else(|| anyhow!("RewardClaimProxy not found"))?;
    let proxy_as_v2 = EspTokenV2::new(proxy_addr, &provider);
    let init_data = proxy_as_v2
        .initializeV2(reward_claim_addr)
        .calldata()
        .to_owned();

    tracing::info!(
        %reward_claim_addr,
        "Data to be signed: Function: initializeV2 Arguments:"
    );

    encode_upgrade_calldata(proxy_addr, esp_token_v2_addr, init_data)
}

#[derive(Clone, Debug)]
pub struct StakeTableV2UpgradeParams {
    pub multisig_address: Address,
    pub pauser: Address,
}

/// Upgrade the stake table proxy to use StakeTableV2.
/// Deploys new implementation and returns encoded upgrade calldata.
pub async fn upgrade_stake_table_v2_multisig_owner(
    provider: impl Provider,
    l1_client: L1Client,
    contracts: &mut Contracts,
    params: StakeTableV2UpgradeParams,
    multisig_owner_check: MultisigOwnerCheck,
) -> Result<CalldataInfo> {
    tracing::info!("Upgrading StakeTableProxy to StakeTableV2 using multisig owner");
    let Some(proxy_addr) = contracts.address(Contract::StakeTableProxy) else {
        anyhow::bail!("StakeTableProxy not found, can't upgrade")
    };

    let proxy = StakeTable::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?;

    if owner_addr != params.multisig_address {
        anyhow::bail!(
            "Proxy not owned by multisig. expected: {:#x}, got: {owner_addr:#x}",
            params.multisig_address
        );
    }
    if multisig_owner_check == MultisigOwnerCheck::RequireContract
        && !crate::is_contract(&provider, owner_addr).await?
    {
        anyhow::bail!(
            "StakeTableProxy owner {owner_addr:#x} is not a contract (expected multisig)"
        );
    }

    let (_init_commissions, _init_active_stake, init_data) =
        crate::prepare_stake_table_v2_upgrade(l1_client, proxy_addr, params.pauser, owner_addr)
            .await?;

    let stake_table_v2_addr = contracts
        .deploy(
            Contract::StakeTableV2,
            StakeTableV2::deploy_builder(&provider),
        )
        .await?;

    encode_upgrade_calldata(
        proxy_addr,
        stake_table_v2_addr,
        init_data.unwrap_or_default(),
    )
}

/// Upgrade the FeeContract proxy to a new implementation (patch upgrade).
/// Deploys new implementation, then returns encoded upgrade calldata.
pub async fn upgrade_fee_contract_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    multisig_owner_check: MultisigOwnerCheck,
) -> Result<CalldataInfo> {
    let proxy_addr = contracts
        .address(Contract::FeeContractProxy)
        .ok_or_else(|| anyhow!("FeeContractProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("FeeContractProxy found at {proxy_addr:#x}");
    let proxy = FeeContract::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?;
    if multisig_owner_check == MultisigOwnerCheck::RequireContract
        && !crate::is_contract(&provider, owner_addr).await?
    {
        anyhow::bail!(
            "FeeContractProxy owner {owner_addr:#x} is not a contract (expected multisig)"
        );
    }

    let curr_version = proxy.getVersion().call().await?;
    if curr_version.majorVersion != 1 {
        anyhow::bail!(
            "Expected FeeContract V1.x for upgrade to V1.0.1, found V{}.{}.{}",
            curr_version.majorVersion,
            curr_version.minorVersion,
            curr_version.patchVersion
        );
    }

    let cached_fee_contract_addr = contracts.address(Contract::FeeContract);
    if let Some(cached_fee_contract_addr) = cached_fee_contract_addr {
        anyhow::bail!(
            "FeeContract implementation address is already set in cache ({:#x}). For patch \
             upgrades, the implementation must be redeployed. Please unset \
             ESPRESSO_FEE_CONTRACT_ADDRESS or remove it from the cache first.",
            cached_fee_contract_addr
        );
    }

    let fee_contract_addr = contracts
        .deploy(
            Contract::FeeContract,
            FeeContract::deploy_builder(&provider),
        )
        .await?;

    encode_upgrade_calldata(proxy_addr, fee_contract_addr, Bytes::new())
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, Bytes, U256};

    use super::*;

    #[test]
    fn test_encode_upgrade_calldata() {
        let proxy = Address::random();
        let impl_addr = Address::random();
        let info = encode_upgrade_calldata(proxy, impl_addr, Bytes::new()).unwrap();
        assert_eq!(info.to, proxy);
        assert!(info.data.len() > 4);
        assert_eq!(info.value, U256::ZERO);
        let fi = info.function_info.unwrap();
        assert_eq!(
            fi.signature,
            "upgradeToAndCall(address newImplementation, bytes data)"
        );
        assert_eq!(fi.args.len(), 2);
    }

    #[test]
    fn test_encode_upgrade_calldata_with_init_data() {
        let proxy = Address::random();
        let impl_addr = Address::random();
        let empty_calldata = encode_upgrade_calldata(proxy, impl_addr, Bytes::new()).unwrap();
        let with_data =
            encode_upgrade_calldata(proxy, impl_addr, Bytes::from(vec![1, 2, 3, 4])).unwrap();
        assert!(with_data.data.len() > empty_calldata.data.len());
    }

    #[test]
    fn test_encode_transfer_ownership_calldata() {
        let proxy = Address::random();
        let new_owner = Address::random();
        let info = encode_transfer_ownership_calldata(proxy, new_owner).unwrap();
        assert_eq!(info.to, proxy);
        assert!(info.data.len() > 4);
        assert_eq!(info.value, U256::ZERO);
        let fi = info.function_info.unwrap();
        assert_eq!(fi.signature, "transferOwnership(address newOwner)");
        assert_eq!(fi.args, vec![new_owner.to_string()]);
    }

    #[test]
    fn test_encode_generic_calldata() {
        let target = Address::random();
        let addr = Address::random();
        let info = encode_generic_calldata(
            target,
            "transfer(address to, uint256 amount)",
            vec![addr.to_string(), "1000".to_string()],
            U256::ZERO,
        )
        .unwrap();
        assert_eq!(info.to, target);
        assert!(info.data.len() > 4);
        let fi = info.function_info.unwrap();
        assert_eq!(fi.signature, "transfer(address to, uint256 amount)");
        assert_eq!(fi.args, vec![addr.to_string(), "1000".to_string()]);
    }

    #[test]
    fn test_encode_generic_calldata_arg_mismatch() {
        let target = Address::random();
        let result = encode_generic_calldata(
            target,
            "transfer(address to, uint256 amount)",
            vec!["0x000000000000000000000000000000000000dead".to_string()], // missing arg
            U256::ZERO,
        );
        assert!(result.is_err());
    }
}
