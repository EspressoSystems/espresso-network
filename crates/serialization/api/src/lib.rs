//! Generated API schema types from protobuf schemas.
//!
//! **DO NOT MODIFY FILES IN THIS CRATE MANUALLY**
//!
//! Types are generated from .proto files in the `proto/` directory.
//! To change API schemas, edit the .proto files and run: cargo build -p serialization-api
//!
//! Generated Rust types are committed to git for visibility in code review.

// Generated code - committed to git for visibility in code review
pub mod v1 {
    include!("espresso.api.v1.rs");
}

pub use v1::*;
