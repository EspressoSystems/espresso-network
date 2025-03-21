// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::{Duration, Instant};

/// Track (with exponential backoff)
/// sending of some sort of message
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ExponentialBackoff {
    /// Value to reset to when reset is called
    reset_val: Duration,
    /// factor to back off by
    backoff_factor: u32,
    /// the current timeout amount
    timeout: Duration,
    /// when we started the timeout
    started: Option<Instant>,
}

impl ExponentialBackoff {
    /// Create new backoff
    #[must_use]
    pub fn new(backoff_factor: u32, next_timeout: Duration) -> Self {
        ExponentialBackoff {
            backoff_factor,
            timeout: next_timeout * backoff_factor,
            reset_val: next_timeout,
            started: None,
        }
    }

    /// reset backoff
    pub fn reset(&mut self) {
        self.timeout = self.reset_val;
    }

    /// start next timeout
    /// result: whether or not we succeeded
    /// if we succeeded, reset the timeout
    /// else increment the timeout by a factor
    /// of `timeout`
    pub fn start_next(&mut self, result: bool) {
        // success
        if result {
            self.timeout = self.reset_val;
            self.started = Some(Instant::now());
        }
        // failure
        else {
            // note we want to prevent overflow.
            if let Some(r) = self.timeout.checked_mul(self.backoff_factor) {
                self.timeout = r;
            }
            self.started = Some(Instant::now());
        }
    }

    /// Return the timeout duration and start the next timeout.
    pub fn next_timeout(&mut self, result: bool) -> Duration {
        let timeout = self.timeout;
        self.start_next(result);
        timeout
    }
    /// Whether or not the timeout is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(then) = self.started {
            then.elapsed() > self.timeout
        } else {
            true
        }
    }
    /// Marked as expired regardless of time left.
    pub fn expire(&mut self) {
        self.started = None;
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            reset_val: Duration::from_millis(500),
            backoff_factor: 2,
            timeout: Duration::from_millis(500),
            started: None,
        }
    }
}
