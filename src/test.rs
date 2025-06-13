use std::io::{Cursor, Read};
use crate::config::DebugMetricsConfig;
use crate::debug_metrics::{DebugMetricsTrait, EventType};
use crate::DebugMetrics;

#[test]
fn metrics_are_displayed_if_no_rules() {
    let mut c = Cursor::new(Vec::new());
    let events = {
        let mut debug_metrics = DebugMetrics::new(&mut c, DebugMetricsConfig::default());
        debug_metrics.inc("example");
        debug_metrics.events_for_key("example")
    };
    c.set_position(0);
    let mut output = String::new();
    c.read_to_string(&mut output).unwrap();
    assert_eq!(events, vec![
        EventType::MetricChange {
            metric: "example".to_string(),
            count: 1,
            dependencies: Default::default(),
            labels: Default::default(),
        }
    ]);
    let expected = r#"example: 1 :: {}"#;
    assert_eq!(output, expected);
}

#[test]
fn can_use_labels() {
    let mut c = Cursor::new(Vec::new());
    let events = {
        let mut debug_metrics = DebugMetrics::new(&mut c, DebugMetricsConfig::default());
        debug_metrics.set_label("stage", "zero");
        debug_metrics.set("example", 42, vec![("stage", "one")]);
        debug_metrics.events_for_key("example")
    };
    assert_eq!(events, vec![])
}