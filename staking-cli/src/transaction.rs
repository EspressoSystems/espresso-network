//! State-changing operations for staking.
//!
//! The [`Transaction`] enum provides a single dispatch point for all state-changing operations.
//! Each variant uses the same [`Transaction::calldata`] method for both execute and export modes,
//! ensuring identical calldata generation.

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, U256},
    providers::{PendingTransactionBuilder, Provider},
    rpc::types::{TransactionInput, TransactionRequest},
    sol_types::SolCall,
};
use anyhow::{Context, Result, bail};
use espresso_safe_tx_builder::FunctionInfo;
use hotshot_contract_adapter::{
    evm::DecodeRevert,
    sol_types::{
        EdOnBN254PointSol,
        EspToken::{EspTokenErrors, approveCall, transferCall},
        G1PointSol, G2PointSol,
        RewardClaim::{RewardClaimErrors, claimRewardsCall},
        StakeTableV2::{
            StakeTableV2Errors, claimValidatorExitCall, claimWithdrawalCall, delegateCall,
            deregisterValidatorCall, registerValidatorCall, registerValidatorV2Call,
            undelegateCall, updateCommissionCall, updateConsensusKeysCall,
            updateConsensusKeysV2Call, updateMetadataUriCall,
        },
        StakeTableV3::{setNetworkConfigCall, updateP2pAddrCall},
    },
    stake_table::{StakeTableContractVersion, StateSignatureSol},
};

use crate::{
    metadata::MetadataUri, output::format_esp, parse::Commission, signature::NodeSignatures,
};

#[derive(Clone)]
pub enum Transaction {
    Approve {
        token: Address,
        spender: Address,
        amount: U256,
    },
    Delegate {
        stake_table: Address,
        validator: Address,
        amount: U256,
    },
    Undelegate {
        stake_table: Address,
        validator: Address,
        amount: U256,
    },
    ClaimWithdrawal {
        stake_table: Address,
        validator: Address,
    },
    ClaimValidatorExit {
        stake_table: Address,
        validator: Address,
    },
    ClaimRewards {
        reward_claim: Address,
        lifetime_rewards: U256,
        auth_data: Bytes,
    },
    RegisterValidator {
        stake_table: Address,
        commission: Commission,
        metadata_uri: MetadataUri,
        payload: NodeSignatures,
        version: StakeTableContractVersion,
    },
    UpdateConsensusKeys {
        stake_table: Address,
        payload: NodeSignatures,
        version: StakeTableContractVersion,
    },
    DeregisterValidator {
        stake_table: Address,
    },
    UpdateCommission {
        stake_table: Address,
        new_commission: Commission,
    },
    UpdateMetadataUri {
        stake_table: Address,
        metadata_uri: MetadataUri,
    },
    SetNetworkConfig {
        stake_table: Address,
        x25519_key: alloy::primitives::FixedBytes<32>,
        p2p_addr: String,
    },
    UpdateP2pAddr {
        stake_table: Address,
        p2p_addr: String,
    },
    Transfer {
        token: Address,
        to: Address,
        amount: U256,
    },
}

