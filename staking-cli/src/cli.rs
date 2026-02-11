#![doc = include_str!("../README.md")]
use std::path::PathBuf;

use alloy::{
    self,
    eips::BlockId,
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::Log,
    sol_types::SolEventInterface,
};
use anyhow::{Context, Result};
use clap::Parser;
use clap_serde_derive::ClapSerde;
use hotshot_contract_adapter::sol_types::{
    EspToken::{self, EspTokenEvents},
    RewardClaim::RewardClaimEvents,
    StakeTableV2::StakeTableV2Events,
};
use hotshot_types::{
    light_client::{StateKeyPair, StateVerKey},
    signature_key::BLSPubKey,
};
use sysinfo::System;

use crate::{
    claim::fetch_claim_rewards_inputs,
    demo::stake_for_demo,
    info::{
        display_stake_table, fetch_stake_table_version, fetch_token_address, stake_table_info,
        StakeTableContractVersion,
    },
    metadata::{fetch_metadata, validate_metadata_uri, MetadataUri},
    output::{
        format_esp, output_calldata, output_error, output_success, output_warn, CalldataInfo,
    },
    signature::{NodeSignatureDestination, NodeSignatureInput, NodeSignatures},
    transaction::Transaction,
    Commands, Config, ValidSignerConfig,
};

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

trait AddressExt {
    fn or_from_wallet(self, wallet: Option<&EthereumWallet>) -> Option<Address>;
}

