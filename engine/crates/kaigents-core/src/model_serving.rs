//! File: engine/crates/kaigents-core/src/model_serving.rs
//! Purpose: Model serving integration with OpenAI-compatible endpoints, discovery, and timeline events.
//! Product/business importance: enables chat/embeddings with Lemonade/OpenAI-compatible servers and observability.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::run_id::RunId;
use crate::timeline::{EventType, TimelineEvent, TimelineStore};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// ModelEndpoint represents a discovered model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEndpoint {
    pub name: String,
    pub url: String,
    pub capabilities: ModelCapabilities,
    pub provider: String,
    pub metadata: HashMap<String, String>,
}

/// ModelCapabilities describes what the endpoint supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub chat: bool,
    pub embeddings: bool,
    pub max_tokens: Option<u32>,
    pub supports_streaming: bool,
}

/// ModelClient defines the minimal interface for model serving.
#[async_trait::async_trait]
pub trait ModelClient: Send + Sync {
    /// Discover available endpoints (in-cluster DNS, dev-local).
    async fn discover_endpoints(&self) -> Result<Vec<ModelEndpoint>, String>;
    /// Chat completion request.
    async fn chat_completion(
        &self,
        endpoint_name: &str,
        request: ChatCompletionRequest,
        timeout: Duration,
    ) -> Result<ChatCompletionResponse, String>;
    /// Embeddings request.
    async fn embeddings(
        &self,
        endpoint_name: &str,
        request: EmbeddingsRequest,
        timeout: Duration,
    ) -> Result<EmbeddingsResponse, String>;
}

/// ChatCompletionRequest mirrors OpenAI chat completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

/// ChatCompletionResponse mirrors OpenAI chat completions response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionStreamChunk {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Vec<ChatCompletionStreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionStreamChoice {
    pub index: u32,
    pub delta: ChatCompletionDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChatCompletionDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

/// EmbeddingsRequest mirrors OpenAI embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: Vec<String>, // array of strings or array of token arrays
    pub encoding_format: Option<String>,
}

/// EmbeddingsResponse mirrors OpenAI embeddings response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub object: String,
    pub model: String,
    pub data: Vec<Embedding>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct HttpOpenAIModelClient {
    http: reqwest::Client,
    endpoints: Vec<ModelEndpoint>,
    api_key: Option<String>,
}

impl HttpOpenAIModelClient {
    /// Create a client from environment variables.
    ///
    /// Required:
    /// - KAIGENTS_MODEL_ENDPOINT_URL
    ///
    /// Optional:
    /// - KAIGENTS_MODEL_ENDPOINT_NAME (default: "default")
    /// - KAIGENTS_MODEL_ENDPOINT_PROVIDER (default: "openai-compatible")
    /// - KAIGENTS_MODEL_ENDPOINT_CHAT (default: "true")
    /// - KAIGENTS_MODEL_ENDPOINT_EMBEDDINGS (default: "false")
    /// - KAIGENTS_MODEL_ENDPOINT_SUPPORTS_STREAMING (default: "false")
    /// - KAIGENTS_MODEL_ENDPOINT_MAX_TOKENS (u32)
    /// - KAIGENTS_MODEL_API_KEY (optional)
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("KAIGENTS_MODEL_ENDPOINT_URL")
            .map_err(|_| "KAIGENTS_MODEL_ENDPOINT_URL is required".to_string())?;
        let name =
            std::env::var("KAIGENTS_MODEL_ENDPOINT_NAME").unwrap_or_else(|_| "default".to_string());
        let provider = std::env::var("KAIGENTS_MODEL_ENDPOINT_PROVIDER")
            .unwrap_or_else(|_| "openai-compatible".to_string());

        let chat = std::env::var("KAIGENTS_MODEL_ENDPOINT_CHAT")
            .ok()
            .map(|v| v != "false")
            .unwrap_or(true);
        let embeddings = std::env::var("KAIGENTS_MODEL_ENDPOINT_EMBEDDINGS")
            .ok()
            .map(|v| v == "true")
            .unwrap_or(false);
        let supports_streaming = std::env::var("KAIGENTS_MODEL_ENDPOINT_SUPPORTS_STREAMING")
            .ok()
            .map(|v| v == "true")
            .unwrap_or(false);
        let max_tokens = std::env::var("KAIGENTS_MODEL_ENDPOINT_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok());

        let api_key = std::env::var("KAIGENTS_MODEL_API_KEY").ok();

        let endpoint = ModelEndpoint {
            name,
            url,
            capabilities: ModelCapabilities {
                chat,
                embeddings,
                max_tokens,
                supports_streaming,
            },
            provider,
            metadata: HashMap::new(),
        };

        Ok(Self {
            http: reqwest::Client::new(),
            endpoints: vec![endpoint],
            api_key,
        })
    }

