use crate::debug_metrics::DefaultExt;

#[derive(Clone, Copy)]
pub struct DebugMetricsConfig {
    /// When true, events will always be recorded and printed, even if there is no rule
    pub process_all_events: bool,
    /// Record label change events
    pub record_label_changes: bool,
    /// Include all labels for every event
    pub all_labels_every_event: bool,
}

impl Default for DebugMetricsConfig {
    fn default() -> Self {
        DebugMetricsConfig {
            process_all_events: false,
            record_label_changes: false,
            all_labels_every_event: false,
        }
    }
}

impl DefaultExt for DebugMetricsConfig {
    fn default_on() -> Self {
        DebugMetricsConfig {
            process_all_events: true,
            record_label_changes: true,
            all_labels_every_event: true,
        }
    }
}
