// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use hotshot_task_impls::events::HotShotEvent;
use hotshot_types::traits::node_implementation::NodeType;

use crate::predicates::{Predicate, PredicateResult};

pub enum InputOrder<TYPES: NodeType> {
    Random(Vec<HotShotEvent<TYPES>>),
    Serial(Vec<HotShotEvent<TYPES>>),
}

#[macro_export]
macro_rules! random {
    ($($x:expr),* $(,)?) => {
        {
            let inputs = vec![$($x),*];
            InputOrder::Random(inputs)
        }
    };
}

#[macro_export]
macro_rules! serial {
    ($($x:expr),* $(,)?) => {
        {
            let inputs = vec![$($x),*];
            InputOrder::Serial(inputs)
        }
    };
}

pub struct TaskScript<TYPES: NodeType, S> {
    /// The time to wait on the receiver for this script.
    pub timeout: Duration,
    pub state: S,
    pub expectations: Vec<Expectations<TYPES, S>>,
}

pub struct Expectations<TYPES: NodeType, S> {
    pub output_asserts: Vec<Box<dyn Predicate<Arc<HotShotEvent<TYPES>>>>>,
    pub task_state_asserts: Vec<Box<dyn Predicate<S>>>,
}

impl<TYPES: NodeType, S> Expectations<TYPES, S> {
    pub fn from_outputs(output_asserts: Vec<Box<dyn Predicate<Arc<HotShotEvent<TYPES>>>>>) -> Self {
        Self {
            output_asserts,
            task_state_asserts: vec![],
        }
    }
    pub fn from_outputs_and_task_states(
        output_asserts: Vec<Box<dyn Predicate<Arc<HotShotEvent<TYPES>>>>>,
        task_state_asserts: Vec<Box<dyn Predicate<S>>>,
    ) -> Self {
        Self {
            output_asserts,
            task_state_asserts,
        }
    }
}

pub fn panic_extra_output_in_script<S>(stage_number: usize, script_name: String, output: &S)
where
    S: std::fmt::Debug,
{
    let extra_output_error = format!(
        "Stage {} | Received unexpected additional output in {}:\n\n{:?}",
        stage_number, script_name, output
    );

    panic!("{}", extra_output_error);
}

pub fn panic_missing_output_in_script<S>(stage_number: usize, script_name: String, predicate: &S)
where
    S: std::fmt::Debug,
{
    let output_missing_error = format!(
        "Stage {} | Failed to receive output for predicate in {}: {:?}",
        stage_number, script_name, predicate
    );

    panic!("{}", output_missing_error);
}

pub async fn validate_task_state_or_panic_in_script<S>(
    stage_number: usize,
    script_name: String,
    state: &S,
    assert: &dyn Predicate<S>,
) {
    assert!(
        assert.evaluate(state).await == PredicateResult::Pass,
        "Stage {} | Task state in {} failed to satisfy: {:?}",
        stage_number,
        script_name,
        assert
    );
}

pub async fn validate_output_or_panic_in_script<S: std::fmt::Debug>(
    stage_number: usize,
    script_name: String,
    output: &S,
    assert: &dyn Predicate<S>,
) -> PredicateResult {
    let result = assert.evaluate(output).await;

    match result {
        PredicateResult::Pass => result,
        PredicateResult::Incomplete => result,
        PredicateResult::Fail => {
            panic!(
                "Stage {} | Output in {} failed to satisfy: {:?}.\n\nReceived:\n\n{:?}",
                stage_number, script_name, assert, output
            )
        },
    }
}
