// Copyright (c) 2021-2026 Espresso Systems (espressosys.com)
// This file is part of the Espresso Network repository.

//! A vendor-neutral way for every component to publish metrics, with a no-op implementation
//! ([`NoMetrics`]) and a Prometheus backend (`prometheus::PrometheusMetrics`, feature `prometheus`).
//!
//! # Contract
//!
//! ## What a caller gets
//!
//! - [`Metrics`] creates write-only handles: a [`Counter`] only goes up, a [`Gauge`] is set or
//!   moved by a delta, a [`Histogram`] records points, and a text metric records a constant string.
//!   Each kind also comes as a family ([`MetricsFamily`]), partitioned by label values.
//! - [`Metrics::subgroup`] returns a child group. The same name returns the same group.
//! - Handles are `Send + Sync` and can be cloned.
//!
//! ## Names
//!
//! - A metric's full name is its group path and its own name joined with `_`. The first group is
//!   the Prometheus namespace and the rest form the subsystem, so the paths `a`/`b_c` and
//!   `a`/`b`/`c` produce the same full names.
//! - A full name must be unique in the whole tree. Create each metric once and keep its handle.
//! - A duplicate is not handled the same way by every implementation: `PrometheusMetrics` panics,
//!   [`NoMetrics`] accepts it silently. A duplicate therefore passes tests that use [`NoMetrics`]
//!   and fails only where Prometheus runs. This is known and accepted: checking names would give
//!   [`NoMetrics`] state, and callers create it by value in many places.
//! - A family's [`create`](MetricsFamily::create) takes exactly one value per label name, in
//!   order; Prometheus panics otherwise. The same label values return the same series.
//!   [`destroy`](MetricsFamily::destroy) of unknown label values only logs a warning.
//!
//! ## Implementations
//!
//! - [`NoMetrics`]: every call does nothing and [`Metrics::is_recording`] returns `false`.
//! - `PrometheusMetrics`: the unit label becomes the help text (the name if there is none),
//!   gauges hold unsigned values, histograms use the Prometheus default buckets, and a text
//!   metric is a gauge set to 1. It also reads metrics back by name within a group and exports
//!   the Prometheus text format.
//! - The `prometheus` feature is off by default, so the core depends only on `dyn-clone` and
//!   crates in the SP1 zkVM build can use it.
//!
//! ## Not covered
//!
//! - The trait has no read side: reading a value back needs the concrete backend.
//! - Histogram buckets cannot be configured.
//! - `cliquenet` has its own, separate `Metrics` trait.
//!
//! This crate depends on no other workspace crate, and must not.

use std::fmt::Debug;

use dyn_clone::DynClone;

#[cfg(feature = "prometheus")]
pub mod prometheus;

/// The metrics type.
pub trait Metrics: Send + Sync + DynClone + Debug {
    /// Create a [`Counter`] with an optional `unit_label`.
    ///
    /// The `unit_label` can be used to indicate what the unit of the value is, e.g. "kb" or "seconds"
    fn create_counter(&self, name: String, unit_label: Option<String>) -> Box<dyn Counter>;
    /// Create a [`Gauge`] with an optional `unit_label`.
    ///
    /// The `unit_label` can be used to indicate what the unit of the value is, e.g. "kb" or "seconds"
    fn create_gauge(&self, name: String, unit_label: Option<String>) -> Box<dyn Gauge>;
    /// Create a [`Histogram`] with an optional `unit_label`.
    ///
    /// The `unit_label` can be used to indicate what the unit of the value is, e.g. "kb" or "seconds"
    fn create_histogram(&self, name: String, unit_label: Option<String>) -> Box<dyn Histogram>;

    /// Create a text metric.
    ///
    /// Unlike other metrics, a textmetric  does not have a value. It exists only to record a text
    /// string in the collected metrics, and possibly to care other key-value pairs as part of a
    /// [`TextFamily`]. Thus, the act of creating the text itself is sufficient to populate the text
    /// in the collect metrics; no setter function needs to be called.
    fn create_text(&self, name: String);

    /// Create a family of related counters, partitioned by their label values.
    fn counter_family(&self, name: String, labels: Vec<String>) -> Box<dyn CounterFamily>;

    /// Create a family of related gauges, partitioned by their label values.
    fn gauge_family(&self, name: String, labels: Vec<String>) -> Box<dyn GaugeFamily>;