impl Transaction {
    /// Returns the contract address, encoded calldata, and optional function info for this state
    /// change. Function info is `None` for calls with struct arguments that cannot be represented
    /// as simple string values for Safe TX Builder.
    pub fn calldata(self) -> (Address, Bytes, Option<FunctionInfo>) {
        match self {
            Self::Approve {
                token,
                spender,
                amount,
            } => (
                token,
                approveCall {
                    spender,
                    value: amount,
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "approve(address spender, uint256 value)".to_string(),
                    args: vec![spender.to_string(), amount.to_string()],
                }),
            ),
            Self::Delegate {
                stake_table,
                validator,
                amount,
            } => (
                stake_table,
                delegateCall { validator, amount }.abi_encode().into(),
                Some(FunctionInfo {
                    signature: "delegate(address validator, uint256 amount)".to_string(),
                    args: vec![validator.to_string(), amount.to_string()],
                }),
            ),
            Self::Undelegate {
                stake_table,
                validator,
                amount,
            } => (
                stake_table,
                undelegateCall { validator, amount }.abi_encode().into(),
                Some(FunctionInfo {
                    signature: "undelegate(address validator, uint256 amount)".to_string(),
                    args: vec![validator.to_string(), amount.to_string()],
                }),
            ),
            Self::ClaimWithdrawal {
                stake_table,
                validator,
            } => (
                stake_table,
                claimWithdrawalCall { validator }.abi_encode().into(),
                Some(FunctionInfo {
                    signature: "claimWithdrawal(address validator)".to_string(),
                    args: vec![validator.to_string()],
                }),
            ),
            Self::ClaimValidatorExit {
                stake_table,
                validator,
            } => (
                stake_table,
                claimValidatorExitCall { validator }.abi_encode().into(),
                Some(FunctionInfo {
                    signature: "claimValidatorExit(address validator)".to_string(),
                    args: vec![validator.to_string()],
                }),
            ),
            Self::ClaimRewards {
                reward_claim,
                lifetime_rewards,
                auth_data,
            } => (
                reward_claim,
                claimRewardsCall {
                    lifetimeRewards: lifetime_rewards,
                    authData: auth_data.clone(),
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "claimRewards(uint256 lifetimeRewards, bytes authData)".to_string(),
                    args: vec![lifetime_rewards.to_string(), auth_data.to_string()],
                }),
            ),
            // RegisterValidator and UpdateConsensusKeys use complex struct args (BLS/Schnorr keys)
            // that cannot be represented as simple strings for Safe TX Builder.
            Self::RegisterValidator {
                stake_table,
                commission,
                metadata_uri,
                payload,
                version,
            } => match version {
                StakeTableContractVersion::V1 => (
                    stake_table,
                    registerValidatorCall::from((
                        G2PointSol::from(payload.bls_vk),
                        EdOnBN254PointSol::from(payload.schnorr_vk),
                        G1PointSol::from(payload.bls_signature).into(),
                        commission.to_evm(),
                    ))
                    .abi_encode()
                    .into(),
                    None,
                ),
                StakeTableContractVersion::V2 => (
                    stake_table,
                    registerValidatorV2Call::from((
                        G2PointSol::from(payload.bls_vk),
                        EdOnBN254PointSol::from(payload.schnorr_vk),
                        G1PointSol::from(payload.bls_signature).into(),
                        StateSignatureSol::from(payload.schnorr_signature).into(),
                        commission.to_evm(),
                        metadata_uri.to_string(),
                    ))
                    .abi_encode()
                    .into(),
                    None,
                ),
            },
            Self::UpdateConsensusKeys {
                stake_table,
                payload,
                version,
            } => match version {
                StakeTableContractVersion::V1 => (
                    stake_table,
                    updateConsensusKeysCall::from((
                        G2PointSol::from(payload.bls_vk),
                        EdOnBN254PointSol::from(payload.schnorr_vk),
                        G1PointSol::from(payload.bls_signature).into(),
                    ))
                    .abi_encode()
                    .into(),
                    None,
                ),
                StakeTableContractVersion::V2 => (
                    stake_table,
                    updateConsensusKeysV2Call::from((
                        G2PointSol::from(payload.bls_vk),
                        EdOnBN254PointSol::from(payload.schnorr_vk),
                        G1PointSol::from(payload.bls_signature).into(),
                        StateSignatureSol::from(payload.schnorr_signature).into(),
                    ))
                    .abi_encode()
                    .into(),
                    None,
                ),
            },
            Self::DeregisterValidator { stake_table } => (
                stake_table,
                deregisterValidatorCall {}.abi_encode().into(),
                Some(FunctionInfo {
                    signature: deregisterValidatorCall::SIGNATURE.to_string(),
                    args: vec![],
                }),
            ),
            Self::UpdateCommission {
                stake_table,
                new_commission,
            } => (
                stake_table,
                updateCommissionCall {
                    newCommission: new_commission.to_evm(),
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "updateCommission(uint16 newCommission)".to_string(),
                    args: vec![new_commission.to_evm().to_string()],
                }),
            ),
            Self::UpdateMetadataUri {
                stake_table,
                metadata_uri,
            } => (
                stake_table,
                updateMetadataUriCall {
                    metadataUri: metadata_uri.to_string(),
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "updateMetadataUri(string metadataUri)".to_string(),
                    args: vec![metadata_uri.to_string()],
                }),
            ),
            Self::SetNetworkConfig {
                stake_table,
                x25519_key,
                p2p_addr,
            } => (
                stake_table,
                setNetworkConfigCall {
                    x25519Key: x25519_key,
                    p2pAddr: p2p_addr.clone(),
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "setNetworkConfig(bytes32 x25519Key, string p2pAddr)".to_string(),
                    args: vec![x25519_key.to_string(), p2p_addr],
                }),
            ),
            Self::UpdateP2pAddr {
                stake_table,
                p2p_addr,
            } => (
                stake_table,
                updateP2pAddrCall {
                    p2pAddr: p2p_addr.clone(),
                }
                .abi_encode()
                .into(),
                Some(FunctionInfo {
                    signature: "updateP2pAddr(string p2pAddr)".to_string(),
                    args: vec![p2p_addr],
                }),
            ),
            Self::Transfer { token, to, amount } => (
                token,
                transferCall { to, value: amount }.abi_encode().into(),
                Some(FunctionInfo {
                    signature: "transfer(address to, uint256 value)".to_string(),
                    args: vec![to.to_string(), amount.to_string()],
                }),
            ),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Approve {
                spender, amount, ..
            } => format!("Approve {} ESP for {}", format_esp(*amount), spender),
            Self::Delegate {
                validator, amount, ..
            } => format!(
                "Delegate {} ESP to validator {}",
                format_esp(*amount),
                validator
            ),
            Self::Undelegate {
                validator, amount, ..
            } => format!(
                "Undelegate {} ESP from validator {}",
                format_esp(*amount),
                validator
            ),
            Self::ClaimWithdrawal { validator, .. } => {
                format!("Claim withdrawal for validator {}", validator)
            },
            Self::ClaimValidatorExit { validator, .. } => {
                format!("Claim validator exit for {}", validator)
            },
            Self::ClaimRewards { reward_claim, .. } => {
                format!("Claim rewards from {}", reward_claim)
            },
            Self::RegisterValidator {
                payload,
                commission,
                ..
            } => format!(
                "Register validator {} with {} commission",
                payload.address, commission
            ),
            Self::UpdateConsensusKeys { payload, .. } => {
                format!("Update consensus keys for {}", payload.address)
            },
            Self::DeregisterValidator { .. } => "Deregister validator".to_string(),
            Self::UpdateCommission { new_commission, .. } => {
                format!("Update commission to {}", new_commission)
            },
            Self::UpdateMetadataUri { .. } => "Update metadata URI".to_string(),
            Self::SetNetworkConfig { .. } => {
                "Set network config (x25519 key and p2p address)".to_string()
            },
            Self::UpdateP2pAddr { p2p_addr, .. } => {
                format!("Update p2p address to {}", p2p_addr)
            },
            Self::Transfer { to, amount, .. } => {
                format!("Transfer {} ESP to {}", format_esp(*amount), to)
            },
        }
    }

    fn to_transaction_request(&self) -> TransactionRequest {
        let (to, data, _) = self.clone().calldata();
        TransactionRequest::default()
            .to(to)
            .input(TransactionInput::new(data))
    }

    /// Validates the delegate amount against the minimum required by the contract.
    ///
    /// Currently only validates `Delegate` transactions; other variants pass through.
    pub async fn validate_delegate_amount(&self, provider: &impl Provider) -> Result<()> {
        if let Self::Delegate {
            stake_table,
            amount,
            ..
        } = self
        {
            use hotshot_contract_adapter::sol_types::StakeTableV2;
            let st = StakeTableV2::new(*stake_table, provider);
            let version: StakeTableContractVersion = st.getVersion().call().await?.try_into()?;
            if let StakeTableContractVersion::V2 = version {
                let min_amount = st.minDelegateAmount().call().await?;
                if amount < &min_amount {
                    bail!(
                        "delegation amount {} is below minimum of {}",
                        format_esp(*amount),
                        format_esp(min_amount)
                    );
                }
            }
        }
        Ok(())
    }

    fn decode_revert<T>(&self, result: impl DecodeRevert<T>) -> Result<T> {
        match self {
            Self::Approve { .. } | Self::Transfer { .. } => {
                result.maybe_decode_revert::<EspTokenErrors>()
            },
            Self::ClaimRewards { .. } => result.maybe_decode_revert::<RewardClaimErrors>(),
            Self::Delegate { .. }
            | Self::Undelegate { .. }
            | Self::ClaimWithdrawal { .. }
            | Self::ClaimValidatorExit { .. }
            | Self::RegisterValidator { .. }
            | Self::UpdateConsensusKeys { .. }
            | Self::DeregisterValidator { .. }
            | Self::UpdateCommission { .. }
            | Self::UpdateMetadataUri { .. }
            | Self::SetNetworkConfig { .. }
            | Self::UpdateP2pAddr { .. } => result.maybe_decode_revert::<StakeTableV2Errors>(),
        }
    }

    pub async fn simulate(&self, provider: &impl Provider, from: Address) -> Result<()> {
        let tx = self.to_transaction_request().from(from);
        let result = provider.call(tx).await;
        self.decode_revert(result)
            .context("Transaction simulation failed")?;
        Ok(())
    }

    fn log_intent(&self) {
        tracing::info!("{}", self.description());
    }

    pub async fn send(
        &self,
        provider: impl Provider,
    ) -> Result<PendingTransactionBuilder<Ethereum>> {
        self.log_intent();
        let tx = self.to_transaction_request();
        let pending = provider.send_transaction(tx).await;
        self.decode_revert(pending)
    }
}

