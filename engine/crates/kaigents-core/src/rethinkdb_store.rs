//! File: engine/crates/kaigents-core/src/rethinkdb_store.rs
//! Purpose: RethinkDB-backed persistence for timeline events and artifact index (optional backend).
//! Product/business importance: enables durable, queryable run timelines and artifact lookup in production deployments.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use crate::artifacts::{ArtifactId, ArtifactKind};
use crate::file_backed::{FileArtifactStore, StoredArtifactRecord};
use crate::run_id::RunId;
use crate::timeline::{EventType, TimelineEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use unreql::{cmd::connect::Options, cmd::options::BetweenOptions, r, Session};

const DEFAULT_DATABASE_NAME: &str = "kaigents";
const DEFAULT_TIMELINE_TABLE_NAME: &str = "timeline_events";

/// RethinkDbConfig controls how the RethinkDB backend connects and where it stores data.
#[derive(Debug, Clone)]
pub struct RethinkDbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl Default for RethinkDbConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 28015,
            database: DEFAULT_DATABASE_NAME.to_string(),
            user: "admin".to_string(),
            password: "".to_string(),
        }
    }
}

impl RethinkDbConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("KAIGENTS_RETHINKDB_HOST") {
            if !v.is_empty() {
                cfg.host = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_RETHINKDB_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                cfg.port = port;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_RETHINKDB_DB") {
            if !v.is_empty() {
                cfg.database = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_RETHINKDB_USER") {
            if !v.is_empty() {
                cfg.user = v;
            }
        }
        if let Ok(v) = std::env::var("KAIGENTS_RETHINKDB_PASSWORD") {
            cfg.password = v;
        }
        cfg
    }

    pub fn to_unreql_options(&self) -> Options {
        Options::new()
            .host(self.host.clone())
            .port(self.port)
            .db(self.database.clone())
            .user(self.user.clone())
            .password(self.password.clone())
    }
}

/// RethinkDbTimelineStore persists timeline events in RethinkDB.
#[derive(Debug, Clone)]
pub struct RethinkDbTimelineStore {
    pub database: String,
    pub table: String,
}

impl Default for RethinkDbTimelineStore {
    fn default() -> Self {
        Self {
            database: DEFAULT_DATABASE_NAME.to_string(),
            table: DEFAULT_TIMELINE_TABLE_NAME.to_string(),
        }
    }
}

impl RethinkDbTimelineStore {
    pub async fn connect_session(cfg: &RethinkDbConfig) -> Result<Session, String> {
        r.connect(cfg.to_unreql_options())
            .await
            .map_err(|e| format!("RethinkDB connect failed: {e}"))
    }

    pub async fn ensure_schema(&self, session: &mut Session) -> Result<(), String> {
        ensure_database(session, &self.database).await?;
        ensure_table(session, &self.database, &self.table).await?;

        // Secondary indexes
        ensure_index(session, &self.database, &self.table, "run_id").await?;
        ensure_index(session, &self.database, &self.table, "correlation_id").await?;
        ensure_index(session, &self.database, &self.table, "event_type").await?;

        // Compound index for efficient ordered timeline queries.
        // index function: [run_id, timestamp_ms]
        ensure_compound_index_run_id_timestamp(
            session,
            &self.database,
            &self.table,
            "run_id_timestamp",
        )
        .await?;

        Ok(())
    }

    pub async fn append(&self, session: &mut Session, event: &TimelineEvent) -> Result<(), String> {
        let doc = timeline_event_to_document(event)?;
        r.db(self.database.clone())
            .table(self.table.clone())
            .insert(doc)
            .exec::<_, serde_json::Value>(&mut *session)
            .await
            .map_err(|e| format!("RethinkDB insert timeline event failed: {e}"))?;
        Ok(())
    }

