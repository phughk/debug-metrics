use crate::config::DebugMetricsConfig;
use crate::drop_hook::DropHook;
use crate::DebugMetricsSafe;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{stdout, Stdout, Write};

/// DebugMetrics that serve as a convenient way to debug complex code.
///
/// This is not at all OTEL production metrics.
pub struct DebugMetrics<W: Write> {
    /// Which other metrics need to be taken
    /// Regexes to match against keys.
    rules: BTreeMap<String, BTreeSet<&'static str>>,
    counts: BTreeMap<String, u64>,
    labels: BTreeMap<String, String>,
    events: Vec<EventType>,
    drop_print: BTreeSet<String>,
    output_writer: W,
    config: DebugMetricsConfig,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum EventType {
    MetricChange {
        metric: String,
        count: u64,
        dependencies: BTreeMap<String, u64>,
        labels: BTreeMap<String, String>,
    },
    LabelChange {
        label: String,
        value: String,
        dependencies: BTreeMap<String, u64>,
        labels: BTreeMap<String, String>,
    },
    CascadeMetricChange {
        cause: String,
        metric: String,
        count: u64,
        dependencies: BTreeMap<String, u64>,
        labels: BTreeMap<String, String>,
    },
    CascadeLabelChange {
        cause: String,
        label: String,
        value: String,
        dependencies: BTreeMap<String, u64>,
        labels: BTreeMap<String, String>,
    },
}

impl EventType {
    pub fn promote_to_cascade(self, cause: &str) -> Self {
        match self {
            EventType::MetricChange {
                metric,
                count,
                dependencies,
                labels,
            } => EventType::CascadeMetricChange {
                cause: cause.to_string(),
                metric,
                count,
                dependencies,
                labels,
            },
            EventType::LabelChange {
                label,
                value,
                dependencies,
                labels,
            } => EventType::CascadeLabelChange {
                cause: cause.to_string(),
                label,
                value,
                dependencies,
                labels,
            },
            _ => {
                unreachable!("Unable to promote to cascade: {:?}", self)
            }
        }
    }
}

impl Default for DebugMetrics<Stdout> {
    fn default() -> Self {
        let config = DebugMetricsConfig::default();
        DebugMetrics::new(stdout(), config)
    }
}

pub trait DefaultExt {
    fn default_on() -> Self;
}

impl DefaultExt for DebugMetrics<Stdout> {
    fn default_on() -> Self {
        let config = DebugMetricsConfig::default_on();
        DebugMetrics::new(stdout(), config)
    }
}

enum Value {
    Metric(u64),
    Label(String),
}

pub trait DebugMetricsTrait {
    fn add_recording_rule<Key: Into<String>>(&mut self, metric: Key, additional: &[&'static str]);

    fn add_drop_hook<Key: Into<String>>(&mut self, key: Key);

    fn inc<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &mut self,
        key: Key,
        labels: Vec<(LabelKey, LabelVal)>,
    );

    fn set<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &mut self,
        key: Key,
        value: u64,
        labels: Vec<(LabelKey, LabelVal)>,
    );

    fn set_label<Key: Into<String>, Value: Into<String>>(&mut self, key: Key, value: Value);

    fn events_for_key<Key: Into<String>>(&self, key: Key) -> Vec<EventType>;

    fn with_drop_hook<CallFn>(&mut self, call_fn: CallFn) -> DropHook<Self, CallFn>
    where
        CallFn: Fn(&mut Self),
    {
        DropHook {
            debug_metrics: self,
            call_fn,
        }
    }
}

impl<W: Write> DebugMetrics<W> {
    pub fn new(writer: W, config: DebugMetricsConfig) -> DebugMetrics<W> {
        DebugMetrics {
            rules: Default::default(),
            counts: Default::default(),
            labels: Default::default(),
            events: Default::default(),
            drop_print: Default::default(),
            output_writer: writer,
            config,
        }
    }

    pub fn safe(self) -> DebugMetricsSafe<DebugMetrics<W>> {
        DebugMetricsSafe::new(self)
    }

