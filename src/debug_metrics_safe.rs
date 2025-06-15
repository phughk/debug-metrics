use crate::debug_metrics::{DebugMetricsTrait, EventType};
use crate::drop_hook_safe::DropHookSafe;
use std::sync::{Arc, Mutex};

pub struct DebugMetricsSafe<DM: DebugMetricsTrait> {
    inner: Arc<Mutex<DM>>,
}

// Derive does not work, because it expects the generic to be Clone as well
impl<DM: DebugMetricsTrait> Clone for DebugMetricsSafe<DM> {
    fn clone(&self) -> Self {
        DebugMetricsSafe {
            inner: self.inner.clone(),
        }
    }
}

pub trait DebugMetricsSafeTrait: Clone {
    fn add_recording_rule<Key: Into<String>>(&self, metric: Key, additional: &[&'static str]);

    fn add_drop_hook<Key: Into<String>>(&self, key: Key);

    fn inc<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &self,
        key: Key,
        labels: Vec<(LabelKey, LabelVal)>,
    );

    fn set<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &self,
        key: Key,
        value: u64,
        labels: Vec<(LabelKey, LabelVal)>,
    );

    fn set_label<Key: Into<String>, Value: Into<String>>(&self, key: Key, value: Value);

    fn events_for_key<Key: Into<String>>(&self, key: Key) -> Vec<EventType>;

    fn with_drop_hook<CallFn>(&self, call_fn: CallFn) -> DropHookSafe<Self, CallFn>
    where
        CallFn: Fn(&Self),
    {
        DropHookSafe {
            debug_metrics: self.clone(),
            call_fn,
        }
    }
}

impl<DM: DebugMetricsTrait> DebugMetricsSafe<DM> {
    pub fn new(debug_metrics: DM) -> Self {
        DebugMetricsSafe {
            inner: Arc::new(Mutex::new(debug_metrics)),
        }
    }
}

impl<DM: DebugMetricsTrait> DebugMetricsSafeTrait for DebugMetricsSafe<DM> {
    fn add_recording_rule<Key: Into<String>>(&self, metric: Key, additional: &[&'static str]) {
        let mut lock = self.inner.lock().unwrap();
        lock.add_recording_rule(metric, additional);
    }

    fn add_drop_hook<Key: Into<String>>(&self, key: Key) {
        let mut lock = self.inner.lock().unwrap();
        lock.add_drop_hook(key);
    }

    fn inc<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &self,
        key: Key,
        labels: Vec<(LabelKey, LabelVal)>,
    ) {
        let mut lock = self.inner.lock().unwrap();
        lock.inc(key, labels);
    }

    fn set<Key: Into<String>, LabelKey: Into<String>, LabelVal: Into<String>>(
        &self,
        key: Key,
        value: u64,
        labels: Vec<(LabelKey, LabelVal)>,
    ) {
        let mut lock = self.inner.lock().unwrap();
        lock.set(key, value, labels);
    }

    fn set_label<Key: Into<String>, Value: Into<String>>(&self, key: Key, value: Value) {
        let mut lock = self.inner.lock().unwrap();
        lock.set_label(key, value);
    }

    fn events_for_key<Key: Into<String>>(&self, key: Key) -> Vec<EventType> {
        let lock = self.inner.lock().unwrap();
        lock.events_for_key(key)
    }
}
