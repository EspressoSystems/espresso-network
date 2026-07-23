//! Axum route registration for every v1 API module.
//!
//! One `router_*` builder per module (one file each). Each module file lists its
//! version-agnostic routes in a single `router_*` table, with the handlers as named `async fn`s
//! above it; the builder nests the routes under the module's base prefix. The shared
//! response/encoding/websocket helpers live in the grandparent [`super::super`] module
//! ([`mod.rs`](super::super)).

use super::*;

mod availability;
mod block_state;
mod catchup;
mod config;
mod database;
mod explorer;
mod fee_state;
mod hotshot_events;
mod light_client;
mod node;
mod reward;
mod state_signature;
mod status;
mod submit;
mod token;

pub(crate) use availability::router_availability;
pub(crate) use block_state::router_block_state;
pub(crate) use catchup::router_catchup;
pub(crate) use config::router_config;
pub(crate) use database::router_database;
pub(crate) use explorer::router_explorer;
pub(crate) use fee_state::router_fee_state;
pub(crate) use hotshot_events::router_hotshot_events;
pub(crate) use light_client::router_light_client;
pub(crate) use node::router_node;
pub(crate) use reward::router_reward;
pub(crate) use state_signature::router_state_signature;
pub(crate) use status::router_status;
pub(crate) use submit::router_submit;
pub(crate) use token::router_token;
