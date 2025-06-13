pub struct DebugMetricsConfig {
    /// When true, events will always be recorded and printed, even if there is no rule
    pub process_all_events: bool,
}

impl Default for DebugMetricsConfig {
    fn default() -> Self {
        DebugMetricsConfig {
            process_all_events: true,
        }
    }
}
