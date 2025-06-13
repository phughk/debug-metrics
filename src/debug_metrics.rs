use crate::config::DebugMetricsConfig;
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
}

impl Default for DebugMetrics<Stdout> {
    fn default() -> Self {
        let config = DebugMetricsConfig::default();
        DebugMetrics::new(stdout(), config)
    }
}

pub trait DebugMetricsTrait {
    fn add_recording_rule<Key: Into<String>>(&mut self, metric: Key, additional: &[&'static str]);

    fn add_drop_hook<Key: Into<String>>(&mut self, key: Key);

    fn inc<Key: Into<String>>(&mut self, key: Key);

    fn set<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &mut self,
        key: Key,
        value: u64,
        labels: Vec<(LabelKey, LabelVal)>,
    );

    fn set_label<Key: Into<String>, Value: Into<String>>(&mut self, key: Key, value: Value);

    fn events_for_key<Key: Into<String>>(&self, key: Key) -> Vec<EventType>;
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

    fn inc<Key: Into<String>>(&mut self, key: Key) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            // Increment
            *self.counts.entry(key.to_string()).or_default() += 1;
            let mut event = None;
            if let Some(rules) = self.rules.get(&key) {
                let mut read = matching_rules_for_regexes(rules, &self.counts);
                let c = self.counts[&key];
                event = Some(EventType::MetricChange {
                    metric: key.to_string(),
                    count: c,
                    dependencies: read,
                    labels: Default::default(),
                });
            }
            if event.is_none() && self.config.process_all_events {
                // If no rules match, we still want to record the event
                let count = self.counts[&key];
                event = Some(EventType::MetricChange {
                    metric: key,
                    count,
                    dependencies: Default::default(),
                    labels: Default::default(),
                });
            }
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
                self.labels.insert(label_key.into(), label_value.into());
            }
            if let Some(rules) = self.rules.get(&key) {
                let mut read = matching_rules_for_regexes(rules, &self.counts);
                let c = self.counts[&key];
                let event = EventType::MetricChange {
                    metric: key,
                    count: c,
                    dependencies: read,
                    labels: Default::default(),
                };
                self.events.push(event);
            }
        }
    }

    fn set_label<Key: Into<String>, Value: Into<String>>(&mut self, key: Key, value: Value) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let value = value.into();
            self.labels.insert(key, value);
            todo!()
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
                    EventType::LabelChange { .. } => false,
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
    /// TODO drop should really be for the entire metrics; Cloning will cause multiple drops.
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
                        self.output_writer
                            .write_fmt(format_args!("{metric}: {count} :: {dependencies:?}"))
                            .unwrap();
                    }
                }
                EventType::LabelChange { .. } => {
                    todo!()
                }
            }
        }
        self.output_writer.flush().unwrap();
    }
}

#[cfg(debug_assertions)]
fn matching_rules_for_regexes(
    regexes: &BTreeSet<&'static str>,
    counts: &BTreeMap<String, u64>,
) -> BTreeMap<String, u64> {
    let mut found = BTreeSet::new();
    let mut ret = BTreeMap::new();
    for patt in regexes {
        for (k, v) in counts {
            // Create regex from pattern
            let re = regex::Regex::new(patt).unwrap();
            if !found.contains(k) && re.is_match(k) {
                found.insert(k);
                ret.insert(k.to_string(), *v);
            }
        }
    }
    ret
}
