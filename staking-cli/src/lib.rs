use alloy::primitives::{Address, U256};
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
mod l1;
pub mod parse;
pub mod registration;

pub mod deploy;

pub const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";

#[derive(ClapSerde, Debug, Deserialize, Serialize)]
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

#[derive(Default, Subcommand, Debug)]
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
    #[default]
    Info,
    /// Register to become a validator.
    RegisterValidator {
        /// The consensus signing key. Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_bls_priv_key)]
        consensus_private_key: BLSPrivKey,

        /// The state signing key.
        ///
        /// TODO: Used to sign a message to prove ownership of the key.
        #[clap(long, value_parser = parse::parse_state_priv_key)]
        state_private_key: StateSignKey,

        /// The commission to charge delegators
        #[clap(long, value_parser = parse::parse_commission)]
        commission: Commission,
    },
    /// Deregister a validator.
    DeregisterValidator {},
    /// Delegate funds to a validator.
    /// Approve stake table contract to move tokens
    Approve {
        #[clap(long)]
        amount: U256,
    },
    Delegate {
        #[clap(long)]
        validator_address: Address,

        #[clap(long)]
        amount: U256,
    },
    /// Initiate a withdrawal of delegated funds from a validator.
    Undelegate {
        #[clap(long)]
        validator_address: Address,

        #[clap(long)]
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
    /// Register the validators and delegates for the local demo.
    StakeForDemo {
        /// The number of validators to register.
        ///
        /// The default (5) works for the local native and docker demos.
        #[clap(long, default_value_t = 5)]
        num_validators: u16,
    },
}