    pub async fn query_by_run(
        &self,
        session: &mut Session,
        run_id: &RunId,
    ) -> Result<Vec<TimelineEvent>, String> {
        let run_id = run_id.as_uuid().to_string();

        let start = vec![
            serde_json::Value::String(run_id.clone()),
            serde_json::Value::Number(0.into()),
        ];
        let end = vec![
            serde_json::Value::String(run_id),
            serde_json::Value::Number(serde_json::Number::from(u64::MAX)),
        ];

        let options = BetweenOptions::new().index("run_id_timestamp".to_string());

        let docs: Vec<serde_json::Value> = r
            .db(self.database.clone())
            .table(self.table.clone())
            .between(start, end, options)
            .exec_to_vec(&mut *session)
            .await
            .map_err(|e| format!("RethinkDB query timeline by run failed: {e}"))?;

        let mut events = Vec::new();
        for doc in docs {
            events.push(document_to_timeline_event(doc)?);
        }

        events.sort_by_key(|e| e.timestamp_ms);
        Ok(events)
    }
}

/// RethinkDbArtifactStore persists an artifact index record in RethinkDB while storing bytes on disk.
#[derive(Debug, Clone)]
pub struct RethinkDbArtifactStore {
    pub database: String,
    pub table: String,
    pub file_store: FileArtifactStore,
}

impl RethinkDbArtifactStore {
    pub fn new(
        database: String,
        table: String,
        artifact_root_dir: PathBuf,
    ) -> Result<Self, String> {
        Ok(Self {
            database,
            table,
            file_store: FileArtifactStore::new(artifact_root_dir)?,
        })
    }

    pub async fn ensure_schema(&self, session: &mut Session) -> Result<(), String> {
        ensure_database(session, &self.database).await?;
        ensure_table(session, &self.database, &self.table).await?;
        ensure_index(session, &self.database, &self.table, "run_id").await?;
        ensure_index(session, &self.database, &self.table, "name").await?;
        ensure_compound_index_run_id_created_at(
            session,
            &self.database,
            &self.table,
            "run_id_created_at",
        )
        .await?;
        Ok(())
    }

    pub fn store_bytes(
        &self,
        run_id: RunId,
        name: String,
        kind: ArtifactKind,
        mime_type: String,
        bytes: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<(crate::artifacts::Artifact, StoredArtifactRecord), String> {
        self.file_store
            .store_bytes(run_id, name, kind, mime_type, bytes, metadata)
    }

    pub async fn upsert_index_record(
        &self,
        session: &mut Session,
        record: &StoredArtifactRecord,
    ) -> Result<(), String> {
        let doc = stored_artifact_record_to_document(record)?;
        r.db(self.database.clone())
            .table(self.table.clone())
            .insert(doc)
            .exec::<_, serde_json::Value>(&mut *session)
            .await
            .map_err(|e| format!("RethinkDB insert artifact record failed: {e}"))?;
        Ok(())
    }

    pub fn retrieve_bytes(&self, artifact_id: &ArtifactId) -> Result<Vec<u8>, String> {
        self.file_store.retrieve_bytes(artifact_id)
    }

    pub async fn find_record(
        &self,
        session: &mut Session,
        artifact_id: &ArtifactId,
    ) -> Result<Option<StoredArtifactRecord>, String> {
        let id = artifact_id.as_uuid().to_string();
        let doc: Option<serde_json::Value> = r
            .db(self.database.clone())
            .table(self.table.clone())
            .get(id)
            .exec(&mut *session)
            .await
            .map_err(|e| format!("RethinkDB get artifact record failed: {e}"))?;

        match doc {
            None => Ok(None),
            Some(v) => Ok(Some(document_to_stored_artifact_record(v)?)),
        }
    }

    pub async fn list_by_run(
        &self,
        session: &mut Session,
        run_id: &RunId,
    ) -> Result<Vec<StoredArtifactRecord>, String> {
        let run_id = run_id.as_uuid().to_string();
        let options = r.with_opt(r.args([run_id]), r.index("run_id"));

        let docs: Vec<serde_json::Value> = r
            .db(self.database.clone())
            .table(self.table.clone())
            .get_all(options)
            .exec_to_vec(&mut *session)
            .await
            .map_err(|e| format!("RethinkDB list artifacts by run failed: {e}"))?;

        let mut out = Vec::new();
        for doc in docs {
            out.push(document_to_stored_artifact_record(doc)?);
        }

        out.sort_by_key(|record| record.created_at_ms);
        Ok(out)
    }
}

async fn ensure_database(session: &mut Session, db: &str) -> Result<(), String> {
    let db = db.to_string();
    let result = r
        .db_create(db)
        .exec::<_, serde_json::Value>(&mut *session)
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") {
                Ok(())
            } else {
                Err(format!("RethinkDB db_create failed: {msg}"))
            }
        }
    }
}

