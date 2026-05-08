//! File: engine/crates/kaigents-core/src/logging.rs
//! Purpose: Standardized structured logging for all Kaigents Rust components.
//! Product/business importance: enables first-class observability via Loki and cloud-native logging stacks.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize structured logging.
/// Defaults to JSON format if KAIGENTS_LOG_FORMAT=json or if running in a production environment.
pub fn init_logging() {
    let format = std::env::var("KAIGENTS_LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    if format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().flatten_event(true))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }
}
