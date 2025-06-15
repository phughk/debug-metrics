use crate::config::DebugMetricsConfig;
use crate::debug_metrics::{DebugMetricsTrait, DefaultExt, EventType};
use crate::DebugMetrics;
use indoc::indoc;
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

#[test]
fn metrics_are_displayed_if_no_rules() {
    let mut c = Cursor::new(Vec::new());
    let events = {
        let mut debug_metrics = DebugMetrics::new(&mut c, DebugMetricsConfig::default_on());
        debug_metrics.inc("example", vec![("", "")]);
        debug_metrics.events_for_key("example")
    };
    assert_eq!(
        events,
        vec![EventType::MetricChange {
            metric: "example".to_string(),
            count: 1,
            dependencies: Default::default(),
            labels: Default::default(),
        }]
    );
    c.set_position(0);
    let mut output = String::new();
    c.read_to_string(&mut output).unwrap();
    let expected = indoc!(
        r#"
        example: 1 :: {}
    "#
    );
    assert_eq!(output, expected);
}

#[test]
fn can_use_labels() {
    let mut c = Cursor::new(Vec::new());
    let events = {
        let mut debug_metrics = DebugMetrics::new(&mut c, DebugMetricsConfig::default_on());
        debug_metrics.set_label("stage", "zero");
        debug_metrics.set("example", 42, vec![("stage", "one")]);
        debug_metrics.events_for_key("example")
    };
    assert_eq!(
        events,
        vec![
            EventType::CascadeLabelChange {
                cause: "example".to_string(),
                label: "stage".to_string(),
                value: "one".to_string(),
                dependencies: Default::default(),
                labels: BTreeMap::from([("stage".to_string(), "one".to_string())]),
            },
            EventType::MetricChange {
                metric: "example".to_string(),
                count: 42,
                dependencies: BTreeMap::from([]),
                labels: BTreeMap::from([("stage".to_string(), "one".to_string())]),
            }
        ]
    );
    c.set_position(0);
    let mut output = String::new();
    c.read_to_string(&mut output).unwrap();
    let expected = indoc!(
        r#"
        stage: zero :: {"stage": "zero"}
        stage (caused by example): one :: {"stage": "one"}
        example: 42 :: {"stage": "one"}
    "#
    );
    assert_eq!(output, expected);
}

#[test]
fn label_changes_get_recorded_as_events() {
    struct TestCase {
        name: &'static str,
        config: DebugMetricsConfig,
        pre_setup: &'static dyn Fn(&mut DebugMetrics<&mut Cursor<Vec<u8>>>),
        events: Vec<EventType>,
        output: &'static str,
    }

    let cases: &[TestCase] = &[
        TestCase {
            name: "Disabled config and enabled recording rule",
            config: Default::default(),
            pre_setup: &|debug_metrics| {
                debug_metrics.add_recording_rule("stage", &[".+"]);
                debug_metrics.add_drop_hook("stage");
            },
            events: vec![
                EventType::LabelChange {
                    label: "stage".to_string(),
                    value: "zero".to_string(),
                    dependencies: BTreeMap::from([]),
                    labels: BTreeMap::from([("stage".to_string(), "zero".to_string())]),
                },
                EventType::CascadeLabelChange {
                    cause: "metric".to_string(),
                    label: "stage".to_string(),
                    value: "one".to_string(),
                    dependencies: BTreeMap::from([("metric".to_string(), 1)]),
                    labels: BTreeMap::from([("stage".to_string(), "one".to_string())]),
                },
            ],
            output: indoc!(
                r#"
                stage: zero :: {"stage": "zero"}
                stage (caused by metric): one :: {"metric": "1", "stage": "one"}
                "#
            ),
        },
        TestCase {
            name: "Enabled capture all config and no recording rule",
            config: DebugMetricsConfig::default_on(),
            pre_setup: &|debug_metrics| {},
            events: vec![
                EventType::LabelChange {
                    label: "stage".to_string(),
                    value: "zero".to_string(),
                    dependencies: BTreeMap::from([]),
                    labels: BTreeMap::from([("stage".to_string(), "zero".to_string())]),
                },
                EventType::CascadeLabelChange {
                    cause: "metric".to_string(),
                    label: "stage".to_string(),
                    value: "one".to_string(),
                    // Metrics are empty, because there is no rule to record them alongside
                    dependencies: BTreeMap::from([]),
                    labels: BTreeMap::from([("stage".to_string(), "one".to_string())]),
                },
            ],
            output: indoc!(
                r#"
                stage: zero :: {"stage": "zero"}
                stage (caused by metric): one :: {"stage": "one"}
                metric: 1 :: {"stage": "one"}
                "#
            ),
        },
    ];
    for case in cases {
        let mut c = Cursor::new(Vec::new());
        let events = {
            let mut debug_metrics = DebugMetrics::new(&mut c, case.config);
            let pre_setup = case.pre_setup;
            pre_setup(&mut debug_metrics);
            debug_metrics.set_label("stage", "zero");
            debug_metrics.inc("metric", vec![("stage", "one")]);
            debug_metrics.events_for_key("stage")
        };
        assert_eq!(&events, &case.events, "{}", case.name);
        c.set_position(0);
        let mut output = String::new();
        c.read_to_string(&mut output).unwrap();
        assert_eq!(output, case.output, "{}", case.name);
    }
}
