//! File: engine/crates/kaigents-core/src/tool_plane.rs
//! Purpose: MCP-first tool plane integration with timeouts, bounded outputs, and timeline events.
//! Product/business importance: enables observable, auditable tool invocations with contract snapshotting.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::run_id::RunId;
use crate::timeline::{EventType, TimelineEvent, TimelineStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// ToolContract represents a versioned snapshot of a tool's contract from MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContract {
    pub server_name: String,
    pub tool_name: String,
    pub version: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub description: Option<String>,
}

pub trait TimelineSink: Send + Sync {
    fn append(&self, event: TimelineEvent) -> Result<(), String>;
}

impl TimelineSink for TimelineStore {
    fn append(&self, event: TimelineEvent) -> Result<(), String> {
        TimelineStore::append(self, event)
    }
}

pub trait ToolContractSink: Send + Sync {
    fn store_contract(&self, contract: &ToolContract) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct InMemoryToolContractSink {
    inner: Arc<std::sync::Mutex<Vec<ToolContract>>>,
}

impl InMemoryToolContractSink {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryToolContractSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolContractSink for InMemoryToolContractSink {
    fn store_contract(&self, contract: &ToolContract) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "InMemoryToolContractSink mutex poisoned".to_string())?;
        guard.push(contract.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HttpMcpClient {
    server_name: String,
    endpoint: String,
    http: reqwest::Client,
    headers: HashMap<String, String>,
}

impl HttpMcpClient {
    pub fn new(server_name: String, endpoint: String) -> Self {
        Self {
            server_name,
            endpoint,
            http: reqwest::Client::new(),
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    async fn jsonrpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let mut request_builder = self.http.post(self.endpoint.clone()).json(&req);
        for (k, v) in &self.headers {
            request_builder = request_builder.header(k, v);
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("MCP HTTP error: {}", e))?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("MCP JSON decode error: {}", e))?;

        if !status.is_success() {
            return Err(format!("MCP HTTP status {}: {}", status, body));
        }

        if let Some(err) = body.get("error") {
            return Err(format!("MCP JSON-RPC error: {}", err));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| format!("MCP missing result field: {}", body))
    }
}

#[async_trait::async_trait]
impl MCPClient for HttpMcpClient {
    async fn list_tools(&self) -> Result<Vec<ToolContract>, String> {
        let result = self.jsonrpc("tools/list", serde_json::json!({})).await?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("MCP tools/list missing tools array: {}", result))?;

        let mut out = Vec::with_capacity(tools.len());
        for t in tools {
            let name = t
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("MCP tool missing name: {}", t))?;
            let input_schema = t
                .get("inputSchema")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let description = t
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            out.push(ToolContract {
                server_name: self.server_name.clone(),
                tool_name: name.to_string(),
                version: "unknown".to_string(),
                input_schema,
                output_schema: serde_json::Value::Null,
                description,
            });
        }

        Ok(out)
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        let result = self
            .jsonrpc(
                "tools/call",
                serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments,
                }),
            )
            .await?;
        Ok(result)
    }
}

/// MCPClient defines the minimal interface for connecting to MCP servers and invoking tools.
#[async_trait::async_trait]
pub trait MCPClient: Send + Sync {
    /// List available tools from the server.
    async fn list_tools(&self) -> Result<Vec<ToolContract>, String>;
    /// Call a tool with arguments and timeout.
    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, String>;
}

/// InMemoryMCPClient is a placeholder MCP client for testing and MVP.
pub struct InMemoryMCPClient {
    tools: Arc<Mutex<HashMap<String, ToolContract>>>,
}

impl InMemoryMCPClient {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a tool contract (simulating MCP server discovery).
    pub async fn register_tool(&self, contract: ToolContract) -> Result<(), String> {
        let mut tools = self.tools.lock().await;
        tools.insert(contract.tool_name.clone(), contract);
        Ok(())
    }
}

impl Default for InMemoryMCPClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MCPClient for InMemoryMCPClient {
    async fn list_tools(&self) -> Result<Vec<ToolContract>, String> {
        let tools = self.tools.lock().await;
        Ok(tools.values().cloned().collect())
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        let tools = self.tools.lock().await;
        if let Some(_contract) = tools.get(tool_name) {
            // Simulate work and bounded output
            tokio::time::sleep(Duration::from_millis(50)).await;
            // For MVP, echo the arguments as output (bounded)
            let output = serde_json::json!({
                "tool": tool_name,
                "input": arguments,
                "output": "simulated result"
            });
            Ok(output)
        } else {
            Err(format!("Tool '{}' not found", tool_name))
        }
    }
}

/// ToolPlane manages MCP clients, contract snapshots, and invocation with timeline events.
pub struct ToolPlane {
    clients: HashMap<String, Box<dyn MCPClient>>,
    contracts: Arc<Mutex<HashMap<String, ToolContract>>>,
    timeline: Arc<dyn TimelineSink>,
    contract_sink: Option<Arc<dyn ToolContractSink>>,
}

impl ToolPlane {
    pub fn new(timeline: Arc<dyn TimelineSink>) -> Self {
        Self {
            clients: HashMap::new(),
            contracts: Arc::new(Mutex::new(HashMap::new())),
            timeline,
            contract_sink: None,
        }
    }

    pub fn with_contract_sink(mut self, sink: Arc<dyn ToolContractSink>) -> Self {
        self.contract_sink = Some(sink);
        self
    }

    /// Register an MCP client by server name.
    pub fn register_client(&mut self, server_name: String, client: Box<dyn MCPClient>) {
        self.clients.insert(server_name, client);
    }

