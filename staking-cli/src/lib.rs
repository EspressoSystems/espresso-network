use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{utils::parse_ether, Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder, PrivateKeySigner},
};
use anyhow::{bail, Result};
use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand};
use clap_serde_derive::ClapSerde;
use demo::DelegationConfig;
use espresso_contract_deployer::provider::connect_ledger;
pub(crate) use hotshot_types::{light_client::StateSignKey, signature_key::BLSPrivKey};
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use metadata::MetadataUri;
use parse::Commission;
use sequencer_utils::logging;
use serde::{Deserialize, Serialize};
use signature::OutputArgs;
use url::Url;

pub(crate) mod claim;
mod cli;
pub(crate) mod delegation;
/// Used by sequencer, espresso-dev-node, staking-ui-service tests.
pub mod demo;
pub(crate) mod info;
pub(crate) mod l1;
pub(crate) mod metadata;
pub(crate) mod output;
pub(crate) mod parse;
pub(crate) mod receipt;
/// Used by sequencer tests (fetch_commission, update_commission).
pub mod registration;
/// Used by staking-cli integration tests (NodeSignatures).
pub mod signature;
pub(crate) mod transaction;

/// Used by staking-cli integration tests.
#[cfg(feature = "testing")]
pub mod deploy;

pub use cli::run;

/// Used by staking-ui-service, sequencer tests, staking-cli integration tests.
pub const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";
/// Private key for account index 0 derived from DEV_MNEMONIC.
///
/// Used by staking-cli integration tests.
pub const DEV_PRIVATE_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// CLI to interact with the Espresso stake table contract.
///
/// Used by staking-cli integration tests.
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

    /// Espresso sequencer API URL for reward claims.
    #[clap(long, env = "ESPRESSO_URL")]
    pub espresso_url: Option<Url>,

    #[clap(flatten)]
    pub signer: SignerConfig,

    /// Export calldata for multisig wallets instead of sending transaction.
    #[clap(
        long,
        env = "EXPORT_CALLDATA",
        action = ArgAction::SetTrue,
        conflicts_with_all = ["mnemonic", "private_key", "ledger"]
    )]
    #[serde(skip)]
    pub export_calldata: bool,

    /// Sender address for calldata export (required for simulation).
    #[clap(long, env = "SENDER_ADDRESS")]
    #[serde(skip)]
    pub sender_address: Option<Address>,

    /// Skip eth_call validation when exporting calldata.
    #[clap(long, env = "SKIP_SIMULATION", action = ArgAction::SetTrue, requires = "export_calldata")]
    #[serde(skip)]
    pub skip_simulation: bool,

    #[clap(flatten)]
    #[serde(skip)]
    pub output: OutputArgs,

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

    /// Raw private key (hex-encoded with or without 0x prefix).
    #[clap(long, env = "PRIVATE_KEY")]
    pub private_key: Option<String>,

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
    PrivateKey {
        private_key: String,
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
        if config.ledger {
            Ok(ValidSignerConfig::Ledger {
                account_index: account_index as usize,
            })
        } else if let Some(private_key) = config.private_key {
            Ok(ValidSignerConfig::PrivateKey { private_key })
        } else if let Some(mnemonic) = config.mnemonic {
            Ok(ValidSignerConfig::Mnemonic {
                mnemonic,
                account_index,
            })
        } else {
            bail!("Either --mnemonic, --private-key, or --ledger flag must be provided")
        }
    }
}

impl ValidSignerConfig {
    pub async fn wallet(&self) -> Result<EthereumWallet> {
        match self {
            ValidSignerConfig::Mnemonic {
                mnemonic,
                account_index,
            } => {
                let signer = MnemonicBuilder::<English>::default()
                    .phrase(mnemonic)
                    .index(*account_index)?
                    .build()?;
                Ok(EthereumWallet::from(signer))
            },
            ValidSignerConfig::PrivateKey { private_key } => {
                let signer: PrivateKeySigner = private_key.parse()?;
                Ok(EthereumWallet::from(signer))
            },
            ValidSignerConfig::Ledger { account_index } => {
                let signer = connect_ledger(*account_index).await?;
                Ok(EthereumWallet::from(signer))
            },
        }
    }
}

#[derive(ClapArgs, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct MetadataUriArgs {
    #[clap(long, env = "METADATA_URI")]
    metadata_uri: Option<String>,

    #[clap(long, env = "NO_METADATA_URI")]
    no_metadata_uri: bool,
}

impl TryFrom<MetadataUriArgs> for MetadataUri {
    type Error = anyhow::Error;

    fn try_from(args: MetadataUriArgs) -> Result<Self> {
        if args.no_metadata_uri {
            Ok(MetadataUri::empty())
        } else if let Some(uri_str) = args.metadata_uri {
            uri_str.parse()
        } else {
            bail!("Either --metadata-uri or --no-metadata-uri must be provided")
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

impl Commands {
    pub(crate) fn needs_token_address(&self) -> bool {
        matches!(
            self,
            Commands::Approve { .. }
                | Commands::Transfer { .. }
                | Commands::TokenBalance { .. }
                | Commands::TokenAllowance { .. }
        )
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
        #[clap(long, env = "MNEMONIC", required_unless_present_any = ["ledger", "private_key"])]
        mnemonic: Option<String>,

        /// Raw private key (hex-encoded with or without 0x prefix).
        #[clap(long, env = "PRIVATE_KEY", required_unless_present_any = ["ledger", "mnemonic"], conflicts_with = "account_index")]
        private_key: Option<String>,

        /// The account index for key derivation (only used with mnemonic or ledger).
        #[clap(long, env = "ACCOUNT_INDEX", default_value_t = 0)]
        account_index: u32,

        /// Use a ledger hardware wallet.
        #[clap(long, env = "LEDGER_INDEX", required_unless_present_any = ["mnemonic", "private_key"])]
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

        #[clap(flatten)]
        metadata_uri_args: MetadataUriArgs,
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
    /// Update validator metadata URL.
    UpdateMetadataUri {
        #[clap(flatten)]
        metadata_uri_args: MetadataUriArgs,
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
    /// Claim staking rewards.
    ClaimRewards {},
    /// Check unclaimed staking rewards.
    UnclaimedRewards {
        /// The address to check.
        #[clap(long)]
        address: Option<Address>,
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

        /// The number of delegators to create per validator.
        ///
        /// If not specified, a random number (2-5) of delegators is created per validator.
        /// Must be <= 100,000.
        #[clap(long, env = "NUM_DELEGATORS_PER_VALIDATOR", value_parser = clap::value_parser!(u64).range(..=100000))]
        num_delegators_per_validator: Option<u64>,

        #[arg(long, value_enum, env = "DELEGATION_CONFIG", default_value_t = DelegationConfig::default())]
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
