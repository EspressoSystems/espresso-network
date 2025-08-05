#![allow(dead_code)]

// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use hotshot_types::traits::metrics;
use itertools::Itertools;
use prometheus::{
    core::{AtomicU64, GenericCounter, GenericCounterVec, GenericGauge, GenericGaugeVec},
    Encoder, HistogramVec, Opts, Registry, TextEncoder,
};
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum MetricsError {
    NoSuchSubgroup {
        path: Vec<String>,
    },
    NoSuchMetric {
        namespace: Vec<String>,
        name: String,
    },
    Prometheus {
        source: prometheus::Error,
    },
}

impl From<prometheus::Error> for MetricsError {
    fn from(source: prometheus::Error) -> Self {
        Self::Prometheus { source }
    }
}

/// A Prometheus-based implementation of a [Metrics](metrics::Metrics) registry.
///
/// [PrometheusMetrics] provides a collection of metrics including [Counter], [Gauge], and
/// [Histogram]. These metrics can be created and associated with a [PrometheusMetrics] collection
/// and then used as handles for updating and populating. The [PrometheusMetrics] registry can then
/// be used to collect all of the associated metrics and export them in the Prometheus text format.
///
/// This implementation provides a few features beyond the basic [prometheus] features. It supports
/// hierarchical namespaces; any [PrometheusMetrics] can be used to derive a subgroup with a certain
/// name. The subgroup is then related to the parent, and any [PrometheusMetrics] in the tree of
/// related groups can be used to collect _all_ registered metrics. The namespacing will be
/// reflected in the fully qualified name of each metric in the Prometheus output. The subgroup
/// relationship is pure and deterministic -- calling
/// [get_subgroup](PrometheusMetrics::get_subgroup) with the same subgroup name will always return a
/// handle to the same underlying [PrometheusMetrics] object.
///
/// [PrometheusMetrics] also supports querying for individual metrics by name, unlike
/// [prometheus::Registry]. This provides a programming interface for inspecting the values of
/// specific metrics at run-time, if that is preferable to exporting all metrics wholesale.
#[derive(Clone, Debug, Default)]
pub struct PrometheusMetrics {
    metrics: Registry,
    namespace: Vec<String>,
    children: Arc<RwLock<HashMap<String, PrometheusMetrics>>>,
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    counter_families: Arc<RwLock<HashMap<String, CounterFamily>>>,
    gauge_families: Arc<RwLock<HashMap<String, GaugeFamily>>>,
    histogram_families: Arc<RwLock<HashMap<String, HistogramFamily>>>,
}

impl PrometheusMetrics {
    /// Get a counter in this sub-group by name.
    pub fn get_counter(&self, name: &str) -> Result<Counter, MetricsError> {
        self.get_metric(&self.counters, name)
    }

    /// Get a gauge in this sub-group by name.
    pub fn get_gauge(&self, name: &str) -> Result<Gauge, MetricsError> {
        self.get_metric(&self.gauges, name)
    }

    /// Get a histogram in this sub-group by name.
    pub fn get_histogram(&self, name: &str) -> Result<Histogram, MetricsError> {
        self.get_metric(&self.histograms, name)
    }

    /// Get a counter family in this sub-group by name.
    pub fn get_counter_family(&self, name: &str) -> Result<CounterFamily, MetricsError> {
        self.get_metric(&self.counter_families, name)
    }

    /// Get a gauge family in this sub-group by name.
    pub fn gauge_family(&self, name: &str) -> Result<GaugeFamily, MetricsError> {
        self.get_metric(&self.gauge_families, name)
    }

    /// Get a histogram family in this sub-group by name.
    pub fn get_histogram_family(&self, name: &str) -> Result<HistogramFamily, MetricsError> {
        self.get_metric(&self.histogram_families, name)
    }

    /// Get a (possibly nested) subgroup of this group by its path.
    pub fn get_subgroup<I>(&self, path: I) -> Result<PrometheusMetrics, MetricsError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut curr = self.clone();
        for seg in path.into_iter() {
            let next = curr
                .children
                .read()
                .unwrap()
                .get(seg.as_ref())
                .ok_or_else(|| MetricsError::NoSuchSubgroup {
                    path: {
                        let mut path = curr.namespace.clone();
                        path.push(seg.as_ref().to_string());
                        path
                    },
                })?
                .clone();
            curr = next;
        }
        Ok(curr)
    }

    fn get_metric<M: Clone>(
        &self,
        metrics: &Arc<RwLock<HashMap<String, M>>>,
        name: &str,
    ) -> Result<M, MetricsError> {
        metrics
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| MetricsError::NoSuchMetric {
                namespace: self.namespace.clone(),
                name: name.to_string(),
            })
    }

    fn metric_opts(&self, name: String, unit_label: Option<String>) -> Opts {
        let help = unit_label.unwrap_or_else(|| name.clone());
        let mut opts = Opts::new(name, help);
        let mut group_names = self.namespace.iter();
        if let Some(namespace) = group_names.next() {
            opts = opts
                .namespace(namespace.clone())
                .subsystem(group_names.join("_"));
        }
        opts
    }
}

