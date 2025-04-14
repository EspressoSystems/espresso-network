use alloy::{
    eips::BlockId,
    primitives::{utils::parse_ether, Address, U256},
};
use anyhow::Result;
use clap::Subcommand;
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

#[derive(ClapSerde, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    // # TODO for mainnet we should support hardware wallets. Alloy has support for this.
    #[default(DEV_MNEMONIC.to_string())]
    #[clap(long, env = "MNEMONIC")]
    #[serde(alias = "mnemonic", alias = "MNEMONIC")]
    pub mnemonic: String,

    #[clap(long, env = "ACCOUNT_INDEX", default_value = "0")]
    pub account_index: u32,

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
    #[serde(skip)]
    pub logging: logging::Config,

    #[command(subcommand)]
    #[serde(skip)]
    pub commands: Commands,
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Info {
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
    Version,
    /// Initialize the config file with a new mnemonic.
    Init,
    /// Remove the config file.
    Purge {
        /// Don't ask for confirmation.
        #[clap(long)]
        force: bool,
    },
    /// Show information about delegation, withdrawals, etc.
    Info {
        /// The block numberto use for the stake table.
        ///
        /// Defaults to the latest block for convenience.
        #[clap(long)]
        l1_block_number: Option<BlockId>,

        /// Abbreviate the very long BLS public keys.
        #[clap(long)]
        compact: bool,
    },
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
    /// Deregister a validator.
    DeregisterValidator {},
    /// Delegate funds to a validator.
    /// Approve stake table contract to move tokens
    Approve {
        #[clap(long, value_parser = parse_ether)]
        amount: U256,
    },
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
