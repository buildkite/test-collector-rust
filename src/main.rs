//! # buildkite-test-collector
//!
//! A command-line utility to send Rust test output to the Buildkite test
//! analytics service.
//!
//! Parses the inbound stream of JSON events from Rust's JSON output (ie `cargo
//! test -- -Z unstable-options --format json --report-time`) - eg:
//!
//! ```
//! { "type": "suite", "event": "started", "test_count": 6 }
//! { "type": "test", "event": "started", "name": "payload::test::batchify_works_as_expected" }
//! { "type": "test", "event": "started", "name": "run_env::test::detect_circle_ci_environment" }
//! { "type": "test", "event": "started", "name": "run_env::test::detect_failed" }
//! { "type": "test", "event": "started", "name": "run_env::test::detect_generic_environment" }
//! { "type": "test", "event": "started", "name": "run_env::test::detect_github_actions_environment" }
//! { "type": "test", "event": "started", "name": "run_env::test::detects_buildkite_environment" }
//! { "type": "test", "name": "run_env::test::detect_generic_environment", "event": "ok", "exec_time": 0.000291028 }
//! { "type": "test", "name": "run_env::test::detect_circle_ci_environment", "event": "ok", "exec_time": 0.000441465 }
//! { "type": "test", "name": "run_env::test::detect_failed", "event": "ok", "exec_time": 0.000706932 }
//! { "type": "test", "name": "run_env::test::detect_github_actions_environment", "event": "ok", "exec_time": 0.000759033 }
//! { "type": "test", "name": "payload::test::batchify_works_as_expected", "event": "ok", "exec_time": 0.001719557 }
//! { "type": "test", "name": "run_env::test::detects_buildkite_environment", "event": "ok", "exec_time": 0.001703423 }
//! { "type": "suite", "event": "ok", "passed": 6, "failed": 0, "ignored": 0, "measured": 0, "filtered_out": 0, "exec_time": 0.002269416 }
//! ```
//!
//! We take this output and use it to generate analytics information about the
//! test suite and submit it to the Buildkite test analytics API.
//!
//! It also echos `stdin` back to `stdout` unchanged, so that you can use it
//! with other tools as needed.

extern crate serde;
extern crate ureq;
extern crate uuid;

#[cfg(test)]
#[macro_use]
extern crate serial_test;

#[cfg(test)]
extern crate rand;

mod api;
mod input;
mod payload;
mod run_env;

use payload::Payload;
use run_env::RuntimeEnvironment;
use std::io::*;

static BATCH_SIZE: usize = 500;
static ENDPOINT: &str = "https://analytics-api.buildkite.com/v1/uploads";

// https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The entrypoint for the binary.  Takes no arguments.
///
/// ## Emits warnings
///  - If the CI environment cannot be detected.
fn main() {
    let mut args = std::env::args();
    let prog = args.next().unwrap_or(NAME.to_string());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" => {
                println!("{} {}", NAME, VERSION);
                return;
            }
            "--help" => {
                help(prog);
                return;
            }
            _ => {}
        }
    }

    let stdin = std::io::stdin();
    let stdin = stdin.lock();

    if let Some(run_env) = RuntimeEnvironment::detect() {
        let mut payload = Payload::new(run_env);

        for line in stdin.lines().flatten() {
            input::parse_line(&line, &mut payload);
            println!("{}", line);
        }

        for payload in payload.batchify(BATCH_SIZE) {
            api::submit(payload, ENDPOINT);
        }
    } else {
        eprintln!("Unable to detect CI environment.  No analytics will be sent.");
        for line in stdin.lines().flatten() {
            println!("{}", line)
        }
    }
}

fn help(prog: String) {
    println!("\n{} {}", NAME, VERSION);
    print!(
        "
Expects BUILDKITE_ANALYTICS_TOKEN in environment, and test result JSON on stdin.
Test results may be piped like:

  cargo test -- -Z unstable-options --format json --report-time | {}

For more help, see:
  - https://buildkite.com/docs/test-analytics/rust-collectors
  - https://github.com/buildkite/test-collector-rust

",
        prog
    );
}