    /// Create a family of related histograms, partitioned by their label values.
    fn histogram_family(&self, name: String, labels: Vec<String>) -> Box<dyn HistogramFamily>;

    /// Create a family of related text metricx, partitioned by their label values.
    fn text_family(&self, name: String, labels: Vec<String>) -> Box<dyn TextFamily>;

    /// Create a subgroup with a specified prefix.
    fn subgroup(&self, subgroup_name: String) -> Box<dyn Metrics>;

    /// Is this type storing metrics values?
    fn is_recording(&self) -> bool {
        true
    }
}

/// A family of related metrics, partitioned by their label values.
///
/// All metrics in a family have the same name. They are distinguished by a vector of strings
/// called labels. Each label has a name and a value, and each distinct vector of label values
/// within a family acts like a distinct metric.
///
/// The family object is used to instantiate individual metrics within the family via the
/// [`create`](Self::create) method.
///
/// # Examples
///
/// ## Counting HTTP requests, partitioned by method.
///
/// ```
/// # use hotshot_metrics::{Metrics, MetricsFamily, Counter};
/// # fn doc(_metrics: Box<dyn Metrics>) {
/// let metrics: Box<dyn Metrics>;
/// # metrics = _metrics;
/// let http_count = metrics.counter_family("http".into(), vec!["method".into()]);
/// let get_count = http_count.create(vec!["GET".into()]);
/// let post_count = http_count.create(vec!["POST".into()]);
///
/// get_count.add(1);
/// post_count.add(2);
/// # }
/// ```
///
/// This creates Prometheus metrics like
/// ```text
/// http{method="GET"} 1
/// http{method="POST"} 2
/// ```
///
/// ## Using labels to store key-value text pairs.
///
/// ```
/// # use hotshot_metrics::{Metrics, MetricsFamily};
/// # fn doc(_metrics: Box<dyn Metrics>) {
/// let metrics: Box<dyn Metrics>;
/// # metrics = _metrics;
/// metrics
///     .text_family("version".into(), vec!["semver".into(), "rev".into()])
///     .create(vec!["0.1.0".into(), "891c5baa5".into()]);
/// # }
/// ```
///
/// This creates Prometheus metrics like
/// ```text
/// version{semver="0.1.0", rev="891c5baa5"} 1
/// ```
pub trait MetricsFamily<M>: Send + Sync + DynClone + Debug {
    /// Instantiate a metric in this family with a specific label vector.
    ///
    /// The given values of `labels` are used to identify this metric within its family. It must
    /// contain exactly one value for each label name defined when the family was created, in the
    /// same order.
    fn create(&self, labels: Vec<String>) -> M;

    /// Remove a metric in this family identified by its label values.
    ///
    /// The given values of `labels` must match exactly a label vector previously passed to
    /// [`create`](Self::create).
    fn destroy(&self, labels: &[&str]);
}

/// A family of related counters, partitioned by their label values.
pub trait CounterFamily: MetricsFamily<Box<dyn Counter>> {}
impl<T: MetricsFamily<Box<dyn Counter>>> CounterFamily for T {}

/// A family of related gauges, partitioned by their label values.
pub trait GaugeFamily: MetricsFamily<Box<dyn Gauge>> {}
impl<T: MetricsFamily<Box<dyn Gauge>>> GaugeFamily for T {}

/// A family of related histograms, partitioned by their label values.
pub trait HistogramFamily: MetricsFamily<Box<dyn Histogram>> {}
impl<T: MetricsFamily<Box<dyn Histogram>>> HistogramFamily for T {}

/// A family of related text metrics, partitioned by their label values.
pub trait TextFamily: MetricsFamily<()> {}
impl<T: MetricsFamily<()>> TextFamily for T {}

/// Use this if you're not planning to use any metrics. All methods are implemented as a no-op
#[derive(Clone, Copy, Debug, Default)]
pub struct NoMetrics;

impl NoMetrics {
    /// Create a new `Box<dyn Metrics>` with this [`NoMetrics`]
    #[must_use]
    pub fn boxed() -> Box<dyn Metrics> {
        Box::<Self>::default()
    }
}

impl Metrics for NoMetrics {
    fn create_counter(&self, _: String, _: Option<String>) -> Box<dyn Counter> {
        Box::new(NoMetrics)
    }

    fn create_gauge(&self, _: String, _: Option<String>) -> Box<dyn Gauge> {
        Box::new(NoMetrics)
    }

    fn create_histogram(&self, _: String, _: Option<String>) -> Box<dyn Histogram> {
        Box::new(NoMetrics)
    }

