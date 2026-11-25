// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! configurable constants for hotshot

pub const EPOCH_PARTICIPATION_HISTORY: u64 = 100;

/// The default network data request delay in milliseconds
pub const REQUEST_DATA_DELAY: u64 = 5000;

/// Default channel size for HotShot -> application communication
pub const EXTERNAL_EVENT_CHANNEL_SIZE: usize = 10_000;

/// default number of rounds to run
pub const ORCHESTRATOR_DEFAULT_NUM_ROUNDS: usize = 100;
/// default number of transactions per round
pub const ORCHESTRATOR_DEFAULT_TRANSACTIONS_PER_ROUND: usize = 10;
/// default size of transactions
pub const ORCHESTRATOR_DEFAULT_TRANSACTION_SIZE: usize = 100;
