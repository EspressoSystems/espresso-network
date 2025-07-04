#![doc = include_str!("../../README.md")]
use std::path::PathBuf;

use alloy::{
    self,
    eips::BlockId,
    primitives::{utils::format_ether, Address},
    providers::{Provider, ProviderBuilder},
};
use anyhow::Result;
use clap::Parser;
use clap_serde_derive::ClapSerde;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::EspToken::{self, EspTokenErrors},
};
use hotshot_types::light_client::StateKeyPair;
use staking_cli::{
    claim::{claim_validator_exit, claim_withdrawal},
    delegation::{approve, delegate, undelegate},
    demo::stake_for_demo,
    info::{display_stake_table, fetch_token_address, stake_table_info},
    registration::{deregister_validator, register_validator, update_consensus_keys},
    Commands, Config, ValidSignerConfig,
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

    // initialize the logging ASAP so we don't accidentally hide any messages.
    cli.config.logging.clone().unwrap_or_default().init();

    let config_path = cli.config_path();
    // Get config file
    let config = if let Ok(f) = std::fs::read_to_string(&config_path) {
        // parse toml
        match toml::from_str::<Config>(&f) {
            Ok(config) => config.merge(&mut cli.config),
            Err(err) => {
                // This is a user error print the hopefully helpful error
                // message without backtrace and exit.
                exit_err(
                    format!("Error in configuration file at {}", config_path.display()),
                    err,
                );
            },
        }
    } else {
        // If there is no config file return only config parsed from clap
        Config::from(&mut cli.config)
    };

    if config.token_address.is_some() {
        tracing::warn!("The `--token_address` argument is no longer necessary , and ignored");
    };

    // Run the init command first because config values required by other
    // commands are not present.
    match config.commands {
        Commands::Init {
            mnemonic,
            account_index,
            ledger,
        } => {
            let mut config = toml::from_str::<Config>(include_str!("../config.decaf.toml"))?;
            config.signer.mnemonic = mnemonic;
            config.signer.account_index = Some(account_index);
            config.signer.ledger = ledger;

            // Create directory where config file will be saved
            std::fs::create_dir_all(cli.config_dir()).unwrap_or_else(|err| {
                exit_err("failed to create config directory", err);
            });

            // Save the config file
            std::fs::write(&config_path, toml::to_string(&config)?)
                .unwrap_or_else(|err| exit_err("failed to write config file", err));

            println!("New config file saved to {}", config_path.display());
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
        Commands::Config => {
            println!("Config file at {}\n", config_path.display());
            let mut config = config;
            config.signer.mnemonic = config.signer.mnemonic.map(|_| "***".to_string());
            println!("{}", toml::to_string_pretty(&config)?);
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

    // Commands that don't need a signer
    if let Commands::StakeTable {
        l1_block_number,
        compact,
    } = config.commands
    {
        let provider = ProviderBuilder::new().on_http(config.rpc_url.clone());
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
    }

    let (wallet, account) = TryInto::<ValidSignerConfig>::try_into(config.signer.clone())?
        .wallet()
        .await?;
    if let Commands::Account = config.commands {
        println!("{account}");
        return Ok(());
    };

    // Clap serde will put default value if they aren't set. We check some
    // common configuration mistakes.
    if config.stake_table_address == Address::ZERO {
        exit("Stake table address is not set use --stake-table-address or STAKE_TABLE_ADDRESS")
    };

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(config.rpc_url.clone());
    let stake_table_addr = config.stake_table_address;
    let token_addr = fetch_token_address(config.rpc_url.clone(), stake_table_addr).await?;
    let token = EspToken::new(token_addr, &provider);

    // Command that just read from chain, do not require a balance
    match config.commands {
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
        _ => {
            // Continue with the rest of the commands that require a signer
        },
    };

    // Check that our Ethereum balance isn't zero before proceeding.
    let balance = provider.get_balance(account).await?;
    if balance.is_zero() {
        exit(format!(
            "zero Ethereum balance for account {account}, please fund account"
        ));
    }

    // Commands that require a signer
    let result = match config.commands {
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
                StateKeyPair::from_sign_key(state_private_key),
            )
            .await
        },
        Commands::UpdateConsensusKeys {
            consensus_private_key,
            state_private_key,
        } => {
            tracing::info!("Updating validator {account} with new keys");
            update_consensus_keys(
                &provider,
                stake_table_addr,
                account,
                (consensus_private_key).into(),
                StateKeyPair::from_sign_key(state_private_key),
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
        Commands::StakeForDemo {
            num_validators,
            delegation_config,
        } => {
            tracing::info!(
                "Staking for demo with {num_validators} validators and config {delegation_config}"
            );
            stake_for_demo(&config, num_validators, delegation_config)
                .await
                .unwrap();
            return Ok(());
        },
        Commands::Transfer { amount, to } => {
            let amount_esp = format_ether(amount);
            tracing::info!("Transferring {amount_esp} ESP to {to}");
            Ok(token
                .transfer(to, amount)
                .send()
                .await
                .maybe_decode_revert::<EspTokenErrors>()?
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
