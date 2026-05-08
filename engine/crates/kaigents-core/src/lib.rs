//! File: engine/crates/kaigents-core/src/lib.rs
//! Purpose: Core domain primitives for the Kaigents execution engine.
//! Product/business importance: defines stable run identifiers, DAG execution model, run timeline events, MCP tool plane integration, model serving, and artifacts used across the platform.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

pub mod artifacts;
pub mod dag;
pub mod file_backed;
pub mod model_serving;
#[cfg(feature = "rethinkdb")]
pub mod rethinkdb_store;
pub mod run_id;
pub mod temporal_adapter;
pub mod timeline;
pub mod tool_plane;

// Re-export key public types for convenience
pub use artifacts::{
    Artifact, ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactPlane, ArtifactStorageRef,
    ArtifactStore, InMemoryArtifactStore,
};
pub use dag::{CancellationToken, DAGExecutor, ExecutionResult, Node, NodeId, StepType, DAG};
pub use file_backed::{
    artifacts_root_dir, default_state_dir, parse_uuid, timeline_events_path, FileArtifactStore,
    FileTimelineStore, FileToolContractStore,
};
pub use model_serving::{
    ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Embedding,
    EmbeddingsRequest, EmbeddingsResponse, HttpOpenAIModelClient, InMemoryModelClient,
    ModelCapabilities, ModelClient, ModelEndpoint, ModelPlane, Usage,
};
pub use run_id::RunId;
pub use temporal_adapter::{
    StartWorkRequestRequest, TemporalAdapterClient, WorkItemDef as TemporalWorkItemDef,
    WorkRequestState as TemporalWorkRequestState,
};
pub use timeline::{EventId, EventType, TimelineEvent, TimelineStore};
pub use tool_plane::{
    HttpMcpClient, InMemoryMCPClient, InMemoryToolContractSink, MCPClient, TimelineSink,
    ToolContract, ToolContractSink, ToolPlane,
};

#[cfg(feature = "rethinkdb")]
pub use rethinkdb_store::{RethinkDbArtifactStore, RethinkDbConfig, RethinkDbTimelineStore};
