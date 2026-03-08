//! File: engine/crates/kaigents-core/src/artifacts.rs
//! Purpose: First-class artifact concept with storage integration, signed/proxy URLs, large-object support, and timeline events.
//! Product/business importance: enables durable, shareable run outputs without exposing storage credentials.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::run_id::RunId;
use crate::timeline::{EventType, TimelineEvent, TimelineStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

/// ArtifactId uniquely identifies an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying Uuid.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Create an ArtifactId from a Uuid.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

/// ArtifactKind distinguishes artifact types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Input,
    Intermediate,
    Output,
}

/// Artifact represents a first-class artifact associated with a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub run_id: RunId,
    pub name: String,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub size_bytes: Option<u64>,
    pub checksum_sha256: Option<String>,
    pub storage_ref: ArtifactStorageRef,
    pub created_at_ms: u64,
    pub metadata: HashMap<String, String>,
}

/// ArtifactStorageRef points to where the artifact is stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactStorageRef {
    /// In-memory artifact (for testing or very small artifacts).
    InMemory(Vec<u8>),
    /// Object store reference with bucket and key.
    ObjectStore { bucket: String, key: String },
    /// File system path (for local dev).
    FileSystem { path: String },
}

/// ArtifactStore defines the minimal interface for artifact storage.
#[async_trait::async_trait]
pub trait ArtifactStore: Send + Sync {
    /// Store an artifact and return a stable reference.
    async fn store(&self, artifact: &Artifact, data: Vec<u8>)
        -> Result<ArtifactStorageRef, String>;
    /// Retrieve an artifact by reference.
    async fn retrieve(&self, storage_ref: &ArtifactStorageRef) -> Result<Vec<u8>, String>;
    /// Generate a signed/proxy URL for artifact access.
    async fn generate_url(
        &self,
        storage_ref: &ArtifactStorageRef,
        expires_in: Duration,
    ) -> Result<String, String>;
    /// Get metadata for an artifact reference.
    async fn metadata(&self, storage_ref: &ArtifactStorageRef) -> Result<ArtifactMetadata, String>;
}

/// ArtifactMetadata contains information about a stored artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub size_bytes: u64,
    pub content_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<u64>,
}

/// InMemoryArtifactStore is a placeholder for testing and MVP.
pub struct InMemoryArtifactStore {
    artifacts: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    next_url_id: Arc<Mutex<u64>>,
}

impl InMemoryArtifactStore {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            next_url_id: Arc::new(Mutex::new(1)),
        }
    }
}

impl Default for InMemoryArtifactStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ArtifactStore for InMemoryArtifactStore {
    async fn store(
        &self,
        artifact: &Artifact,
        data: Vec<u8>,
    ) -> Result<ArtifactStorageRef, String> {
        let key = format!("artifact-{}", artifact.id.as_uuid());
        let mut artifacts = self.artifacts.lock().await;
        artifacts.insert(key.clone(), data);
        Ok(ArtifactStorageRef::ObjectStore {
            bucket: "kaigents-artifacts".to_string(),
            key,
        })
    }

    async fn retrieve(&self, storage_ref: &ArtifactStorageRef) -> Result<Vec<u8>, String> {
        match storage_ref {
            ArtifactStorageRef::InMemory(data) => Ok(data.clone()),
            ArtifactStorageRef::ObjectStore { bucket: _, key } => {
                let artifacts = self.artifacts.lock().await;
                // Directly use the key as stored (no bucket prefix)
                artifacts
                    .get(key.as_str())
                    .cloned()
                    .ok_or_else(|| format!("Artifact not found: {}", key))
            }
            ArtifactStorageRef::FileSystem { path } => {
                let artifacts = self.artifacts.lock().await;
                artifacts
                    .get(path)
                    .cloned()
                    .ok_or_else(|| format!("Artifact not found: {}", path))
            }
        }
    }

    async fn generate_url(
        &self,
        _storage_ref: &ArtifactStorageRef,
        _expires_in: Duration,
    ) -> Result<String, String> {
        let mut id = self.next_url_id.lock().await;
        let url = format!("http://kaigents.local/artifacts/{}", *id);
        *id += 1;
        // In a real implementation, we would sign the URL with a secret and embed expiration
        Ok(url)
    }

    async fn metadata(&self, storage_ref: &ArtifactStorageRef) -> Result<ArtifactMetadata, String> {
        let data = self.retrieve(storage_ref).await?;
        Ok(ArtifactMetadata {
            size_bytes: data.len() as u64,
            content_type: "application/octet-stream".to_string(),
            etag: None,
            last_modified: None,
        })
    }
}

/// ArtifactPlane manages artifact storage, URLs, and timeline events.
pub struct ArtifactPlane {
    store: Box<dyn ArtifactStore>,
    timeline: Arc<TimelineStore>,
}

impl ArtifactPlane {
    pub fn new(store: Box<dyn ArtifactStore>, timeline: Arc<TimelineStore>) -> Self {
        Self { store, timeline }
    }

