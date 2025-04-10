use std::path::PathBuf;

use alloy::{
    eips::BlockId,
    network::EthereumWallet,
    primitives::{utils::format_ether, Address},
    providers::{Provider, ProviderBuilder},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use anyhow::Result;
use clap::Parser;
use clap_serde_derive::ClapSerde;
use hotshot_contract_adapter::sol_types::EspToken;
use staking_cli::{
    claim::{claim_validator_exit, claim_withdrawal},
    delegation::{approve, delegate, undelegate},
    demo::stake_for_demo,
    info::{display_stake_table, stake_table_info},
    registration::{deregister_validator, register_validator},
    Commands, Config,
};
use sysinfo::System;

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
                tracing::warn!("Unable to find config directory, using current directory");
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

fn exit_err(msg: impl AsRef<str>, err: impl core::fmt::Display) -> ! {
    tracing::error!("{}: {err}", msg.as_ref());
    std::process::exit(1);
}

fn exit(msg: impl AsRef<str>) -> ! {
    tracing::error!("Error: {}", msg.as_ref());
    std::process::exit(1);
}

#[tokio::main]
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
    config.logging.init();

    // Run the init command first because config values required by other
    // commands are not present.
    match config.commands {
        Commands::Init => {
            let config = toml::from_str::<Config>(include_str!("../../config.decaf.toml"))?;

            // Create directory where config file will be saved
            std::fs::create_dir_all(cli.config_dir()).unwrap_or_else(|err| {
                exit_err("failed to create config directory", err);
            });

            // Save the config file
            std::fs::write(&config_path, toml::to_string(&config)?)
                .unwrap_or_else(|err| exit_err("failed to write config file", err));

            println!("New config file saved to {}", config_path.display());
            println!(
                "Fill in your mnemonic in the config file at {}",
                config_path.display()
            );
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

    // When the staking CLI is used for our testnet, the env var names are different.
    let config = config.apply_env_var_overrides()?;

    // Clap serde will put default value if they aren't set. We check some
    // common configuration mistakes.
    if config.stake_table_address == Address::ZERO {
        exit("Stake table address is not set")
    };

    let signer = MnemonicBuilder::<English>::default()
        .phrase(config.mnemonic.as_str())
        .index(config.account_index)?
        .build()?;
    let account = signer.address();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(config.rpc_url.clone());
    let stake_table_addr = config.stake_table_address;
    let token_addr = config.token_address;
    let token = EspToken::new(config.token_address, &provider);

    let result = match config.commands {
        Commands::Info {
            l1_block_number,
            compact,
        } => {
            let query_block = l1_block_number.unwrap_or(BlockId::latest());
            let l1_block = provider.get_block(query_block).await?.unwrap_or_else(|| {
                exit_err("Failed to get block {query_block}", "Block not found");
            });
            let l1_block_resolved = l1_block.header.number;
            tracing::info!("Getting stake table info at block {l1_block_resolved}");
            let stake_table = stake_table_info(
                config.rpc_url.clone(),
                config.stake_table_address,
                l1_block_resolved,
            )
            .await?;
            display_stake_table(stake_table, compact)?;
            return Ok(());
        },
        Commands::RegisterValidator {
            consensus_private_key,
            state_private_key,
            commission,
        } => {
            tracing::info!("Registering validator {account} with commission {commission}");
            register_validator(
                &provider,
                stake_table_addr,
                commission,
                account,
                (consensus_private_key).into(),
                (&state_private_key).into(),
            )
            .await
        },
        Commands::DeregisterValidator {} => {
            tracing::info!("Deregistering validator {account}");
            deregister_validator(&provider, stake_table_addr).await
        },
        Commands::Approve { amount } => {
            tracing::info!(
                "Approving stake table {} to spend {amount}",
                config.stake_table_address
            );
            approve(&provider, token_addr, stake_table_addr, amount).await
        },
        Commands::Delegate {
            validator_address,
            amount,
        } => {
            tracing::info!("Delegating {amount} to {validator_address}");
            delegate(&provider, stake_table_addr, validator_address, amount).await
        },
        Commands::Undelegate {
            validator_address,
            amount,
        } => {
            tracing::info!("Undelegating {amount} from {validator_address}");
            undelegate(&provider, stake_table_addr, validator_address, amount).await
        },
        Commands::ClaimWithdrawal { validator_address } => {
            tracing::info!("Claiming withdrawal for {validator_address}");
            claim_withdrawal(&provider, stake_table_addr, validator_address).await
        },
        Commands::ClaimValidatorExit { validator_address } => {
            tracing::info!("Claiming validator exit for {validator_address}");
            claim_validator_exit(&provider, stake_table_addr, validator_address).await
        },
        Commands::StakeForDemo { num_validators } => {
            tracing::info!("Staking for demo with {num_validators} validators");
            stake_for_demo(&config, num_validators).await.unwrap();
            return Ok(());
        },
        Commands::TokenBalance { address } => {
            let address = address.unwrap_or(account);
            let balance = format_ether(token.balanceOf(address).call().await?._0);
            tracing::info!("Token balance for {address}: {balance} ESP");
            return Ok(());
        },
        Commands::TokenAllowance { owner } => {
            let owner = owner.unwrap_or(account);
            let allowance = format_ether(
                token
                    .allowance(owner, config.stake_table_address)
                    .call()
                    .await?
                    ._0,
            );
            tracing::info!("Stake table token allowance for {owner}: {allowance} ESP");
            return Ok(());
        },
        Commands::Transfer { amount, to } => {
            let amount_esp = format_ether(amount);
            tracing::info!("Transferring {amount_esp} ESP to {to}");
            Ok(token
                .transfer(to, amount)
                .send()
                .await?
                .get_receipt()
                .await?)
        },
        _ => unreachable!(),
    };

    match result {
        Ok(receipt) => tracing::info!("Success! transaction hash: {}", receipt.transaction_hash),
        Err(err) => exit_err("Failed:", err),
    };

    Ok(())
}
