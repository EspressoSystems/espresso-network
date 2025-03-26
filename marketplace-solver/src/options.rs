use std::time::Duration;

use clap::Parser;
use espresso_types::parse_duration;
use tide_disco::Url;

use crate::database::PostgresClient;

#[derive(Parser)]
pub struct Options {
    /// Port to run the server on.
    #[arg(short, long, env = "ESPRESSO_MARKETPLACE_SOLVER_API_PORT")]
    pub solver_api_port: u16,

    /// Hotshot events service api URL
    #[arg(short, long, env = "ESPRESSO_SEQUENCER_HOTSHOT_EVENT_API_URL")]
    pub events_api_url: Url,

    #[command(flatten)]
    pub database_options: DatabaseOptions,
}

/// Arguments for establishing a database connection
#[derive(Clone, Debug, Parser)]
pub struct DatabaseOptions {
    // Postgres URL connection string
    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_URL")]
    pub url: Option<String>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_HOST")]
    pub host: Option<String>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_PORT")]
    pub port: Option<u16>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_DATABASE_NAME")]
    pub db_name: Option<String>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_USER")]
    pub username: Option<String>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_PASSWORD")]
    pub password: Option<String>,

    #[arg(long, env = "MARKETPLACE_SOLVER_POSTGRES_MAX_CONNECTIONS")]
    pub max_connections: Option<u32>,

    #[arg(long,value_parser = parse_duration, env = "MARKETPLACE_SOLVER_DATABASE_ACQUIRE_TIMEOUT")]
    pub acquire_timeout: Option<Duration>,

    #[arg(
        long,
        env = "MARKETPLACE_SOLVER_DATABASE_REQUIRE_SSL",
        default_value_t = false
    )]
    pub require_ssl: bool,

    #[arg(
        long,
        env = "MARKETPLACE_SOLVER_DATABASE_RUN_MIGRATIONS",
        default_value_t = true
    )]
    pub migrations: bool,

    #[arg(
        long,
        env = "MARKETPLACE_SOLVER_DATABASE_RESET",
        default_value_t = false
    )]
    pub reset: bool,
}

impl DatabaseOptions {
    pub async fn connect(self) -> anyhow::Result<PostgresClient> {
        PostgresClient::connect(self).await
    }

    pub fn reset(mut self) -> Self {
        self.reset = true;
        self
    }
}
