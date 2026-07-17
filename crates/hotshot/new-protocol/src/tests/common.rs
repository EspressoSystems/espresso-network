pub(crate) mod assertions;
pub(crate) mod coordinator_builder;
pub(crate) mod harness;
pub(crate) mod mock;
pub(crate) mod runner;
pub(crate) mod utils;

use std::collections::BTreeSet;

use hotshot_types::data::ViewNumber;

/// Collect view numbers from an iterator into a sorted set.
pub(crate) fn views(iter: impl IntoIterator<Item = u64>) -> BTreeSet<ViewNumber> {
    iter.into_iter().map(ViewNumber::new).collect()
}
