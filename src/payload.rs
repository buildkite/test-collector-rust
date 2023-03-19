//! # payload
//!
//! Information about the payload to send to the API.

use crate::input::{Event, SuiteEvent, TestEvent};
use crate::run_env::RuntimeEnvironment;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// # Payload
///
/// A data-structure which represents the (possibly) incomplete data to be
/// eventually sent to the API.
///
/// Impements `serde:Serialize` for serialisation into JSON.
#[derive(Debug, PartialEq)]
pub struct Payload {
    run_env: RuntimeEnvironment,
    data: HashMap<String, TestData>,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
}

/// # TestData
///
/// Information about a specific test result.  Contains the test's unique
/// identifier, name, etc, as well as any tracing or failure information.
#[derive(serde::Serialize, Debug, PartialEq, Clone)]
pub struct TestData {
    id: String,
    scope: String,
    name: String,
    #[serde(flatten)]
    result: TestResult,
    history: TestHistory,
}

impl TestData {
    /// Have we received a finishing event for this `TestData`?
    ///
    /// Because Rust sends separate `TestStarted`, `TestOk` and `TestFailed`
    /// events it's possible for a `TestData` to be in an intermediate state
    /// where we have received a start event, but no finishing events.
    pub fn is_finished(&self) -> bool {
        self.history.is_finished()
    }
}

/// # TestHistory
///
/// Contains timing information about the test and possibly finer tracing.
#[derive(serde::Serialize, Debug, PartialEq, Clone)]
pub struct TestHistory {
    section: String,
    start_at: Option<f64>,
    end_at: Option<f64>,
    duration: Option<f64>,
    children: Vec<TestHistory>,
}

impl TestHistory {
    /// Have we received a finishing event for this `TestHistory`?
    ///
    /// Because Rust sends separate `TestStarted`, `TestOk` and `TestFailed`
    /// events it's possible for a `TestHistory` to be in an intermediate state
    /// where we have received a start event, but no finishing events.
    pub fn is_finished(&self) -> bool {
        self.end_at.is_some()
    }
}

/// # TestResult
///
/// Did the test in question pass?  And if not, why not?
#[derive(serde::Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "result")]
pub enum TestResult {
    #[serde(rename = "passed")]
    Passed,
    #[serde(rename = "failed")]
    Failed { failure_reason: Option<String> },
}

impl Serialize for Payload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Payload", 3)?;
        state.serialize_field("format", "json")?;
        state.serialize_field("run_env", &self.run_env)?;
        state.serialize_field("data", &self.closed_data())?;
        state.end()
    }
}

