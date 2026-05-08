//! File: engine/crates/kaigents-core/src/metrics.rs
//! Purpose: Prometheus metrics definitions for the Rust execution engine.
//! Product/business importance: enables first-class monitoring of agent performance, costs (tokens), and errors.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use lazy_static::lazy_static;
use prometheus::{
    register_histogram, register_int_counter_vec, Histogram, IntCounterVec, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref RUNS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "kaigents_runs_total",
        "Total number of agent/process runs started",
        &["target_kind", "status"]
    ).unwrap();

    pub static ref RUN_DURATION_SECONDS: Histogram = register_histogram!(
        "kaigents_run_duration_seconds",
        "Duration of agent/process runs in seconds"
    ).unwrap();

    pub static ref TOOL_INVOCATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "kaigents_tool_invocations_total",
        "Total number of tool calls made",
        &["tool_name", "status"]
    ).unwrap();

    pub static ref MODEL_TOKENS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "kaigents_model_tokens_total",
        "Total number of model tokens consumed",
        &["model", "type"]
    ).unwrap();
}

pub fn init_metrics() {
    // Force initialization of lazy statics
    let _ = *RUNS_TOTAL;
    let _ = *RUN_DURATION_SECONDS;
    let _ = *TOOL_INVOCATIONS_TOTAL;
    let _ = *MODEL_TOKENS_TOTAL;
}

pub fn gather_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