async fn ensure_table(session: &mut Session, db: &str, table: &str) -> Result<(), String> {
    let db = db.to_string();
    let table = table.to_string();
    let result = r
        .db(db)
        .table_create(table)
        .exec::<_, serde_json::Value>(&mut *session)
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") {
                Ok(())
            } else {
                Err(format!("RethinkDB table_create failed: {msg}"))
            }
        }
    }
}

async fn ensure_index(
    session: &mut Session,
    db: &str,
    table: &str,
    index_name: &str,
) -> Result<(), String> {
    let db = db.to_string();
    let table = table.to_string();
    let index_name = index_name.to_string();
    let result = r
        .db(db.clone())
        .table(table.clone())
        .index_create(index_name.clone())
        .exec::<_, serde_json::Value>(&mut *session)
        .await;

    match result {
        Ok(_) => {
            let _ = r
                .db(db)
                .table(table)
                .index_wait(index_name)
                .exec::<_, serde_json::Value>(&mut *session)
                .await;
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") {
                Ok(())
            } else {
                Err(format!("RethinkDB index_create failed: {msg}"))
            }
        }
    }
}

async fn ensure_compound_index_run_id_timestamp(
    session: &mut Session,
    db: &str,
    table: &str,
    index_name: &str,
) -> Result<(), String> {
    let db = db.to_string();
    let table = table.to_string();
    let index_name = index_name.to_string();
    let spec = r.args((
        index_name.clone(),
        unreql::func!(|row| { [row.clone().g("run_id"), row.g("timestamp_ms")] }),
    ));

    let result = r
        .db(db.clone())
        .table(table.clone())
        .index_create(spec)
        .exec::<_, serde_json::Value>(&mut *session)
        .await;

    match result {
        Ok(_) => {
            let _ = r
                .db(db)
                .table(table)
                .index_wait(index_name)
                .exec::<_, serde_json::Value>(&mut *session)
                .await;
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") {
                Ok(())
            } else {
                Err(format!("RethinkDB compound index_create failed: {msg}"))
            }
        }
    }
}

async fn ensure_compound_index_run_id_created_at(
    session: &mut Session,
    db: &str,
    table: &str,
    index_name: &str,
) -> Result<(), String> {
    let db = db.to_string();
    let table = table.to_string();
    let index_name = index_name.to_string();
    let spec = r.args((
        index_name.clone(),
        unreql::func!(|row| { [row.clone().g("run_id"), row.g("created_at_ms")] }),
    ));

    let result = r
        .db(db.clone())
        .table(table.clone())
        .index_create(spec)
        .exec::<_, serde_json::Value>(&mut *session)
        .await;

    match result {
        Ok(_) => {
            let _ = r
                .db(db)
                .table(table)
                .index_wait(index_name)
                .exec::<_, serde_json::Value>(&mut *session)
                .await;
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") {
                Ok(())
            } else {
                Err(format!("RethinkDB compound index_create failed: {msg}"))
            }
        }
    }
}

