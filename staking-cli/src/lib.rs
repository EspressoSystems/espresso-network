use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{utils::parse_ether, Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use clap_serde_derive::ClapSerde;
use demo::DelegationConfig;
use espresso_contract_deployer::provider::connect_ledger;
pub(crate) use hotshot_types::{light_client::StateSignKey, signature_key::BLSPrivKey};
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use parse::Commission;
use sequencer_utils::logging;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod claim;
pub mod delegation;
pub mod demo;
pub mod info;
pub mod l1;
pub mod parse;
pub mod registration;
pub mod signature;

pub mod deploy;

pub const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// CLI to interact with the Espresso stake table contract
#[derive(ClapSerde, Clone, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// L1 Ethereum RPC.
    #[clap(long, env = "L1_PROVIDER")]
    #[default(Url::parse("http://localhost:8545").unwrap())]
    pub rpc_url: Url,

    /// [DEPRECATED] Deployed ESP token contract address.
    ///
    /// [DEPRECATED] This is fetched from the stake table contract now.
    #[clap(long, env = "ESP_TOKEN_ADDRESS")]
    pub token_address: Option<Address>,

    /// Deployed stake table contract address.
    #[clap(long, env = "STAKE_TABLE_ADDRESS")]
    pub stake_table_address: Address,

    #[clap(flatten)]
    pub signer: SignerConfig,

    #[clap(flatten)]
    #[serde(skip)]
    pub logging: logging::Config,

    #[command(subcommand)]
    #[serde(skip)]
    pub commands: Commands,
}

#[derive(ClapSerde, Parser, Clone, Debug, Deserialize, Serialize)]
pub struct SignerConfig {
    /// The mnemonic to use when deriving the key.
    #[clap(long, env = "MNEMONIC")]
    pub mnemonic: Option<String>,

    /// The mnemonic account index to use when deriving the key.
    #[clap(long, env = "ACCOUNT_INDEX")]
    #[default(Some(0))]
    pub account_index: Option<u32>,

    /// Use a ledger device to sign transactions.
    ///
    /// NOTE: ledger must be unlocked, Ethereum app open and blind signing must be enabled in the
    /// Ethereum app settings.
    #[clap(long, env = "USE_LEDGER")]
    pub ledger: bool,
}

#[derive(Clone, Debug)]
pub enum ValidSignerConfig {
    Mnemonic {
        mnemonic: String,
        account_index: u32,
    },
    Ledger {
        account_index: usize,
    },
}

impl TryFrom<SignerConfig> for ValidSignerConfig {
    type Error = anyhow::Error;

    fn try_from(config: SignerConfig) -> Result<Self> {
        let account_index = config
            .account_index
            .ok_or_else(|| anyhow::anyhow!("Account index must be provided"))?;
        if let Some(mnemonic) = config.mnemonic {
            Ok(ValidSignerConfig::Mnemonic {
                mnemonic,
                account_index,
            })
        } else if config.ledger {
            Ok(ValidSignerConfig::Ledger {
                account_index: account_index as usize,
            })
        } else {
            bail!("Either mnemonic or --ledger flag must be provided")
        }
    }
}

impl ValidSignerConfig {
    pub async fn wallet(&self) -> Result<(EthereumWallet, Address)> {
        match self {
            ValidSignerConfig::Mnemonic {
                mnemonic,
                account_index,
            } => {
                let signer = MnemonicBuilder::<English>::default()
                    .phrase(mnemonic)
                    .index(*account_index)?
                    .build()?;
                let account = signer.address();
                let wallet = EthereumWallet::from(signer);
                Ok((wallet, account))
            },
            ValidSignerConfig::Ledger { account_index } => {
                let signer = connect_ledger(*account_index).await?;
                let account = signer.get_address().await?;
                let wallet = EthereumWallet::from(signer);
                Ok((wallet, account))
            },
        }
    }
}

impl Default for Commands {
    fn default() -> Self {
        Commands::StakeTable {
            l1_block_number: None,
            compact: false,
        }
    }
}

