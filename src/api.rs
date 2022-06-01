//! # api
//!
//! Deals with submitting payloads to the API and handling the response.

use crate::payload::Payload;
use serde::Deserialize;
use std::env;
use ureq::post;

#[derive(Deserialize, Debug, PartialEq)]
struct ApiResponse {
    id: String,
    run_id: String,
    queued: usize,
    skipped: usize,
    errors: Vec<String>,
}

/// Submit the payload to the provided endpoint.
///
/// Attempt to serialse the `payload` and submit it to the Buildkite test analytics API.
///
/// ## Panics:
///  - If the `BUILDKITE_ANALYTICS_API_TOKEN` is not set.
///  - If the API response cannot be parsed as JSON.
///  - If the response contains a non-zero number of errors.
pub fn submit(payload: Payload, endpoint: &str) {
    let auth = format!("Token token=\"{}\"", token());

    let body: String = post(endpoint)
        .set("Content-Type", "application/json")
        .set("Authorization", &auth)
        .send_json(payload)
        .expect("HTTP Error sending payload")
        .into_string()
        .expect("Failed to parse JSON response");

    let response: ApiResponse = serde_json::from_str(&body).expect("Failed to parse JSON response");

    if !response.errors.is_empty() {
        panic!("Error response from API: {:?}", response.errors);
    }
}

fn token() -> String {
    env::var("BUILDKITE_ANALYTICS_API_TOKEN")
        .expect("Missing BUILDKITE_ANALYTICS_API_TOKEN environment variable")
}
