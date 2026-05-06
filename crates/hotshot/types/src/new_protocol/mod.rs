//! types for the new HotShot protocol.

pub mod event;
pub mod proposal;

pub use event::{CoordinatorEvent, NewDecideEvent};
pub use proposal::Proposal;
