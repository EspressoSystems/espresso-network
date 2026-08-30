// Empty lib - all tests are integration tests in tests/ directory

use std::time::Duration;

/// How long a leader waits for a block from the builder before proposing an
/// empty one.
///
/// These tests submit no transactions, so the builder's queue is always empty
/// and every view waits this out: it is the seconds per block, and the tests
/// are bound by how many blocks they need. It is set per test rather than on
/// `TestConfigBuilder` because that default is also what the
/// `espresso-dev-node` binary runs on.
pub const BUILDER_TIMEOUT: Duration = Duration::from_millis(250);
