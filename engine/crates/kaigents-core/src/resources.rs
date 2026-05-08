// File: engine/crates/kaigents-core/src/resources.rs
// Purpose: Rust types for Kaigents Kubernetes resources.
// Product/business importance: enables the execution engine and CLI to interact with the control plane.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

use kube::CustomResource;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "core.kaigents.io", version = "v1alpha1", kind = "Agent", namespaced)]
#[kube(status = "AgentStatus")]
pub struct AgentSpec {
    pub runtime: Option<String>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<AgentToolRef>>,
    #[serde(rename = "modelEndpointRef")]
    pub model_endpoint_ref: Option<String>,
    #[serde(rename = "modelName")]
    pub model_name: Option<String>,
    #[serde(rename = "allowedTools")]
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct AgentToolRef {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct AgentStatus {
    pub phase: Option<String>,
}

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "core.kaigents.io", version = "v1alpha1", kind = "Task", namespaced)]
#[kube(status = "TaskStatus")]
pub struct TaskSpec {
    #[serde(rename = "agentName")]
    pub agent_name: Option<String>,
    pub prompt: Option<String>,
    #[serde(rename = "requiresGate")]
    pub requires_gate: Option<bool>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct TaskStatus {
    pub phase: Option<String>,
}

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "core.kaigents.io", version = "v1alpha1", kind = "Process", namespaced)]
#[kube(status = "ProcessStatus")]
pub struct ProcessSpec {
    pub steps: Vec<ProcessStep>,
    #[serde(rename = "maxReworkAttempts")]
    pub max_rework_attempts: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ProcessStep {
    pub id: String,
    pub name: String,
    #[serde(rename = "taskRef")]
    pub task_ref: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ProcessStatus {
    pub phase: Option<String>,
}