    fn endpoint_by_name(&self, endpoint_name: &str) -> Result<&ModelEndpoint, String> {
        self.endpoints
            .iter()
            .find(|e| e.name == endpoint_name)
            .ok_or_else(|| format!("Endpoint '{}' not found", endpoint_name))
    }

    fn endpoint_base_url(endpoint_url: &str) -> String {
        endpoint_url.trim_end_matches('/').to_string()
    }
}

/// InMemoryModelClient is a placeholder for testing and MVP.
pub struct InMemoryModelClient {
    endpoints: Arc<Mutex<HashMap<String, ModelEndpoint>>>,
}

impl InMemoryModelClient {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register an endpoint (simulating discovery).
    pub async fn register_endpoint(&self, endpoint: ModelEndpoint) -> Result<(), String> {
        let mut endpoints_map = self.endpoints.lock().await;
        endpoints_map.insert(endpoint.name.clone(), endpoint);
        Ok(())
    }

    /// Simulate in-cluster DNS discovery.
    pub async fn discover_in_cluster_dns(&self) -> Result<Vec<ModelEndpoint>, String> {
        // Placeholder: return known endpoints
        let endpoints_map = self.endpoints.lock().await;
        Ok(endpoints_map.values().cloned().collect())
    }

    /// Simulate developer-local endpoint discovery.
    pub async fn discover_dev_local(&self) -> Result<Vec<ModelEndpoint>, String> {
        // Placeholder: check for common local endpoints
        let mut discovered = Vec::new();
        let endpoints_map = self.endpoints.lock().await;
        for endpoint in endpoints_map.values() {
            if endpoint.url.starts_with("http://localhost")
                || endpoint.url.starts_with("http://127.0.0.1")
            {
                discovered.push(endpoint.clone());
            }
        }
        Ok(discovered)
    }
}

impl Default for InMemoryModelClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ModelClient for HttpOpenAIModelClient {
    async fn discover_endpoints(&self) -> Result<Vec<ModelEndpoint>, String> {
        Ok(self.endpoints.clone())
    }

    async fn chat_completion(
        &self,
        endpoint_name: &str,
        request: ChatCompletionRequest,
        timeout: Duration,
    ) -> Result<ChatCompletionResponse, String> {
        let endpoint = self.endpoint_by_name(endpoint_name)?;
        if !endpoint.capabilities.chat {
            return Err(format!(
                "Endpoint '{}' does not support chat",
                endpoint_name
            ));
        }

        let url = format!(
            "{}/v1/chat/completions",
            Self::endpoint_base_url(&endpoint.url)
        );

        let mut builder = self
            .http
            .post(url)
            .timeout(timeout)
            .header("accept", "application/json, text/event-stream")
            .json(&request);
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key);
        }

        let response = builder.send().await.map_err(|e| {
            format!(
                "Model request failed: {} (endpoint={}, timeout_ms={})",
                e,
                endpoint.url,
                timeout.as_millis()
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            return Err(format!("Model request failed: HTTP {status} {body}"));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if request.stream || content_type.starts_with("text/event-stream") {
            let mut aggregated_content = String::new();
            let mut finish_reason: Option<String> = None;
            let mut response_id: Option<String> = None;
            let mut response_model: Option<String> = None;
            let mut response_created: Option<u64> = None;
            let mut response_object: Option<String> = None;
            let mut role: Option<String> = None;

            let mut stream = response.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(item) = stream.next().await {
                let chunk = item.map_err(|e| format!("Model stream read failed: {e}"))?;
                buf.extend_from_slice(&chunk);

                while let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
                    let frame = buf.drain(..pos + 2).collect::<Vec<u8>>();
                    let frame_text = String::from_utf8_lossy(&frame);

                    for line in frame_text.lines() {
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("data:") {
                            let data = rest.trim();
                            if data == "[DONE]" {
                                let id = response_id
                                    .unwrap_or_else(|| format!("chatcmpl-{}", Uuid::new_v4()));
                                let model = response_model.unwrap_or_else(|| request.model.clone());
                                let created =
                                    response_created.unwrap_or_else(current_unix_timestamp);
                                let object = response_object
                                    .unwrap_or_else(|| "chat.completion".to_string());
                                let role = role.unwrap_or_else(|| "assistant".to_string());

                                return Ok(ChatCompletionResponse {
                                    id,
                                    object,
                                    created,
                                    model,
                                    choices: vec![ChatChoice {
                                        index: 0,
                                        message: ChatMessage {
                                            role,
                                            content: aggregated_content,
                                        },
                                        finish_reason,
                                    }],
                                    usage: None,
                                });
                            }

                            let parsed: ChatCompletionStreamChunk = serde_json::from_str(data)
                                .map_err(|e| {
                                    format!("Failed to parse model stream chunk: {e} (data={data})")
                                })?;

                            if response_id.is_none() {
                                response_id = parsed.id;
                            }
                            if response_model.is_none() {
                                response_model = parsed.model;
                            }
                            if response_created.is_none() {
                                response_created = parsed.created;
                            }
                            if response_object.is_none() {
                                response_object = parsed.object;
                            }

                            if let Some(choice) = parsed.choices.first() {
                                if role.is_none() {
                                    role = choice.delta.role.clone();
                                }
                                if let Some(content) = &choice.delta.content {
                                    aggregated_content.push_str(content);
                                }
                                if choice.finish_reason.is_some() {
                                    finish_reason = choice.finish_reason.clone();
                                }
                            }
                        }
                    }
                }
            }

            return Err("Model stream ended without [DONE]".to_string());
        }

        response
            .json::<ChatCompletionResponse>()
            .await
            .map_err(|e| format!("Failed to parse model response: {e}"))
    }

    async fn embeddings(
        &self,
        endpoint_name: &str,
        request: EmbeddingsRequest,
        timeout: Duration,
    ) -> Result<EmbeddingsResponse, String> {
        let endpoint = self.endpoint_by_name(endpoint_name)?;
        if !endpoint.capabilities.embeddings {
            return Err(format!(
                "Endpoint '{}' does not support embeddings",
                endpoint_name
            ));
        }

        let url = format!("{}/v1/embeddings", Self::endpoint_base_url(&endpoint.url));

        let mut builder = self.http.post(url).timeout(timeout).json(&request);
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| format!("Embeddings request failed: {e}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            return Err(format!("Embeddings request failed: HTTP {status} {body}"));
        }

        response
            .json::<EmbeddingsResponse>()
            .await
            .map_err(|e| format!("Failed to parse embeddings response: {e}"))
    }
}

