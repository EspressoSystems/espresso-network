//! This binary checks the consistency of the VID commitment of the finalized blocks.
use clap::Parser;
use espresso_types::SeqTypes;
use hotshot_query_service::{
    availability::{PayloadQueryData, VidCommonQueryData},
    VidCommon,
};
use hotshot_types::{
    data::{ns_table::parse_ns_table, VidCommitment},
    traits::EncodeBytes,
    vid::{
        advz::{advz_scheme, ADVZScheme},
        avidm::AvidMScheme,
    },
};
use jf_vid::VidScheme;
use sequencer_utils::logging;
use tide_disco::error::ServerError;
use url::Url;
use vbs::version::StaticVersion;

#[derive(Parser)]
struct Args {
    /// URL of a sequencer query node
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// Block number to start checking
    #[clap(short, long, default_value_t = 1)]
    pub block: usize,

    /// Number of blocks to check
    #[clap(short, long, default_value_t = 1)]
    pub num: usize,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    let client =
        surf_disco::Client::<ServerError, StaticVersion<0, 1>>::new(args.sequencer_url.clone());
    for block in args.block..args.block + args.num {
        tracing::info!("Checking consistency for block {block}");
        let vid_common: VidCommonQueryData<SeqTypes> = match client
            .get(&format!("availability/vid/common/{block}"))
            .send()
            .await
        {
            Ok(common) => common,
            Err(err) => {
                tracing::error!("Error fetching VID common for block {block}: {err}");
                continue;
            },
        };
        let payload_hash = vid_common.payload_hash();
        let payload: PayloadQueryData<SeqTypes> = match client
            .get(&format!("availability/payload/hash/{payload_hash}"))
            .send()
            .await
        {
            Ok(payload) => payload,
            Err(err) => {
                tracing::error!("Error fetching payload for block {block}: {err}");
                continue;
            },
        };
        match (vid_common.common(), payload_hash) {
            (VidCommon::V0(common), VidCommitment::V0(payload_hash)) => {
                let mut vid = advz_scheme(ADVZScheme::get_num_storage_nodes(common) as usize);
                let expected_commit = match vid.commit_only(payload.data().encode()) {
                    Ok(commit) => commit,
                    Err(err) => {
                        tracing::error!("Error committing payload for block {block}: {err}");
                        continue;
                    },
                };
                if expected_commit != payload_hash {
                    tracing::error!("Inconsistent VID commitment for block {block}: expected {expected_commit}, got {payload_hash}");
                    continue;
                }
            },
            (VidCommon::V1(param), VidCommitment::V1(payload_hash)) => {
                let ns_table =
                    parse_ns_table(payload.size() as usize, &payload.data().ns_table().encode());
                let expected_commit =
                    match AvidMScheme::commit(param, &payload.data().encode(), ns_table) {
                        Ok(commit) => commit,
                        Err(err) => {
                            tracing::error!("Error committing payload for block {block}: {err}");
                            continue;
                        },
                    };
                if expected_commit != payload_hash {
                    tracing::error!("Inconsistent VID commitment for block {block}: expected {expected_commit}, got {payload_hash}");
                    continue;
                }
            },
            _ => {
                tracing::error!(
                    "Inconsistent VID version between common data and commitment for block {block}"
                );
                continue;
            },
        }

        tracing::info!("VID commitment is consistent for block {block}");
    }
}
