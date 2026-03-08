//! File: engine/crates/kaigents-core/src/file_backed.rs
//! Purpose: File-backed persistence for timeline events and artifacts (dev/test fallback).
//! Product/business importance: enables durable, queryable runs and artifact retrieval without external services.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::artifacts::{Artifact, ArtifactId, ArtifactKind};
use crate::run_id::RunId;
use crate::timeline::{EventType, TimelineEvent};
use crate::tool_plane::ToolContract;
use crate::tool_plane::{TimelineSink, ToolContractSink};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// FileTimelineStore stores timeline events as newline-delimited JSON.
#[derive(Debug, Clone)]
pub struct FileTimelineStore {
    events_path: PathBuf,
}

impl FileTimelineStore {
    pub fn new(events_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = events_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        Ok(Self { events_path })
    }

    /// Append an event to the store.
    pub fn append(&self, event: TimelineEvent) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .map_err(|e| format!("Failed to open timeline file: {}", e))?;
        let line = serde_json::to_string(&event)
            .map_err(|e| format!("Failed to serialize timeline event: {}", e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Failed to write timeline event: {}", e))?;
        Ok(())
    }

    /// Query events by run ID.
    pub fn query_by_run(&self, run_id: &RunId) -> Result<Vec<TimelineEvent>, String> {
        let file = match File::open(&self.events_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(format!("Failed to open timeline file: {}", e)),
        };

        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Failed to read timeline event: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let event: TimelineEvent = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to deserialize timeline event: {}", e))?;
            if &event.run_id == run_id {
                events.push(event);
            }
        }

        events.sort_by_key(|e| e.timestamp_ms);
        Ok(events)
    }
}

impl TimelineSink for FileTimelineStore {
    fn append(&self, event: TimelineEvent) -> Result<(), String> {
        FileTimelineStore::append(self, event)
    }
}

#[derive(Debug, Clone)]
pub struct FileToolContractStore {
    contracts_path: PathBuf,
}

impl FileToolContractStore {
    pub fn new(contracts_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = contracts_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        Ok(Self { contracts_path })
    }

    pub fn store_contract(&self, contract: &ToolContract) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.contracts_path)
            .map_err(|e| format!("Failed to open tool contract file: {}", e))?;
        let line = serde_json::to_string(contract)
            .map_err(|e| format!("Failed to serialize tool contract: {}", e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Failed to write tool contract: {}", e))?;
        Ok(())
    }

    pub fn list_contracts(&self) -> Result<Vec<ToolContract>, String> {
        let file = match File::open(&self.contracts_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(format!("Failed to open tool contract file: {}", e)),
        };

        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Failed to read tool contract: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let contract: ToolContract = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to deserialize tool contract: {}", e))?;
            out.push(contract);
        }
        Ok(out)
    }
}

impl ToolContractSink for FileToolContractStore {
    fn store_contract(&self, contract: &ToolContract) -> Result<(), String> {
        FileToolContractStore::store_contract(self, contract)
    }
}

/// StoredArtifactRecord is the persisted index record for an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredArtifactRecord {
    pub id: ArtifactId,
    pub run_id: RunId,
    pub name: String,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    pub checksum_sha256: String,
    pub created_at_ms: u64,
    pub metadata: HashMap<String, String>,
    pub blob_path: String,
}

/// FileArtifactStore stores artifact bytes on disk and indexes metadata as newline-delimited JSON.
#[derive(Debug, Clone)]
pub struct FileArtifactStore {
    blobs_dir: PathBuf,
    index_path: PathBuf,
}

impl FileArtifactStore {
    pub fn new(root_dir: PathBuf) -> Result<Self, String> {
        let blobs_dir = root_dir.join("blobs");
        fs::create_dir_all(&blobs_dir).map_err(|e| format!("Failed to create blobs dir: {}", e))?;

        let index_path = root_dir.join("index.jsonl");
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        Ok(Self {
            blobs_dir,
            index_path,
        })
    }

