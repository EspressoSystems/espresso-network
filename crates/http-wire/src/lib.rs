//! The wire protocol spoken by the network's HTTP APIs, shared by servers and clients.
//!
//! The protocol covers content negotiation between VBS binary and JSON via the `Accept` and
//! `Content-Type` headers, an error envelope (`{"status": <u16>, "message": <string>}`),
//! healthcheck types, and per-content-type WebSocket frame formats. Both halves live in this
//! repo: the axum services (via `espresso-api`) serve what `http-client` calls.
//!
//! This crate is the single implementation of that protocol. Servers and clients call the same
//! codec functions, so compatibility between them holds by construction rather than by parallel
//! implementations kept in sync through matching regression tests. The format must also stay
//! byte-compatible with peers running older tide-disco-based releases; the tests pinning that
//! (variant ordinals, the `Unavailabale` misspelling, the numeric status envelope) live here,
//! next to the one implementation they constrain.
//!
//! The codecs name no transport. HTTP types come from the `http` crate, whose [`StatusCode`] and
//! `HeaderMap` are the very types axum and reqwest re-export, so neither side converts anything,
//! and WebSocket frame payloads are plain bytes and strings (binary frames carry VBS, text frames
//! JSON).
//!
//! The `server` feature, on by default, adds the axum glue every service needs regardless of its
//! routes (`respond`, healthcheck responses, `drive_ws_stream`, `cors_layer`), so services depend
//! on this leaf instead of on another service's API crate. Clients depend on the
//! crate with `default-features = false` and get only the codecs, no server stack.
//!
//! [`StatusCode`]: http::StatusCode

mod body;
mod content_type;
mod error;
mod health;
#[cfg(feature = "server")]
mod server;
mod ws;

pub use body::{DecodeFailure, EncodeFailure, decode_response, encode_body};
pub use content_type::{ContentType, wants_binary};
pub use error::{ServerError, WireError};
pub use health::{AppHealth, HealthCheck, HealthStatus};
#[cfg(feature = "server")]
pub use server::{
    MAX_REQUEST_BODY_BYTES, WireVersion, body_limit_layer, cors_layer, decode_body,
    drive_ws_stream, encode_err, encode_ok, healthcheck_response, module_healthcheck_response,
    respond, spawn_serve,
};
pub use ws::{decode_binary_frame, decode_text_frame, encode_binary_frame, encode_text_frame};
