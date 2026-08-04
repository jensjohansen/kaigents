//! File: engine/crates/kaigents-core/src/nebulagraph_store.rs
//! Purpose: NebulaGraph-backed persistence for temporal memory edges.
//! Product/business importance: enables bi-temporal tracking and graph reasoning for agent memory.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// NebulaConfig controls how the NebulaGraph backend connects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NebulaConfig {
    pub host: String,
    pub port: u16,
    pub http_port: u16,
    pub space: String,
    pub user: String,
    pub password: String,
}

impl Default for NebulaConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 9669,
            http_port: 19669,
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
        if let Ok(v) = std::env::var("KAIGENTS_NEBULA_HTTP_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                cfg.http_port = port;
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

    pub fn http_url(&self) -> String {
        format!("http://{}:{}/api/1.0/nebula/db", self.host, self.http_port)
    }
}

/// NebulaGraphStore provides temporal graph operations via NebulaGraph's HTTP API.
pub struct NebulaGraphStore {
    config: NebulaConfig,
    http: reqwest::Client,
    schema_initialized: std::sync::Mutex<bool>,
}

impl NebulaGraphStore {
    pub fn new(config: NebulaConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            schema_initialized: std::sync::Mutex::new(false),
        }
    }

    async fn execute_ngql(&self, stmt: &str) -> Result<serde_json::Value, String> {
        let url = self.config.http_url();
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "stmt": stmt }))
            .send()
            .await
            .map_err(|e| format!("NebulaGraph HTTP error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("NebulaGraph HTTP {status}: {body}"));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("NebulaGraph response parse error: {}", e))
    }

    pub async fn init_schema(&self) -> Result<(), String> {
        {
            let initialized = self.schema_initialized.lock().unwrap();
            if *initialized {
                return Ok(());
            }
        }

        let space = &self.config.space;

        self.execute_ngql(&format!(
            "CREATE SPACE IF NOT EXISTS {} (vid_type=FIXED_STRING(256), partition_num=10, replica_factor=1);",
            space
        ))
        .await?;

        self.execute_ngql(&format!("USE {};", space)).await?;

        self.execute_ngql(
            "CREATE TAG IF NOT EXISTS entity (name string, entity_type string, workspace_id string);",
        )
        .await?;

        self.execute_ngql(
            "CREATE EDGE IF NOT EXISTS depends_on (valid_from int64, valid_to int64, transaction_time int64);",
        )
        .await?;

        self.execute_ngql(
            "CREATE EDGE IF NOT EXISTS consolidated_from (valid_from int64, valid_to int64, transaction_time int64);",
        )
        .await?;

        let mut initialized = self.schema_initialized.lock().unwrap();
        *initialized = true;
        Ok(())
    }

    pub async fn insert_entity(
        &self,
        id: &str,
        name: &str,
        entity_type: &str,
        workspace_id: &str,
    ) -> Result<(), String> {
        self.execute_ngql(&format!(
            "USE {}; INSERT VERTEX entity (name, entity_type, workspace_id) VALUES \"{}\":(\"{}\", \"{}\", \"{}\");",
            self.config.space, id, name, entity_type, workspace_id
        ))
        .await?;
        Ok(())
    }

    pub async fn insert_temporal_edge(
        &self,
        src: &str,
        dst: &str,
        edge_type: &str,
        valid_from: i64,
        valid_to: i64,
        transaction_time: i64,
    ) -> Result<(), String> {
        self.execute_ngql(&format!(
            "USE {}; INSERT EDGE {} (valid_from, valid_to, transaction_time) VALUES \"{}\"->\"{}\":({}, {}, {});",
            self.config.space, edge_type, src, dst, valid_from, valid_to, transaction_time
        ))
        .await?;
        Ok(())
    }

    pub async fn invalidate_edge(
        &self,
        src: &str,
        dst: &str,
        edge_type: &str,
        valid_to: i64,
    ) -> Result<(), String> {
        self.execute_ngql(&format!(
            "USE {}; UPDATE EDGE {} \"{}\"->\"{}\"@0 SET valid_to = {};",
            self.config.space, edge_type, src, dst, valid_to
        ))
        .await?;
        Ok(())
    }

    pub async fn as_of_query(
        &self,
        entity_id: &str,
        edge_type: &str,
        timestamp: i64,
    ) -> Result<Vec<String>, String> {
        let result = self
            .execute_ngql(&format!(
                "USE {}; GO FROM \"{}\" OVER {} WHERE {}.valid_from <= {} AND ({}.valid_to == 0 OR {}.valid_to > {}) YIELD {}._dst;",
                self.config.space, entity_id, edge_type, edge_type, timestamp, edge_type, edge_type, timestamp, edge_type
            ))
            .await?;

        let mut destinations = Vec::new();
        if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
            for row in data {
                if let Some(dst) = row.as_str() {
                    destinations.push(dst.to_string());
                } else if let Some(arr) = row.as_array() {
                    if let Some(dst) = arr.first().and_then(|v| v.as_str()) {
                        destinations.push(dst.to_string());
                    }
                }
            }
        }
        Ok(destinations)
    }

    pub async fn traverse_dependents(
        &self,
        entity_id: &str,
        edge_type: &str,
    ) -> Result<Vec<String>, String> {
        let result = self
            .execute_ngql(&format!(
                "USE {}; GO FROM \"{}\" OVER {} YIELD {}._dst;",
                self.config.space, entity_id, edge_type, edge_type
            ))
            .await?;

        let mut destinations = Vec::new();
        if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
            for row in data {
                if let Some(dst) = row.as_str() {
                    destinations.push(dst.to_string());
                } else if let Some(arr) = row.as_array() {
                    if let Some(dst) = arr.first().and_then(|v| v.as_str()) {
                        destinations.push(dst.to_string());
                    }
                }
            }
        }
        Ok(destinations)
    }

    pub async fn traverse_dependents_recursive(
        &self,
        entity_id: &str,
        edge_type: &str,
    ) -> Result<Vec<String>, String> {
        let mut all_dependents = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(entity_id.to_string());
        let mut queue = vec![entity_id.to_string()];

        while let Some(current) = queue.pop() {
            let dependents = self.traverse_dependents(&current, edge_type).await?;
            for dep in dependents {
                if visited.insert(dep.clone()) {
                    all_dependents.push(dep.clone());
                    queue.push(dep);
                }
            }
        }

        Ok(all_dependents)
    }

    pub async fn check_connection(&self) -> Result<bool, String> {
        match self.execute_ngql("SHOW SPACES;").await {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}

pub fn current_timestamp_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nebula_unreachable_returns_error() {
        let cfg = NebulaConfig {
            host: "127.0.0.1".to_string(),
            http_port: 1,
            ..Default::default()
        };
        let store = NebulaGraphStore::new(cfg);
        let result = store.check_connection().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nebula_init_schema_unreachable_returns_error() {
        let cfg = NebulaConfig {
            host: "127.0.0.1".to_string(),
            http_port: 1,
            ..Default::default()
        };
        let store = NebulaGraphStore::new(cfg);
        let result = store.init_schema().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nebula_insert_entity_unreachable_returns_error() {
        let cfg = NebulaConfig {
            host: "127.0.0.1".to_string(),
            http_port: 1,
            ..Default::default()
        };
        let store = NebulaGraphStore::new(cfg);
        let result = store
            .insert_entity("test-id", "test", "belief", "ws-1")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nebula_as_of_query_unreachable_returns_error() {
        let cfg = NebulaConfig {
            host: "127.0.0.1".to_string(),
            http_port: 1,
            ..Default::default()
        };
        let store = NebulaGraphStore::new(cfg);
        let result = store.as_of_query("entity-1", "depends_on", 1000).await;
        assert!(result.is_err());
    }
}