fn timeline_event_to_document(event: &TimelineEvent) -> Result<serde_json::Value, String> {
    let mut payload = serde_json::Map::new();

    for (k, v) in &event.payload {
        payload.insert(k.clone(), serde_json::Value::String(v.clone()));
    }

    match &event.event_type {
        EventType::ToolInvoked { tool_name } => {
            payload.insert(
                "tool_name".to_string(),
                serde_json::Value::String(tool_name.clone()),
            );
        }
        EventType::ToolFailed { error } => {
            payload.insert(
                "error".to_string(),
                serde_json::Value::String(error.clone()),
            );
        }
        EventType::ModelInvoked { endpoint } => {
            payload.insert(
                "endpoint".to_string(),
                serde_json::Value::String(endpoint.clone()),
            );
        }
        EventType::ModelFailed { error } => {
            payload.insert(
                "error".to_string(),
                serde_json::Value::String(error.clone()),
            );
        }
        EventType::NodeFailed { error, retries } => {
            payload.insert(
                "error".to_string(),
                serde_json::Value::String(error.clone()),
            );
            payload.insert(
                "retries".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*retries)),
            );
        }
        EventType::ArtifactProduced { artifact_id } => {
            payload.insert(
                "artifact_id".to_string(),
                serde_json::Value::String(artifact_id.clone()),
            );
        }
        _ => {}
    }

    Ok(serde_json::json!({
        "id": event.id.as_uuid().to_string(),
        "run_id": event.run_id.as_uuid().to_string(),
        "node_id": event.node_id.as_ref().map(|_| serde_json::Value::Null).unwrap_or(serde_json::Value::Null),
        "event_type": event_type_name(&event.event_type),
        "timestamp_ms": event.timestamp_ms,
        "correlation_id": event.correlation_id,
        "payload": payload,
    }))
}

fn document_to_timeline_event(doc: serde_json::Value) -> Result<TimelineEvent, String> {
    let id = doc
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "TimelineEvent missing id".to_string())?;
    let run_id = doc
        .get("run_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "TimelineEvent missing run_id".to_string())?;

    let timestamp_ms = doc
        .get("timestamp_ms")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "TimelineEvent missing timestamp_ms".to_string())?;

    let correlation_id = doc
        .get("correlation_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let payload_obj = doc
        .get("payload")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut payload = HashMap::new();
    for (k, v) in payload_obj {
        if let Some(s) = v.as_str() {
            payload.insert(k, s.to_string());
        }
    }

    let event_type_str = doc
        .get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "TimelineEvent missing event_type".to_string())?;

    let event_type = event_type_from_name(event_type_str, &doc)?;

    Ok(TimelineEvent {
        id: crate::timeline::EventId::from_uuid(crate::file_backed::parse_uuid(id)?),
        run_id: RunId::from_uuid(crate::file_backed::parse_uuid(run_id)?),
        node_id: None,
        event_type,
        timestamp_ms,
        correlation_id,
        payload,
    })
}

fn event_type_name(event_type: &EventType) -> String {
    match event_type {
        EventType::RunStarted => "RunStarted".to_string(),
        EventType::RunFinished => "RunFinished".to_string(),
        EventType::NodeStarted => "NodeStarted".to_string(),
        EventType::NodeFinished => "NodeFinished".to_string(),
        EventType::NodeFailed { .. } => "NodeFailed".to_string(),
        EventType::NodeCancelled => "NodeCancelled".to_string(),
        EventType::ToolInvoked { .. } => "ToolInvoked".to_string(),
        EventType::ToolFinished => "ToolFinished".to_string(),
        EventType::ToolFailed { .. } => "ToolFailed".to_string(),
        EventType::ModelInvoked { .. } => "ModelInvoked".to_string(),
        EventType::ModelFinished => "ModelFinished".to_string(),
        EventType::ModelFailed { .. } => "ModelFailed".to_string(),
        EventType::ArtifactProduced { .. } => "ArtifactProduced".to_string(),
    }
}