#[async_trait::async_trait]
impl ModelClient for InMemoryModelClient {
    async fn discover_endpoints(&self) -> Result<Vec<ModelEndpoint>, String> {
        let mut endpoints = Vec::new();
        endpoints.extend(self.discover_in_cluster_dns().await?);
        endpoints.extend(self.discover_dev_local().await?);
        Ok(endpoints)
    }

    async fn chat_completion(
        &self,
        endpoint_name: &str,
        request: ChatCompletionRequest,
        _timeout: Duration,
    ) -> Result<ChatCompletionResponse, String> {
        let endpoints_map = self.endpoints.lock().await;
        let endpoint = endpoints_map
            .get(endpoint_name)
            .ok_or_else(|| format!("Endpoint '{}' not found", endpoint_name))?;
        if !endpoint.capabilities.chat {
            return Err(format!(
                "Endpoint '{}' does not support chat",
                endpoint_name
            ));
        }
        // Simulate work
        tokio::time::sleep(Duration::from_millis(200)).await;
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to read system time: {}", e))?
            .as_secs();

        let response = ChatCompletionResponse {
            id: format!("chatcmpl-{}", Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created,
            model: request.model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Simulated chat completion response".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 8,
                total_tokens: 18,
            }),
        };
        Ok(response)
    }

    async fn embeddings(
        &self,
        endpoint_name: &str,
        request: EmbeddingsRequest,
        _timeout: Duration,
    ) -> Result<EmbeddingsResponse, String> {
        let endpoints_map = self.endpoints.lock().await;
        let endpoint = endpoints_map
            .get(endpoint_name)
            .ok_or_else(|| format!("Endpoint '{}' not found", endpoint_name))?;
        if !endpoint.capabilities.embeddings {
            return Err(format!(
                "Endpoint '{}' does not support embeddings",
                endpoint_name
            ));
        }
        // Simulate work
        tokio::time::sleep(Duration::from_millis(150)).await;
        let data: Vec<Embedding> = request
            .input
            .iter()
            .enumerate()
            .map(|(i, _txt)| Embedding {
                object: "embedding".to_string(),
                embedding: vec![0.0; 1536], // placeholder embedding
                index: i as u32,
            })
            .collect();
        let response = EmbeddingsResponse {
            object: "list".to_string(),
            model: request.model.clone(),
            data,
            usage: Usage {
                prompt_tokens: request.input.len() as u32 * 6, // rough estimate
                completion_tokens: 0,
                total_tokens: request.input.len() as u32 * 6,
            },
        };
        Ok(response)
    }
}

