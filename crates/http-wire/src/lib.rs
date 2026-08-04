//! The wire protocol spoken by the network's HTTP APIs, shared by servers and clients.
//!
//! The protocol covers content negotiation between VBS binary and JSON via the `Accept` and
//! `Content-Type` headers, an error envelope (`{"status": <u16>, "message": <string>}`),
//! healthcheck types, and per-content-type WebSocket frame formats. Both halves live in this
//! repo: the axum services (via `espresso-api`) serve what `http-client` calls, and vice versa.
//!
//! This crate is the single implementation of that protocol. Servers and clients call the same
//! codec functions, so compatibility between them holds by construction rather than by parallel
//! implementations kept in sync through matching regression tests. The format must also stay
//! byte-compatible with peers running older tide-disco-based releases; the tests pinning that
//! (variant ordinals, the `Unavailabale` misspelling, the numeric status envelope) live here,
//! next to the one implementation they constrain.
//!
//! Nothing here names a transport. HTTP types come from the `http` crate, whose [`StatusCode`]
//! and `HeaderMap` are the very types axum and reqwest re-export, so neither side converts
//! anything. WebSocket frame payloads are plain bytes and strings (binary frames carry VBS,
//! text frames JSON), so any WebSocket implementation can map them onto its own frame type.
//!
//! [`StatusCode`]: http::StatusCode

mod body;
mod content_type;
mod error;
mod health;
mod ws;

pub use body::{DecodeFailure, EncodeFailure, decode_body, decode_response, encode_body};
pub use content_type::{ContentType, wants_binary};
pub use error::{ServerError, WireError};
pub use health::{AppHealth, HealthCheck, HealthStatus};
pub use ws::{decode_binary_frame, decode_text_frame, encode_binary_frame, encode_text_frame};
