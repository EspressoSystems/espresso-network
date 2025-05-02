// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

pub mod event;
pub mod upgrade_with_proposal;
pub mod upgrade_with_vote;

use async_trait::async_trait;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum PredicateResult {
    Pass,

    Fail,

    Incomplete,
}

impl From<bool> for PredicateResult {
    fn from(boolean: bool) -> Self {
        match boolean {
            true => PredicateResult::Pass,
            false => PredicateResult::Fail,
        }
    }
}

#[async_trait]
pub trait Predicate<INPUT>: std::fmt::Debug {
    async fn evaluate(&self, input: &INPUT) -> PredicateResult;
    async fn info(&self) -> String;
}
