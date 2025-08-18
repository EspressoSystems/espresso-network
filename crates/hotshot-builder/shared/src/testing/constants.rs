//! Shared constants for tests

use std::time::Duration;

/// Controls the maximum block size of this protocol instance.
/// This is an arbitrary default value for testing.
pub const TEST_PROTOCOL_MAX_BLOCK_SIZE: u64 = 1_000_000;

/// Controls period between optimistic increments of maximum block size limit.
/// This is an arbitrary default value for testing.
pub const TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD: Duration = Duration::from_secs(60);

/// Controls the number of nodes that are used in the VID computation for the tests.
/// This is an arbitrary default value for testing.
pub const TEST_NUM_NODES_IN_VID_COMPUTATION: usize = 4;

/// Controls the number of attempts that the simulated consensus will
/// perform when an error is returned from the Builder when asking for available blocks.
pub const TEST_NUM_CONSENSUS_RETRIES: usize = 4;

/// Governs the buffer size used for the test channels.
/// All of the channels created need a capacity. The concrete capacity isn't
/// specifically bounded in tests, so it is set to an arbitrary value.
pub const TEST_CHANNEL_BUFFER_SIZE: usize = 81920;

/// Governs the target space used by the mapping from txn to its status.
/// This is expressed as a target number of transactions.
/// This is an arbitrary default value for testing.
pub const TEST_TX_STATUS_CACHE_CAPACITY: usize = 10_000_000;

pub const TEST_MAX_TX_NUM: usize = TEST_TX_STATUS_CACHE_CAPACITY;

/// Governs the included transaction GC period used in tests.
/// This is an arbitrary default value for testing.
pub const TEST_INCLUDED_TX_GC_PERIOD: Duration = Duration::from_secs(1);

/// Governs API timeout for builder API.
/// This is an arbitrary default value for testing.
pub const TEST_API_TIMEOUT: Duration = Duration::from_secs(1);

/// Governs timeout when waiting to fill transaction queue on incoming bundle/block request.
/// This is an arbitrary default value for testing.
pub const TEST_MAXIMIZE_TX_CAPTURE_TIMEOUT: Duration = Duration::from_millis(100);

/// Governs fee per byte used by builders.
/// This is an arbitrary default value for testing.
pub const TEST_BASE_FEE: u64 = 1;
