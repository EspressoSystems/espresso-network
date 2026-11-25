//! Test helper: tracing events captured into a vector, optionally stamped with a `ManualClock`.

use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use tracing::{Event, Level, Subscriber, field::Field, subscriber::DefaultGuard};
use tracing_subscriber::{
    Layer,
    layer::{Context, SubscriberExt},
};

use crate::system::{Clock, ManualClock};

/// One captured event; `message` is among the `fields`.
#[derive(Clone, Debug)]
pub struct Captured {
    pub level: Level,
    pub target: String,
    pub fields: BTreeMap<String, String>,
    /// The clock's wall time when the event was recorded.
    pub at: Option<DateTime<Utc>>,
}

impl Captured {
    /// The `message` field.
    pub fn message(&self) -> &str {
        self.fields.get("message").map_or("", String::as_str)
    }
}

/// The events collected so far; install with `install`.
#[derive(Clone, Default)]
pub struct Capture {
    events: Arc<Mutex<Vec<Captured>>>,
    clock: Option<ManualClock>,
}

#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

impl<S: Subscriber> Layer<S> for Capture {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.events.lock().unwrap().push(Captured {
            level: *event.metadata().level(),
            target: event.metadata().target().to_owned(),
            fields: fields.0,
            at: self.clock.as_ref().map(Clock::now),
        });
    }
}

impl Capture {
    /// Stamps every event with `clock.now()`.
    pub fn with_clock(clock: ManualClock) -> Self {
        Self {
            events: Arc::default(),
            clock: Some(clock),
        }
    }

    /// Captures events of this thread until the guard drops.
    pub fn install(&self) -> DefaultGuard {
        tracing::subscriber::set_default(tracing_subscriber::registry().with(self.clone()))
    }

    /// Events so far with `message`.
    pub fn with_message(&self, message: &str) -> Vec<Captured> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.message() == message)
            .cloned()
            .collect()
    }
}
