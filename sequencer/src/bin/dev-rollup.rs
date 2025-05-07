use std::str::FromStr;

use anyhow::Result;
use clap::{Parser, Subcommand};
use committable::Committable;
use espresso_types::{
    v0_99::{RollupRegistration, RollupRegistrationBody, RollupUpdate, RollupUpdatebody},
    MarketplaceVersion, SeqTypes, Update,
};
use hotshot::types::BLSPubKey;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::SignatureKey};
use marketplace_solver::{SolverError, SOLVER_API_PATH};
use sequencer_utils::logging;
use tagged_base64::TaggedBase64;
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    #[command(flatten)]
    logging: logging::Config,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Register(RegisterArgs),
    Update(UpdateArgs),
}

// Options for registering a rollup
#[derive(Parser, Debug)]
struct RegisterArgs {
    #[arg(short, long, env = "ESPRESSO_MARKETPLACE_SOLVER_API_URL")]
    pub solver_url: Url,

    #[arg(short, long = "ns")]
    pub namespace_id: u64,

    #[arg(long, env = "ESPRESSO_MARKETPLACE_RESERVE_BUILDER_URL")]
    pub reserve_url: Option<Url>,

    #[arg(long, default_value_t = 200)]
    pub reserve_price: u64,

    #[arg(long, default_value_t = false)]
    pub active: bool,

    #[arg(long, default_value = "test")]
    pub text: String,

    /// The private key is provided in tagged-base64 format.
    /// If not provided, a default private key with a seed of `[0; 32]` and an index of `9876` will be used.
    #[arg(long = "privkey")]
    pub private_key: Option<String>,
}

// Options for updating an already registered rollup
#[derive(Parser, Debug)]
struct UpdateArgs {
    #[arg(short, long, env = "ESPRESSO_MARKETPLACE_SOLVER_API_URL")]
    pub solver_url: Url,

    #[arg(short, long = "ns")]
    pub namespace_id: u64,

    #[arg(
        long,
        env = "ESPRESSO_MARKETPLACE_RESERVE_BUILDER_URL",
        default_value = "",
        value_parser = parse_update_option::<Url>
    )]
    pub reserve_url: Update<Option<Url>>,

    #[arg(long, value_parser = parse_update::<u64>, default_value = "")]
    pub reserve_price: Update<u64>,

    #[arg(long, value_parser = parse_update::<bool>, default_value = "")]
    pub active: Update<bool>,

    #[arg(long, value_parser = parse_update::<String>, default_value = "")]
    pub text: Update<String>,

    /// The private key is provided in tagged-base64 format.
    /// If not provided, a default private key with a seed of `[0; 32]` and an index of `9876` will be used.
    #[arg(long = "privkey")]
    pub private_key: Option<String>,
}

fn parse_update<T: FromStr>(s: &str) -> Result<Update<T>, T::Err> {
    match s {
        "" => Ok(Update::Skip),
        s => Ok(Update::Set(s.parse()?)),
    }
}

fn parse_update_option<T: FromStr>(s: &str) -> Result<Update<Option<T>>, T::Err> {
    Ok(match s {
        "" => Update::Skip,
        "none" => Update::Set(None),
        s => Update::Set(Some(s.parse()?)),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    match opt.command {
        Command::Register(opt) => register(opt).await,
        Command::Update(opt) => update(opt).await,
    }
}

async fn register(opt: RegisterArgs) -> Result<()> {
    let RegisterArgs {
        solver_url,
        namespace_id,
        reserve_url,
        reserve_price,
        active,
        text,
        private_key,
    } = opt;

    let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(
        solver_url.join(SOLVER_API_PATH).unwrap(),
    );

    let (pubkey, privkey) = if let Some(privkey) = private_key {
        let privkey = TaggedBase64::parse(&privkey)?
            .try_into()
            .expect("invalid private key provided");
        let pubkey = BLSPubKey::from_private(&privkey);

        (pubkey, privkey)
    } else {
        BLSPubKey::generated_from_seed_indexed([0; 32], 9876)
    };

    let reg_body = RollupRegistrationBody {
        namespace_id: namespace_id.into(),
        reserve_url: reserve_url.clone(),
        reserve_price: reserve_price.into(),
        active,
        signature_keys: vec![pubkey],
        text,
        signature_key: pubkey,
    };

    // Sign the registration body
    let signature =
        <SeqTypes as NodeType>::SignatureKey::sign(&privkey, reg_body.commit().as_ref())
            .expect("failed to sign");

    let reg = RollupRegistration {
        body: reg_body.clone(),
        signature,
    };

    // registering a rollup
    client
        .post::<RollupRegistration>("register_rollup")
        .body_json(&reg)
        .unwrap()
        .send()
        .await
        .unwrap();

    tracing::info!("rollup with namespace {namespace_id} registered");

    Ok(())
}

async fn update(opt: UpdateArgs) -> Result<()> {
    let UpdateArgs {
        solver_url,
        namespace_id,
        reserve_url,
        reserve_price,
        active,
        text,
        private_key,
    } = opt;

    let client = surf_disco::Client::<SolverError, MarketplaceVersion>::new(
        solver_url.join(SOLVER_API_PATH).unwrap(),
    );
    let (pubkey, privkey) = if let Some(privkey) = private_key {
        let privkey = TaggedBase64::parse(&privkey)?
            .try_into()
            .expect("invalid private key provided");
        let pubkey = BLSPubKey::from_private(&privkey);

        (pubkey, privkey)
    } else {
        BLSPubKey::generated_from_seed_indexed([0; 32], 9876)
    };

    let body = RollupUpdatebody {
        namespace_id: namespace_id.into(),
        reserve_url,
        reserve_price: reserve_price.map(Into::into),
        active,
        signature_keys: Update::Skip,
        signature_key: pubkey,
        text,
    };

    // Sign the rollup update body
    let signature = <SeqTypes as NodeType>::SignatureKey::sign(&privkey, body.commit().as_ref())
        .expect("failed to sign");

    let update = RollupUpdate {
        body: body.clone(),
        signature,
    };

    // update a rollup
    client
        .post::<RollupRegistration>("register_rollup")
        .body_json(&update)
        .unwrap()
        .send()
        .await
        .unwrap();

    tracing::info!("rollup with namespace {namespace_id} updated");

    Ok(())
}
