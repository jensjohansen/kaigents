//! File: engine/crates/kaigents-core/src/temporal_adapter.rs
//! Purpose: HTTP client for the Kaigents Temporal adapter service.
//!
//! Keeps all Temporal concepts out of the Rust engine — the adapter is called
//! over plain HTTP/JSON using Kaigents domain types only.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use serde::{Deserialize, Serialize};

/// A single step in a WorkRequest process graph, passed to the adapter.
/// Includes per-step resolved agent config so the adapter can execute
/// with the correct system prompt, model endpoint, and MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemDef {
    #[serde(rename = "workItemId")]
    pub work_item_id: String,
    #[serde(rename = "stepName")]
    pub step_name: String,
    #[serde(rename = "agentName", skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(rename = "prompt", skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(rename = "requiresGate", skip_serializing_if = "Option::is_none")]
    pub requires_gate: Option<bool>,
    #[serde(rename = "systemPrompt", skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(rename = "modelEndpointUrl", skip_serializing_if = "Option::is_none")]
    pub model_endpoint_url: Option<String>,
    #[serde(rename = "modelName", skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(rename = "mcpServerUrl", skip_serializing_if = "Option::is_none")]
    pub mcp_server_url: Option<String>,
    #[serde(rename = "searchToolName", skip_serializing_if = "Option::is_none")]
    pub search_tool_name: Option<String>,
    #[serde(rename = "readToolName", skip_serializing_if = "Option::is_none")]
    pub read_tool_name: Option<String>,
    #[serde(rename = "metadata", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Request body for `POST /v1/workrequests`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkRequestRequest {
    #[serde(rename = "workRequestId")]
    pub work_request_id: String,
    #[serde(rename = "processName", skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    pub steps: Vec<WorkItemDef>,
}

/// Query response from `GET /v1/workrequests/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkRequestState {
    #[serde(rename = "workRequestId")]
    pub work_request_id: String,
    pub phase: String,
    #[serde(rename = "currentStep", skip_serializing_if = "Option::is_none")]
    pub current_step: Option<String>,
    #[serde(rename = "reworkCount")]
    pub rework_count: u32,
    pub message: Option<String>,
}

/// Request body for `POST /v1/consolidate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationRequest {
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
}

/// Query response from `GET /v1/consolidate/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationState {
    #[serde(rename = "consolidationId")]
    pub consolidation_id: String,
    pub phase: String,
    #[serde(rename = "episodeId", skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    pub message: Option<String>,
}

/// Minimal HTTP client for the Kaigents temporal adapter.
/// No Temporal SDK or concepts appear in this module.
pub struct TemporalAdapterClient {
    base_url: String,
    http: reqwest::Client,
}

impl TemporalAdapterClient {
    /// Create a client pointing at the adapter base URL (e.g. `http://kaigents-temporal-adapter:8080`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Start a new WorkRequest in the adapter.
    pub async fn start_work_request(&self, req: StartWorkRequestRequest) -> Result<(), String> {
        let url = format!("{}/v1/workrequests", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("temporal adapter unreachable: {e}"))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!(
                "temporal adapter start_work_request failed {status}: {body}"
            ))
        }
    }

    /// Query the current state of a WorkRequest.
    pub async fn query_work_request(
        &self,
        work_request_id: &str,
    ) -> Result<WorkRequestState, String> {
        let url = format!("{}/v1/workrequests/{}", self.base_url, work_request_id);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("temporal adapter unreachable: {e}"))?;

        if resp.status().is_success() {
            resp.json::<WorkRequestState>()
                .await
                .map_err(|e| format!("failed to parse WorkRequestState: {e}"))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!(
                "temporal adapter query_work_request failed {status}: {body}"
            ))
        }
    }

    /// Start a memory consolidation workflow in the adapter.
    /// The workflow is durable and resumable — if the adapter crashes,
    /// Temporal resumes the workflow on restart.
    pub async fn start_consolidation(
        &self,
        req: ConsolidationRequest,
    ) -> Result<ConsolidationState, String> {
        let url = format!("{}/v1/consolidate", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("temporal adapter unreachable: {e}"))?;

        if resp.status().is_success() {
            resp.json::<ConsolidationState>()
                .await
                .map_err(|e| format!("failed to parse ConsolidationState: {e}"))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!(
                "temporal adapter start_consolidation failed {status}: {body}"
            ))
        }
    }

    /// Query the state of a consolidation workflow.
    pub async fn query_consolidation(
        &self,
        consolidation_id: &str,
    ) -> Result<ConsolidationState, String> {
        let url = format!("{}/v1/consolidate/{}", self.base_url, consolidation_id);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("temporal adapter unreachable: {e}"))?;

        if resp.status().is_success() {
            resp.json::<ConsolidationState>()
                .await
                .map_err(|e| format!("failed to parse ConsolidationState: {e}"))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!(
                "temporal adapter query_consolidation failed {status}: {body}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidation_request_serializes_correctly() {
        let req = ConsolidationRequest {
            workspace_id: "ws-123".to_string(),
            run_id: "run-456".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"workspaceId\":\"ws-123\""));
        assert!(json.contains("\"runId\":\"run-456\""));
    }

    #[test]
    fn consolidation_state_deserializes_correctly() {
        let json = serde_json::json!({
            "consolidationId": "cons-789",
            "phase": "completed",
            "episodeId": "ep-abc",
            "message": "Consolidation successful"
        });
        let state: ConsolidationState = serde_json::from_value(json).unwrap();
        assert_eq!(state.consolidation_id, "cons-789");
        assert_eq!(state.phase, "completed");
        assert_eq!(state.episode_id, Some("ep-abc".to_string()));
        assert_eq!(state.message, Some("Consolidation successful".to_string()));
    }

    #[test]
    fn consolidation_state_deserializes_without_optional_fields() {
        let json = serde_json::json!({
            "consolidationId": "cons-000",
            "phase": "running"
        });
        let state: ConsolidationState = serde_json::from_value(json).unwrap();
        assert_eq!(state.consolidation_id, "cons-000");
        assert_eq!(state.phase, "running");
        assert_eq!(state.episode_id, None);
        assert_eq!(state.message, None);
    }

    #[test]
    fn temporal_adapter_client_trims_trailing_slash() {
        let client = TemporalAdapterClient::new("http://adapter:8080/");
        assert_eq!(client.base_url, "http://adapter:8080");
    }

    #[tokio::test]
    async fn consolidation_unreachable_adapter_returns_error() {
        let client = TemporalAdapterClient::new("http://127.0.0.1:1");
        let req = ConsolidationRequest {
            workspace_id: "ws-test".to_string(),
            run_id: "run-test".to_string(),
        };
        let result = client.start_consolidation(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreachable"));
    }

    #[tokio::test]
    async fn query_consolidation_unreachable_returns_error() {
        let client = TemporalAdapterClient::new("http://127.0.0.1:1");
        let result = client.query_consolidation("cons-123").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreachable"));
    }

    #[tokio::test]
    async fn start_work_request_unreachable_returns_error() {
        let client = TemporalAdapterClient::new("http://127.0.0.1:1");
        let req = StartWorkRequestRequest {
            work_request_id: "wr-test".to_string(),
            process_name: None,
            steps: vec![],
        };
        let result = client.start_work_request(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreachable"));
    }

    #[tokio::test]
    async fn query_work_request_unreachable_returns_error() {
        let client = TemporalAdapterClient::new("http://127.0.0.1:1");
        let result = client.query_work_request("wr-test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unreachable"));
    }
}
