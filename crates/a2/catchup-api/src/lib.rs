//! The catchup protocol: what a consensus node that is behind asks API services for, and what an
//! API service answers. Neither side owns it, so neither depends on the other.

/// The protocol, generated from `proto/espresso/a2/catchup/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.catchup.v1");
}
