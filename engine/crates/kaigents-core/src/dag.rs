//! File: engine/crates/kaigents-core/src/dag.rs
//! Purpose: Embedded DAG execution model with dependencies, concurrency, retries, and cancellation.
//! Product/business importance: enables reliable multi-step workflows with offload escape hatch.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use k8s_openapi::api::core::v1 as k8s_core;
use kube::api::{Api, PostParams};

/// NodeId uniquely identifies a node within a DAG.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// StepType defines how a node should be executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepType {
    /// In-process function call (e.g., tool invocation, model call).
    Inline,
    /// Offload to Kubernetes Job/Pod.
    K8sOffload { image: String, command: Vec<String> },
}

/// Node represents a single step in the workflow.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub step_type: StepType,
    pub dependencies: Vec<NodeId>,
}

/// NodeStatus tracks execution state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    Pending,
    Running,
    Succeeded,
    Failed { error: String, retries: u32 },
    Cancelled,
}

/// ExecutionResult holds outputs/errors.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub outputs: HashMap<String, String>,
    pub error: Option<String>,
}

/// DAG represents a directed acyclic graph of steps.
#[derive(Debug, Clone)]
pub struct DAG {
    pub nodes: HashMap<NodeId, Node>,
}

impl DAG {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Add a node to the DAG.
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Validate that the DAG has no cycles.
    pub fn validate(&self) -> Result<(), String> {
        // Simple depth-first visitation to detect cycles
        let mut visiting = HashMap::new();
        for node_id in self.nodes.keys() {
            if self.dfs_visit(node_id, &mut visiting).is_err() {
                return Err("Cycle detected in DAG".to_string());
            }
        }
        Ok(())
    }

    fn dfs_visit(
        &self,
        node_id: &NodeId,
        visiting: &mut HashMap<NodeId, bool>,
    ) -> Result<(), String> {
        match visiting.get(node_id) {
            Some(true) => return Err("cycle".to_string()),
            Some(false) => return Ok(()),
            None => (),
        }
        visiting.insert(node_id.clone(), true);
        if let Some(node) = self.nodes.get(node_id) {
            for dep in &node.dependencies {
                self.dfs_visit(dep, visiting)?;
            }
        }
        visiting.insert(node_id.clone(), false);
        Ok(())
    }

    /// Topological sort returns nodes in execution order.
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, String> {
        let mut sorted = Vec::new();
        let mut visiting = HashMap::new();
        for node_id in self.nodes.keys() {
            self.dfs_sort(node_id, &mut visiting, &mut sorted)?;
        }
        Ok(sorted)
    }

    fn dfs_sort(
        &self,
        node_id: &NodeId,
        visiting: &mut HashMap<NodeId, bool>,
        sorted: &mut Vec<NodeId>,
    ) -> Result<(), String> {
        if visiting.get(node_id).copied() == Some(true) {
            return Err("cycle".to_string());
        }
        if visiting.get(node_id).copied() == Some(false) {
            return Ok(());
        }
        visiting.insert(node_id.clone(), true);
        if let Some(node) = self.nodes.get(node_id) {
            for dep in &node.dependencies {
                self.dfs_sort(dep, visiting, sorted)?;
            }
        }
        visiting.insert(node_id.clone(), false);
        if !sorted.contains(node_id) {
            sorted.push(node_id.clone());
        }
        Ok(())
    }
}

impl Default for DAG {
    fn default() -> Self {
        Self::new()
    }
}

/// CancellationToken allows cooperative cancellation of a DAG run.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// DAGExecutor runs a DAG with concurrency, retries, and cancellation.
pub struct DAGExecutor {
    max_retries: u32,
    // In real implementation, we would add cancellation token and concurrency limiter.
}