    pub fn store_bytes(
        &self,
        run_id: RunId,
        name: String,
        kind: ArtifactKind,
        mime_type: String,
        bytes: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<(Artifact, StoredArtifactRecord), String> {
        let id = ArtifactId::new();
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get time: {}", e))?
            .as_millis() as u64;
        let checksum_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let blob_filename = format!("{}.blob", id.as_uuid());
        let blob_path = self.blobs_dir.join(blob_filename);

        fs::write(&blob_path, &bytes)
            .map_err(|e| format!("Failed to write artifact bytes: {}", e))?;

        let artifact = Artifact {
            id: id.clone(),
            run_id: run_id.clone(),
            name: name.clone(),
            kind: kind.clone(),
            mime_type: mime_type.clone(),
            size_bytes: Some(bytes.len() as u64),
            checksum_sha256: Some(checksum_sha256.clone()),
            storage_ref: crate::artifacts::ArtifactStorageRef::FileSystem {
                path: blob_path.to_string_lossy().to_string(),
            },
            created_at_ms,
            metadata: metadata.clone(),
        };

        let record = StoredArtifactRecord {
            id,
            run_id,
            name,
            kind,
            mime_type,
            size_bytes: bytes.len() as u64,
            checksum_sha256,
            created_at_ms,
            metadata,
            blob_path: blob_path.to_string_lossy().to_string(),
        };

        self.append_record(&record)?;
        Ok((artifact, record))
    }

    pub fn retrieve_bytes(&self, artifact_id: &ArtifactId) -> Result<Vec<u8>, String> {
        let record = self
            .find_record(artifact_id)?
            .ok_or_else(|| format!("Artifact not found: {}", artifact_id.as_uuid()))?;
        fs::read(&record.blob_path).map_err(|e| format!("Failed to read artifact bytes: {}", e))
    }

    pub fn find_record(
        &self,
        artifact_id: &ArtifactId,
    ) -> Result<Option<StoredArtifactRecord>, String> {
        let file = match File::open(&self.index_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("Failed to open artifact index: {}", e)),
        };

        let reader = BufReader::new(file);
        let mut last_match: Option<StoredArtifactRecord> = None;
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Failed to read artifact index: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let record: StoredArtifactRecord = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to deserialize artifact index record: {}", e))?;
            if &record.id == artifact_id {
                last_match = Some(record);
            }
        }

        Ok(last_match)
    }

    fn append_record(&self, record: &StoredArtifactRecord) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.index_path)
            .map_err(|e| format!("Failed to open artifact index: {}", e))?;
        let line = serde_json::to_string(record)
            .map_err(|e| format!("Failed to serialize artifact index record: {}", e))?;
        writeln!(file, "{}", line)
            .map_err(|e| format!("Failed to write artifact index record: {}", e))?;
        Ok(())
    }
}

/// Helper for building default state directories.
pub fn default_state_dir() -> PathBuf {
    let base = std::env::var("KAIGENTS_STATE_DIR").unwrap_or_else(|_| ".kaigents".to_string());
    Path::new(&base).to_path_buf()
}

/// Helper to build a stable events path under a state dir.
pub fn timeline_events_path(state_dir: &Path) -> PathBuf {
    state_dir.join("timeline").join("events.jsonl")
}

/// Helper to build a stable artifact store dir under a state dir.
pub fn artifacts_root_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("artifacts")
}

/// Helper to create a minimal artifact-produced timeline event.
pub fn artifact_produced_event(run_id: RunId, artifact_id: &ArtifactId) -> TimelineEvent {
    TimelineEvent::new(
        run_id,
        EventType::ArtifactProduced {
            artifact_id: artifact_id.as_uuid().to_string(),
        },
    )
}

/// Helper to create a run-finished event.
pub fn run_finished_event(run_id: RunId) -> TimelineEvent {
    TimelineEvent::new(run_id, EventType::RunFinished)
}

/// Helper to create a run-started event.
pub fn run_started_event(run_id: RunId) -> TimelineEvent {
    TimelineEvent::new(run_id, EventType::RunStarted)
}

/// Helper to parse a UUID string.
pub fn parse_uuid(value: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("Invalid UUID: {}", value))
}