    fn matching_rules_for_regexes(
        &self,
        regexes: &BTreeSet<&'static str>,
        counts: &BTreeMap<String, u64>,
        labels: &BTreeMap<String, String>,
    ) -> (BTreeMap<String, u64>, BTreeMap<String, String>) {
        let mut found = BTreeSet::new();
        let mut count_ret = BTreeMap::new();
        let mut label_ret = BTreeMap::new();
        for patt in regexes {
            for (k, v) in counts {
                // Create regex from pattern
                let re = regex::Regex::new(patt).unwrap();
                if !found.contains(k) && re.is_match(k) {
                    found.insert(k);
                    count_ret.insert(k.to_string(), *v);
                }
            }
            for (k, v) in labels {
                // Create regex from pattern
                let re = regex::Regex::new(patt).unwrap();
                if !found.contains(k) && re.is_match(k) {
                    found.insert(k);
                    label_ret.insert(k.to_string(), v.clone());
                }
            }
        }
        (count_ret, label_ret)
    }
    fn maybe_include_all_labels_with_event(&self, event: &mut Option<EventType>) {
        if self.config.all_labels_every_event {
            if let Some(event) = event {
                for (label_key, label_value) in &self.labels {
                    match event {
                        EventType::MetricChange {
                            metric,
                            count,
                            dependencies,
                            labels,
                        } => {
                            labels.insert(label_key.clone(), label_value.clone());
                        }
                        EventType::LabelChange {
                            label,
                            value,
                            dependencies,
                            labels,
                        } => {
                            labels.insert(label_key.clone(), label_value.clone());
                        }
                        _ => {
                            unreachable!("Unexpected event type: {:?}", event);
                        }
                    }
                }
            }
        }
    }
    fn maybe_find_matching_rule(&self, event: &mut Option<EventType>, metric_or_label: &str) {
        if let Some(rules) = self.rules.get(metric_or_label) {
            let (matching_metrics, matching_labels) =
                self.matching_rules_for_regexes(rules, &self.counts, &self.labels);
            let c = self.get_metric_or_label(metric_or_label);
            match c {
                None => {}
                Some(Value::Metric(c)) => {
                    *event = Some(EventType::MetricChange {
                        metric: metric_or_label.to_string(),
                        count: c,
                        dependencies: matching_metrics,
                        labels: matching_labels,
                    });
                }
                Some(Value::Label(l)) => {
                    *event = Some(EventType::LabelChange {
                        label: metric_or_label.to_string(),
                        value: l,
                        dependencies: matching_metrics,
                        labels: matching_labels,
                    })
                }
            }
        }
    }

    fn maybe_include_all_events(&self, event: &mut Option<EventType>, metric_or_label: &str) {
        if event.is_none() && self.config.process_all_events {
            // If no rules match, we still want to record the event
            let count = self.get_metric_or_label(metric_or_label);
            match count {
                None => {}
                Some(Value::Metric(count)) => {
                    *event = Some(EventType::MetricChange {
                        metric: metric_or_label.to_string(),
                        count,
                        dependencies: Default::default(),
                        labels: Default::default(),
                    });
                }
                Some(Value::Label(label)) => {
                    *event = Some(EventType::LabelChange {
                        label: metric_or_label.to_string(),
                        value: label,
                        dependencies: Default::default(),
                        labels: Default::default(),
                    });
                }
            }
        }
    }

    fn get_metric_or_label(&self, key: &str) -> Option<Value> {
        if let Some(count) = self.counts.get(key) {
            Some(Value::Metric(*count))
        } else if let Some(label) = self.labels.get(key) {
            Some(Value::Label(label.clone()))
        } else {
            None
        }
    }
}

impl<W: Write> DebugMetricsTrait for DebugMetrics<W> {
    /// Include regex recording rules.
    fn add_recording_rule<Key: Into<String>>(&mut self, metric: Key, additional: &[&'static str]) {
        #[cfg(debug_assertions)]
        {
            let metric = metric.into();
            if let Some(existing) = self.rules.get_mut(&metric) {
                existing.extend(additional);
            } else {
                let mut set = BTreeSet::new();
                set.extend(additional);
                self.rules.insert(metric, set);
            }
        }
    }

    fn add_drop_hook<Key: Into<String>>(&mut self, key: Key) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            self.drop_print.insert(key);
        }
    }

