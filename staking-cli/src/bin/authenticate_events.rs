use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::{SolEvent, SolEventInterface},
};
use anyhow::Result;
use clap::Parser;
use hotshot_contract_adapter::sol_types::StakeTableV2::{
    self, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, StakeTableV2Events, ValidatorRegistered,
    ValidatorRegisteredV2,
};
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "authenticate-events")]
#[command(about = "Fetch and authenticate all StakeTable events from decaf/sepolia")]
struct Args {
    #[clap(
        long,
        default_value = "https://ethereum-sepolia-rpc.publicnode.com",
        env = "L1_PROVIDER"
    )]
    rpc_url: Url,

    #[clap(
        long,
        default_value = "0x40304fbe94d5e7d1492dd90c53a2d63e8506a037",
        env = "STAKE_TABLE_ADDRESS"
    )]
    stake_table_address: Address,
}

#[derive(Debug)]
struct AuthResult {
    block_number: u64,
    log_index: u64,
    tx_hash: String,
    account: Address,
    event_type: &'static str,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    println!(
        "Fetching events from StakeTable at {} via {}",
        args.stake_table_address, args.rpc_url
    );

    let provider = ProviderBuilder::new().connect_http(args.rpc_url);

    let stake_table = StakeTableV2::new(args.stake_table_address, provider.clone());
    let from_block = stake_table.initializedAtBlock().call().await?.to::<u64>();
    let to_block = provider.get_block_number().await?;

    println!(
        "Scanning blocks {} to {} ({} blocks)",
        from_block,
        to_block,
        to_block - from_block
    );

    println!("\nEvent signatures being queried:");
    println!(
        "  ValidatorRegistered (V1):   {}",
        ValidatorRegistered::SIGNATURE
    );
    println!(
        "  ValidatorRegisteredV2:      {}",
        ValidatorRegisteredV2::SIGNATURE
    );
    println!(
        "  ConsensusKeysUpdated (V1):  {}",
        ConsensusKeysUpdated::SIGNATURE
    );
    println!(
        "  ConsensusKeysUpdatedV2:     {}",
        ConsensusKeysUpdatedV2::SIGNATURE
    );

    let filter = Filter::new()
        .events([
            ValidatorRegistered::SIGNATURE,
            ValidatorRegisteredV2::SIGNATURE,
            ConsensusKeysUpdated::SIGNATURE,
            ConsensusKeysUpdatedV2::SIGNATURE,
        ])
        .address(args.stake_table_address)
        .from_block(from_block)
        .to_block(to_block);

    let logs = provider.get_logs(&filter).await?;
    println!("Found {} registration/key-update events", logs.len());

    let mut results: Vec<AuthResult> = Vec::new();
    let mut v1_events = 0;
    let mut v2_events = 0;
    let mut auth_failures = 0;

    for log in logs {
        let block_number = log.block_number.unwrap_or(0);
        let log_index = log.log_index.unwrap_or(0);
        let tx_hash = log
            .transaction_hash
            .map(|h| format!("{h:#x}"))
            .unwrap_or_else(|| "unknown".to_string());

        let event = match StakeTableV2Events::decode_raw_log(log.topics(), &log.data().data) {
            Ok(e) => e,
            Err(e) => {
                println!(
                    "WARN: Failed to decode event at block {} log {}: {}",
                    block_number, log_index, e
                );
                continue;
            },
        };

        match event {
            StakeTableV2Events::ValidatorRegistered(ref evt) => {
                v1_events += 1;
                results.push(AuthResult {
                    block_number,
                    log_index,
                    tx_hash,
                    account: evt.account,
                    event_type: "ValidatorRegistered (V1)",
                    error: None,
                });
            },
            StakeTableV2Events::ValidatorRegisteredV2(ref evt) => {
                v2_events += 1;
                let error = match evt.authenticate() {
                    Ok(()) => None,
                    Err(e) => {
                        auth_failures += 1;
                        Some(format!("{e}"))
                    },
                };
                results.push(AuthResult {
                    block_number,
                    log_index,
                    tx_hash,
                    account: evt.account,
                    event_type: "ValidatorRegisteredV2",
                    error,
                });
            },
            StakeTableV2Events::ConsensusKeysUpdated(ref evt) => {
                v1_events += 1;
                results.push(AuthResult {
                    block_number,
                    log_index,
                    tx_hash,
                    account: evt.account,
                    event_type: "ConsensusKeysUpdated (V1)",
                    error: None,
                });
            },
            StakeTableV2Events::ConsensusKeysUpdatedV2(ref evt) => {
                v2_events += 1;
                let error = match evt.authenticate() {
                    Ok(()) => None,
                    Err(e) => {
                        auth_failures += 1;
                        Some(format!("{e}"))
                    },
                };
                results.push(AuthResult {
                    block_number,
                    log_index,
                    tx_hash,
                    account: evt.account,
                    event_type: "ConsensusKeysUpdatedV2",
                    error,
                });
            },
            _ => {},
        }
    }

    println!("\n=== Summary ===");
    println!("V1 events (no signatures): {}", v1_events);
    println!("V2 events (with signatures): {}", v2_events);
    println!("Authentication failures: {}", auth_failures);

    if auth_failures > 0 {
        println!("\n=== Failed Authentications ===");
        for result in &results {
            if let Some(ref error) = result.error {
                println!(
                    "Block {} | Log {} | TX {} | Account {} | {} | Error: {}",
                    result.block_number,
                    result.log_index,
                    result.tx_hash,
                    result.account,
                    result.event_type,
                    error
                );
            }
        }
    } else {
        println!("\nAll V2 events authenticated successfully.");
    }

    println!("\n=== All Events ===");
    for result in &results {
        let status = match &result.error {
            None => "OK",
            Some(_) => "FAILED",
        };
        println!(
            "Block {:>8} | Log {:>3} | {} | {} | {}",
            result.block_number, result.log_index, result.account, result.event_type, status
        );
    }

    Ok(())
}
