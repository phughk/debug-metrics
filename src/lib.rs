
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// DebugMetrics that serve as a convenient way to debug complex code.
/// This is not at all production metrics.
#[derive(Clone)]
pub struct DebugMetrics {
    /// Which other metrics need to be taken
    /// Regexes to match against keys.
    rules: Arc<Mutex<BTreeMap<String, BTreeSet<&'static str>>>>,
    counts: Arc<Mutex<BTreeMap<String, u64>>>,
    events: Arc<Mutex<Vec<(String, u64, BTreeMap<String, u64>)>>>,
    drop_print: Arc<Mutex<BTreeSet<String>>>,
}

impl DebugMetrics {
    pub fn new() -> Self {
        DebugMetrics {
            rules: Arc::default(),
            counts: Arc::default(),
            events: Arc::default(),
            drop_print: Arc::default(),
        }
    }

    /// Include regex recording rules.
    pub fn add_recording_rule<Key: Into<String>>(&self, metric: Key, additional: &[&'static str]) {
        #[cfg(debug_assertions)]
        {
            let metric = metric.into();
            let mut rules = self.rules.lock().unwrap();
            if let Some(existing) = rules.get_mut(&metric) {
                existing.extend(additional);
            } else {
                let mut set = BTreeSet::new();
                set.extend(additional);
                rules.insert(metric, set);
            }
        }
    }

    pub fn add_drop_hook<Key: Into<String>>(&self, key: Key) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let mut drop_print = self.drop_print.lock().unwrap();
            drop_print.insert(key);
        }
    }

    pub fn inc<Key: Into<String>>(&self, key: Key) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let mut rules = self.rules.lock().unwrap();
            let mut counts = self.counts.lock().unwrap();
            let mut events = self.events.lock().unwrap();
            // Increment
            *counts.entry(key.to_string()).or_default() += 1;
            if let Some(rules) = rules.get(&key) {
                let mut read = matching_rules_for_regexes(rules, &counts);
                let c = counts[&key];
                events.push((key, c, read));
            }
        }
    }

    pub fn set<Key: Into<String>>(&self, key: Key, value: u64) {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let mut rules = self.rules.lock().unwrap();
            let mut counts = self.counts.lock().unwrap();
            let mut events = self.events.lock().unwrap();
            // Increment
            *counts.entry(key.to_string()).or_default() = value;
            if let Some(rules) = rules.get(&key) {
                let mut read = matching_rules_for_regexes(rules, &counts);
                let c = counts[&key];
                events.push((key, c, read));
            }
        }
    }

    pub fn events_for_key<Key: Into<String>>(
        &self,
        key: Key,
    ) -> Vec<(String, u64, BTreeMap<String, u64>)> {
        #[cfg(debug_assertions)]
        {
            let key = key.into();
            let events = self.events.lock().unwrap();
            events
                .iter()
                .filter(|(k, _, _)| *k == key)
                .cloned()
                .collect()
        }
        #[cfg(not(debug_assertions))]
        {
            Vec::new()
        }
    }
}

impl Drop for DebugMetrics {
    /// TODO drop should really be for the entire metrics; Cloning will cause multiple drops.
    fn drop(&mut self) {
        let events = self.events.lock().unwrap();
        let drop_print = self.drop_print.lock().unwrap();

        for (event_key, count, event_deps) in events.iter() {
            if drop_print.contains(event_key) {
                println!("{event_key}: {count} :: {event_deps:?}");
            }
        }
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