/// ModelPlane manages model endpoints, discovery, and invocation with timeline events.
pub struct ModelPlane {
    client: Box<dyn ModelClient>,
    endpoints: Arc<Mutex<HashMap<String, ModelEndpoint>>>,
    timeline: Arc<TimelineStore>,
}

impl ModelPlane {
    pub fn new(client: Box<dyn ModelClient>, timeline: Arc<TimelineStore>) -> Self {
        Self {
            client,
            endpoints: Arc::new(Mutex::new(HashMap::new())),
            timeline,
        }
    }

    /// Refresh endpoint discovery.
    pub async fn refresh_endpoints(&self) -> Result<(), String> {
        let discovered = self.client.discover_endpoints().await?;
        let mut eps = self.endpoints.lock().await;
        for ep in discovered {
            eps.insert(ep.name.clone(), ep);
        }
        Ok(())
    }

    /// Chat completion with timeline events.
    pub async fn chat_completion(
        &self,
        run_id: RunId,
        endpoint_name: &str,
        request: ChatCompletionRequest,
        timeout: Duration,
    ) -> Result<ChatCompletionResponse, String> {
        let correlation_id = format!("chat-{}", Uuid::new_v4());
        let start = SystemTime::now();
        // Emit ModelInvoked event
        let invoked = TimelineEvent::new(
            run_id.clone(),
            EventType::ModelInvoked {
                endpoint: endpoint_name.to_string(),
            },
        )
        .with_correlation(correlation_id.clone())
        .with_payload("model".to_string(), request.model.clone())
        .with_payload("timeout_ms".to_string(), timeout.as_millis().to_string());
        self.timeline
            .append(invoked)
            .map_err(|e| format!("Timeline error: {}", e))?;

        let result = self
            .client
            .chat_completion(endpoint_name, request.clone(), timeout)
            .await;
        let elapsed = start.elapsed().map_err(|e| format!("Timer error: {}", e))?;
        match result {
            Ok(response) => {
                // Emit ModelFinished event with latency and token counts
                let mut payload = HashMap::new();
                payload.insert("latency_ms".to_string(), elapsed.as_millis().to_string());
                if let Some(usage) = &response.usage {
                    payload.insert("prompt_tokens".to_string(), usage.prompt_tokens.to_string());
                    payload.insert(
                        "completion_tokens".to_string(),
                        usage.completion_tokens.to_string(),
                    );
                    payload.insert("total_tokens".to_string(), usage.total_tokens.to_string());
                }
                let finished = TimelineEvent::new(run_id, EventType::ModelFinished)
                    .with_correlation(correlation_id)
                    .with_payload_map(payload);
                self.timeline
                    .append(finished)
                    .map_err(|e| format!("Timeline error: {}", e))?;
                Ok(response)
            }
            Err(e) => {
                let failed =
                    TimelineEvent::new(run_id, EventType::ModelFailed { error: e.clone() })
                        .with_correlation(correlation_id);
                self.timeline
                    .append(failed)
                    .map_err(|e2| format!("Timeline error: {}", e2))?;
                Err(e)
            }
        }
    }

    /// Embeddings with timeline events.
    pub async fn embeddings(
        &self,
        run_id: RunId,
        endpoint_name: &str,
        request: EmbeddingsRequest,
        timeout: Duration,
    ) -> Result<EmbeddingsResponse, String> {
        let correlation_id = format!("embed-{}", Uuid::new_v4());
        let start = SystemTime::now();
        // Emit ModelInvoked event
        let invoked = TimelineEvent::new(
            run_id.clone(),
            EventType::ModelInvoked {
                endpoint: endpoint_name.to_string(),
            },
        )
        .with_correlation(correlation_id.clone())
        .with_payload("model".to_string(), request.model.clone())
        .with_payload("input_count".to_string(), request.input.len().to_string());
        self.timeline
            .append(invoked)
            .map_err(|e| format!("Timeline error: {}", e))?;

        let result = self
            .client
            .embeddings(endpoint_name, request.clone(), timeout)
            .await;
        let elapsed = start.elapsed().map_err(|e| format!("Timer error: {}", e))?;
        match result {
            Ok(response) => {
                // Emit ModelFinished event with latency and token counts
                let mut payload = HashMap::new();
                payload.insert("latency_ms".to_string(), elapsed.as_millis().to_string());
                payload.insert(
                    "prompt_tokens".to_string(),
                    response.usage.prompt_tokens.to_string(),
                );
                payload.insert(
                    "total_tokens".to_string(),
                    response.usage.total_tokens.to_string(),
                );
                let finished = TimelineEvent::new(run_id, EventType::ModelFinished)
                    .with_correlation(correlation_id)
                    .with_payload_map(payload);
                self.timeline
                    .append(finished)
                    .map_err(|e| format!("Timeline error: {}", e))?;
                Ok(response)
            }
            Err(e) => {
                let failed =
                    TimelineEvent::new(run_id, EventType::ModelFailed { error: e.clone() })
                        .with_correlation(correlation_id);
                self.timeline
                    .append(failed)
                    .map_err(|e2| format!("Timeline error: {}", e2))?;
                Err(e)
            }
        }
    }
}