impl Config {
    pub fn apply_env_var_overrides(self) -> Result<Self> {
        let mut config = self.clone();
        if self.stake_table_address == Address::ZERO {
            let stake_table_env_var = "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS";
            if let Ok(stake_table_address) = std::env::var(stake_table_env_var) {
                config.stake_table_address = stake_table_address.parse()?;
                tracing::info!(
                    "Using stake table address from env {stake_table_env_var}: \
                     {stake_table_address}",
                );
            }
        }
        Ok(config)
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Display version information of the staking-cli.
    Version,
    /// Display the current configuration
    Config,
    /// Initialize the config file with deployment and wallet info.
    Init {
        /// The mnemonic to use when deriving the key.
        #[clap(long, env = "MNEMONIC", required_unless_present = "ledger")]
        mnemonic: Option<String>,

        /// The mnemonic account index to use when deriving the key.
        #[clap(long, env = "ACCOUNT_INDEX", default_value_t = 0)]
        account_index: u32,

        /// The ledger account index to use when deriving the key.
        #[clap(long, env = "LEDGER_INDEX", required_unless_present = "mnemonic")]
        ledger: bool,
    },
    /// Remove the config file.
    Purge {
        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,
    },
    /// Show the stake table in the Espresso stake table contract.
    StakeTable {
        /// The block numberto use for the stake table.
        ///
        /// Defaults to the latest block for convenience.
        #[clap(long)]
        l1_block_number: Option<BlockId>,

        /// Abbreviate the very long BLS public keys.
        #[clap(long)]
        compact: bool,
    },
    /// Print the signer account address.
    Account,
    /// Register to become a validator.
    RegisterValidator {
        #[clap(flatten)]
        signature_args: signature::NodeSignatureArgs,

        /// The commission to charge delegators
        #[clap(long, value_parser = parse::parse_commission, env = "COMMISSION")]
        commission: Commission,
    },
    /// Update a validators Espresso consensus signing keys.
    UpdateConsensusKeys {
        #[clap(flatten)]
        signature_args: signature::NodeSignatureArgs,
    },
    /// Deregister a validator.
    DeregisterValidator {},
    /// Update validator commission rate.
    UpdateCommission {
        /// The new commission rate to set
        #[clap(long, value_parser = parse::parse_commission, env = "NEW_COMMISSION")]
        new_commission: Commission,
    },
    /// Approve stake table contract to move tokens
    Approve {
        #[clap(long, value_parser = parse_ether)]
        amount: U256,
    },
    /// Delegate funds to a validator.
    Delegate {
        #[clap(long)]
        validator_address: Address,

        #[clap(long, value_parser = parse_ether)]
        amount: U256,
    },
    /// Initiate a withdrawal of delegated funds from a validator.
    Undelegate {
        #[clap(long)]
        validator_address: Address,

        #[clap(long, value_parser = parse_ether)]
        amount: U256,
    },
    /// Claim withdrawal after an undelegation.
    ClaimWithdrawal {
        #[clap(long)]
        validator_address: Address,
    },
    /// Claim withdrawal after validator exit.
    ClaimValidatorExit {
        #[clap(long)]
        validator_address: Address,
    },
    /// Check ESP token balance.
    TokenBalance {
        /// The address to check.
        #[clap(long)]
        address: Option<Address>,
    },
    /// Check ESP token allowance of stake table contract.
    TokenAllowance {
        /// The address to check.
        #[clap(long)]
        owner: Option<Address>,
    },
    /// Transfer ESP tokens
    Transfer {
        /// The address to transfer to.
        #[clap(long)]
        to: Address,

        /// The amount to transfer
        #[clap(long, value_parser = parse_ether)]
        amount: U256,
    },
    /// Register the validators and delegates for the local demo.
    StakeForDemo {
        /// The number of validators to register.
        ///
        /// The default (5) works for the local native and docker demos.
        #[clap(long, default_value_t = 5)]
        num_validators: u16,

        #[arg(long, value_enum, default_value_t = DelegationConfig::default())]
        delegation_config: DelegationConfig,
    },
    /// Export validator node signatures for address validation.
    ExportNodeSignatures {
        /// The Ethereum address to sign.
        #[clap(long)]
        address: Address,

        /// The BLS private key for signing.
        #[clap(long, value_parser = parse::parse_bls_priv_key, env = "BLS_PRIVATE_KEY")]
        consensus_private_key: BLSPrivKey,

        /// The Schnorr private key for signing.
        #[clap(long, value_parser = parse::parse_state_priv_key, env = "SCHNORR_PRIVATE_KEY")]
        state_private_key: StateSignKey,

        #[clap(flatten)]
        output_args: signature::OutputArgs,
    },
}
