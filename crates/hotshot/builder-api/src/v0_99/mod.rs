pub mod builder;
pub mod data_source;
/// No changes to this module
pub use super::v0_1::query_data;

pub type Version = vbs::version::StaticVersion<0, 99>;