    fn inc<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &mut self,
        key: Key,
        labels: Vec<(LabelKey, LabelVal)>,
    ) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            // Increment
            *self.counts.entry(key.to_string()).or_default() += 1;
            for (label_key, label_value) in labels {
                let label_key: String = label_key.into();
                let label_value: String = label_value.into();
                if label_key.is_empty() {
                    // TODO this is a hack, because Vecs need a type and sometimes its just easier
                    // with empty strings. It will be fixed with a proper iterator API.
                    continue;
                }
                self.labels.insert(label_key.to_string(), label_value);
                let mut event = None;
                self.maybe_find_matching_rule(&mut event, &label_key);
                self.maybe_include_all_events(&mut event, &label_key);
                self.maybe_include_all_labels_with_event(&mut event);
                if let Some(event) = event {
                    let event = event.promote_to_cascade(&key);
                    self.events.push(event);
                }
            }
            let mut event = None;
            self.maybe_find_matching_rule(&mut event, &key);
            self.maybe_include_all_events(&mut event, &key);
            self.maybe_include_all_labels_with_event(&mut event);
            if let Some(event) = event {
                self.events.push(event);
            }
        }
    }

    fn set<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &mut self,
        key: Key,
        value: u64,
        labels: Vec<(LabelKey, LabelVal)>,
    ) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            // Increment
            *self.counts.entry(key.to_string()).or_default() = value;
            for (label_key, label_value) in labels {
                let label_key: String = label_key.into();
                let label_value: String = label_value.into();
                self.labels.insert(label_key.to_string(), label_value);
                let mut event = None;
                self.maybe_find_matching_rule(&mut event, &label_key);
                self.maybe_include_all_events(&mut event, &label_key);
                self.maybe_include_all_labels_with_event(&mut event);
                if let Some(event) = event {
                    let event = event.promote_to_cascade(&key);
                    self.events.push(event);
                }
            }
            let mut event = None;
            self.maybe_find_matching_rule(&mut event, &key);
            self.maybe_include_all_events(&mut event, &key);
            self.maybe_include_all_labels_with_event(&mut event);
            if let Some(event) = event {
                self.events.push(event);
            }
        }
    }

    fn set_label<Key: Into<String>, Value: Into<String>>(&mut self, key: Key, value: Value) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let value = value.into();
            self.labels.insert(key.to_string(), value.to_string());
            let mut event = None;
            self.maybe_find_matching_rule(&mut event, &key);
            self.maybe_include_all_events(&mut event, &key);
            self.maybe_include_all_labels_with_event(&mut event);
            if let Some(event) = event {
                self.events.push(event);
            }
        }
    }

    fn events_for_key<Key: Into<String>>(&self, key: Key) -> Vec<EventType> {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            self.events
                .iter()
                .filter(|e| match e {
                    EventType::MetricChange {
                        metric,
                        count,
                        dependencies,
                        labels,
                    } => metric == &key,
                    EventType::LabelChange {
                        label,
                        value,
                        dependencies,
                        labels,
                    } => label == &key,
                    EventType::CascadeMetricChange {
                        cause,
                        metric,
                        count,
                        dependencies,
                        labels,
                    } => metric == &key || cause == &key,
                    EventType::CascadeLabelChange {
                        cause,
                        label,
                        value,
                        dependencies,
                        labels,
                    } => label == &key || cause == &key,
                })
                .cloned()
                .collect()
        }
        #[cfg(not(debug_assertions))]
        {
            Vec::new()
        }
    }
}

impl<W: Write> Drop for DebugMetrics<W> {
    fn drop(&mut self) {
        for e in self.events.iter() {
            match e {
                EventType::MetricChange {
                    metric,
                    count,
                    dependencies,
                    labels,
                } => {
                    if self.config.process_all_events | self.drop_print.contains(metric) {
                        let mut all_deps = BTreeMap::new();
                        dependencies.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.to_string());
                        });
                        labels.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.clone());
                        });
                        self.output_writer
                            .write_fmt(format_args!("{metric}: {count} :: {all_deps:?}\n"))
                            .unwrap();
                    }
                }
                EventType::LabelChange {
                    label,
                    value,
                    dependencies,
                    labels,
                } => {
                    if self.config.process_all_events | self.drop_print.contains(label) {
                        let mut all_deps = BTreeMap::new();
                        dependencies.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.to_string());
                        });
                        labels.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.clone());
                        });
                        self.output_writer
                            .write_fmt(format_args!("{label}: {value} :: {all_deps:?}\n"))
                            .unwrap();
                    }
                }
                EventType::CascadeMetricChange {
                    cause,
                    metric,
                    count,
                    dependencies,
                    labels,
                } => {
                    if self.config.process_all_events | self.drop_print.contains(metric) {
                        let mut all_deps = BTreeMap::new();
                        dependencies.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.to_string());
                        });
                        labels.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.clone());
                        });
                        self.output_writer
                            .write_fmt(format_args!(
                                "{metric} (caused by {cause}): {count} :: {all_deps:?}\n"
                            ))
                            .unwrap();
                    }
                }
                EventType::CascadeLabelChange {
                    cause,
                    label,
                    value,
                    dependencies,
                    labels,
                } => {
                    if self.config.process_all_events | self.drop_print.contains(label) {
                        let mut all_deps = BTreeMap::new();
                        dependencies.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.to_string());
                        });
                        labels.iter().for_each(|(k, v)| {
                            all_deps.insert(k.clone(), v.clone());
                        });
                        self.output_writer
                            .write_fmt(format_args!(
                                "{label} (caused by {cause}): {value} :: {all_deps:?}\n"
                            ))
                            .unwrap();
                    }
                }
            }
        }
        self.output_writer.flush().unwrap();
    }
}