impl tide_disco::metrics::Metrics for PrometheusMetrics {
    type Error = MetricsError;

    fn export(&self) -> Result<String, Self::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.metrics.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|err| MetricsError::Prometheus {
            source: prometheus::Error::Msg(format!(
                "could not convert Prometheus output to UTF-8: {err}"
            )),
        })
    }
}

impl metrics::Metrics for PrometheusMetrics {
    fn create_counter(
        &self,
        name: String,
        unit_label: Option<String>,
    ) -> Box<dyn metrics::Counter> {
        let counter = Counter::new(&self.metrics, self.metric_opts(name.clone(), unit_label));
        self.counters.write().unwrap().insert(name, counter.clone());
        Box::new(counter)
    }

    fn create_gauge(&self, name: String, unit_label: Option<String>) -> Box<dyn metrics::Gauge> {
        let gauge = Gauge::new(&self.metrics, self.metric_opts(name.clone(), unit_label));
        self.gauges.write().unwrap().insert(name, gauge.clone());
        Box::new(gauge)
    }

    fn create_histogram(
        &self,
        name: String,
        unit_label: Option<String>,
    ) -> Box<dyn metrics::Histogram> {
        let histogram = Histogram::new(&self.metrics, self.metric_opts(name.clone(), unit_label));
        self.histograms
            .write()
            .unwrap()
            .insert(name, histogram.clone());
        Box::new(histogram)
    }

    fn create_text(&self, name: String) {
        self.create_gauge(name, None).set(1);
    }

    fn counter_family(&self, name: String, labels: Vec<String>) -> Box<dyn metrics::CounterFamily> {
        let family =
            CounterFamily::new(&self.metrics, self.metric_opts(name.clone(), None), &labels);
        self.counter_families
            .write()
            .unwrap()
            .insert(name, family.clone());
        Box::new(family)
    }

    fn gauge_family(&self, name: String, labels: Vec<String>) -> Box<dyn metrics::GaugeFamily> {
        let family = GaugeFamily::new(&self.metrics, self.metric_opts(name.clone(), None), &labels);
        self.gauge_families
            .write()
            .unwrap()
            .insert(name, family.clone());
        Box::new(family)
    }

    fn histogram_family(
        &self,
        name: String,
        labels: Vec<String>,
    ) -> Box<dyn metrics::HistogramFamily> {
        let family =
            HistogramFamily::new(&self.metrics, self.metric_opts(name.clone(), None), &labels);
        self.histogram_families
            .write()
            .unwrap()
            .insert(name, family.clone());
        Box::new(family)
    }

    fn text_family(&self, name: String, labels: Vec<String>) -> Box<dyn metrics::TextFamily> {
        Box::new(TextFamily::new(
            &self.metrics,
            self.metric_opts(name.clone(), None),
            &labels,
        ))
    }

    fn subgroup(&self, subgroup_name: String) -> Box<dyn metrics::Metrics> {
        Box::new(
            self.children
                .write()
                .unwrap()
                .entry(subgroup_name.clone())
                .or_insert_with(|| Self {
                    metrics: self.metrics.clone(),
                    namespace: {
                        let mut namespace = self.namespace.clone();
                        namespace.push(subgroup_name);
                        namespace
                    },
                    ..Default::default()
                })
                .clone(),
        )
    }
}

/// A [Counter](metrics::Counter) metric.
#[derive(Clone, Debug)]
pub struct Counter(GenericCounter<AtomicU64>);

impl Counter {
    fn new(registry: &Registry, opts: Opts) -> Self {
        let counter = GenericCounter::with_opts(opts).unwrap();
        registry.register(Box::new(counter.clone())).unwrap();
        Self(counter)
    }

    pub fn get(&self) -> usize {
        self.0.get() as usize
    }
}

impl metrics::Counter for Counter {
    fn add(&self, amount: usize) {
        self.0.inc_by(amount as u64);
    }
}

/// A [Gauge](metrics::Gauge) metric.
#[derive(Clone, Debug)]
pub struct Gauge(GenericGauge<AtomicU64>);

impl Gauge {
    fn new(registry: &Registry, opts: Opts) -> Self {
        let gauge = GenericGauge::with_opts(opts).unwrap();
        registry.register(Box::new(gauge.clone())).unwrap();
        Self(gauge)
    }

    pub fn get(&self) -> usize {
        self.0.get() as usize
    }
}

