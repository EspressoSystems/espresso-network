//! A storage instance: serves the chain store over gRPC, and takes its share of the decide log
//! from the deployment's consensus node into the store.

pub mod ingest;
