use std::time::Duration;

use clap::Parser;
use espresso_types::{parse_duration, SeqTypes};
use hotshot_types::{
    light_client::{LCV2StateSignaturesBundle, DEFAULT_STAKE_TABLE_CAPACITY},
    signature_key::SchnorrPubKey,
    stake_table::HSStakeTable,
    traits::signature_key::LCV2StateSignatureKey,
    utils::epoch_from_block_number,
    PeerConfig,
};
use sequencer_utils::logging;
use url::Url;
use vbs::version::StaticVersion;

#[derive(Parser)]
struct Args {
    /// Url of the state relay server
    #[clap(
        long,
        default_value = "https://state-relay.water.devnet.espresso.network/",
        env = "ESPRESSO_STATE_RELAY_SERVER_URL"
    )]
    relay_server: Url,

    /// The frequency of updating the light client state, expressed in update interval
    #[clap(short, long = "freq", value_parser = parse_duration, default_value = "1m", env = "ESPRESSO_STATE_PROVER_UPDATE_INTERVAL")]
    update_interval: Duration,

    /// URL of a sequencer node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "https://query.water.devnet.espresso.network/"
    )]
    pub sequencer_url: Url,

    /// Stake table capacity for the prover circuit
    #[clap(short, long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY", default_value_t = DEFAULT_STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    // let state_relay_url =
    //     Url::from_str("https://state-relay.water.devnet.espresso.network/").unwrap();
    // let sequencer_url = Url::from_str("https://query.water.devnet.espresso.network/").unwrap();
    let state_relay =
        surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
            args.relay_server.clone(),
        );
    let sequencer = surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
        args.sequencer_url.clone(),
    );
    loop {
        let bundle = state_relay
            .get::<LCV2StateSignaturesBundle>("api/state")
            .send()
            .await
            .unwrap();
        tracing::info!("Checking bundle for block {}", bundle.state.block_height);
        for (key, sig) in bundle.signatures.iter() {
            assert!(<SchnorrPubKey as LCV2StateSignatureKey>::verify_state_sig(
                key,
                sig,
                &bundle.state,
                &bundle.next_stake
            ));
        }
        let epoch = epoch_from_block_number(bundle.state.block_height, 300);
        let stake_table: HSStakeTable<SeqTypes> = sequencer
            .get::<Vec<PeerConfig<SeqTypes>>>(&format!("node/stake-table/{epoch}"))
            .send()
            .await
            .unwrap()
            .into();
        assert!(!stake_table.is_empty());
        assert_eq!(stake_table.commitment(200).unwrap(), bundle.next_stake);
        tracing::info!("Verification complete!");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use espresso_types::config::PublicNetworkConfig;
    use url::Url;
    use vbs::version::StaticVersion;

    #[tokio::test]
    async fn test_hotshot_config() -> anyhow::Result<()> {
        let sequencer_url = Url::from_str("https://query.decaf.testnet.espresso.network/").unwrap();
        let epoch_config = loop {
            match surf_disco::Client::<tide_disco::error::ServerError, StaticVersion<0, 1>>::new(
                sequencer_url.clone(),
            )
            .get::<PublicNetworkConfig>("config/hotshot")
            .send()
            .await
            {
                Ok(resp) => {
                    let config = resp.hotshot_config();
                    break (config.blocks_per_epoch(), config.epoch_start_block());
                },
                Err(e) => {
                    tracing::error!("Failed to fetch the network config: {e}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                },
            }
        };
        println!(
            "blocks_per_epoch: {}, epoch_start_block: {}",
            epoch_config.0, epoch_config.1
        );
        Ok(())
    }
}