    fn create_text(&self, _: String) {}

    fn counter_family(&self, _: String, _: Vec<String>) -> Box<dyn CounterFamily> {
        Box::new(NoMetrics)
    }

    fn gauge_family(&self, _: String, _: Vec<String>) -> Box<dyn GaugeFamily> {
        Box::new(NoMetrics)
    }

    fn histogram_family(&self, _: String, _: Vec<String>) -> Box<dyn HistogramFamily> {
        Box::new(NoMetrics)
    }

    fn text_family(&self, _: String, _: Vec<String>) -> Box<dyn TextFamily> {
        Box::new(NoMetrics)
    }

    fn subgroup(&self, _: String) -> Box<dyn Metrics> {
        Box::new(NoMetrics)
    }

    fn is_recording(&self) -> bool {
        false
    }
}

impl Counter for NoMetrics {
    fn add(&self, _: usize) {}
}
impl Gauge for NoMetrics {
    fn set(&self, _: usize) {}
    fn update(&self, _: i64) {}
}
impl Histogram for NoMetrics {
    fn add_point(&self, _: f64) {}
}
impl MetricsFamily<Box<dyn Counter>> for NoMetrics {
    fn create(&self, _: Vec<String>) -> Box<dyn Counter> {
        Box::new(NoMetrics)
    }

    fn destroy(&self, _: &[&str]) {}
}
impl MetricsFamily<Box<dyn Gauge>> for NoMetrics {
    fn create(&self, _: Vec<String>) -> Box<dyn Gauge> {
        Box::new(NoMetrics)
    }

    fn destroy(&self, _: &[&str]) {}
}
impl MetricsFamily<Box<dyn Histogram>> for NoMetrics {
    fn create(&self, _: Vec<String>) -> Box<dyn Histogram> {
        Box::new(NoMetrics)
    }

    fn destroy(&self, _: &[&str]) {}
}

impl MetricsFamily<()> for NoMetrics {
    fn create(&self, _: Vec<String>) {}

    fn destroy(&self, _: &[&str]) {}
}

/// An ever-incrementing counter
pub trait Counter: Send + Sync + Debug + DynClone {
    /// Add a value to the counter
    fn add(&self, amount: usize);
}

/// A gauge that stores the latest value.
pub trait Gauge: Send + Sync + Debug + DynClone {
    /// Set the gauge value
    fn set(&self, amount: usize);

    /// Update the gauge value
    fn update(&self, delta: i64);
}

/// A histogram which will record a series of points.
pub trait Histogram: Send + Sync + Debug + DynClone {
    /// Add a point to this histogram.
    fn add_point(&self, point: f64);
}

