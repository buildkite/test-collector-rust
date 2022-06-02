//! # input
//!
//! Deserialisation of JSON input from Rust.

use crate::payload::Payload;
use serde::Deserialize;

/// # SuiteEvent
///
/// An event relating to the entire test suite.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "event")]
pub enum SuiteEvent {
    #[serde(rename = "started")]
    Started { test_count: usize },
    #[serde(rename = "ok")]
    Ok {
        #[serde(flatten)]
        results: SuiteResults,
    },
    #[serde(rename = "failed")]
    Failed {
        #[serde(flatten)]
        results: SuiteResults,
    },
}

/// # SuiteResults
///
/// When a suite is finished Rust tells us how many tests passed and failed and
/// how long it took.
#[derive(Deserialize, Debug, PartialEq)]
pub struct SuiteResults {
    passed: usize,
    failed: usize,
    ignored: usize,
    measured: usize,
    filtered_out: usize,
    exec_time: f64,
}

/// # TestEvent
///
/// An event relating to an individual test.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "event")]
pub enum TestEvent {
    #[serde(rename = "started")]
    Started { name: String },
    #[serde(rename = "ok")]
    Ok { name: String, exec_time: f64 },
    #[serde(rename = "failed")]
    Failed {
        name: String,
        exec_time: f64,
        stdout: Option<String>,
        stderr: Option<String>,
    },
    #[serde(rename = "ignored")]
    Ignored { name: String },
    #[serde(rename = "timeout")]
    Timeout { name: String },
}

/// # Event
///
/// Incoming events can either be `SuiteEvent` or `TestEvent`.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "suite")]
    Suite {
        #[serde(flatten)]
        event: SuiteEvent,
    },
    #[serde(rename = "test")]
    Test {
        #[serde(flatten)]
        event: TestEvent,
    },
}

/// Attempt to parse a single line if JSON.
///
/// Attempts to convert `line` into an `Event` and push it into `Payload`.
pub fn parse_line(line: &str, payload: &mut Payload) {
    if line.chars().find(|c| !c.is_whitespace()) != Some('{') {
        return;
    }

    let maybe_event: serde_json::Result<Event> = serde_json::from_str(line);

    if let Ok(event) = maybe_event {
        payload.push(event);
    }
}
