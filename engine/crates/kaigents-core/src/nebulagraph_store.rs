//! File: engine/crates/kaigents-core/src/nebulagraph_store.rs
//! Purpose: NebulaGraph-backed persistence for temporal memory edges.
//! Product/business importance: enables bi-temporal tracking and graph reasoning for agent memory.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use serde::{Deserialize, Serialize};

/// NebulaConfig controls how the NebulaGraph backend connects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NebulaConfig {
    pub host: String,
    pub port: u16,
    pub space: String,
    pub user: String,
    pub password: String,
}

impl Default for NebulaConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 9669,
            space: "kaigents".to_string(),
            user: "root".to_string(),
            password: "nebula_password".to_string(),
        }
    }
}

impl NebulaConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_HOST") {
            if !v.is_empty() {
                cfg.host = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                cfg.port = port;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_SPACE") {
            if !v.is_empty() {
                cfg.space = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_USER") {
            if !v.is_empty() {
                cfg.user = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_PASSWORD") {
            cfg.password = v;
        }
        cfg
    }
}

#[derive(Default)]
pub struct NebulaGraphStore {
    // Future: connection pool
}

impl NebulaGraphStore {
    pub fn new() -> Self {
        Self::default()
    }
}