impl Payload {
    /// Initialise a new empty payload given a specific runtime environment.
    pub fn new(run_env: RuntimeEnvironment) -> Self {
        Payload {
            run_env,
            data: HashMap::new(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Push an event into the payload.
    pub fn push(&mut self, event: Event) {
        match event {
            Event::Suite { event: suite_event } => self.push_suite_event(suite_event),
            Event::Test { event: test_event } => self.push_test_event(test_event),
        }
    }

    /// Split the payload into batches of `batch_size`.
    ///
    /// Currently the analytics API allows a maximum of 5000 tests to be
    /// uploaded in a single call, however it is possible to upload more than
    /// that by splitting the payload into separate batches.
    ///
    /// Returns a vector of payloads containing their individual batches of
    /// `TestData`.
    pub fn batchify(self, batch_size: usize) -> Vec<Self> {
        let (complete, incomplete): (Vec<TestData>, Vec<TestData>) = self
            .data
            .values()
            .cloned()
            .partition(|test_data| test_data.is_finished());

        let result = complete
            .chunks(batch_size)
            .map(|chunk| {
                let mut payload = self.new_clean();

                for test_data in chunk.iter() {
                    payload
                        .data
                        .insert(test_data.name.clone(), test_data.clone());
                }

                if payload.data.len() < batch_size {
                    for test_data in incomplete.iter() {
                        payload
                            .data
                            .insert(test_data.name.clone(), test_data.clone());
                    }
                }

                payload
            })
            .collect();

        result
    }

    fn new_clean(&self) -> Self {
        Payload {
            run_env: self.run_env.clone(),
            data: HashMap::new(),
            started_at: self.started_at,
            finished_at: self.finished_at,
        }
    }

    fn closed_data(&self) -> Vec<&TestData> {
        self.data
            .values()
            .filter(|event| event.history.end_at.is_some())
            .collect()
    }

    fn push_suite_event(&mut self, suite_event: SuiteEvent) {
        match suite_event {
            SuiteEvent::Started { .. } => self.started_at = Some(Instant::now()),
            SuiteEvent::Ok { .. } => self.finished_at = Some(Instant::now()),
            SuiteEvent::Failed { .. } => self.finished_at = Some(Instant::now()),
        }
    }

    fn push_test_event(&mut self, test_event: TestEvent) {
        match test_event {
            TestEvent::Started { name } => {
                let name_chunks = name.split("::").collect::<Vec<&str>>();

                let data = TestData {
                    id: Uuid::new_v4().to_string(),
                    name: name_chunks.iter().last().unwrap().to_string(),
                    scope: name_chunks
                        .iter()
                        .rev()
                        .skip(1)
                        .rev()
                        .copied()
                        .collect::<Vec<&str>>()
                        .join("::"),
                    result: TestResult::Passed,
                    history: TestHistory {
                        section: "top".to_string(),
                        start_at: Some(
                            Instant::now()
                                .duration_since(self.started_at.unwrap())
                                .as_millis() as f64
                                / 1000000.0,
                        ),
                        end_at: None,
                        duration: None,
                        children: Vec::new(),
                    },
                };

                self.data.insert(name, data);
            }
            TestEvent::Ok { name, exec_time } => {
                let data = self.data.get_mut(&name).unwrap();
                data.history.end_at = Some(
                    Instant::now()
                        .duration_since(self.started_at.unwrap())
                        .as_millis() as f64
                        / 1000000.0,
                );
                data.history.duration = Some(exec_time);
            }
            TestEvent::Failed {
                name,
                exec_time,
                stdout,
                ..
            } => {
                let data = self.data.get_mut(&name).unwrap();
                data.history.end_at = Some(
                    Instant::now()
                        .duration_since(self.started_at.unwrap())
                        .as_millis() as f64
                        / 1000000.0,
                );
                data.history.duration = Some(exec_time);
                data.result = TestResult::Failed {
                    failure_reason: stdout,
                }
            }
            TestEvent::Ignored { .. } => {}
            TestEvent::Timeout { .. } => {}
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;

    #[test]
    fn batchify_works_as_expected() {
        let mut rng = rand::thread_rng();

        let mut payload = Payload::new(RuntimeEnvironment::generic());

        let batch_size = rng.gen_range(10..100);
        let finished_size = (batch_size as f32 * 1.5) as usize;
        let unfinished_size = (batch_size as f32 * 0.25) as usize;

        for _ in 0..finished_size {
            let td = stub_test_data(true);
            payload.data.insert(td.name.clone(), td);
        }

        for _ in 0..unfinished_size {
            let td = stub_test_data(false);
            payload.data.insert(td.name.clone(), td);
        }

        let payloads = payload.batchify(batch_size);

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0].data.len(), batch_size);

        for td in payloads[0].data.values() {
            assert!(td.is_finished());
        }

        assert_eq!(
            payloads[1].data.len(),
            finished_size + unfinished_size - batch_size
        );

        let (finished, unfinished): (Vec<TestData>, Vec<TestData>) = payloads[1]
            .data
            .values()
            .cloned()
            .partition(|td| td.is_finished());

        assert_eq!(finished.len(), finished_size - batch_size);
        assert_eq!(unfinished.len(), unfinished_size);
    }

    fn stub_test_data(finished: bool) -> TestData {
        let uuid = Uuid::new_v4().to_string();

        TestData {
            id: uuid.clone(),
            scope: uuid.clone(),
            name: uuid.clone(),
            result: stub_test_result(),
            history: stub_test_history(finished),
        }
    }

    fn stub_test_result() -> TestResult {
        let mut rng = rand::thread_rng();

        if rng.gen::<bool>() {
            TestResult::Passed
        } else {
            TestResult::Failed {
                failure_reason: None,
            }
        }
    }

    fn stub_test_history(finished: bool) -> TestHistory {
        let mut rng = rand::thread_rng();

        let start_at = rng.gen();

        if finished {
            let end_at = rng.gen::<f64>() + start_at;

            TestHistory {
                section: "top".to_string(),
                start_at: Some(start_at),
                end_at: Some(end_at),
                duration: Some(end_at - start_at),
                children: vec![],
            }
        } else {
            TestHistory {
                section: "top".to_string(),
                start_at: Some(start_at),
                end_at: None,
                duration: None,
                children: vec![],
            }
        }
    }
}
