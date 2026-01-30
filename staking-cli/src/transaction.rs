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
use anyhow::{bail, Context, Result};
use hotshot_contract_adapter::{
    evm::DecodeRevert,
    sol_types::{
        EdOnBN254PointSol,
        EspToken::{approveCall, transferCall, EspTokenErrors},
        G1PointSol, G2PointSol,
        RewardClaim::{claimRewardsCall, RewardClaimErrors},
        StakeTableV2::{
            claimValidatorExitCall, claimWithdrawalCall, delegateCall, deregisterValidatorCall,
            registerValidatorCall, registerValidatorV2Call, undelegateCall, updateCommissionCall,
            updateConsensusKeysCall, updateConsensusKeysV2Call, updateMetadataUriCall,
            StakeTableV2Errors,
        },
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
    Transfer {
        token: Address,
        to: Address,
        amount: U256,
    },
}

impl Transaction {
    /// Returns the contract address and encoded calldata for this state change.
    pub fn calldata(self) -> (Address, Bytes) {
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
            ),
            Self::Delegate {
                stake_table,
                validator,
                amount,
            } => (
                stake_table,
                delegateCall { validator, amount }.abi_encode().into(),
            ),
            Self::Undelegate {
                stake_table,
                validator,
                amount,
            } => (
                stake_table,
                undelegateCall { validator, amount }.abi_encode().into(),
            ),
            Self::ClaimWithdrawal {
                stake_table,
                validator,
            } => (
                stake_table,
                claimWithdrawalCall { validator }.abi_encode().into(),
            ),
            Self::ClaimValidatorExit {
                stake_table,
                validator,
            } => (
                stake_table,
                claimValidatorExitCall { validator }.abi_encode().into(),
            ),
            Self::ClaimRewards {
                reward_claim,
                lifetime_rewards,
                auth_data,
            } => (
                reward_claim,
                claimRewardsCall {
                    lifetimeRewards: lifetime_rewards,
                    authData: auth_data,
                }
                .abi_encode()
                .into(),
            ),
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
                ),
            },
            Self::DeregisterValidator { stake_table } => {
                (stake_table, deregisterValidatorCall {}.abi_encode().into())
            },
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
            ),
            Self::Transfer { token, to, amount } => (
                token,
                transferCall { to, value: amount }.abi_encode().into(),
            ),
        }
    }

    fn to_transaction_request(&self) -> TransactionRequest {
        let (to, data) = self.clone().calldata();
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
            | Self::UpdateMetadataUri { .. } => result.maybe_decode_revert::<StakeTableV2Errors>(),
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
        match self {
            Self::Approve {
                spender, amount, ..
            } => {
                tracing::info!("approve {} for {}", format_esp(*amount), spender);
            },
            Self::Delegate {
                validator, amount, ..
            } => {
                tracing::info!("delegate {} to {}", format_esp(*amount), validator);
            },
            Self::Undelegate {
                validator, amount, ..
            } => {
                tracing::info!("undelegate {} from {}", format_esp(*amount), validator);
            },
            Self::ClaimWithdrawal { validator, .. } => {
                tracing::info!("claiming withdrawal for {}", validator);
            },
            Self::ClaimValidatorExit { validator, .. } => {
                tracing::info!("claiming validator exit for {}", validator);
            },
            Self::ClaimRewards { reward_claim, .. } => {
                tracing::info!("claiming rewards from {}", reward_claim);
            },
            Self::RegisterValidator {
                payload,
                commission,
                ..
            } => {
                tracing::info!(
                    "register validator {} with commission {}",
                    payload.address,
                    commission
                );
            },
            Self::UpdateConsensusKeys { payload, .. } => {
                tracing::info!("updating consensus keys for {}", payload.address);
            },
            Self::DeregisterValidator { .. } => {
                tracing::info!("deregistering validator");
            },
            Self::UpdateCommission { new_commission, .. } => {
                tracing::info!("updating commission to {}", new_commission);
            },
            Self::UpdateMetadataUri { .. } => {
                tracing::info!("updating metadata URI");
            },
            Self::Transfer { to, amount, .. } => {
                tracing::info!("transferring {} to {}", format_esp(*amount), to);
            },
        }
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