    /// Refresh contracts from all registered clients.
    pub async fn refresh_contracts(&self) -> Result<(), String> {
        let mut all_contracts = Vec::new();
        for (server_name, client) in &self.clients {
            let tools = client.list_tools().await.map_err(|e| {
                format!("Failed to list tools from server '{}': {}", server_name, e)
            })?;
            all_contracts.extend(tools);
        }
        let mut contracts = self.contracts.lock().await;
        for contract in all_contracts {
            contracts.insert(contract.tool_name.clone(), contract);
        }

        if let Some(sink) = &self.contract_sink {
            for contract in contracts.values() {
                sink.store_contract(contract)?;
            }
        }

        Ok(())
    }

    /// Invoke a tool with timeout, write timeline events, and enforce output bounds.
    pub async fn invoke_tool(
        &self,
        run_id: RunId,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        const MAX_OUTPUT_BYTES: usize = 64 * 1024;

        let correlation_id = format!("tool-{}", Uuid::new_v4());
        // Emit ToolInvoked event
        let invoked = TimelineEvent::new(
            run_id.clone(),
            EventType::ToolInvoked {
                tool_name: tool_name.to_string(),
            },
        )
        .with_correlation(correlation_id.clone())
        .with_payload("timeout_ms".to_string(), timeout.as_millis().to_string());
        self.timeline
            .append(invoked)
            .map_err(|e| format!("Timeline error: {}", e))?;

        // Find a client that provides the tool
        let contracts = self.contracts.lock().await;
        let _contract = contracts
            .get(tool_name)
            .ok_or_else(|| format!("Tool '{}' not registered", tool_name))?
            .clone();
        drop(contracts);

        let client = self.clients.get(&_contract.server_name).ok_or_else(|| {
            format!(
                "No MCP client registered for server '{}'",
                _contract.server_name
            )
        })?;

        let result = tokio::time::timeout(
            timeout,
            client.call_tool(tool_name, arguments.clone(), timeout),
        )
        .await;
        match result {
            Ok(Ok(output)) => {
                let mut bounded = output;
                let serialized = bounded.to_string();
                if serialized.len() > MAX_OUTPUT_BYTES {
                    bounded = serde_json::json!({
                        "truncated": true,
                        "output": serialized.chars().take(MAX_OUTPUT_BYTES).collect::<String>(),
                    });
                }
                // Emit ToolFinished event
                let finished = TimelineEvent::new(run_id, EventType::ToolFinished)
                    .with_correlation(correlation_id)
                    .with_payload(
                        "output_size".to_string(),
                        bounded.to_string().len().to_string(),
                    );
                self.timeline
                    .append(finished)
                    .map_err(|e| format!("Timeline error: {}", e))?;
                Ok(bounded)
            }
            Ok(Err(e)) => {
                // Emit ToolFailed event
                let failed = TimelineEvent::new(run_id, EventType::ToolFailed { error: e.clone() })
                    .with_correlation(correlation_id);
                self.timeline
                    .append(failed)
                    .map_err(|e2| format!("Timeline error: {}", e2))?;
                Err(e)
            }
            Err(_) => {
                let msg = format!("Tool '{}' timed out after {:?}", tool_name, timeout);
                let failed =
                    TimelineEvent::new(run_id, EventType::ToolFailed { error: msg.clone() })
                        .with_correlation(correlation_id);
                self.timeline
                    .append(failed)
                    .map_err(|e| format!("Timeline error: {}", e))?;
                Err(msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::TimelineStore;

    #[tokio::test]
    async fn tool_plane_register_and_invoke() {
        let timeline = Arc::new(TimelineStore::new());
        let mut tool_plane = ToolPlane::new(timeline.clone());
        let client = InMemoryMCPClient::new();
        let contract = ToolContract {
            server_name: "test-server".to_string(),
            tool_name: "echo".to_string(),
            version: "1.0".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            description: None,
        };
        client.register_tool(contract.clone()).await.unwrap();
        tool_plane.register_client("test-server".to_string(), Box::new(client));
        tool_plane.refresh_contracts().await.unwrap();
        let run_id = RunId::new();
        let result = tool_plane
            .invoke_tool(
                run_id.clone(),
                "echo",
                serde_json::json!({"msg": "hello"}),
                Duration::from_millis(100),
            )
            .await
            .unwrap();
        assert_eq!(result["tool"], "echo");
        assert_eq!(result["input"]["msg"], "hello");
        // Verify timeline events
        let events = timeline.query_by_run(&run_id).unwrap();
        assert_eq!(events.len(), 2); // ToolInvoked + ToolFinished
        assert!(matches!(
            events[0].event_type,
            EventType::ToolInvoked { .. }
        ));
        assert!(matches!(events[1].event_type, EventType::ToolFinished));
    }

    #[tokio::test]
    async fn tool_plane_timeout() {
        let timeline = Arc::new(TimelineStore::new());
        let mut tool_plane = ToolPlane::new(timeline.clone());
        let client = InMemoryMCPClient::new();
        let contract = ToolContract {
            server_name: "test-server".to_string(),
            tool_name: "slow".to_string(),
            version: "1.0".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            description: None,
        };
        client.register_tool(contract).await.unwrap();
        tool_plane.register_client("test-server".to_string(), Box::new(client));
        tool_plane.refresh_contracts().await.unwrap();
        let run_id = RunId::new();
        let err = tool_plane
            .invoke_tool(
                run_id.clone(),
                "slow",
                serde_json::json!({}),
                Duration::from_millis(10), // too short
            )
            .await
            .unwrap_err();
        assert!(err.contains("timed out"));
        // Verify ToolFailed event
        let events = timeline.query_by_run(&run_id).unwrap();
        assert_eq!(events.len(), 2); // ToolInvoked + ToolFailed
        assert!(matches!(events[1].event_type, EventType::ToolFailed { .. }));
    }
}