#[cfg(test)]
mod tests {
    use alloy::{json_abi::Function, sol_types::SolCall};

    use super::*;

    /// Verify hand-written named signatures produce the same 4-byte selector as alloy's
    /// `SIGNATURE` const. Catches typos or drift if the Solidity ABI changes.
    #[test]
    fn named_signatures_match_selectors() {
        let cases: &[(&str, [u8; 4])] = &[
            (
                "approve(address spender, uint256 value)",
                approveCall::SELECTOR,
            ),
            (
                "delegate(address validator, uint256 amount)",
                delegateCall::SELECTOR,
            ),
            (
                "undelegate(address validator, uint256 amount)",
                undelegateCall::SELECTOR,
            ),
            (
                "claimWithdrawal(address validator)",
                claimWithdrawalCall::SELECTOR,
            ),
            (
                "claimValidatorExit(address validator)",
                claimValidatorExitCall::SELECTOR,
            ),
            (
                "claimRewards(uint256 lifetimeRewards, bytes authData)",
                claimRewardsCall::SELECTOR,
            ),
            (
                "updateCommission(uint16 newCommission)",
                updateCommissionCall::SELECTOR,
            ),
            (
                "updateMetadataUri(string metadataUri)",
                updateMetadataUriCall::SELECTOR,
            ),
            (
                "transfer(address to, uint256 value)",
                transferCall::SELECTOR,
            ),
            (
                "setNetworkConfig(bytes32 x25519Key, string p2pAddr)",
                setNetworkConfigCall::SELECTOR,
            ),
            ("updateP2pAddr(string p2pAddr)", updateP2pAddrCall::SELECTOR),
        ];

        for (named_sig, expected_selector) in cases {
            let func = Function::parse(named_sig)
                .unwrap_or_else(|e| panic!("failed to parse '{named_sig}': {e}"));
            assert_eq!(
                func.selector().as_slice(),
                expected_selector,
                "selector mismatch for '{named_sig}'"
            );
        }
    }
}