fn event_type_from_name(name: &str, doc: &serde_json::Value) -> Result<EventType, String> {
    match name {
        "RunStarted" => Ok(EventType::RunStarted),
        "RunFinished" => Ok(EventType::RunFinished),
        "NodeStarted" => Ok(EventType::NodeStarted),
        "NodeFinished" => Ok(EventType::NodeFinished),
        "NodeCancelled" => Ok(EventType::NodeCancelled),
        "ToolFinished" => Ok(EventType::ToolFinished),
        "ModelFinished" => Ok(EventType::ModelFinished),
        "ArtifactProduced" => {
            let artifact_id = doc
                .get("payload")
                .and_then(|p| p.get("artifact_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(EventType::ArtifactProduced { artifact_id })
        }
        "ToolInvoked" => {
            let tool_name = doc
                .get("payload")
                .and_then(|p| p.get("tool_name"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(EventType::ToolInvoked { tool_name })
        }
        "ToolFailed" => {
            let error = doc
                .get("payload")
                .and_then(|p| p.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(EventType::ToolFailed { error })
        }
        "ModelInvoked" => {
            let endpoint = doc
                .get("payload")
                .and_then(|p| p.get("endpoint"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(EventType::ModelInvoked { endpoint })
        }
        "ModelFailed" => {
            let error = doc
                .get("payload")
                .and_then(|p| p.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok(EventType::ModelFailed { error })
        }
        "NodeFailed" => {
            let error = doc
                .get("payload")
                .and_then(|p| p.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let retries = doc
                .get("payload")
                .and_then(|p| p.get("retries"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            Ok(EventType::NodeFailed { error, retries })
        }
        _ => Err(format!("Unknown event_type: {name}")),
    }
}

fn stored_artifact_record_to_document(
    record: &StoredArtifactRecord,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "id": record.id.as_uuid().to_string(),
        "run_id": record.run_id.as_uuid().to_string(),
        "name": record.name,
        "kind": format!("{:?}", record.kind),
        "mime_type": record.mime_type,
        "size_bytes": record.size_bytes,
        "checksum_sha256": record.checksum_sha256,
        "created_at_ms": record.created_at_ms,
        "metadata": record.metadata,
        "blob_path": record.blob_path,
    }))
}

fn document_to_stored_artifact_record(
    doc: serde_json::Value,
) -> Result<StoredArtifactRecord, String> {
    let id = doc
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Artifact record missing id".to_string())?;
    let run_id = doc
        .get("run_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Artifact record missing run_id".to_string())?;

    let name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let kind = match doc.get("kind").and_then(|v| v.as_str()).unwrap_or("Output") {
        "Input" => ArtifactKind::Input,
        "Intermediate" => ArtifactKind::Intermediate,
        _ => ArtifactKind::Output,
    };

    let mime_type = doc
        .get("mime_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream")
        .to_string();

    let size_bytes = doc.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
    let checksum_sha256 = doc
        .get("checksum_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let created_at_ms = doc
        .get("created_at_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let metadata = doc
        .get("metadata")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<HashMap<String, String>>()
        })
        .unwrap_or_default();

    let blob_path = doc
        .get("blob_path")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    Ok(StoredArtifactRecord {
        id: ArtifactId::from_uuid(crate::file_backed::parse_uuid(id)?),
        run_id: RunId::from_uuid(crate::file_backed::parse_uuid(run_id)?),
        name,
        kind,
        mime_type,
        size_bytes,
        checksum_sha256,
        created_at_ms,
        metadata,
        blob_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn rethinkdb_schema_bootstrap_and_basic_write() {
        let cfg = RethinkDbConfig::from_env();
        let mut session = RethinkDbTimelineStore::connect_session(&cfg).await.unwrap();

        let timeline = RethinkDbTimelineStore::default();
        timeline.ensure_schema(&mut session).await.unwrap();

        let run_id = RunId::new();
        let event = TimelineEvent::new(run_id.clone(), EventType::RunStarted);
        timeline.append(&mut session, &event).await.unwrap();

        let events = timeline.query_by_run(&mut session, &run_id).await.unwrap();
        assert!(!events.is_empty());
    }
}
