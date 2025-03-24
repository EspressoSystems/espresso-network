//! sequencer utility programs

use clap::{Parser, Subcommand};
use sequencer_utils::logging;
mod keygen;
mod ns_aggregator;
mod pubkey;
mod reset_storage;

#[derive(Debug, Parser)]
struct Options {
    #[clap(flatten)]
    logging: logging::Config,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Keygen(keygen::Options),
    Pubkey(pubkey::Options),
    #[command(subcommand)]
    ResetStorage(reset_storage::Commands),
    NsAggregator(ns_aggregator::Options),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    match opt.command {
        Command::Keygen(opt) => keygen::run(opt),
        Command::Pubkey(opt) => {
            pubkey::run(opt);
            Ok(())
        },
        Command::ResetStorage(opt) => reset_storage::run(opt).await,
        Command::NsAggregator(opt) => ns_aggregator::run(opt).await,
    }
}
