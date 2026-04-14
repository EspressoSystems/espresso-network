use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{Address, U256, utils::parse_ether},
    signers::local::{MnemonicBuilder, PrivateKeySigner, coins_bip39::English},
};
use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use clap_serde_derive::ClapSerde;
use espresso_contract_deployer::provider::connect_ledger;
use espresso_utils::logging;
pub(crate) use hotshot_types::{
    addr::NetAddr,
    light_client::StateSignKey,
    signature_key::{BLSPrivKey, BLSPubKey},
    x25519,
};
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use metadata::MetadataUriArgs;
use serde::{Deserialize, Serialize};
use signature::OutputArgs;
use thiserror::Error;
use url::Url;

pub(crate) mod claim;
mod cli;
pub(crate) mod concurrent;
pub(crate) mod delegation;
/// Used by sequencer, espresso-dev-node, staking-ui-service tests.
pub mod demo;
pub(crate) mod info;
pub(crate) mod l1;
pub(crate) mod metadata;
// TODO: Replace with imports from staking-ui-service once version compatibility is resolved
pub(crate) mod metadata_types;
// TODO: Replace with imports from staking-ui-service once version compatibility is resolved
pub(crate) mod openmetrics;
pub(crate) mod output;
pub(crate) mod parse;
pub(crate) mod receipt;
pub(crate) mod registration;
pub(crate) mod signature;
pub(crate) mod transaction;
pub(crate) mod tx_log;

/// Used by staking-cli integration tests.
#[cfg(feature = "testing")]
pub mod deploy;

pub use cli::run;
// Used by staking-cli integration tests.
pub use metadata::fetch_metadata;
// Used by staking-cli integration tests.
pub use parse::Commission;
// Used by sequencer tests.
pub use registration::{fetch_commission, update_commission};
// Used by staking-cli integration tests.
pub use signature::NodeSignatures;
// Used by staking-cli integration tests.
pub use transaction::Transaction;
// Used by staking-cli integration tests.
pub use tx_log::TxLog;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum Network {
    Mainnet,
    Decaf,
    Local,
}

/// Used by staking-ui-service, sequencer tests, staking-cli integration tests.
pub const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";
/// Private key for account index 0 derived from DEV_MNEMONIC.
///
/// Used by staking-cli integration tests.
pub const DEV_PRIVATE_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// Mnemonic account index where demo validators start (indices 0-19 reserved for other uses).
pub const DEMO_VALIDATOR_START_INDEX: u32 = 20;

/// CLI to interact with the Espresso stake table contract.
#[derive(ClapSerde, Clone, Debug, Deserialize, Serialize)]
#[command(version, long_version = espresso_utils::build_info!().clap_version(), about, long_about = None)]
pub(crate) struct Config {
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
    #[serde(default)]
    #[clap_serde]
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
pub(crate) struct SignerConfig {
    /// The mnemonic to use when deriving the key.
    #[clap(long, env = "MNEMONIC", conflicts_with_all = ["private_key", "ledger"])]
    pub mnemonic: Option<String>,

    /// Raw private key (hex-encoded with or without 0x prefix).
    #[clap(long, env = "PRIVATE_KEY", conflicts_with_all = ["mnemonic", "ledger"])]
    pub private_key: Option<String>,

    /// The mnemonic account index to use when deriving the key.
    #[clap(long, env = "ACCOUNT_INDEX")]
    #[default(Some(0))]
    pub account_index: Option<u32>,

    /// Use a ledger device to sign transactions.
    ///
    /// NOTE: ledger must be unlocked, Ethereum app open and blind signing must be enabled in the
    /// Ethereum app settings.
    #[clap(long, env = "USE_LEDGER", conflicts_with_all = ["mnemonic", "private_key"])]
    pub ledger: bool,
}

#[derive(Debug, Error)]
pub(crate) enum SignerConfigError {
    #[error(
        "Multiple signers provided: {provided}. Only one of --mnemonic, --private-key, or \
         --ledger can be specified."
    )]
    MultipleSigners { provided: String },

    #[error("--account-index is required when using --{signer}")]
    MissingAccountIndex { signer: &'static str },

    #[error("Either --mnemonic, --private-key, or --ledger flag must be provided")]
    NoSigner,
}

