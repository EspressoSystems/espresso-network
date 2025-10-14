#![doc = include_str!("../../README.md")]
use std::path::PathBuf;

use alloy::{
    self,
    eips::BlockId,
    primitives::{utils::format_ether, Address},
    providers::{Provider, ProviderBuilder},
    rpc::types::Log,
    sol_types::SolEventInterface,
};
use anyhow::Result;
use clap::Parser;
use clap_serde_derive::ClapSerde;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::{
        EspToken::{self, EspTokenErrors, EspTokenEvents},
        RewardClaim::RewardClaimEvents,
        StakeTableV2::StakeTableV2Events,
    },
};
use hotshot_types::{
    light_client::{StateKeyPair, StateVerKey},
    signature_key::BLSPubKey,
};
use staking_cli::{
    claim::{claim_reward, claim_validator_exit, claim_withdrawal},
    delegation::{approve, delegate, undelegate},
    demo::stake_for_demo,
    info::{display_stake_table, fetch_token_address, stake_table_info},
    registration::{
        deregister_validator, register_validator, update_commission, update_consensus_keys,
    },
    signature::{NodeSignatureDestination, NodeSignatureInput, NodeSignatures},
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

fn output_success(msg: impl AsRef<str>) {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::info!("{}", msg.as_ref());
    } else {
        println!("{}", msg.as_ref());
    }
}

fn output_error(msg: impl AsRef<str>) -> ! {
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing::error!("{}", msg.as_ref());
    } else {
        eprintln!("{}", msg.as_ref());
    }
    std::process::exit(1);
}

fn exit_err(msg: impl AsRef<str>, err: impl core::fmt::Display) -> ! {
    output_error(format!("{}: {err}", msg.as_ref()))
}

fn exit(msg: impl AsRef<str>) -> ! {
    output_error(format!("Error: {}", msg.as_ref()))
}