// Helper to add multiple payload key-values at once
trait TimelineEventExt {
    fn with_payload_map(self, map: HashMap<String, String>) -> Self;
}

impl TimelineEventExt for TimelineEvent {
    fn with_payload_map(mut self, map: HashMap<String, String>) -> Self {
        for (k, v) in map {
            self.payload.insert(k, v);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::TimelineStore;

    #[tokio::test]
    async fn model_plane_discover_and_chat() {
        let timeline = Arc::new(TimelineStore::new());
        let client = InMemoryModelClient::new();
        let endpoint = ModelEndpoint {
            name: "lemonade-local".to_string(),
            url: "http://localhost:8080".to_string(),
            capabilities: ModelCapabilities {
                chat: true,
                embeddings: true,
                max_tokens: Some(4096),
                supports_streaming: false,
            },
            provider: "Lemonade".to_string(),
            metadata: HashMap::new(),
        };
        client.register_endpoint(endpoint.clone()).await.unwrap();
        let model_plane = ModelPlane::new(Box::new(client), timeline.clone());
        model_plane.refresh_endpoints().await.unwrap();

        let run_id = RunId::new();
        let request = ChatCompletionRequest {
            model: "llama3".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
        };
        let response = model_plane
            .chat_completion(
                run_id.clone(),
                "lemonade-local",
                request,
                Duration::from_millis(500),
            )
            .await
            .unwrap();
        assert_eq!(response.object, "chat.completion");
        assert_eq!(
            response.choices[0].message.content,
            "Simulated chat completion response"
        );
        // Verify timeline events
        let events = timeline.query_by_run(&run_id).unwrap();
        assert_eq!(events.len(), 2); // ModelInvoked + ModelFinished
        assert!(matches!(
            events[0].event_type,
            EventType::ModelInvoked { .. }
        ));
        assert!(matches!(events[1].event_type, EventType::ModelFinished));
        assert!(events[1].payload.contains_key("latency_ms"));
        assert!(events[1].payload.contains_key("total_tokens"));
    }

    #[tokio::test]
    async fn model_plane_embeddings() {
        let timeline = Arc::new(TimelineStore::new());
        let client = InMemoryModelClient::new();
        let endpoint = ModelEndpoint {
            name: "embeddings".to_string(),
            url: "http://localhost:8081".to_string(),
            capabilities: ModelCapabilities {
                chat: false,
                embeddings: true,
                max_tokens: None,
                supports_streaming: false,
            },
            provider: "Lemonade".to_string(),
            metadata: HashMap::new(),
        };
        client.register_endpoint(endpoint).await.unwrap();
        let model_plane = ModelPlane::new(Box::new(client), timeline.clone());
        model_plane.refresh_endpoints().await.unwrap();

        let run_id = RunId::new();
        let request = EmbeddingsRequest {
            model: "all-minilm-l6-v2".to_string(),
            input: vec!["hello world".to_string(), "test".to_string()],
            encoding_format: Some("float".to_string()),
        };
        let response = model_plane
            .embeddings(
                run_id.clone(),
                "embeddings",
                request,
                Duration::from_millis(500),
            )
            .await
            .unwrap();
        assert_eq!(response.object, "list");
        assert_eq!(response.data.len(), 2);
        // Verify timeline events
        let events = timeline.query_by_run(&run_id).unwrap();
        assert_eq!(events.len(), 2); // ModelInvoked + ModelFinished
        assert!(matches!(
            events[0].event_type,
            EventType::ModelInvoked { .. }
        ));
        assert!(matches!(events[1].event_type, EventType::ModelFinished));
        assert!(events[1].payload.contains_key("latency_ms"));
        assert!(events[1].payload.contains_key("prompt_tokens"));
    }
}
