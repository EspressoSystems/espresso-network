pub mod block_info;
pub mod builder;
pub mod data_source;
pub mod query_data;
/// The axum server for this API; client-only consumers skip the server stack by leaving the
/// `server` feature off.
#[cfg(feature = "server")]
pub mod router;

pub type Version = vbs::version::StaticVersion<0, 1>;
