//! Tracks whether this node is currently dispersing a block it is building.
//!
//! Erasure-coding a block, fanning its shares out, and reconstructing someone
//! else's block all saturate the same rayon pool. When a leader's dispersal
//! overlaps a reconstruction, the dispersal slows down measurably (the encode
//! step alone runs 1.6-1.9x longer, and a reconstruction that starts mid-encode
//! inflates it 3-6x), which delays the shares every replica needs before it can
//! reconstruct and vote — so the contention lands on the certificate that gates
//! the next view.
//!
//! The gate lets the coordinator defer *starting* new reconstructions while a
//! dispersal is in flight. It counts dispersals rather than holding a flag so
//! overlapping builds (a timeout re-request, an epoch boundary) can't clear it
//! early, and hands out an RAII [`DispersalGuard`] so a panicked or aborted
//! build still releases it.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Shared count of in-flight leader dispersals (erasure-code + share fan-out).
///
/// Cloned into the block-building path and read by the coordinator; a default
/// gate that nothing ever enters simply reports inactive.
#[derive(Clone, Debug, Default)]
pub struct DispersalGate(Arc<AtomicUsize>);

/// Marks one dispersal in flight until dropped.
#[derive(Debug)]
pub struct DispersalGuard(Arc<AtomicUsize>);

impl DispersalGate {
    /// Mark a dispersal in flight. Hold the guard for as long as the dispersal
    /// occupies the rayon pool — i.e. move it into the fan-out task, not just
    /// the encode, since the fan-out is the larger half of the work.
    pub fn enter(&self) -> DispersalGuard {
        self.0.fetch_add(1, Ordering::SeqCst);
        DispersalGuard(Arc::clone(&self.0))
    }

    /// Whether any dispersal is in flight.
    pub fn active(&self) -> bool {
        self.0.load(Ordering::SeqCst) > 0
    }
}

impl Drop for DispersalGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
