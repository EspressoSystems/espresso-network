//! Append-only segmented log lanes: frame codec, group-commit writer thread, recovery and GC.
//!
//! Knows nothing about the records stored in frames; callers tag frames with a `format::Kind`.

pub mod format;
pub mod lane;
#[cfg(any(test, feature = "testing"))]
pub mod mem;