    /// Store an artifact and emit timeline events.
    pub async fn store_artifact(
        &self,
        run_id: RunId,
        name: String,
        kind: ArtifactKind,
        mime_type: String,
        data: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Artifact, String> {
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let artifact_id = ArtifactId::new();
        let checksum_sha256 = format!("{:x}", Sha256::digest(&data));
        let storage_ref = self
            .store
            .store(
                &Artifact {
                    id: artifact_id.clone(),
                    run_id: run_id.clone(),
                    name: name.clone(),
                    kind: kind.clone(),
                    mime_type: mime_type.clone(),
                    size_bytes: Some(data.len() as u64),
                    checksum_sha256: Some(checksum_sha256.clone()),
                    storage_ref: ArtifactStorageRef::InMemory(vec![]), // placeholder
                    created_at_ms,
                    metadata: metadata.clone(),
                },
                data.clone(),
            )
            .await?;

        let artifact = Artifact {
            id: artifact_id.clone(),
            run_id: run_id.clone(),
            name,
            kind,
            mime_type,
            size_bytes: Some(data.len() as u64),
            checksum_sha256: Some(checksum_sha256.clone()),
            storage_ref: storage_ref.clone(),
            created_at_ms,
            metadata,
        };

        // Emit ArtifactProduced event
        let correlation_id = format!("artifact-{}", artifact_id.as_uuid());
        let event = TimelineEvent::new(
            run_id,
            EventType::ArtifactProduced {
                artifact_id: artifact_id.as_uuid().to_string(),
            },
        )
        .with_correlation(correlation_id)
        .with_payload("name".to_string(), artifact.name.clone())
        .with_payload(
            "size_bytes".to_string(),
            artifact.size_bytes.unwrap_or(0).to_string(),
        )
        .with_payload("mime_type".to_string(), artifact.mime_type.clone());
        self.timeline
            .append(event)
            .map_err(|e| format!("Timeline error: {}", e))?;

        Ok(artifact)
    }

    /// Retrieve an artifact.
    pub async fn retrieve_artifact(&self, _artifact_id: &ArtifactId) -> Result<Vec<u8>, String> {
        // In a real implementation, we would look up the artifact by ID from a persistent store
        // For MVP, we assume the storage_ref is embedded in the artifact (simpl detail)
        // This is a simplified lookup; real implementation would query a database/index
        Err("Artifact lookup by ID not implemented in MVP".to_string())
    }

    /// Generate a signed/proxy URL for artifact access.
    pub async fn generate_artifact_url(
        &self,
        artifact: &Artifact,
        expires_in: Duration,
    ) -> Result<String, String> {
        self.store
            .generate_url(&artifact.storage_ref, expires_in)
            .await
    }

    /// Get metadata for an artifact.
    pub async fn artifact_metadata(&self, artifact: &Artifact) -> Result<ArtifactMetadata, String> {
        self.store.metadata(&artifact.storage_ref).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::TimelineStore;

    #[tokio::test]
    async fn artifact_plane_store_and_url() {
        let timeline = Arc::new(TimelineStore::new());
        let store = InMemoryArtifactStore::new();
        let artifact_plane = ArtifactPlane::new(Box::new(store), timeline.clone());

        let run_id = RunId::new();
        let data = b"Hello, artifact world!".to_vec();
        let artifact = artifact_plane
            .store_artifact(
                run_id.clone(),
                "greeting.txt".to_string(),
                ArtifactKind::Output,
                "text/plain".to_string(),
                data.clone(),
                HashMap::from([("source".to_string(), "test".to_string())]),
            )
            .await
            .unwrap();

        assert_eq!(artifact.name, "greeting.txt");
        assert_eq!(artifact.kind, ArtifactKind::Output);
        assert_eq!(artifact.size_bytes, Some(22));
        assert_eq!(artifact.mime_type, "text/plain");
        assert_eq!(artifact.metadata.get("source"), Some(&"test".to_string()));

        // Verify timeline event
        let events = timeline.query_by_run(&run_id).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            EventType::ArtifactProduced { .. }
        ));
        assert_eq!(
            events[0].payload.get("name"),
            Some(&"greeting.txt".to_string())
        );

        // Generate URL
        let url = artifact_plane
            .generate_artifact_url(&artifact, Duration::from_secs(3600))
            .await
            .unwrap();
        assert!(url.starts_with("http://kaigents.local/artifacts/"));
    }

    #[tokio::test]
    async fn artifact_plane_metadata() {
        let timeline = Arc::new(TimelineStore::new());
        let store = InMemoryArtifactStore::new();
        let artifact_plane = ArtifactPlane::new(Box::new(store), timeline.clone());

        let run_id = RunId::new();
        let data = vec![0u8; 1024];
        let artifact = artifact_plane
            .store_artifact(
                run_id,
                "binary.bin".to_string(),
                ArtifactKind::Intermediate,
                "application/octet-stream".to_string(),
                data,
                HashMap::new(),
            )
            .await
            .unwrap();

        let metadata = artifact_plane.artifact_metadata(&artifact).await.unwrap();
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.content_type, "application/octet-stream");
    }
}
