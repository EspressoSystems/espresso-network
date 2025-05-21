use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use rand::Rng;
use tokio::time::sleep;

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
/// What type of delay we want to apply to
pub enum DelayOptions {
    None,
    Random,
    Fixed,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
/// Current implementations that are supported for testing async delays
pub enum SupportedTraitTypesForAsyncDelay {
    Storage,
    ValidatedState,
    BlockHeader,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
/// Config for each supported type
pub struct DelaySettings {
    // Option to tell the async function what to do
    pub delay_option: DelayOptions,
    // Rng min time
    pub min_time_in_milliseconds: u64,
    // Rng max time
    pub max_time_in_milliseconds: u64,
    // Fixed time for fixed delay option
    pub fixed_time_in_milliseconds: u64,
}

impl Default for DelaySettings {
    fn default() -> Self {
        DelaySettings {
            delay_option: DelayOptions::None,
            min_time_in_milliseconds: 0,
            max_time_in_milliseconds: 0,
            fixed_time_in_milliseconds: 0,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Default)]
/// Settings for each type
pub struct DelayConfig {
    config: HashMap<SupportedTraitTypesForAsyncDelay, DelaySettings>,
}

impl DelayConfig {
    pub fn new(config: HashMap<SupportedTraitTypesForAsyncDelay, DelaySettings>) -> Self {
        DelayConfig { config }
    }

    pub fn add_settings_for_all_types(&mut self, settings: DelaySettings) {
        let iterator = SupportedTraitTypesForAsyncDelayIterator::new();

        for supported_type in iterator {
            self.config.insert(supported_type, settings.clone());
        }
    }

    pub fn add_setting(
        &mut self,
        supported_type: SupportedTraitTypesForAsyncDelay,
        settings: &DelaySettings,
    ) {
        self.config.insert(supported_type, settings.clone());
    }

    pub fn get_setting(
        &self,
        supported_type: &SupportedTraitTypesForAsyncDelay,
    ) -> Option<&DelaySettings> {
        self.config.get(supported_type)
    }
}

#[async_trait]
/// Implement this method to add some delay to async call
pub trait TestableDelay {
    /// Add a delay from settings
    async fn handle_async_delay(settings: &DelaySettings) {
        match settings.delay_option {
            DelayOptions::None => {},
            DelayOptions::Fixed => {
                sleep(Duration::from_millis(settings.fixed_time_in_milliseconds)).await;
            },
            DelayOptions::Random => {
                let sleep_in_millis = rand::thread_rng().gen_range(
                    settings.min_time_in_milliseconds..=settings.max_time_in_milliseconds,
                );
                sleep(Duration::from_millis(sleep_in_millis)).await;
            },
        }
    }

    /// Look for settings in the config and run it
    async fn run_delay_settings_from_config(delay_config: &DelayConfig);
}

/// Iterator to iterate over enum
struct SupportedTraitTypesForAsyncDelayIterator {
    index: usize,
}

impl SupportedTraitTypesForAsyncDelayIterator {
    fn new() -> Self {
        SupportedTraitTypesForAsyncDelayIterator { index: 0 }
    }
}

impl Iterator for SupportedTraitTypesForAsyncDelayIterator {
    type Item = SupportedTraitTypesForAsyncDelay;

    fn next(&mut self) -> Option<Self::Item> {
        let supported_type = match self.index {
            0 => Some(SupportedTraitTypesForAsyncDelay::Storage),
            1 => Some(SupportedTraitTypesForAsyncDelay::ValidatedState),
            2 => Some(SupportedTraitTypesForAsyncDelay::BlockHeader),
            _ => {
                assert_eq!(self.index, 3, "Need to ensure that newly added or removed `SupportedTraitTypesForAsyncDelay` enum is handled in iterator");
                return None;
            },
        };
        self.index += 1;
        supported_type
    }
}
