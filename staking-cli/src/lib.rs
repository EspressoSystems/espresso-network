use std::path::PathBuf;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use anyhow::Result;
use claim::{claim_validator_exit, claim_withdrawal};
use clap::{Parser, Subcommand};
use clap_serde_derive::ClapSerde;
use contract_bindings_alloy::staketable::StakeTable::StakeTableInstance;
use delegation::{delegate, undelegate};
use demo::stake_for_demo;
pub(crate) use hotshot_types::{
    light_client::{StateSignKey, StateVerKey},
    signature_key::BLSPrivKey,
};
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use parse::Commission;
use registration::{deregister_validator, register_validator};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use url::Url;

mod claim;
mod delegation;
mod demo;
mod l1;
mod parse;
mod registration;

#[cfg(any(test, feature = "testing"))]
pub mod deploy;

pub const DEV_MNEMONIC: &str = "test test test test test test test test test test test junk";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Config file
    #[arg(short, long = "config")]
    config_path: Option<PathBuf>,

    /// Rest of arguments
    #[command(flatten)]
    pub config: <Config as ClapSerde>::Opt,
}

impl Args {
    fn config_path(&self) -> PathBuf {
        // If the user provided a config path, use it.
        self.config_path.clone().unwrap_or_else(|| {
            // Otherwise create a config.toml in a platform specific config directory.
            //
            // (empty) qualifier, espresso organization, and application name
            // see more <https://docs.rs/directories/5.0.1/directories/struct.ProjectDirs.html#method.from>
            let project_dir =
                directories::ProjectDirs::from("", "espresso", "espresso-staking-cli");
            let basename = "config.toml";
            if let Some(project_dir) = project_dir {
                project_dir.config_dir().to_path_buf().join(basename)
            } else {
                // In the unlikely case that we can't find the config directory,
                // create the config file in the current directory and issue a
                // warning.
                eprintln!("WARN: Unable to find config directory, using current directory");
                basename.into()
            }
        })
    }

    fn config_dir(&self) -> PathBuf {
        if let Some(path) = self.config_path().parent() {
            path.to_path_buf()
        } else {
            // Try to use the current directory
            PathBuf::from(".")
        }
    }
}

#[derive(ClapSerde, Debug, Deserialize, Serialize)]
pub struct Config {
    // # TODO for mainnet we should support hardware wallets. Alloy has support for this.
    #[default(DEV_MNEMONIC.to_string())]
    #[clap(long, env = "MNEMONIC")]
    #[serde(alias = "mnemonic", alias = "MNEMONIC")]
    pub mnemonic: String,

    #[clap(long, env = "ACCOUNT_INDEX", default_value = "0")]
    account_index: u32,

    /// L1 Ethereum RPC.
    #[clap(long, env = "RPC_URL")]
    #[default(Url::parse("http://localhost:8545").unwrap())]
    rpc_url: Url,

    /// Deployed stake table contract address.
    #[clap(long, env = "STAKE_TABLE_ADDRESS")]
    stake_table_address: Address,

    #[command(subcommand)]
    #[serde(skip)]
    commands: Commands,
}

#[derive(Default, Subcommand, Debug)]
enum Commands {
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
    StakeForDemo,
}

fn exit_err(msg: impl AsRef<str>, err: impl core::fmt::Display) -> ! {
    eprintln!("{}: {err}", msg.as_ref());
    std::process::exit(1);
}

pub async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut cli = Args::parse();
    let config_path = cli.config_path();
    // Get config file
    let config = if let Ok(f) = std::fs::read_to_string(&config_path) {
        // parse toml
        match toml::from_str::<Config>(&f) {
            Ok(config) => config.merge(&mut cli.config),
            Err(err) => {
                // This is a user error print the hopefully helpful error
                // message without backtrace and exit.
                exit_err("Error in configuration file", err);
            },
        }
    } else {
        // If there is no config file return only config parsed from clap
        Config::from(&mut cli.config)
    };

    // Run the init command first because config values required by other
    // commands are not present.
    match config.commands {
        Commands::Init => {
            let config = toml::from_str::<Config>(include_str!("../config.demo.toml"))?;

            // Create directory where config file will be saved
            std::fs::create_dir_all(cli.config_dir()).unwrap_or_else(|err| {
                exit_err("failed to create config directory", err);
            });

            // Save the config file
            std::fs::write(&config_path, toml::to_string(&config)?)
                .unwrap_or_else(|err| exit_err("failed to write config file", err));

            println!("Config file saved to {}", config_path.display());
            return Ok(());
        },
        Commands::Purge { force } => {
            // Check if the file exists
            if !config_path.exists() {
                println!("Config file not found at {}", config_path.display());
                return Ok(());
            }
            if !force {
                // Get a confirmation from the user before removing the config file.
                println!(
                    "Are you sure you want to remove the config file at {}? [y/N]",
                    config_path.display()
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Aborted");
                    return Ok(());
                }
            }
            // Remove the config file
            std::fs::remove_file(&config_path).unwrap_or_else(|err| {
                exit_err("failed to remove config file", err);
            });

            println!("Config file removed from {}", config_path.display());
            return Ok(());
        },
        Commands::Version => {
            println!("staking-cli version: {}", env!("CARGO_PKG_VERSION"));
            println!("{}", git_version::git_version!(prefix = "git rev: "));
            println!("OS: {}", System::long_os_version().unwrap_or_default());
            println!("Arch: {}", System::cpu_arch());
            return Ok(());
        },
        _ => {}, // Other commands handled after shared setup.
    }

    let signer = MnemonicBuilder::<English>::default()
        .phrase(config.mnemonic.as_str())
        .index(config.account_index)?
        .build()?;
    let account = signer.address();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(config.rpc_url.clone());
    let stake_table = StakeTableInstance::new(config.stake_table_address, provider.clone());

    let result = match config.commands {
        // TODO: The info command is not implemented yet. It's not very useful for local testing or
        // the demo and requires code that is not yet merged into main, so it's left for later.
        Commands::Info => todo!(),
        Commands::RegisterValidator {
            consensus_private_key,
            state_private_key,
            commission,
        } => {
            register_validator(
                stake_table,
                commission,
                account,
                (consensus_private_key).into(),
                (&state_private_key).into(),
            )
            .await
        },
        Commands::DeregisterValidator {} => deregister_validator(stake_table).await,
        Commands::Delegate {
            validator_address,
            amount,
        } => delegate(stake_table, validator_address, amount).await,
        Commands::Undelegate {
            validator_address,
            amount,
        } => undelegate(stake_table, validator_address, amount).await,
        Commands::ClaimWithdrawal { validator_address } => {
            claim_withdrawal(stake_table, validator_address).await
        },
        Commands::ClaimValidatorExit { validator_address } => {
            claim_validator_exit(stake_table, validator_address).await
        },
        Commands::StakeForDemo => {
            stake_for_demo(&config).await.unwrap();
            return Ok(());
        },
        _ => unreachable!(),
    };
    tracing::info!("Result: {:?}", result);
    Ok(())
}
