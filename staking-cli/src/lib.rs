use std::path::PathBuf;

use alloy::{
    primitives::{Address, U256},
    signers::local::{
        coins_bip39::{English, Mnemonic},
        MnemonicBuilder,
    },
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_serde_derive::ClapSerde;
use hotshot_types::{light_client::StateSignKey, signature_key::BLSPrivKey};
use parse::Commission;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use url::Url;

mod parse;

#[cfg(any(test, feature = "testing"))]
mod deploy;

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
    /// Claim withdrawals from the stake table.
    ClaimWithdrawal,
}

fn exit_err(msg: impl AsRef<str>, err: impl core::fmt::Display) -> ! {
    eprintln!("{}: {err}", msg.as_ref());
    std::process::exit(1);
}

pub async fn main() -> Result<()> {
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

    match config.commands {
        Commands::Info => todo!(),
        Commands::RegisterValidator {
            consensus_private_key,
            state_private_key,
            commission,
        } => todo!(),
        Commands::DeregisterValidator {} => todo!(),
        Commands::Delegate {
            validator_address,
            amount,
        } => todo!(),
        Commands::Undelegate {
            validator_address,
            amount,
        } => todo!(),
        Commands::ClaimWithdrawal => todo!(),
        _ => unreachable!(),
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Output};

    use anyhow::Result;

    use super::*;

    trait AssertSuccess {
        fn assert_success(&self) -> &Self;
    }

    impl AssertSuccess for Output {
        fn assert_success(&self) -> &Self {
            if !self.status.success() {
                let stderr = String::from_utf8(self.stderr.clone()).expect("stderr is utf8");
                let stdout = String::from_utf8(self.stdout.clone()).expect("stdout is utf8");
                panic!("Command failed:\nstderr: {}\nstdout: {}", stderr, stdout);
            }
            self
        }
    }

    fn cmd() -> Command {
        escargot::CargoBuild::new()
            .bin("staking-cli")
            .current_release()
            .current_target()
            .run()
            .unwrap()
            .command()
    }

    #[test]
    fn test_version() -> Result<()> {
        cmd().arg("version").output()?.assert_success();
        Ok(())
    }

    #[test]
    fn test_created_and_remove_config_file() -> anyhow::Result<()> {
        let tmpdir = tempfile::tempdir()?;
        let config_path = tmpdir.path().join("config.toml");

        assert!(!config_path.exists());

        cmd()
            .arg("-c")
            .arg(&config_path)
            .arg("init")
            .output()?
            .assert_success();

        assert!(config_path.exists());

        cmd()
            .arg("-c")
            .arg(&config_path)
            .arg("purge")
            .arg("--force")
            .output()?
            .assert_success();

        assert!(!config_path.exists());

        Ok(())
    }
}
