// SPDX-License-Identifier: Apache-2.0

//! The napi-rs addon surface: thin `#[napi]` delegators to [`crate::wire`].
//!
//! All logic lives in `wire`; this layer only maps the `String` error onto
//! `napi::Error`, so the addon has no untested behaviour of its own. Present
//! only under the `napi` feature.

use napi_derive::napi;

use crate::wire;

fn map(r: Result<String, String>) -> napi::Result<String> {
    r.map_err(napi::Error::from_reason)
}

/// See [`wire::evaluate_gate`].
#[napi]
pub fn evaluate_gate(context_json: String, params_json: String) -> napi::Result<String> {
    map(wire::evaluate_gate(&context_json, &params_json))
}

/// See [`wire::decide_autonomy`].
#[napi]
pub fn decide_autonomy(
    decision_json: String,
    trust_level: String,
    tier: String,
) -> napi::Result<String> {
    map(wire::decide_autonomy(&decision_json, &trust_level, &tier))
}

/// See [`wire::build_record`].
#[napi]
pub fn build_record(
    prev_hash: String,
    id: String,
    timestamp: String,
    message_decision_json: String,
) -> napi::Result<String> {
    map(wire::build_record(
        &prev_hash,
        &id,
        &timestamp,
        &message_decision_json,
    ))
}

/// See [`wire::verify_chain`].
#[napi]
pub fn verify_chain(records_json: String) -> napi::Result<String> {
    map(wire::verify_chain(&records_json))
}

/// See [`wire::score`].
#[napi]
pub fn score(
    config_json: String,
    snapshot_json: Option<String>,
    samples_json: String,
) -> napi::Result<String> {
    map(wire::score(
        &config_json,
        snapshot_json.as_deref(),
        &samples_json,
    ))
}

/// See [`wire::default_window_config`].
#[napi]
pub fn default_window_config() -> napi::Result<String> {
    map(wire::default_window_config())
}
