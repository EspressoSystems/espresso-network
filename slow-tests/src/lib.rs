// Empty lib - all tests are integration tests in tests/ directory

use std::time::Duration;

/// How long a leader waits for a block before proposing an empty one.
///
/// These tests submit no transactions, so this is their seconds per block. Set
/// per test rather than on `TestConfigBuilder`, whose default the
/// `espresso-dev-node` binary also runs on.
pub const BUILDER_TIMEOUT: Duration = Duration::from_millis(250);