impl DAGExecutor {
    /// Create a new DAGExecutor with the given maximum retries per node.
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    /// Get the configured maximum retries.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Execute the DAG to completion or failure, respecting cancellation.
    /// Nodes are only spawned after all their dependencies have completed.
    pub async fn execute(
        &self,
        dag: &DAG,
        cancel: CancellationToken,
    ) -> Result<HashMap<NodeId, ExecutionResult>, String> {
        dag.validate()?;
        let order = dag.topological_sort()?;

        let mut results: HashMap<NodeId, ExecutionResult> = HashMap::new();
        let mut completed: HashSet<NodeId> = HashSet::new();

        // Map: node_id -> set of dependencies that must complete before it can run
        let mut dependents: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut remaining_deps: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        for node_id in &order {
            let node = dag
                .nodes
                .get(node_id)
                .ok_or_else(|| "DAG internal error: node missing".to_string())?;
            remaining_deps.insert(node_id.clone(), node.dependencies.iter().cloned().collect());
            for dep in &node.dependencies {
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(node_id.clone());
            }
        }

        // Initially ready nodes (no dependencies)
        let mut pending: Vec<NodeId> = order
            .iter()
            .filter(|node_id| {
                dag.nodes
                    .get(node_id)
                    .map(|node| node.dependencies.is_empty())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        while !pending.is_empty() {
            if cancel.is_cancelled() {
                return Err("Execution cancelled".to_string());
            }

            // Spawn all currently-ready nodes
            let mut handles = Vec::new();
            for node_id in &pending {
                let node = dag
                    .nodes
                    .get(node_id)
                    .ok_or_else(|| "DAG internal error: node missing".to_string())?
                    .clone();
                let cancel_clone = cancel.clone();
                let max_retries = self.max_retries;
                let handle = tokio::spawn(async move {
                    Self::execute_node_with_retries(&node, cancel_clone, max_retries).await
                });
                handles.push((node_id.clone(), handle));
            }
            pending.clear();

            // Await all spawned tasks before enqueuing dependents
            for (node_id, handle) in handles {
                if cancel.is_cancelled() {
                    return Err("Execution cancelled".to_string());
                }
                match handle.await {
                    Ok(Ok(result)) => {
                        results.insert(node_id.clone(), result);
                        completed.insert(node_id.clone());

                        // Enqueue dependents whose dependencies are now all satisfied
                        if let Some(deps) = dependents.get(&node_id) {
                            for dependent_node_id in deps {
                                if let Some(remaining) = remaining_deps.get_mut(dependent_node_id) {
                                    remaining.remove(&node_id);
                                    if remaining.is_empty()
                                        && !completed.contains(dependent_node_id)
                                    {
                                        pending.push(dependent_node_id.clone());
                                    }
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(_) => {
                        return Err("Task panicked".to_string());
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute a single node with retries and cancellation.
    async fn execute_node_with_retries(
        node: &Node,
        cancel: CancellationToken,
        max_retries: u32,
    ) -> Result<ExecutionResult, String> {
        let mut retries = 0;
        loop {
            if cancel.is_cancelled() {
                return Err("Cancelled".to_string());
            }
            match Self::execute_node_once(node).await {
                Ok(result) => return Ok(result),
                Err(_error) if retries < max_retries => {
                    retries += 1;
                    let backoff = Duration::from_millis(100 * (1 << retries)); // exponential backoff
                    sleep(backoff).await;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Single execution attempt for a node.
    async fn execute_node_once(node: &Node) -> Result<ExecutionResult, String> {
        match &node.step_type {
            StepType::Inline => {
                // Simulate work
                Ok(ExecutionResult {
                    outputs: HashMap::from([("output".to_string(), "done".to_string())]),
                    error: None,
                })
            }
            StepType::K8sOffload { image, command } => submit_k8s_offload(image, command).await,
        }
    }
}

async fn submit_k8s_offload(image: &str, command: &[String]) -> Result<ExecutionResult, String> {
    let client = kube::Client::try_default()
        .await
        .map_err(|e| format!("Failed to create Kubernetes client: {}", e))?;

    let namespace =
        std::env::var("KAIGENTS_OFFLOAD_NAMESPACE").unwrap_or_else(|_| "default".to_string());

    let pods: Api<k8s_core::Pod> = Api::namespaced(client.clone(), &namespace);

    let pod_name = format!("kaigents-offload-{}", uuid::Uuid::new_v4());

    let pod = k8s_core::Pod {
        metadata: kube::api::ObjectMeta {
            name: Some(pod_name.clone()),
            labels: Some(std::collections::BTreeMap::from([(
                "app".to_string(),
                "kaigents-offload".to_string(),
            )])),
            ..Default::default()
        },
        spec: Some(k8s_core::PodSpec {
            restart_policy: Some("Never".to_string()),
            containers: vec![k8s_core::Container {
                name: "offload".to_string(),
                image: Some(image.to_string()),
                command: Some(command.to_vec()),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    pods.create(&PostParams::default(), &pod)
        .await
        .map_err(|e| format!("Failed to create offload pod: {}", e))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = pods
                .delete(&pod_name, &kube::api::DeleteParams::default())
                .await;
            return Err(format!("Offload pod '{}' timed out after 600s", pod_name));
        }

        let pod_obj = pods
            .get(&pod_name)
            .await
            .map_err(|e| format!("Failed to get pod status: {}", e))?;

        if let Some(status) = &pod_obj.status {
            if let Some(phase) = &status.phase {
                match phase.as_str() {
                    "Succeeded" => {
                        let _ = pods
                            .delete(&pod_name, &kube::api::DeleteParams::default())
                            .await;
                        return Ok(ExecutionResult {
                            outputs: HashMap::from([
                                ("pod".to_string(), pod_name.clone()),
                                ("status".to_string(), "Succeeded".to_string()),
                                ("image".to_string(), image.to_string()),
                                ("command".to_string(), command.join(" ")),
                            ]),
                            error: None,
                        });
                    }
                    "Failed" => {
                        let _ = pods
                            .delete(&pod_name, &kube::api::DeleteParams::default())
                            .await;
                        let msg = status
                            .container_statuses
                            .as_ref()
                            .and_then(|cs| cs.first())
                            .and_then(|c| c.state.as_ref())
                            .and_then(|s| s.terminated.as_ref())
                            .map(|t| t.reason.clone().unwrap_or_default())
                            .unwrap_or_else(|| "Unknown failure".to_string());
                        return Ok(ExecutionResult {
                            outputs: HashMap::from([
                                ("pod".to_string(), pod_name.clone()),
                                ("status".to_string(), "Failed".to_string()),
                            ]),
                            error: Some(format!("Offload pod failed: {}", msg)),
                        });
                    }
                    _ => {}
                }
            }
        }

        sleep(Duration::from_secs(2)).await;
    }
}

impl Default for DAGExecutor {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_validate_no_cycle() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        let b = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![],
        });
        dag.add_node(Node {
            id: b.clone(),
            name: "B".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![a.clone()],
        });
        assert!(dag.validate().is_ok());
    }

    #[test]
    fn dag_validate_cycle() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        let b = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![b.clone()],
        });
        dag.add_node(Node {
            id: b.clone(),
            name: "B".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![a.clone()],
        });
        assert!(dag.validate().is_err());
    }

    #[tokio::test]
    async fn dag_executor_simple() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![],
        });
        let executor = DAGExecutor::new(3);
        let cancel = CancellationToken::new();
        let results = executor.execute(&dag, cancel).await.unwrap();
        assert!(results.contains_key(&a));
        assert!(results[&a].error.is_none());
    }

    #[tokio::test]
    async fn dag_executor_concurrency() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        let b = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![],
        });
        dag.add_node(Node {
            id: b.clone(),
            name: "B".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![],
        });
        let executor = DAGExecutor::new(3);
        let cancel = CancellationToken::new();
        let results = executor.execute(&dag, cancel).await.unwrap();
        assert!(results.contains_key(&a));
        assert!(results.contains_key(&b));
        assert!(results[&a].error.is_none());
        assert!(results[&b].error.is_none());
    }

    #[tokio::test]
    async fn dag_executor_cancellation() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::Inline,
            dependencies: vec![],
        });
        let executor = DAGExecutor::new(3);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = executor.execute(&dag, cancel).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Execution cancelled");
    }

    #[tokio::test]
    async fn dag_executor_k8s_offload_no_cluster() {
        let mut dag = DAG::new();
        let a = NodeId::new();
        dag.add_node(Node {
            id: a.clone(),
            name: "A".to_string(),
            step_type: StepType::K8sOffload {
                image: "ubuntu:latest".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
            },
            dependencies: vec![],
        });
        let executor = DAGExecutor::new(3);
        let cancel = CancellationToken::new();
        let result = executor.execute(&dag, cancel).await;
        match result {
            Ok(_) => {}
            Err(e) => {
                assert!(
                    e.contains("Failed to create Kubernetes client")
                        || e.contains("Failed to create offload pod")
                        || e.contains("timed out"),
                    "Unexpected error: {}",
                    e
                );
            }
        }
    }
}