impl AddressExt for Option<Address> {
    fn or_from_wallet(self, wallet: Option<&EthereumWallet>) -> Option<Address> {
        self.or_else(|| wallet.map(NetworkWallet::<Ethereum>::default_signer_address))
    }
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
                     commission: {}, metadataUri: {} }}",
                    e.account,
                    BLSPubKey::from(e.blsVK),
                    StateVerKey::from(e.schnorrVK),
                    e.commission,
                    e.metadataUri
                )),
                StakeTableV2Events::Delegated(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::Undelegated(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::UndelegatedV2(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::ValidatorExit(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::ValidatorExitV2(e) => output_success(format!("event: {e:?}")),
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
                StakeTableV2Events::MetadataUriUpdated(e) => output_success(format!(
                    "event: MetadataUriUpdated {{ validator: {}, metadataUri: {} }}",
                    e.validator, e.metadataUri
                )),
                StakeTableV2Events::Withdrawal(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::WithdrawalClaimed(e) => output_success(format!("event: {e:?}")),
                StakeTableV2Events::ValidatorExitClaimed(e) => {
                    output_success(format!("event: {e:?}"))
                },

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

fn resolve_node_signatures(
    signature_args: &crate::signature::NodeSignatureArgs,
    export_calldata: bool,
    wallet: Option<&EthereumWallet>,
    sender_address: Option<Address>,
) -> Result<NodeSignatures> {
    if export_calldata {
        let input = NodeSignatureInput::try_from((signature_args.clone(), sender_address))?;
        NodeSignatures::try_from(input)
    } else {
        let wallet = wallet.ok_or_else(|| anyhow::anyhow!("Signer configuration required"))?;
        let address = NetworkWallet::<Ethereum>::default_signer_address(wallet);
        let input = NodeSignatureInput::try_from((signature_args.clone(), Some(address)))?;
        NodeSignatures::try_from((input, wallet))
    }
}

pub async fn run() -> Result<()> {
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
            private_key,
            account_index,
            ledger,
            network,
        } => {
            let config_template = match network {
                crate::Network::Mainnet => include_str!("../config.mainnet.toml"),
                crate::Network::Decaf => include_str!("../config.decaf.toml"),
                crate::Network::Local => include_str!("../config.demo-native.toml"),
            };
            let mut config = toml::from_str::<Config>(config_template)?;
            config.signer.mnemonic = mnemonic;
            config.signer.private_key = private_key;
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
            if !config_path.exists() {
                println!("No config file found at {}", config_path.display());
                println!("Run `staking-cli init --network <network>` to create one.");
                return Ok(());
            }
            println!("Config file at {}\n", config_path.display());
            let mut config = config;
            config.signer.mnemonic = config.signer.mnemonic.map(|_| "***".to_string());
            config.signer.private_key = config.signer.private_key.map(|_| "***".to_string());
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
        Commands::PreviewMetadata { metadata_uri } => {
            let url = url::Url::parse(&metadata_uri)
                .with_context(|| format!("Invalid URL: {metadata_uri}"))?;
            let metadata = fetch_metadata(&url)
                .await
                .with_context(|| format!("from {url}"))?;
            output_success(serde_json::to_string_pretty(&metadata)?);
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

    // Clap serde will put default value if they aren't set. We check some
    // common configuration mistakes.
    if config.stake_table_address == Address::ZERO {
        exit("Stake table address is not set use --stake-table-address or STAKE_TABLE_ADDRESS")
    };

    let stake_table_addr = config.stake_table_address;

    // For export_calldata mode, we may not need a signer for most commands.
    // We create the provider without a wallet first for token address fetching
    // and contract version detection.
    let readonly_provider = ProviderBuilder::new().connect_http(config.rpc_url.clone());

    // Check if we need token address for this command
    let token_addr = if config.commands.needs_token_address() {
        fetch_token_address(config.rpc_url.clone(), stake_table_addr).await?
    } else {
        Address::ZERO
    };

    let wallet =
        if let Ok(signer_config) = TryInto::<ValidSignerConfig>::try_into(config.signer.clone()) {
            signer_config.wallet().await.ok()
        } else {
            None
        };

    // Commands that just read from chain
    if let Commands::Account = config.commands {
        let account = NetworkWallet::<Ethereum>::default_signer_address(
            wallet.as_ref().context("Signer configuration required")?,
        );
        println!("{account}");
        return Ok(());
    }

    if let Commands::TokenBalance { address } = config.commands {
        let address = address
            .or_from_wallet(wallet.as_ref())
            .context("Address required - provide --address or configure a signer")?;
        let token = EspToken::new(token_addr, &readonly_provider);
        let balance = format_esp(token.balanceOf(address).call().await?);
        output_success(format!("Token balance for {address}: {balance}"));
        return Ok(());
    }

    if let Commands::TokenAllowance { owner } = config.commands {
        let owner = owner
            .or_from_wallet(wallet.as_ref())
            .context("Owner address required - provide --owner or configure a signer")?;
        let token = EspToken::new(token_addr, &readonly_provider);
        let allowance = format_esp(
            token
                .allowance(owner, config.stake_table_address)
                .call()
                .await?,
        );
        output_success(format!(
            "Stake table token allowance for {owner}: {allowance}"
        ));
        return Ok(());
    }

    if let Commands::UnclaimedRewards { address } = config.commands {
        let address = address
            .or_from_wallet(wallet.as_ref())
            .context("Address required - provide --address or configure a signer")?;
        let espresso_url = config.espresso_url.ok_or_else(|| {
            anyhow::anyhow!("espresso_url not set, use --espresso-url or ESPRESSO_URL")
        })?;
        let unclaimed = crate::claim::unclaimed_rewards(
            &readonly_provider,
            stake_table_addr,
            espresso_url,
            address,
        )
        .await
        .unwrap_or_else(|err| {
            exit_err("Failed to check unclaimed rewards", err);
        });
        println!("{}", format_esp(unclaimed));
        return Ok(());
    }

    if let Commands::StakeForDemo {
        num_validators,
        num_delegators_per_validator,
        delegation_config,
    } = config.commands
    {
        tracing::info!(
            "Staking for demo with {num_validators} validators and config {delegation_config}"
        );
        stake_for_demo(
            &config,
            num_validators,
            num_delegators_per_validator,
            delegation_config,
        )
        .await
        .unwrap();
        return Ok(());
    }

    // Build Transaction for state-changing commands
    let tx: Transaction = match &config.commands {
        Commands::RegisterValidator {
            signature_args,
            commission,
            metadata_uri_args,
        } => {
            let version = fetch_stake_table_version(&readonly_provider, stake_table_addr).await?;
            if config.export_calldata && matches!(version, StakeTableContractVersion::V1) {
                anyhow::bail!(
                    "Calldata export is not supported for V1 stake table contracts. V1 is \
                     deprecated."
                );
            }
            let payload = resolve_node_signatures(
                signature_args,
                config.export_calldata,
                wallet.as_ref(),
                config.sender_address,
            )?;
            let metadata_uri: MetadataUri = metadata_uri_args.clone().try_into()?;

            // Validate metadata URI if present and validation not skipped
            if let Some(url) = metadata_uri.url() {
                if !metadata_uri_args.skip_metadata_validation {
                    validate_metadata_uri(url, &payload.bls_vk)
                        .await
                        .context("use --skip-metadata-validation to skip")?;
                }
            }

            Transaction::RegisterValidator {
                stake_table: stake_table_addr,
                commission: *commission,
                metadata_uri,
                payload,
                version,
            }
        },
        Commands::UpdateConsensusKeys { signature_args } => {
            let version = fetch_stake_table_version(&readonly_provider, stake_table_addr).await?;
            if config.export_calldata && matches!(version, StakeTableContractVersion::V1) {
                anyhow::bail!(
                    "Calldata export is not supported for V1 stake table contracts. V1 is \
                     deprecated."
                );
            }
            if let Some(w) = wallet.as_ref() {
                let addr = NetworkWallet::<Ethereum>::default_signer_address(w);
                tracing::info!("Updating validator {} with new keys", addr);
            }
            let payload = resolve_node_signatures(
                signature_args,
                config.export_calldata,
                wallet.as_ref(),
                config.sender_address,
            )?;
            Transaction::UpdateConsensusKeys {
                stake_table: stake_table_addr,
                payload,
                version,
            }
        },
        Commands::DeregisterValidator {} => Transaction::DeregisterValidator {
            stake_table: stake_table_addr,
        },
        Commands::UpdateCommission { new_commission } => Transaction::UpdateCommission {
            stake_table: stake_table_addr,
            new_commission: *new_commission,
        },
        Commands::UpdateMetadataUri {
            metadata_uri_args,
            consensus_public_key,
        } => {
            let metadata_uri: MetadataUri = metadata_uri_args.clone().try_into()?;

            // Validate metadata URI if present and validation not skipped
            if let Some(url) = metadata_uri.url() {
                if !metadata_uri_args.skip_metadata_validation {
                    let bls_vk = consensus_public_key.ok_or_else(|| {
                        anyhow::anyhow!(
                            "--consensus-public-key is required for metadata validation (use \
                             --skip-metadata-validation to skip)"
                        )
                    })?;
                    validate_metadata_uri(url, &bls_vk)
                        .await
                        .context("use --skip-metadata-validation to skip")?;
                }
            }

            Transaction::UpdateMetadataUri {
                stake_table: stake_table_addr,
                metadata_uri,
            }
        },
        Commands::Approve { amount } => Transaction::Approve {
            token: token_addr,
            spender: stake_table_addr,
            amount: *amount,
        },
        Commands::Delegate {
            validator_address,
            amount,
        } => Transaction::Delegate {
            stake_table: stake_table_addr,
            validator: *validator_address,
            amount: *amount,
        },
        Commands::Undelegate {
            validator_address,
            amount,
        } => Transaction::Undelegate {
            stake_table: stake_table_addr,
            validator: *validator_address,
            amount: *amount,
        },
        Commands::ClaimWithdrawal { validator_address } => Transaction::ClaimWithdrawal {
            stake_table: stake_table_addr,
            validator: *validator_address,
        },
        Commands::ClaimValidatorExit { validator_address } => Transaction::ClaimValidatorExit {
            stake_table: stake_table_addr,
            validator: *validator_address,
        },
        Commands::ClaimRewards {} => {
            let espresso_url = config.espresso_url.clone().ok_or_else(|| {
                anyhow::anyhow!("espresso_url not set, use --espresso-url or ESPRESSO_URL")
            })?;
            let claimer_address = if config.export_calldata {
                config.sender_address.ok_or_else(|| {
                    anyhow::anyhow!(
                        "claim-rewards with --export-calldata requires --sender-address"
                    )
                })?
            } else {
                NetworkWallet::<Ethereum>::default_signer_address(
                    wallet.as_ref().context("Signer configuration required")?,
                )
            };
            fetch_claim_rewards_inputs(
                &readonly_provider,
                stake_table_addr,
                &espresso_url,
                claimer_address,
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("No reward claim data found for address"))?
        },
        Commands::Transfer { amount, to } => Transaction::Transfer {
            token: token_addr,
            to: *to,
            amount: *amount,
        },
        Commands::Version
        | Commands::Config
        | Commands::Init { .. }
        | Commands::Purge { .. }
        | Commands::StakeTable { .. }
        | Commands::Account
        | Commands::UnclaimedRewards { .. }
        | Commands::TokenBalance { .. }
        | Commands::TokenAllowance { .. }
        | Commands::ExportNodeSignatures { .. }
        | Commands::PreviewMetadata { .. }
        | Commands::StakeForDemo { .. } => {
            unreachable!("Non-state-change commands are handled earlier in the function")
        },
    };

    // Validate even for export mode to fail early if the transaction would fail on-chain.
    tx.validate_delegate_amount(&readonly_provider).await?;

    // Single code path for both export and execute modes
    if config.export_calldata {
        if config.skip_simulation {
            output_warn("Skipping calldata validation (--skip-simulation)");
        } else {
            let sender = config.sender_address.ok_or_else(|| {
                anyhow::anyhow!(
                    "--sender-address is required for calldata simulation (use --skip-simulation \
                     to skip)"
                )
            })?;
            tx.simulate(&readonly_provider, sender).await?;
        }
        let (to, data) = tx.calldata();
        return output_calldata(&CalldataInfo::new(to, data), &config.output);
    }

    // For execution, we need the wallet
    let wallet = wallet.ok_or_else(|| {
        anyhow::anyhow!("Signer configuration required for transaction execution")
    })?;
    let account = NetworkWallet::<Ethereum>::default_signer_address(&wallet);

    // Check that our Ethereum balance isn't zero before proceeding.
    let balance = readonly_provider.get_balance(account).await?;
    if balance.is_zero() {
        exit(format!(
            "zero Ethereum balance for account {account}, please fund account"
        ));
    }

    // Create provider with wallet for signing
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(config.rpc_url.clone());

    // Execute the state change
    let pending_tx = tx
        .send(&provider)
        .await
        .unwrap_or_else(|err| exit_err("Error", err));

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
