//! File: engine/crates/kaigents-core/src/timeline.rs
//! Purpose: Run timeline event model, storage, and query API.
//! Product/business importance: provides durable, queryable run timeline for observability and audit.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::dag::NodeId;
use crate::run_id::RunId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// EventId uniquely identifies a timeline event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Create an EventId from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// EventType defines categories of timeline events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    RunStarted,
    RunFinished,
    NodeStarted,
    NodeFinished,
    NodeFailed { error: String, retries: u32 },
    NodeCancelled,
    ToolInvoked { tool_name: String },
    ToolFinished,
    ToolFailed { error: String },
    ModelInvoked { endpoint: String },
    ModelFinished,
    ModelFailed { error: String },
    ArtifactProduced { artifact_id: String },
}

/// TimelineEvent represents a single event in the run timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: EventId,
    pub run_id: RunId,
    pub node_id: Option<NodeId>,
    pub event_type: EventType,
    pub timestamp_ms: u64,
    pub correlation_id: Option<String>,
    pub payload: HashMap<String, String>,
}

impl TimelineEvent {
    /// Create a new TimelineEvent with the current timestamp.
    pub fn new(run_id: RunId, event_type: EventType) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            id: EventId::new(),
            run_id,
            node_id: None,
            event_type,
            timestamp_ms,
            correlation_id: None,
            payload: HashMap::new(),
        }
    }

    /// Attach a node identifier.
    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Attach a correlation identifier.
    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Add a payload key-value.
    pub fn with_payload(mut self, key: String, value: String) -> Self {
        self.payload.insert(key, value);
        self
    }
}

/// In-memory timeline store (placeholder for RethinkDB integration).
#[derive(Debug, Default)]
pub struct TimelineStore {
    events: std::sync::Mutex<Vec<TimelineEvent>>,
}

impl TimelineStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event to the timeline.
    pub fn append(&self, event: TimelineEvent) -> Result<(), String> {
        let mut events = self.events.lock().map_err(|_| "Lock poisoned")?;
        events.push(event);
        Ok(())
    }

    /// Query events by run ID.
    pub fn query_by_run(&self, run_id: &RunId) -> Result<Vec<TimelineEvent>, String> {
        let events = self.events.lock().map_err(|_| "Lock poisoned")?;
        Ok(events
            .iter()
            .filter(|e| &e.run_id == run_id)
            .cloned()
            .collect())
    }

    /// Query events by event type.
    pub fn query_by_type(&self, event_type: &EventType) -> Result<Vec<TimelineEvent>, String> {
        let events = self.events.lock().map_err(|_| "Lock poisoned")?;
        Ok(events
            .iter()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(event_type))
            .cloned()
            .collect())
    }

    /// Query events by correlation ID.
    pub fn query_by_correlation(&self, correlation_id: &str) -> Result<Vec<TimelineEvent>, String> {
        let events = self.events.lock().map_err(|_| "Lock poisoned")?;
        Ok(events
            .iter()
            .filter(|e| e.correlation_id.as_deref() == Some(correlation_id))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_event_creation() {
        let run_id = RunId::new();
        let event = TimelineEvent::new(run_id.clone(), EventType::RunStarted)
            .with_correlation("corr-123".to_string())
            .with_payload("key".to_string(), "value".to_string());
        assert_eq!(event.run_id, run_id);
        assert!(matches!(event.event_type, EventType::RunStarted));
        assert_eq!(event.correlation_id, Some("corr-123".to_string()));
        assert_eq!(event.payload.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn timeline_store_append_and_query() {
        let store = TimelineStore::new();
        let run_id = RunId::new();
        let event = TimelineEvent::new(run_id.clone(), EventType::RunStarted);
        store.append(event.clone()).unwrap();
        let by_run = store.query_by_run(&run_id).unwrap();
        assert_eq!(by_run.len(), 1);
        assert_eq!(by_run[0].id, event.id);
        let by_type = store.query_by_type(&EventType::RunStarted).unwrap();
        assert_eq!(by_type.len(), 1);
        assert_eq!(by_type[0].id, event.id);
        let by_corr = store.query_by_correlation("none").unwrap();
        assert!(by_corr.is_empty());
    }
}
