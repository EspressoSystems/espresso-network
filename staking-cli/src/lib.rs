use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{utils::parse_ether, Address, U256},
    signers::{
        ledger::{HDPath, LedgerError, LedgerSigner},
        local::{coins_bip39::English, MnemonicBuilder},
    },
};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use clap_serde_derive::ClapSerde;
pub(crate) use hotshot_types::{
    light_client::{StateSignKey, StateVerKey},
    signature_key::BLSPrivKey,
};
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use parse::Commission;
use sequencer_utils::logging;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod claim;
pub mod delegation;
pub mod demo;
pub mod info;
mod l1;
pub mod parse;
pub mod registration;

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

    /// Deployed ESP token contract address.
    #[clap(long, env = "ESP_TOKEN_ADDRESS")]
    pub token_address: Address,

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
                let mut attempt = 1;
                let max_attempts = 20;
                let signer = loop {
                    match LedgerSigner::new(HDPath::LedgerLive(*account_index), None).await {
                        Ok(signer) => break signer,
                        Err(err) => {
                            match err {
                                // Sadly, at this point, if we keep the app running unlocking the
                                // ledger does not make it show up.
                                LedgerError::LedgerError(ref ledger_error) => {
                                    bail!("Error: {ledger_error:#}. Please unlock ledger and try again")
                                },
                                LedgerError::UnexpectedNullResponse => {
                                    eprintln!(
                                        "Failed to access ledger {attempt}/{max_attempts}: {err:#}, please unlock ledger and open the Ethereum app"
                                    );
                                },
                                _ => {
                                    bail!("Unexpected error accessing the ledger device: {err:#}")
                                },
                            };
                            if attempt >= max_attempts {
                                bail!(
                                    "Failed to create Ledger signer after {max_attempts} attempts"
                                );
                            }
                            attempt += 1;
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        },
                    };
                };
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
        if self.token_address == Address::ZERO {
            let token_env_var = "ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS";
            if let Ok(token_address) = std::env::var(token_env_var) {
                config.token_address = token_address.parse()?;
                tracing::info!("Using ESP token address from env {token_env_var}: {token_address}",);
            }
        }
        if self.stake_table_address == Address::ZERO {
            let stake_table_env_var = "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS";
            if let Ok(stake_table_address) = std::env::var(stake_table_env_var) {
                config.stake_table_address = stake_table_address.parse()?;
                tracing::info!(
                    "Using stake table address from env {stake_table_env_var}: {stake_table_address}",
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
        /// The consensus signing key. Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_bls_priv_key, env = "CONSENSUS_PRIVATE_KEY")]
        consensus_private_key: BLSPrivKey,

        /// The state signing key.
        ///
        /// TODO: Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_state_priv_key, env = "STATE_PRIVATE_KEY")]
        state_private_key: StateSignKey,

        /// The commission to charge delegators
        #[clap(long, value_parser = parse::parse_commission, env = "COMMISSION")]
        commission: Commission,
    },
    /// Update a validators Espresso consensus signing keys.
    UpdateConsensusKeys {
        /// The consensus signing key. Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_bls_priv_key, env = "CONSENSUS_PRIVATE_KEY")]
        consensus_private_key: BLSPrivKey,

        /// The state signing key.
        ///
        /// TODO: Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_state_priv_key, env = "STATE_PRIVATE_KEY")]
        state_private_key: StateSignKey,
    },
    /// Deregister a validator.
    DeregisterValidator {},
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
    },
}
