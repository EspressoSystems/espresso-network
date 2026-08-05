//! Error types, re-exported from [`http_wire`] under the names surf-disco consumers expect.
//!
//! The envelope's wire shape and its regression tests live in `http-wire`. Servers keep their
//! own envelope types and describe them to the axum glue in `espresso-api`.

pub use http_wire::{ServerError as ClientErr, WireError as ClientError};