#[derive(Clone, Debug)]
pub(crate) enum ValidSignerConfig {
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
    type Error = SignerConfigError;

    fn try_from(config: SignerConfig) -> std::result::Result<Self, SignerConfigError> {
        let signers: Vec<&str> = [
            config.ledger.then_some("--ledger"),
            config.private_key.as_ref().map(|_| "--private-key"),
            config.mnemonic.as_ref().map(|_| "--mnemonic"),
        ]
        .into_iter()
        .flatten()
        .collect();

        if signers.len() > 1 {
            return Err(SignerConfigError::MultipleSigners {
                provided: signers.join(", "),
            });
        }

        if config.ledger {
            let account_index = config
                .account_index
                .ok_or(SignerConfigError::MissingAccountIndex { signer: "ledger" })?;
            Ok(ValidSignerConfig::Ledger {
                account_index: account_index as usize,
            })
        } else if let Some(private_key) = config.private_key {
            Ok(ValidSignerConfig::PrivateKey { private_key })
        } else if let Some(mnemonic) = config.mnemonic {
            let account_index = config
                .account_index
                .ok_or(SignerConfigError::MissingAccountIndex { signer: "mnemonic" })?;
            Ok(ValidSignerConfig::Mnemonic {
                mnemonic,
                account_index,
            })
        } else {
            Err(SignerConfigError::NoSigner)
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
pub(crate) enum Commands {
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

        /// Network to configure (mainnet, decaf, or local).
        #[clap(long, value_enum, env = "NETWORK")]
        network: Network,
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

        /// x25519 public key (bs58-encoded, output by keygen). Required for V3 stake tables.
        #[clap(long, value_parser = parse::parse_x25519_key, env = "X25519_KEY")]
        x25519_key: Option<x25519::PublicKey>,

        /// p2p address in host:port format. Required for V3 stake tables.
        #[clap(long, value_parser = parse::parse_net_addr, env = "P2P_ADDR")]
        p2p_addr: Option<NetAddr>,
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

        /// The consensus public key for metadata validation.
        ///
        /// Required for metadata validation unless --skip-metadata-validation is set.
        #[clap(long, value_parser = parse::parse_bls_pub_key, env = "CONSENSUS_PUBLIC_KEY")]
        consensus_public_key: Option<BLSPubKey>,
    },
    /// Set x25519 key and p2p address for a validator.
    ///
    /// Primary use: initial configuration for validators registered before V3.
    /// Also usable to rotate the x25519 key.
    UpdateNetworkConfig {
        /// The x25519 public key (bs58-encoded, output by keygen)
        #[clap(long, value_parser = parse::parse_x25519_key, env = "X25519_KEY")]
        x25519_key: x25519::PublicKey,

        /// The p2p address in host:port format
        #[clap(long, value_parser = parse::parse_net_addr, env = "P2P_ADDR")]
        p2p_addr: NetAddr,
    },
    /// Set x25519 encryption key for a validator.
    UpdateX25519Key {
        /// The x25519 public key (bs58-encoded, output by keygen)
        #[clap(long, value_parser = parse::parse_x25519_key, env = "X25519_KEY")]
        x25519_key: x25519::PublicKey,
    },
    /// Update p2p address for a validator.
    UpdateP2pAddr {
        /// The p2p address in host:port format
        #[clap(long, value_parser = parse::parse_net_addr, env = "P2P_ADDR")]
        p2p_addr: NetAddr,
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
    /// Demo commands for testing and development
    Demo(demo::Demo),
    /// [DEPRECATED] Use `demo stake` instead. Register validators and create delegators for demo.
    #[clap(hide = true)]
    StakeForDemo {
        /// The number of validators to register.
        #[clap(long, default_value_t = 5)]
        num_validators: u16,

        /// The number of delegators to create per validator.
        #[clap(long, env = "NUM_DELEGATORS_PER_VALIDATOR", value_parser = clap::value_parser!(u64).range(..=100000))]
        num_delegators_per_validator: Option<u64>,

        #[clap(long, value_enum, env = "DELEGATION_CONFIG", default_value_t = demo::DelegationConfig::default())]
        delegation_config: demo::DelegationConfig,

        /// Number of concurrent transaction submissions
        #[clap(long, default_value_t = tx_log::DEFAULT_CONCURRENCY)]
        concurrency: usize,
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
    /// Preview metadata from a URL without registering.
    ///
    /// Fetches and displays validator metadata from a URL. Useful for verifying
    /// your metadata endpoint before registration.
    PreviewMetadata {
        /// URL where validator metadata is hosted (JSON or OpenMetrics format).
        #[clap(long, env = "METADATA_URI")]
        metadata_uri: String,
    },
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_matches;

    use super::*;

    const EMPTY: SignerConfig = SignerConfig {
        private_key: None,
        mnemonic: None,
        ledger: false,
        account_index: None,
    };

    #[test]
    fn test_private_key_without_account_index() {
        let config = SignerConfig {
            private_key: Some(DEV_PRIVATE_KEY.into()),
            ..EMPTY
        };
        let valid = ValidSignerConfig::try_from(config).unwrap();
        assert_matches!(valid, ValidSignerConfig::PrivateKey { .. });
    }

    #[test]
    fn test_private_key_with_account_index() {
        let config = SignerConfig {
            private_key: Some(DEV_PRIVATE_KEY.into()),
            account_index: Some(0),
            ..EMPTY
        };
        let valid = ValidSignerConfig::try_from(config).unwrap();
        assert_matches!(valid, ValidSignerConfig::PrivateKey { .. });
    }

    #[test]
    fn test_mnemonic_without_account_index() {
        let config = SignerConfig {
            mnemonic: Some(DEV_MNEMONIC.into()),
            ..EMPTY
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(
            err,
            SignerConfigError::MissingAccountIndex { signer: "mnemonic" }
        );
    }

    #[test]
    fn test_mnemonic_with_account_index() {
        let config = SignerConfig {
            mnemonic: Some(DEV_MNEMONIC.into()),
            account_index: Some(5),
            ..EMPTY
        };
        let valid = ValidSignerConfig::try_from(config).unwrap();
        assert_matches!(
            valid,
            ValidSignerConfig::Mnemonic {
                account_index: 5,
                ..
            }
        );
    }

    #[test]
    fn test_ledger_without_account_index() {
        let config = SignerConfig {
            ledger: true,
            ..EMPTY
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(
            err,
            SignerConfigError::MissingAccountIndex { signer: "ledger" }
        );
    }

    #[test]
    fn test_ledger_with_account_index() {
        let config = SignerConfig {
            ledger: true,
            account_index: Some(2),
            ..EMPTY
        };
        let valid = ValidSignerConfig::try_from(config).unwrap();
        assert_matches!(valid, ValidSignerConfig::Ledger { account_index: 2 });
    }

    #[test]
    fn test_no_signer_provided() {
        let err = ValidSignerConfig::try_from(EMPTY).unwrap_err();
        assert_matches!(err, SignerConfigError::NoSigner);
    }

    #[test]
    fn test_multiple_signers_mnemonic_and_private_key() {
        let config = SignerConfig {
            mnemonic: Some(DEV_MNEMONIC.into()),
            private_key: Some(DEV_PRIVATE_KEY.into()),
            account_index: Some(0),
            ..EMPTY
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(err, SignerConfigError::MultipleSigners { .. });
    }

    #[test]
    fn test_multiple_signers_ledger_and_private_key() {
        let config = SignerConfig {
            private_key: Some(DEV_PRIVATE_KEY.into()),
            ledger: true,
            ..EMPTY
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(err, SignerConfigError::MultipleSigners { .. });
    }

    #[test]
    fn test_multiple_signers_ledger_and_mnemonic() {
        let config = SignerConfig {
            mnemonic: Some(DEV_MNEMONIC.into()),
            ledger: true,
            account_index: Some(0),
            ..EMPTY
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(err, SignerConfigError::MultipleSigners { .. });
    }

    #[test]
    fn test_multiple_signers_all_three() {
        let config = SignerConfig {
            mnemonic: Some(DEV_MNEMONIC.into()),
            private_key: Some(DEV_PRIVATE_KEY.into()),
            ledger: true,
            account_index: Some(0),
        };
        let err = ValidSignerConfig::try_from(config).unwrap_err();
        assert_matches!(err, SignerConfigError::MultipleSigners { .. });
    }
}