impl metrics::Gauge for Gauge {
    fn set(&self, amount: usize) {
        self.0.set(amount as u64);
    }

    fn update(&self, delta: i64) {
        if delta >= 0 {
            self.0.add(delta as u64);
        } else {
            self.0.sub(-delta as u64);
        }
    }
}

/// A [Histogram](metrics::Histogram) metric.
#[derive(Clone, Debug)]
pub struct Histogram(prometheus::Histogram);

impl Histogram {
    fn new(registry: &Registry, opts: Opts) -> Self {
        let histogram = prometheus::Histogram::with_opts(opts.into()).unwrap();
        registry.register(Box::new(histogram.clone())).unwrap();
        Self(histogram)
    }

    pub fn sample_count(&self) -> usize {
        self.0.get_sample_count() as usize
    }

    pub fn sum(&self) -> f64 {
        self.0.get_sample_sum()
    }

    pub fn mean(&self) -> f64 {
        self.sum() / (self.sample_count() as f64)
    }
}

impl metrics::Histogram for Histogram {
    fn add_point(&self, point: f64) {
        self.0.observe(point);
    }
}

/// A [CounterFamily](metrics::CounterFamily) metric.
#[derive(Clone, Debug)]
pub struct CounterFamily(GenericCounterVec<AtomicU64>);

impl CounterFamily {
    fn new(registry: &Registry, opts: Opts, labels: &[String]) -> Self {
        let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
        let family = GenericCounterVec::new(opts, &labels).unwrap();
        registry.register(Box::new(family.clone())).unwrap();
        Self(family)
    }

    pub fn get(&self, label_values: &[impl AsRef<str>]) -> Counter {
        let labels = label_values.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        Counter(self.0.get_metric_with_label_values(&labels).unwrap())
    }
}

impl metrics::MetricsFamily<Box<dyn metrics::Counter>> for CounterFamily {
    fn create(&self, labels: Vec<String>) -> Box<dyn metrics::Counter> {
        Box::new(self.get(&labels))
    }
}

/// A [GaugeFamily](metrics::GaugeFamily) metric.
#[derive(Clone, Debug)]
pub struct GaugeFamily(GenericGaugeVec<AtomicU64>);

impl GaugeFamily {
    fn new(registry: &Registry, opts: Opts, labels: &[String]) -> Self {
        let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
        let family = GenericGaugeVec::new(opts, &labels).unwrap();
        registry.register(Box::new(family.clone())).unwrap();
        Self(family)
    }

    pub fn get(&self, label_values: &[impl AsRef<str>]) -> Gauge {
        let labels = label_values.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        Gauge(self.0.get_metric_with_label_values(&labels).unwrap())
    }
}

impl metrics::MetricsFamily<Box<dyn metrics::Gauge>> for GaugeFamily {
    fn create(&self, labels: Vec<String>) -> Box<dyn metrics::Gauge> {
        Box::new(self.get(&labels))
    }
}

/// A [HistogramFamily](metrics::HistogramFamily) metric.
#[derive(Clone, Debug)]
pub struct HistogramFamily(HistogramVec);

impl HistogramFamily {
    fn new(registry: &Registry, opts: Opts, labels: &[String]) -> Self {
        let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
        let family = HistogramVec::new(opts.into(), &labels).unwrap();
        registry.register(Box::new(family.clone())).unwrap();
        Self(family)
    }

    pub fn get(&self, label_values: &[impl AsRef<str>]) -> Histogram {
        let labels = label_values.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        Histogram(self.0.get_metric_with_label_values(&labels).unwrap())
    }
}

impl metrics::MetricsFamily<Box<dyn metrics::Histogram>> for HistogramFamily {
    fn create(&self, labels: Vec<String>) -> Box<dyn metrics::Histogram> {
        Box::new(self.get(&labels))
    }
}

/// A [TextFamily](metrics::TextFamily) metric.
#[derive(Clone, Debug)]
pub struct TextFamily(GaugeFamily);

impl TextFamily {
    fn new(registry: &Registry, opts: Opts, labels: &[String]) -> Self {
        Self(GaugeFamily::new(registry, opts, labels))
    }
}

impl metrics::MetricsFamily<()> for TextFamily {
    fn create(&self, labels: Vec<String>) {
        self.0.create(labels).set(1);
    }
}

#[cfg(test)]
mod test {
    use metrics::Metrics;
    use tide_disco::metrics::Metrics as _;

    use super::*;

