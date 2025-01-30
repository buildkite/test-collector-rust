//! # api
//!
//! Deals with submitting payloads to the API and handling the response.

use crate::payload::Payload;
use serde::Deserialize;
use std::env;
use ureq::post;

type Response = http::Response<ureq::Body>;

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
/// Attempt to serialise the `payload` and submit it to the Buildkite test analytics API.
///
/// ## Emits warnings if:
///  - If the `BUILDKITE_ANALYTICS_TOKEN` is not set.
///  - If the API response cannot be parsed as JSON.
///  - If the response contains a non-zero number of errors.
pub fn submit(payload: Payload, endpoint: &str) -> Option<()> {
    let auth_header = get_auth_header()?;
    let response = send_request(payload, endpoint, &auth_header)?;
    let response = get_response_body(response)?;
    let response = get_api_response(&response)?;

    if !response.errors.is_empty() {
        eprintln!("Error response from API: {:?}", response.errors);
        None
    } else {
        Some(())
    }
}

fn send_request(payload: Payload, endpoint: &str, auth: &str) -> Option<Response> {
    let maybe_response = post(endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", auth)
        .send_json(payload);

    match maybe_response {
        Ok(response) => Some(response),
        Err(err) => {
            eprintln!("HTTP Error sending API request: {:?}", err);
            None
        }
    }
}

fn get_response_body(mut response: Response) -> Option<String> {
    match response.body_mut().read_to_string() {
        Ok(json) => Some(json),
        Err(_) => {
            eprintln!("Failed to parse JSON response");
            None
        }
    }
}

fn get_api_response(json: &str) -> Option<ApiResponse> {
    let maybe_response: serde_json::Result<ApiResponse> = serde_json::from_str(json);

    match maybe_response {
        Ok(response) => Some(response),
        Err(_) => {
            eprintln!("Failed to parse JSON response");
            None
        }
    }
}

fn get_auth_header() -> Option<String> {
    match env::var("BUILDKITE_ANALYTICS_TOKEN") {
        Ok(token) => Some(format!("Token token=\"{}\"", token)),
        Err(_) => {
            eprintln!("Missing BUILDKITE_ANALYTICS_TOKEN environment variable.  No analytics will be sent.");
            None
        }
    }
}