// Events containing custom structs do not get the Debug derive, due to a bug in
// foundry. We instead format those types nicely with tagged base64.
fn decode_and_display_logs(logs: &[Log]) {
    for log in logs {
        if let Ok(decoded) = StakeTableV2Events::decode_log(log.as_ref()) {
            match &decoded.data {
                StakeTableV2Events::ValidatorRegistered(e) => output_success(format!(
                    "event: ValidatorRegistered {{ account: {}, blsVk: {}, schnorrVk: {}, \
                     commission: {} }}",
                    e.account,
                    BLSPubKey::from(e.blsVk),
                    StateVerKey::from(e.schnorrVk),
                    e.commission
                )),
                StakeTableV2Events::ValidatorRegisteredV2(e) => output_success(format!(
                    "event: ValidatorRegisteredV2 {{ account: {}, blsVK: {}, schnorrVK: {}, \
                     commission: {} }}",
                    e.account,
                    BLSPubKey::from(e.blsVK),
                    StateVerKey::from(e.schnorrVK),
                    e.commission
                )),
                StakeTableV2Events::Delegated(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::Undelegated(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::ValidatorExit(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::ConsensusKeysUpdated(e) => output_success(format!(
                    "event: ConsensusKeysUpdated {{ account: {}, blsVK: {}, schnorrVK: {} }}",
                    e.account,
                    BLSPubKey::from(e.blsVK),
                    StateVerKey::from(e.schnorrVK)
                )),
                StakeTableV2Events::ConsensusKeysUpdatedV2(e) => output_success(format!(
                    "event: ConsensusKeysUpdatedV2 {{ account: {}, blsVK: {}, schnorrVK: {} }}",
                    e.account,
                    BLSPubKey::from(e.blsVK),
                    StateVerKey::from(e.schnorrVK)
                )),
                StakeTableV2Events::CommissionUpdated(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::Withdrawal(e) => output_success(format!("event: {e:?}")),

                _ => {},
            }
        } else if let Ok(decoded) = EspTokenEvents::decode_log(log.as_ref()) {
            match &decoded.data {
                EspTokenEvents::Transfer(e) => output_success(format!("event: {e:?}")),
                EspTokenEvents::Approval(e) => output_success(format!("event: {e:?}")),
                _ => {},
            }
        } else if let Ok(decoded) = RewardClaimEvents::decode_log(log.as_ref()) {
            if let RewardClaimEvents::RewardsClaimed(e) = &decoded.data {
                output_success(format!("event: {e:?}"));
            }
        }
    }
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
        Commands::ExportNodeSignatures {
            address,
            consensus_private_key,
            state_private_key,
            output_args,
        } => {
            let destination = NodeSignatureDestination::try_from(output_args)?;

            let payload = NodeSignatures::create(
                address,
                &consensus_private_key.into(),
                &StateKeyPair::from_sign_key(state_private_key),
            );

            payload.handle_output(destination)?;
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
        let provider = ProviderBuilder::new().connect_http(config.rpc_url.clone());
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
        .wallet(wallet.clone())
        .connect_http(config.rpc_url.clone());
    let stake_table_addr = config.stake_table_address;
    let token_addr = fetch_token_address(config.rpc_url.clone(), stake_table_addr).await?;
    let token = EspToken::new(token_addr, &provider);

    // Command that just read from chain, do not require a balance
    match config.commands {
        Commands::TokenBalance { address } => {
            let address = address.unwrap_or(account);
            let balance = format_ether(token.balanceOf(address).call().await?);
            tracing::info!("Token balance for {address}: {balance} ESP");
            return Ok(());
        },
        Commands::TokenAllowance { owner } => {
            let owner = owner.unwrap_or(account);
            let allowance = format_ether(
                token
                    .allowance(owner, config.stake_table_address)
                    .call()
                    .await?,
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
    let pending_tx = match config.commands {
        Commands::RegisterValidator {
            signature_args,
            commission,
        } => {
            let input = NodeSignatureInput::try_from((signature_args, &wallet))?;
            let payload = NodeSignatures::try_from((input, &wallet))?;
            register_validator(&provider, stake_table_addr, commission, payload).await?
        },
        Commands::UpdateConsensusKeys { signature_args } => {
            tracing::info!("Updating validator {account} with new keys");
            let input = NodeSignatureInput::try_from((signature_args, &wallet))?;
            let payload = NodeSignatures::try_from((input, &wallet))?;
            update_consensus_keys(&provider, stake_table_addr, payload).await?
        },
        Commands::DeregisterValidator {} => {
            tracing::info!("Deregistering validator {account}");
            deregister_validator(&provider, stake_table_addr).await?
        },
        Commands::UpdateCommission { new_commission } => {
            tracing::info!("Updating validator {account} commission to {new_commission}");
            update_commission(&provider, stake_table_addr, new_commission).await?
        },
        Commands::Approve { amount } => {
            approve(&provider, token_addr, stake_table_addr, amount).await?
        },
        Commands::Delegate {
            validator_address,
            amount,
        } => delegate(&provider, stake_table_addr, validator_address, amount).await?,
        Commands::Undelegate {
            validator_address,
            amount,
        } => undelegate(&provider, stake_table_addr, validator_address, amount).await?,
        Commands::ClaimWithdrawal { validator_address } => {
            tracing::info!("Claiming withdrawal for {validator_address}");
            claim_withdrawal(&provider, stake_table_addr, validator_address).await?
        },
        Commands::ClaimValidatorExit { validator_address } => {
            tracing::info!("Claiming validator exit for {validator_address}");
            claim_validator_exit(&provider, stake_table_addr, validator_address).await?
        },
        Commands::ClaimReward => {
            let espresso_url = config.espresso_url.ok_or_else(|| {
                anyhow::anyhow!("espresso_url not set, use --espresso-url or ESPRESSO_URL")
            })?;
            tracing::info!("Claiming rewards from {espresso_url}");
            claim_reward(&provider, stake_table_addr, espresso_url, account).await?
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
            token
                .transfer(to, amount)
                .send()
                .await
                .maybe_decode_revert::<EspTokenErrors>()?
        },
        _ => unreachable!(),
    };

    match pending_tx.get_receipt().await {
        Ok(receipt) => {
            output_success(format!(
                "Success! transaction hash: {}",
                receipt.transaction_hash
            ));
            decode_and_display_logs(receipt.inner.logs());
            Ok(())
        },
        Err(err) => exit_err("Failed", err),
    }
}