    #[test_log::test]
    fn test_prometheus_metrics() {
        let metrics = PrometheusMetrics::default();

        // Register one metric of each type.
        let counter = metrics.create_counter("counter".into(), None);
        let gauge = metrics.create_gauge("gauge".into(), None);
        let histogram = metrics.create_histogram("histogram".into(), None);
        metrics.create_text("text".into());

        // Set the metric values.
        counter.add(20);
        gauge.set(42);
        histogram.add_point(20f64);

        // Check the values.
        assert_eq!(metrics.get_counter("counter").unwrap().get(), 20);
        assert_eq!(metrics.get_gauge("gauge").unwrap().get(), 42);
        assert_eq!(
            metrics.get_histogram("histogram").unwrap().sample_count(),
            1
        );
        assert_eq!(metrics.get_histogram("histogram").unwrap().sum(), 20f64);
        assert_eq!(metrics.get_histogram("histogram").unwrap().mean(), 20f64);

        // Set the metric values again, to be sure they update properly.
        counter.add(22);
        gauge.set(100);
        histogram.add_point(22f64);

        // Check the updated values.
        assert_eq!(metrics.get_counter("counter").unwrap().get(), 42);
        assert_eq!(metrics.get_gauge("gauge").unwrap().get(), 100);
        assert_eq!(
            metrics.get_histogram("histogram").unwrap().sample_count(),
            2
        );
        assert_eq!(metrics.get_histogram("histogram").unwrap().sum(), 42f64);
        assert_eq!(metrics.get_histogram("histogram").unwrap().mean(), 21f64);

        // Export to a Prometheus string.
        let string = metrics.export().unwrap();
        // Make sure the output makes sense.
        let lines = string.lines().collect::<Vec<_>>();
        assert!(lines.contains(&"counter 42"));
        assert!(lines.contains(&"gauge 100"));
        assert!(lines.contains(&"histogram_sum 42"));
        assert!(lines.contains(&"histogram_count 2"));
        assert!(lines.contains(&"text 1"));
    }

    #[test_log::test]
    fn test_namespace() {
        let metrics = PrometheusMetrics::default();
        let subgroup1 = metrics.subgroup("subgroup1".into());
        let subgroup2 = subgroup1.subgroup("subgroup2".into());
        let counter = subgroup2.create_counter("counter".into(), None);
        subgroup2.create_text("text".into());
        counter.add(42);

        // Check namespacing.
        assert_eq!(
            metrics.get_subgroup(["subgroup1"]).unwrap().namespace,
            ["subgroup1"]
        );
        assert_eq!(
            metrics
                .get_subgroup(["subgroup1", "subgroup2"])
                .unwrap()
                .namespace,
            ["subgroup1", "subgroup2"]
        );
        assert_eq!(
            metrics
                .get_subgroup(["subgroup1"])
                .unwrap()
                .get_subgroup(["subgroup2"])
                .unwrap()
                .namespace,
            ["subgroup1", "subgroup2"]
        );

        // Check different ways of accessing the counter.
        assert_eq!(
            metrics
                .get_subgroup(["subgroup1", "subgroup2"])
                .unwrap()
                .get_counter("counter")
                .unwrap()
                .get(),
            42
        );
        assert_eq!(
            metrics
                .get_subgroup(["subgroup1"])
                .unwrap()
                .get_subgroup(["subgroup2"])
                .unwrap()
                .get_counter("counter")
                .unwrap()
                .get(),
            42
        );

        // Check fully-qualified counter name in export.
        assert!(metrics
            .export()
            .unwrap()
            .lines()
            .contains(&"subgroup1_subgroup2_counter 42"));

        // Check fully-qualified text name in export.
        assert!(metrics
            .export()
            .unwrap()
            .lines()
            .contains(&"subgroup1_subgroup2_text 1"));
    }

    #[test_log::test]
    fn test_labels() {
        let metrics = PrometheusMetrics::default();

        let http_count = metrics.counter_family("http".into(), vec!["method".into()]);
        let get_count = http_count.create(vec!["GET".into()]);
        let post_count = http_count.create(vec!["POST".into()]);
        get_count.add(1);
        post_count.add(2);

        metrics
            .text_family("version".into(), vec!["semver".into(), "rev".into()])
            .create(vec!["0.1.0".into(), "d1b650a7".into()]);

        assert_eq!(
            metrics
                .get_counter_family("http")
                .unwrap()
                .get(&["GET"])
                .get(),
            1
        );
        assert_eq!(
            metrics
                .get_counter_family("http")
                .unwrap()
                .get(&["POST"])
                .get(),
            2
        );

        // Export to a Prometheus string.
        let string = metrics.export().unwrap();
        // Make sure the output makes sense.
        let lines = string.lines().collect::<Vec<_>>();
        assert!(lines.contains(&"http{method=\"GET\"} 1"), "{lines:?}");
        assert!(lines.contains(&"http{method=\"POST\"} 2"), "{lines:?}");
        assert!(
            lines.contains(&"version{rev=\"d1b650a7\",semver=\"0.1.0\"} 1"),
            "{lines:?}"
        );
    }
}