dyn_clone::clone_trait_object!(Metrics);
dyn_clone::clone_trait_object!(Gauge);
dyn_clone::clone_trait_object!(Counter);
dyn_clone::clone_trait_object!(Histogram);

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[derive(Debug, Clone)]
    struct TestMetrics {
        prefix: String,
        values: Arc<Mutex<Inner>>,
    }

    impl TestMetrics {
        fn sub(&self, name: String) -> Self {
            let prefix = if self.prefix.is_empty() {
                name
            } else {
                format!("{}-{name}", self.prefix)
            };
            Self {
                prefix,
                values: Arc::clone(&self.values),
            }
        }

        fn family(&self, labels: Vec<String>) -> Self {
            let mut curr = self.clone();
            for label in labels {
                curr = curr.sub(label);
            }
            curr
        }
    }

    impl Metrics for TestMetrics {
        fn create_counter(
            &self,
            name: String,
            _unit_label: Option<String>,
        ) -> Box<dyn super::Counter> {
            Box::new(self.sub(name))
        }

        fn create_gauge(&self, name: String, _unit_label: Option<String>) -> Box<dyn super::Gauge> {
            Box::new(self.sub(name))
        }

        fn create_histogram(
            &self,
            name: String,
            _unit_label: Option<String>,
        ) -> Box<dyn super::Histogram> {
            Box::new(self.sub(name))
        }

        fn create_text(&self, name: String) {
            self.create_gauge(name, None).set(1);
        }

        fn counter_family(&self, name: String, _: Vec<String>) -> Box<dyn CounterFamily> {
            Box::new(self.sub(name))
        }

        fn gauge_family(&self, name: String, _: Vec<String>) -> Box<dyn GaugeFamily> {
            Box::new(self.sub(name))
        }

        fn histogram_family(&self, name: String, _: Vec<String>) -> Box<dyn HistogramFamily> {
            Box::new(self.sub(name))
        }

        fn text_family(&self, name: String, _: Vec<String>) -> Box<dyn TextFamily> {
            Box::new(self.sub(name))
        }

        fn subgroup(&self, subgroup_name: String) -> Box<dyn Metrics> {
            Box::new(self.sub(subgroup_name))
        }
    }

    impl Counter for TestMetrics {
        fn add(&self, amount: usize) {
            *self
                .values
                .lock()
                .unwrap()
                .counters
                .entry(self.prefix.clone())
                .or_default() += amount;
        }
    }

    impl Gauge for TestMetrics {
        fn set(&self, amount: usize) {
            *self
                .values
                .lock()
                .unwrap()
                .gauges
                .entry(self.prefix.clone())
                .or_default() = amount;
        }
        fn update(&self, delta: i64) {
            let mut values = self.values.lock().unwrap();
            let value = values.gauges.entry(self.prefix.clone()).or_default();
            let signed_value = i64::try_from(*value).unwrap_or(i64::MAX);
            *value = usize::try_from(signed_value + delta).unwrap_or(0);
        }
    }

    impl Histogram for TestMetrics {
        fn add_point(&self, point: f64) {
            self.values
                .lock()
                .unwrap()
                .histograms
                .entry(self.prefix.clone())
                .or_default()
                .push(point);
        }
    }

    impl MetricsFamily<Box<dyn Counter>> for TestMetrics {
        fn create(&self, labels: Vec<String>) -> Box<dyn Counter> {
            Box::new(self.family(labels))
        }

        fn destroy(&self, _: &[&str]) {}
    }

    impl MetricsFamily<Box<dyn Gauge>> for TestMetrics {
        fn create(&self, labels: Vec<String>) -> Box<dyn Gauge> {
            Box::new(self.family(labels))
        }

        fn destroy(&self, _: &[&str]) {}
    }

    impl MetricsFamily<Box<dyn Histogram>> for TestMetrics {
        fn create(&self, labels: Vec<String>) -> Box<dyn Histogram> {
            Box::new(self.family(labels))
        }

        fn destroy(&self, _: &[&str]) {}
    }

    impl MetricsFamily<()> for TestMetrics {
        fn create(&self, labels: Vec<String>) {
            self.family(labels).set(1);
        }

        fn destroy(&self, _: &[&str]) {}
    }

    #[derive(Default, Debug)]
    struct Inner {
        counters: HashMap<String, usize>,
        gauges: HashMap<String, usize>,
        histograms: HashMap<String, Vec<f64>>,
    }

    #[test]
    fn test() {
        let values = Arc::default();
        // This is all scoped so all the arcs should go out of scope
        {
            let metrics: Box<dyn Metrics> = Box::new(TestMetrics {
                prefix: String::new(),
                values: Arc::clone(&values),
            });

            let gauge = metrics.create_gauge("foo".to_string(), None);
            let counter = metrics.create_counter("bar".to_string(), None);
            let histogram = metrics.create_histogram("baz".to_string(), None);

            gauge.set(5);
            gauge.update(-2);

            for i in 0..5 {
                counter.add(i);
            }

            for i in 0..10 {
                histogram.add_point(f64::from(i));
            }

            let sub = metrics.subgroup("child".to_string());

            let sub_gauge = sub.create_gauge("foo".to_string(), None);
            let sub_counter = sub.create_counter("bar".to_string(), None);
            let sub_histogram = sub.create_histogram("baz".to_string(), None);

            sub_gauge.set(10);

            for i in 0..5 {
                sub_counter.add(i * 2);
            }

            for i in 0..10 {
                sub_histogram.add_point(f64::from(i) * 2.0);
            }
        }

        // The above variables are scoped so they should be dropped at this point
        // One of the rare times we can use `Arc::try_unwrap`!
        let values = Arc::try_unwrap(values).unwrap().into_inner().unwrap();
        assert_eq!(values.gauges["foo"], 3);
        assert_eq!(values.counters["bar"], 10); // 0..5
        assert_eq!(
            values.histograms["baz"],
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        );

        assert_eq!(values.gauges["child-foo"], 10);
        assert_eq!(values.counters["child-bar"], 20); // 0..5 *2
        assert_eq!(
            values.histograms["child-baz"],
            vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0]
        );
    }
}
