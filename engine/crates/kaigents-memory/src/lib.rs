//! File: engine/crates/kaigents-memory/src/lib.rs
//! Purpose: Real-time memory subsystem for Kaigents.
//! Product/business importance: Implements the three-tier learning and context manager.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use async_trait::async_trait;
use flate2::write::GzEncoder;
use flate2::Compression;
use kaigents_core::context_manager::{ContextBudgetStrategy, ContextManager, FittedContext};
use kaigents_core::model_serving::{
    ChatCompletionRequest, ChatMessage, EmbeddingsRequest, ModelClient,
};
use kaigents_core::nebulagraph_store::{NebulaConfig, NebulaGraphStore};
#[cfg(feature = "rethinkdb")]
use kaigents_core::rethinkdb_store::RethinkDbConfig;
use kaigents_core::run_id::RunId;
use kaigents_core::tool_plane::{MCPClient, ToolContract};
use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointId, PointStruct,
    ScrollPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, Value, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(feature = "rethinkdb")]
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[cfg(feature = "rethinkdb")]
const DEFAULT_EPISODES_TABLE_NAME: &str = "memory_episodes";
#[cfg(feature = "rethinkdb")]
const DEFAULT_BELIEFS_TABLE_NAME: &str = "memory_beliefs";

fn current_unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(feature = "rethinkdb")]
fn escape_regex(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('.', "\\.")
        .replace('*', "\\*")
        .replace('+', "\\+")
        .replace('?', "\\?")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('^', "\\^")
        .replace('$', "\\$")
        .replace('|', "\\|")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryTier {
    Short,
    Long,
    Epistemic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub tier: MemoryTier,
    pub workspace_id: String,
    pub run_id: Option<RunId>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
}

/// MemoryManager handles ingestion, retrieval, and tiering of agent memory.
pub struct MemoryManager {
    qdrant: Option<Qdrant>,
    model_client: Option<Arc<dyn ModelClient>>,
    embedding_endpoint: Option<String>,
    chat_endpoint: Option<String>,
    embedding_model: Option<String>,
    chat_model: Option<String>,
    context_manager: ContextManager,
    nebula: Option<Arc<NebulaGraphStore>>,
    #[cfg(feature = "rethinkdb")]
    rethinkdb_session: Option<Arc<Mutex<unreql::Session>>>,
    #[cfg(feature = "rethinkdb")]
    rethinkdb_db: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub score: f32,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub workspace_id: String,
    pub run_id: Option<RunId>,
    pub summary: String,
    pub source_content_ids: Vec<String>,
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_package_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HypothesisStatus {
    Pending,
    Confirmed,
    Falsified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub workspace_id: String,
    pub run_id: Option<RunId>,
    pub content: String,
    pub assumptions: Vec<String>, // IDs of other hypotheses
    pub confidence: f32,
    pub status: HypothesisStatus,
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_package_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tier: Option<String>, // e.g. "core", "recall", "archival"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportedPoint {
    id: Option<String>,
    payload: serde_json::Value,
    vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefOutcome {
    pub hypothesis_id: String,
    pub status: HypothesisStatus,
    pub justification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub source_priority: Vec<String>, // IDs of packages or "local"
}

impl MemoryManager {
    pub fn new(
        qdrant_url: Option<String>,
        model_client: Option<Arc<dyn ModelClient>>,
        embedding_endpoint: Option<String>,
        chat_endpoint: Option<String>,
    ) -> Result<Self, String> {
        let qdrant = if let Some(url) = qdrant_url {
            let client = Qdrant::from_url(&url)
                .build()
                .map_err(|e| format!("Failed to create Qdrant client: {}", e))?;
            Some(client)
        } else {
            None
        };

        Ok(Self {
            qdrant,
            model_client,
            embedding_endpoint,
            chat_endpoint,
            embedding_model: None,
            chat_model: None,
            context_manager: ContextManager::new(),
            nebula: None,
            #[cfg(feature = "rethinkdb")]
            rethinkdb_session: None,
            #[cfg(feature = "rethinkdb")]
            rethinkdb_db: None,
        })
    }

    pub fn with_embedding_model(mut self, model: String) -> Self {
        self.embedding_model = Some(model);
        self
    }

    pub fn with_chat_model(mut self, model: String) -> Self {
        self.chat_model = Some(model);
        self
    }

    pub async fn with_nebula(mut self, cfg: &NebulaConfig) -> Result<Self, String> {
        let store = NebulaGraphStore::new(cfg.clone());
        match store.init_schema().await {
            Ok(()) => {
                info!(
                    "NebulaGraph connected and schema initialized for space '{}'",
                    cfg.space
                );
                self.nebula = Some(Arc::new(store));
            }
            Err(e) => {
                warn!("NebulaGraph connection failed ({}). Graph features disabled. Continuing with RethinkDB fallback.", e);
            }
        }
        Ok(self)
    }

    #[cfg(feature = "rethinkdb")]
    pub async fn with_rethinkdb(mut self, cfg: &RethinkDbConfig) -> Result<Self, String> {
        use unreql::r;
        let mut session = None;
        let max_retries = 5;
        let mut retry_count = 0;

        while retry_count < max_retries {
            match r.connect(cfg.to_unreql_options()).await {
                Ok(s) => {
                    session = Some(s);
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    warn!(
                        "RethinkDB connect failed (attempt {}/{}): {}. Retrying in 2s...",
                        retry_count, max_retries, e
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        let mut session = session
            .ok_or_else(|| format!("RethinkDB connect failed after {} attempts", max_retries))?;

        // Ensure tables exist
        r.db(cfg.database.clone())
            .table_create(DEFAULT_EPISODES_TABLE_NAME)
            .exec::<_, serde_json::Value>(&mut session)
            .await
            .ok();

        r.db(cfg.database.clone())
            .table_create(DEFAULT_BELIEFS_TABLE_NAME)
            .exec::<_, serde_json::Value>(&mut session)
            .await
            .ok();

        self.rethinkdb_session = Some(Arc::new(Mutex::new(session)));
        self.rethinkdb_db = Some(cfg.database.clone());
        Ok(self)
    }

    pub async fn record(&self, mut record: MemoryRecord) -> Result<String, String> {
        info!(
            "Recording memory for tier {:?} in workspace {}",
            record.tier, record.workspace_id
        );

        if record.vector.is_none() {
            if let (Some(client), Some(endpoint)) = (&self.model_client, &self.embedding_endpoint) {
                let req = EmbeddingsRequest {
                    model: self
                        .embedding_model
                        .clone()
                        .unwrap_or_else(|| "ignored".to_string()),
                    input: vec![record.content.clone()],
                    encoding_format: None,
                };
                let resp = client
                    .embeddings(endpoint, req, Duration::from_secs(30))
                    .await
                    .map_err(|e| format!("Embedding failed: {}", e))?;

                if let Some(emb) = resp.data.first() {
                    record.vector = Some(emb.embedding.clone());
                }
            }
        }

        match record.tier {
            MemoryTier::Short => self.record_short_term(record).await,
            MemoryTier::Long => {
                warn!("Direct recording to long-term memory tier not typical; usually via consolidation");
                Ok("Direct long-term recording skipped (use consolidation)".to_string())
            }
            MemoryTier::Epistemic => {
                let hypothesis: Hypothesis =
                    serde_json::from_value(record.metadata.unwrap_or(serde_json::Value::Null))
                        .map_err(|e| format!("Invalid metadata for epistemic memory: {}", e))?;
                let id = self.record_belief(hypothesis).await?;
                Ok(format!("Belief recorded with ID: {}", id))
            }
        }
    }

    pub async fn record_belief(&self, mut hypothesis: Hypothesis) -> Result<String, String> {
        info!("Recording belief: {:?}", hypothesis.content);

        if hypothesis.id.is_none() {
            hypothesis.id = Some(uuid::Uuid::new_v4().to_string());
        }
        hypothesis.timestamp_ms = current_unix_timestamp_ms();
        hypothesis.status = HypothesisStatus::Pending;

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session: tokio::sync::MutexGuard<'_, unreql::Session> =
                session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            r.db(db.clone())
                .table(DEFAULT_BELIEFS_TABLE_NAME)
                .insert(r.expr(serde_json::to_value(&hypothesis).map_err(|e| e.to_string())?))
                .exec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("RethinkDB belief insert failed: {}", e))?;
        }

        if let Some(nebula) = &self.nebula {
            let belief_id = hypothesis.id.as_ref().unwrap();
            let ws = &hypothesis.workspace_id;
            let now = kaigents_core::nebulagraph_store::current_timestamp_i64();

            let _ = nebula
                .insert_entity(belief_id, &hypothesis.content, "belief", ws)
                .await;

            for assumption_id in &hypothesis.assumptions {
                let _ = nebula
                    .insert_temporal_edge(belief_id, assumption_id, "depends_on", now, 0, now)
                    .await;
            }
        }

        Ok(hypothesis.id.unwrap())
    }

    #[cfg_attr(not(feature = "rethinkdb"), allow(unused_variables))]
    pub async fn close_experiment(
        &self,
        workspace_id: &str,
        outcome: BeliefOutcome,
        scope_package_id: Option<&str>,
    ) -> Result<String, String> {
        info!(
            "Closing experiment for hypothesis {}",
            outcome.hypothesis_id
        );

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session: tokio::sync::MutexGuard<'_, unreql::Session> =
                session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            r.db(db.clone())
                .table(DEFAULT_BELIEFS_TABLE_NAME)
                .get(outcome.hypothesis_id.clone())
                .update(r.expr(serde_json::json!({
                    "status": outcome.status,
                    "justification": outcome.justification
                })))
                .exec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("RethinkDB belief update failed: {}", e))?;

            if outcome.status == HypothesisStatus::Falsified {
                if let Some(nebula) = &self.nebula {
                    let now = kaigents_core::nebulagraph_store::current_timestamp_i64();
                    let dependents = nebula
                        .traverse_dependents_recursive(&outcome.hypothesis_id, "depends_on")
                        .await
                        .unwrap_or_default();

                    for dep_id in &dependents {
                        let _ = nebula
                            .invalidate_edge(dep_id, &outcome.hypothesis_id, "depends_on", now)
                            .await;

                        let mut update_q = r
                            .db(db.clone())
                            .table(DEFAULT_BELIEFS_TABLE_NAME)
                            .get(dep_id.clone())
                            .filter(r.row().g("status").ne("falsified"))
                            .filter(r.row().g("workspace_id").eq(workspace_id.to_string()));

                        if let Some(pkg) = scope_package_id {
                            update_q =
                                update_q.filter(r.row().g("origin_package_id").eq(pkg.to_string()));
                        }

                        update_q
                            .update(r.expr(serde_json::json!({
                                "status": "falsified",
                                "justification": format!("Retracted via graph traversal: dependency {} falsified", outcome.hypothesis_id)
                            })))
                            .exec::<_, serde_json::Value>(&mut *session)
                            .await
                            .ok();
                    }
                } else {
                    let mut to_retract = vec![outcome.hypothesis_id.clone()];
                    let mut visited = std::collections::HashSet::new();
                    visited.insert(outcome.hypothesis_id.clone());

                    while let Some(current_id) = to_retract.pop() {
                        let mut query = r
                            .db(db.clone())
                            .table(DEFAULT_BELIEFS_TABLE_NAME)
                            .filter(r.row().g("assumptions").contains(current_id.clone()))
                            .filter(r.row().g("status").ne("falsified"))
                            .filter(r.row().g("workspace_id").eq(workspace_id.to_string()));

                        if let Some(pkg) = scope_package_id {
                            query =
                                query.filter(r.row().g("origin_package_id").eq(pkg.to_string()));
                        }

                        let dependents: Vec<serde_json::Value> =
                            query.exec_to_vec(&mut *session).await.unwrap_or_default();

                        for dep in dependents {
                            if let Some(dep_id) = dep["id"].as_str() {
                                let dep_id_str = dep_id.to_string();
                                if visited.insert(dep_id_str.clone()) {
                                    to_retract.push(dep_id_str.clone());
                                    r.db(db.clone())
                                    .table(DEFAULT_BELIEFS_TABLE_NAME)
                                    .get(dep_id_str)
                                    .update(r.expr(serde_json::json!({
                                        "status": "falsified",
                                        "justification": format!("Retracted due to falsification of dependency {}", current_id)
                                    })))
                                    .exec::<_, serde_json::Value>(&mut *session)
                                    .await
                                    .ok();
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(nebula) = &self.nebula {
            let now = kaigents_core::nebulagraph_store::current_timestamp_i64();
            let _ = nebula
                .invalidate_edge(
                    &outcome.hypothesis_id,
                    &outcome.hypothesis_id,
                    "depends_on",
                    now,
                )
                .await;
        }

        Ok(format!("Experiment closed as {:?}", outcome.status))
    }

    pub async fn reverify_hypothesis(&self, hypothesis_id: &str) -> Result<String, String> {
        info!("Re-verifying hypothesis {}", hypothesis_id);

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session: tokio::sync::MutexGuard<'_, unreql::Session> =
                session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            r.db(db.clone())
                .table(DEFAULT_BELIEFS_TABLE_NAME)
                .get(hypothesis_id.to_string())
                .update(r.expr(serde_json::json!({
                    "status": "pending"
                })))
                .exec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("RethinkDB belief update failed: {}", e))?;
        }

        Ok(format!(
            "Hypothesis {} re-opened for verification",
            hypothesis_id
        ))
    }

    async fn ensure_collection(&self, client: &Qdrant, name: &str, dim: u64) -> Result<(), String> {
        let collections = client
            .list_collections()
            .await
            .map_err(|e| format!("Failed to list Qdrant collections: {}", e))?;

        if collections.collections.iter().any(|c| c.name == name) {
            return Ok(());
        }

        info!("Creating Qdrant collection: {}", name);
        client
            .create_collection(
                CreateCollectionBuilder::new(name)
                    .vectors_config(VectorParamsBuilder::new(dim, Distance::Cosine)),
            )
            .await
            .map(|_| ())
            .map_err(|e| format!("Failed to create Qdrant collection {}: {}", name, e))
    }

    async fn record_short_term(&self, record: MemoryRecord) -> Result<String, String> {
        let client = self
            .qdrant
            .as_ref()
            .ok_or_else(|| "Qdrant client not configured for short-term memory".to_string())?;

        let collection_name = format!("workspace-{}", record.workspace_id);

        if let Some(vector) = record.vector {
            // Ensure collection exists
            self.ensure_collection(client, &collection_name, vector.len() as u64)
                .await?;

            let point_id = PointId {
                point_id_options: Some(PointIdOptions::Uuid(uuid::Uuid::new_v4().to_string())),
            };

            let mut payload = Payload::new();
            payload.insert("content", record.content);
            if let Some(run_id) = record.run_id {
                payload.insert("run_id", run_id.to_string());
            }
            if let Some(meta) = record.metadata {
                payload.insert("metadata", meta);
            }

            let point = PointStruct::new(point_id, vector, payload);

            client
                .upsert_points(
                    UpsertPointsBuilder::new(collection_name.clone(), vec![point]).build(),
                )
                .await
                .map_err(|e| format!("Qdrant upsert failed: {}", e))?;

            Ok(format!(
                "Short-term memory vector upserted to {}",
                collection_name
            ))
        } else {
            error!("Vector missing and embedding failed for short-term memory ingestion");
            Err("Vector required for short-term memory".to_string())
        }
    }

    pub async fn search(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u64,
    ) -> Result<Vec<MemorySearchResult>, String> {
        let qdrant = self
            .qdrant
            .as_ref()
            .ok_or_else(|| "Qdrant client not configured".to_string())?;

        let client = self
            .model_client
            .as_ref()
            .ok_or_else(|| "Model client not configured for search embeddings".to_string())?;

        let endpoint = self
            .embedding_endpoint
            .as_ref()
            .ok_or_else(|| "Embedding endpoint not configured".to_string())?;

        let emb_req = EmbeddingsRequest {
            model: self
                .embedding_model
                .clone()
                .unwrap_or_else(|| "ignored".to_string()),
            input: vec![query.to_string()],
            encoding_format: None,
        };

        let emb_resp = client
            .embeddings(endpoint, emb_req, Duration::from_secs(30))
            .await
            .map_err(|e| format!("Search embedding failed: {}", e))?;

        let vector = emb_resp
            .data
            .first()
            .ok_or_else(|| "No embedding returned for search query".to_string())?
            .embedding
            .clone();

        let collection_name = format!("workspace-{}", workspace_id);

        let search_req = SearchPointsBuilder::new(collection_name, vector, limit)
            .with_payload(true)
            .build();

        let resp = qdrant
            .search_points(search_req)
            .await
            .map_err(|e| format!("Qdrant search failed: {}", e))?;

        let results = resp
            .result
            .into_iter()
            .map(|scored_point| {
                let payload = scored_point.payload;
                let content = payload
                    .get("content")
                    .and_then(|v: &Value| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let run_id = payload
                    .get("run_id")
                    .and_then(|v: &Value| v.as_str())
                    .map(|s| s.to_string());
                let metadata = payload
                    .get("metadata")
                    .map(|v: &Value| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));

                MemorySearchResult {
                    content,
                    metadata,
                    score: scored_point.score,
                    run_id,
                }
            })
            .collect();

        Ok(results)
    }

    pub async fn export_memory(
        &self,
        workspace_id: &str,
        package_id: &str,
    ) -> Result<Vec<u8>, String> {
        #[allow(unused_mut)]
        let mut episodes: Vec<serde_json::Value> = Vec::new();
        #[allow(unused_mut)]
        let mut beliefs: Vec<serde_json::Value> = Vec::new();

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session = session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();

            episodes = r
                .db(db.clone())
                .table(DEFAULT_EPISODES_TABLE_NAME)
                .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                .exec_to_vec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("Failed to export episodes: {}", e))?;

            beliefs = r
                .db(db.clone())
                .table(DEFAULT_BELIEFS_TABLE_NAME)
                .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                .exec_to_vec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("Failed to export beliefs: {}", e))?;
        }

        // Qdrant points
        let mut points = Vec::new();
        if let Some(qdrant) = &self.qdrant {
            let collection_name = format!("workspace-{}", workspace_id);
            let scroll_req = ScrollPointsBuilder::new(collection_name.clone())
                .with_payload(true)
                .with_vectors(true)
                .limit(100)
                .build();

            let mut resp = qdrant
                .scroll(scroll_req)
                .await
                .map_err(|e| format!("Qdrant scroll failed: {}", e))?;
            points.extend(resp.result.clone());

            while let Some(offset) = resp.next_page_offset {
                let scroll_req = ScrollPointsBuilder::new(collection_name.clone())
                    .with_payload(true)
                    .with_vectors(true)
                    .offset(offset)
                    .limit(100)
                    .build();
                resp = qdrant
                    .scroll(scroll_req)
                    .await
                    .map_err(|e| format!("Qdrant scroll failed: {}", e))?;
                points.extend(resp.result.clone());
            }
        }

        let mut buf = Vec::new();
        {
            let encoder = GzEncoder::new(&mut buf, Compression::default());
            let mut tar = tar::Builder::new(encoder);

            // Manifest
            let manifest = serde_json::json!({
                "schema_version": "1",
                "package_id": package_id,
                "origin_workspace_id": workspace_id,
                "embedding_model": self.embedding_model,
                "package_type": "update",
                "timestamp_ms": current_unix_timestamp_ms(),
                "episode_count": episodes.len(),
                "belief_count": beliefs.len(),
                "point_count": points.len(),
            });
            let manifest_json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
            let mut header = tar::Header::new_gnu();
            header.set_size(manifest_json.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "manifest.json", &manifest_json[..])
                .map_err(|e| e.to_string())?;

            // Episodes
            let episodes_jsonl = episodes
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            let mut header = tar::Header::new_gnu();
            header.set_size(episodes_jsonl.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "episodes.jsonl", episodes_jsonl.as_bytes())
                .map_err(|e| e.to_string())?;

            // Beliefs
            let beliefs_jsonl = beliefs
                .iter()
                .map(|b| serde_json::to_string(b).unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            let mut header = tar::Header::new_gnu();
            header.set_size(beliefs_jsonl.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "beliefs.jsonl", beliefs_jsonl.as_bytes())
                .map_err(|e| e.to_string())?;

            // Qdrant points
            let points_jsonl = points
                .iter()
                .map(|p| {
                    let exported = ExportedPoint {
                        id: p.id.as_ref().and_then(|id| match &id.point_id_options {
                            Some(PointIdOptions::Num(n)) => Some(n.to_string()),
                            Some(PointIdOptions::Uuid(u)) => Some(u.clone()),
                            None => None,
                        }),
                        payload: {
                            let map: serde_json::Map<String, serde_json::Value> = p
                                .payload
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone().into()))
                                .collect();
                            serde_json::Value::Object(map)
                        },
                        vector: p
                            .vectors
                            .as_ref()
                            .and_then(|v| v.get_vector())
                            .map(|vec| match vec {
                                qdrant_client::qdrant::vector_output::Vector::Dense(d) => {
                                    d.data
                                }
                                qdrant_client::qdrant::vector_output::Vector::Sparse(s) => {
                                    s.values
                                }
                                qdrant_client::qdrant::vector_output::Vector::MultiDense(m) => {
                                    m.vectors.into_iter().flat_map(|d| d.data).collect()
                                }
                            })
                            .unwrap_or_default(),
                    };
                    serde_json::to_string(&exported).unwrap()
                })
                .collect::<Vec<_>>()
                .join("\n");
            let mut header = tar::Header::new_gnu();
            header.set_size(points_jsonl.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "points.jsonl", points_jsonl.as_bytes())
                .map_err(|e| e.to_string())?;

            // policy.yaml
            let policy_yaml = format!(
                "# Memory policy for workspace {}\nsource_priority:\n  - local\n",
                workspace_id
            );
            let mut header = tar::Header::new_gnu();
            header.set_size(policy_yaml.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "policy.yaml", policy_yaml.as_bytes())
                .map_err(|e| e.to_string())?;

            // distilled-lessons.md
            let lessons_md = {
                let mut md = format!("# Distilled Lessons\n\nWorkspace: {}\n\n", workspace_id);
                for (i, ep) in episodes.iter().enumerate() {
                    let id = ep.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let content = ep.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    md.push_str(&format!("## Episode {}: {}\n\n{}\n\n", i + 1, id, content));
                }
                md
            };
            let mut header = tar::Header::new_gnu();
            header.set_size(lessons_md.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "distilled-lessons.md", lessons_md.as_bytes())
                .map_err(|e| e.to_string())?;

            tar.finish().map_err(|e| e.to_string())?;
        }

        Ok(buf)
    }

    #[cfg_attr(not(feature = "rethinkdb"), allow(dead_code))]
    async fn check_semantic_duplicate(&self, workspace_id: &str, text: &str) -> bool {
        let (client, endpoint) = match (&self.model_client, &self.embedding_endpoint) {
            (Some(c), Some(e)) => (c, e),
            _ => return false,
        };

        let qdrant = match &self.qdrant {
            Some(q) => q,
            None => return false,
        };

        let emb_req = EmbeddingsRequest {
            model: self
                .embedding_model
                .clone()
                .unwrap_or_else(|| "ignored".to_string()),
            input: vec![text.to_string()],
            encoding_format: None,
        };

        let emb_resp = match client
            .embeddings(endpoint, emb_req, Duration::from_secs(30))
            .await
        {
            Ok(resp) => resp,
            Err(_) => return false,
        };

        let vector = match emb_resp.data.first() {
            Some(emb) => emb.embedding.clone(),
            None => return false,
        };

        let collection_name = format!("workspace-{}", workspace_id);
        let search_req = SearchPointsBuilder::new(collection_name, vector, 1)
            .with_payload(false)
            .build();

        match qdrant.search_points(search_req).await {
            Ok(resp) => resp.result.first().map(|r| r.score > 0.95).unwrap_or(false),
            Err(_) => false,
        }
    }

    pub async fn import_memory(
        &self,
        workspace_id: &str,
        package_bytes: &[u8],
    ) -> Result<String, String> {
        let decoder = flate2::read::GzDecoder::new(package_bytes);
        let mut tar = tar::Archive::new(decoder);

        let mut manifest = serde_json::Value::Null;
        let mut episodes = Vec::new();
        let mut beliefs = Vec::new();
        let mut points = Vec::new();

        for entry in tar.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
            let mut content = Vec::new();
            entry.read_to_end(&mut content).map_err(|e| e.to_string())?;

            if path.ends_with("manifest.json") {
                manifest = serde_json::from_slice(&content).map_err(|e| e.to_string())?;
            } else if path.ends_with("episodes.jsonl") {
                let s = String::from_utf8(content).map_err(|e| e.to_string())?;
                for line in s.lines().filter(|l| !l.is_empty()) {
                    episodes
                        .push(serde_json::from_str::<Episode>(line).map_err(|e| e.to_string())?);
                }
            } else if path.ends_with("beliefs.jsonl") {
                let s = String::from_utf8(content).map_err(|e| e.to_string())?;
                for line in s.lines().filter(|l| !l.is_empty()) {
                    beliefs
                        .push(serde_json::from_str::<Hypothesis>(line).map_err(|e| e.to_string())?);
                }
            } else if path.ends_with("points.jsonl") {
                let s = String::from_utf8(content).map_err(|e| e.to_string())?;
                for line in s.lines().filter(|l| !l.is_empty()) {
                    points.push(
                        serde_json::from_str::<ExportedPoint>(line).map_err(|e| e.to_string())?,
                    );
                }
            }
        }

        let package_id = manifest["package_id"]
            .as_str()
            .ok_or("Missing package_id in manifest")?
            .to_string();
        let origin_workspace_id = manifest["origin_workspace_id"]
            .as_str()
            .ok_or("Missing origin_workspace_id in manifest")?
            .to_string();

        if let Some(manifest_model) = manifest["embedding_model"].as_str() {
            if let Some(self_model) = &self.embedding_model {
                if manifest_model != self_model {
                    warn!(
                        "Embedding model mismatch: package uses '{}' but workspace uses '{}'. \
                         Transferred vectors may be in a different vector space.",
                        manifest_model, self_model
                    );
                }
            }
        }

        #[allow(unused_mut)]
        let mut skipped_episodes = 0u32;
        #[allow(unused_mut)]
        let mut skipped_beliefs = 0u32;
        let mut skipped_points = 0u32;

        // 1. Import Episodes (with provenance + dedup)
        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session = session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();

            for mut episode in episodes {
                let semantic_dup = self
                    .check_semantic_duplicate(workspace_id, &episode.summary)
                    .await;

                let text_dup: Vec<serde_json::Value> = r
                    .db(db.clone())
                    .table(DEFAULT_EPISODES_TABLE_NAME)
                    .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                    .filter(r.row().g("summary").eq(episode.summary.clone()))
                    .exec_to_vec(&mut *session)
                    .await
                    .unwrap_or_default();

                if semantic_dup || !text_dup.is_empty() {
                    skipped_episodes += 1;
                    continue;
                }

                episode.id = Some(uuid::Uuid::new_v4().to_string());
                episode.workspace_id = workspace_id.to_string();
                episode.origin_workspace_id = Some(origin_workspace_id.clone());
                episode.origin_package_id = Some(package_id.clone());

                r.db(db.clone())
                    .table(DEFAULT_EPISODES_TABLE_NAME)
                    .insert(r.expr(serde_json::to_value(&episode).map_err(|e| e.to_string())?))
                    .exec::<_, serde_json::Value>(&mut *session)
                    .await
                    .map_err(|e| format!("Failed to import episode: {}", e))?;
            }

            for mut belief in beliefs {
                let semantic_dup = self
                    .check_semantic_duplicate(workspace_id, &belief.content)
                    .await;

                let text_dup: Vec<serde_json::Value> = r
                    .db(db.clone())
                    .table(DEFAULT_BELIEFS_TABLE_NAME)
                    .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                    .filter(r.row().g("content").eq(belief.content.clone()))
                    .exec_to_vec(&mut *session)
                    .await
                    .unwrap_or_default();

                if semantic_dup || !text_dup.is_empty() {
                    skipped_beliefs += 1;
                    continue;
                }

                belief.id = Some(uuid::Uuid::new_v4().to_string());
                belief.workspace_id = workspace_id.to_string();
                belief.origin_workspace_id = Some(origin_workspace_id.clone());
                belief.origin_package_id = Some(package_id.clone());
                belief.status = HypothesisStatus::Pending;

                r.db(db.clone())
                    .table(DEFAULT_BELIEFS_TABLE_NAME)
                    .insert(r.expr(serde_json::to_value(&belief).map_err(|e| e.to_string())?))
                    .exec::<_, serde_json::Value>(&mut *session)
                    .await
                    .map_err(|e| format!("Failed to import belief: {}", e))?;
            }
        }

        // 2. Import Qdrant points (with vector similarity dedup)
        if let Some(qdrant) = &self.qdrant {
            let collection_name = format!("workspace-{}", workspace_id);
            if let Some(first_point) = points.first() {
                let dim = first_point.vector.len() as u64;
                if dim == 0 {
                    return Err("Cannot import package: exported points have empty vectors (dimension 0). The export may have been created with an incompatible Qdrant client version.".to_string());
                }

                self.ensure_collection(qdrant, &collection_name, dim)
                    .await?;

                let mut structs = Vec::new();
                for p in points {
                    let search_req =
                        SearchPointsBuilder::new(collection_name.clone(), p.vector.clone(), 1)
                            .with_payload(false)
                            .build();

                    let is_dup = match qdrant.search_points(search_req).await {
                        Ok(resp) => resp.result.first().map(|r| r.score > 0.95).unwrap_or(false),
                        Err(_) => false,
                    };

                    if is_dup {
                        skipped_points += 1;
                        continue;
                    }

                    let mut payload = Payload::new();
                    if let Some(obj) = p.payload.as_object() {
                        for (k, v) in obj {
                            payload.insert(k.clone(), v.clone());
                        }
                    }
                    payload.insert("origin_workspace_id", origin_workspace_id.clone());
                    payload.insert("origin_package_id", package_id.clone());

                    structs.push(PointStruct::new(
                        uuid::Uuid::new_v4().to_string(),
                        p.vector,
                        payload,
                    ));
                }

                if !structs.is_empty() {
                    qdrant
                        .upsert_points(UpsertPointsBuilder::new(collection_name, structs).build())
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(format!(
            "Imported package {} into workspace {} (skipped {} dup episodes, {} dup beliefs, {} dup points)",
            package_id, workspace_id, skipped_episodes, skipped_beliefs, skipped_points
        ))
    }

    pub async fn remove_package(
        &self,
        workspace_id: &str,
        package_id: &str,
    ) -> Result<String, String> {
        info!(
            "Removing package {} from workspace {}",
            package_id, workspace_id
        );

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;

            // Step 1: Acquire lock, query belief IDs, then release lock
            // (cannot hold lock across close_experiment calls — it also acquires the mutex)
            let belief_ids: Vec<String> = {
                let mut session = session_mutex.lock().await;
                let db = self
                    .rethinkdb_db
                    .as_deref()
                    .unwrap_or("kaigents")
                    .to_string();

                let beliefs: Vec<serde_json::Value> = r
                    .db(db.clone())
                    .table(DEFAULT_BELIEFS_TABLE_NAME)
                    .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                    .filter(r.row().g("origin_package_id").eq(package_id.to_string()))
                    .exec_to_vec(&mut *session)
                    .await
                    .unwrap_or_default();

                beliefs
                    .into_iter()
                    .filter_map(|b| b["id"].as_str().map(|s| s.to_string()))
                    .collect()
            };

            // Step 2: Falsify each belief (close_experiment acquires its own lock)
            for id in &belief_ids {
                self.close_experiment(
                    workspace_id,
                    BeliefOutcome {
                        hypothesis_id: id.clone(),
                        status: HypothesisStatus::Falsified,
                        justification: format!("Package {} removed", package_id),
                    },
                    Some(package_id),
                )
                .await?;
            }

            // Step 3: Re-acquire lock to remove episodes
            {
                let mut session = session_mutex.lock().await;
                let db = self
                    .rethinkdb_db
                    .as_deref()
                    .unwrap_or("kaigents")
                    .to_string();
                r.db(db.clone())
                    .table(DEFAULT_EPISODES_TABLE_NAME)
                    .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                    .filter(r.row().g("origin_package_id").eq(package_id.to_string()))
                    .delete(())
                    .exec::<_, serde_json::Value>(&mut *session)
                    .await
                    .ok();
            }
        }

        // Remove Qdrant points
        if let Some(qdrant) = &self.qdrant {
            let collection_name = format!("workspace-{}", workspace_id);
            let filter = Filter::all(vec![qdrant_client::qdrant::Condition::matches(
                "origin_package_id",
                package_id.to_string(),
            )]);
            qdrant
                .delete_points(
                    DeletePointsBuilder::new(collection_name)
                        .points(filter)
                        .build(),
                )
                .await
                .ok();
        }

        Ok(format!(
            "Package {} removed from workspace {}",
            package_id, workspace_id
        ))
    }

    pub async fn recall(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u64,
    ) -> Result<Vec<MemorySearchResult>, String> {
        // 1. Search short-term memory (Qdrant)
        let mut results = self
            .search(workspace_id, query, limit)
            .await
            .unwrap_or_default();

        // 2. Search long-term episodes (RethinkDB) if available
        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session = session_mutex.lock().await;

            // Simple keyword search on episodes for now, as we don't have episode embeddings yet
            // In a real system, episodes would also be in Qdrant.
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            let escaped_query = escape_regex(query);
            let docs: Vec<serde_json::Value> = r
                .db(db.clone())
                .table(DEFAULT_EPISODES_TABLE_NAME)
                .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                .filter(
                    r.row()
                        .g("summary")
                        .match_(format!("(?i){}", escaped_query)),
                )
                .limit(limit)
                .exec_to_vec(&mut *session)
                .await
                .unwrap_or_default();

            for doc in docs {
                if let Ok(episode) = serde_json::from_value::<Episode>(doc) {
                    results.push(MemorySearchResult {
                        content: format!("[EPISODE] {}", episode.summary),
                        metadata: Some(serde_json::json!({
                            "type": "long-term",
                            "timestamp_ms": episode.timestamp_ms,
                            "run_id": episode.run_id
                        })),
                        score: 0.8, // Placeholder score for keyword matches
                        run_id: episode.run_id.map(|rid| rid.to_string()),
                    });
                }
            }
        }

        // Sort by score
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit as usize);

        Ok(results)
    }

    pub async fn consolidate_run_memory(
        &self,
        workspace_id: &str,
        run_id: &RunId,
    ) -> Result<Episode, String> {
        let qdrant = self
            .qdrant
            .as_ref()
            .ok_or_else(|| "Qdrant client not configured".to_string())?;

        let client = self
            .model_client
            .as_ref()
            .ok_or_else(|| "Model client not configured for consolidation".to_string())?;

        let collection_name = format!("workspace-{}", workspace_id);
        let filter = Filter::all(vec![qdrant_client::qdrant::Condition::matches(
            "run_id",
            run_id.as_uuid().to_string(),
        )]);

        let scroll_req = ScrollPointsBuilder::new(collection_name)
            .filter(filter.clone())
            .with_payload(true)
            .limit(100)
            .build();

        let mut resp = qdrant
            .scroll(scroll_req)
            .await
            .map_err(|e| format!("Qdrant scroll failed: {}", e))?;

        let mut all_points = resp.result.clone();

        while let Some(offset) = resp.next_page_offset {
            let scroll_req = ScrollPointsBuilder::new(format!("workspace-{}", workspace_id))
                .filter(filter.clone())
                .with_payload(true)
                .offset(offset)
                .limit(100)
                .build();
            resp = qdrant
                .scroll(scroll_req)
                .await
                .map_err(|e| format!("Qdrant scroll failed: {}", e))?;
            all_points.extend(resp.result.clone());
        }

        let mut source_content_ids = Vec::new();
        let memories: Vec<String> = all_points
            .into_iter()
            .filter_map(|p| {
                let content = p
                    .payload
                    .get("content")
                    .and_then(|v: &Value| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if !content.is_empty() {
                    if let Some(pid) = p.id {
                        match pid.point_id_options {
                            Some(PointIdOptions::Num(n)) => {
                                source_content_ids.push(n.to_string());
                            }
                            Some(PointIdOptions::Uuid(s)) => {
                                source_content_ids.push(s);
                            }
                            None => {}
                        }
                    }
                    Some(content)
                } else {
                    None
                }
            })
            .collect();

        if memories.is_empty() {
            return Err(format!("No memories found for run {}", run_id));
        }

        let consolidation_prompt = format!(
            "You are a Memory Consolidator. Below are short-term memories from a single agent run.\n\
             Extract the key outcomes, decisions, and facts into a concise 1-2 paragraph 'Episode' summary.\n\n\
             MEMORIES:\n---\n{}\n---\n\nEpisode Summary:",
            memories.join("\n---\n")
        );

        let req = ChatCompletionRequest {
            model: self
                .chat_model
                .clone()
                .unwrap_or_else(|| "ignored".to_string()),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: consolidation_prompt,
            }],
            max_tokens: Some(2000),
            temperature: Some(0.3),
            stream: false,
        };

        // Use the chat endpoint for consolidation reasoning
        let endpoint = self.chat_endpoint.as_deref().unwrap_or("default");

        let chat_resp = client
            .chat_completion(endpoint, req, Duration::from_secs(120))
            .await
            .map_err(|e| format!("Consolidation model call failed: {}", e))?;

        let summary = chat_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "Failed to extract summary".to_string());

        let episode = Episode {
            id: Some(uuid::Uuid::new_v4().to_string()),
            workspace_id: workspace_id.to_string(),
            run_id: Some(run_id.clone()),
            summary,
            source_content_ids,
            timestamp_ms: current_unix_timestamp_ms(),
            origin_workspace_id: None,
            origin_package_id: None,
        };

        info!("Consolidated run {} into episode {:?}", run_id, episode.id);

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session = session_mutex.lock().await;
            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            r.db(db.clone())
                .table(DEFAULT_EPISODES_TABLE_NAME)
                .insert(r.expr(serde_json::to_value(&episode).map_err(|e| e.to_string())?))
                .exec::<_, serde_json::Value>(&mut *session)
                .await
                .map_err(|e| format!("RethinkDB insert failed: {}", e))?;
            info!("Episode persisted to RethinkDB");
        }

        if let Some(nebula) = &self.nebula {
            let episode_id = episode.id.as_ref().unwrap();
            let now = kaigents_core::nebulagraph_store::current_timestamp_i64();

            let _ = nebula
                .insert_entity(episode_id, &episode.summary, "episode", workspace_id)
                .await;

            for src_id in &episode.source_content_ids {
                let _ = nebula
                    .insert_temporal_edge(episode_id, src_id, "consolidated_from", now, 0, now)
                    .await;
            }
            info!("Episode temporal edges inserted into NebulaGraph");
        }

        Ok(episode)
    }

    pub async fn validate_approach(
        &self,
        workspace_id: &str,
        query: &str,
    ) -> Result<Vec<Hypothesis>, String> {
        info!(
            "Validating approach for workspace {} with query {}",
            workspace_id, query
        );

        #[cfg(feature = "rethinkdb")]
        if let Some(session_mutex) = &self.rethinkdb_session {
            use unreql::r;
            let mut session = session_mutex.lock().await;

            let db = self
                .rethinkdb_db
                .as_deref()
                .unwrap_or("kaigents")
                .to_string();
            let escaped_query = escape_regex(query);
            let falsified_beliefs: Vec<serde_json::Value> = r
                .db(db.clone())
                .table(DEFAULT_BELIEFS_TABLE_NAME)
                .filter(r.row().g("workspace_id").eq(workspace_id.to_string()))
                .filter(r.row().g("status").eq("falsified"))
                .filter(
                    r.row()
                        .g("content")
                        .match_(format!("(?i){}", escaped_query)),
                )
                .exec_to_vec(&mut *session)
                .await
                .unwrap_or_default();

            let mut violations = Vec::new();
            for val in falsified_beliefs {
                if let Ok(h) = serde_json::from_value::<Hypothesis>(val) {
                    violations.push(h);
                }
            }
            return Ok(violations);
        }

        Ok(Vec::new())
    }

    pub async fn assemble_context(
        &self,
        workspace_id: &str,
        system_prompt: &str,
        task_state: &str,
        query: &str,
        budget: u32,
        policy: Option<MemoryPolicy>,
    ) -> Result<FittedContext, String> {
        info!("Assembling context for workspace {}", workspace_id);

        // 1. Recall relevant information
        let mut search_results = self.recall(workspace_id, query, 10).await?;

        // 2. Apply Policy (Source Priority)
        if let Some(p) = &policy {
            search_results.sort_by(|a, b| {
                let a_origin = a
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("origin_package_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("local");
                let b_origin = b
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("origin_package_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("local");

                let a_pri = p
                    .source_priority
                    .iter()
                    .position(|x| x == a_origin)
                    .unwrap_or(p.source_priority.len());
                let b_pri = p
                    .source_priority
                    .iter()
                    .position(|x| x == b_origin)
                    .unwrap_or(p.source_priority.len());

                a_pri.cmp(&b_pri).then_with(|| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });
        }

        // 3. Categorize results
        let mut episodes = Vec::new();
        let mut short_term = Vec::new();

        for res in search_results {
            if res.content.starts_with("[EPISODE]") {
                episodes.push(res.content.replace("[EPISODE] ", ""));
            } else {
                short_term.push(res.content);
            }
        }

        // 4. Fetch beliefs (Epistemic memory)
        let beliefs = self
            .validate_approach(workspace_id, query)
            .await?
            .into_iter()
            .map(|h| h.content)
            .collect();

        // 5. Fit to budget
        Ok(self.context_manager.fit_to_budget(
            system_prompt,
            task_state,
            episodes,
            short_term,
            beliefs,
            budget,
            ContextBudgetStrategy::Auto,
        ))
    }
}

/// InternalMemoryToolClient exposes memory operations as MCP tools within the engine.
pub struct InternalMemoryToolClient {
    manager: Arc<MemoryManager>,
}

impl InternalMemoryToolClient {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl MCPClient for InternalMemoryToolClient {
    async fn list_tools(&self) -> Result<Vec<ToolContract>, String> {
        Ok(vec![
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "memory.record".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["tier", "workspace_id", "content"],
                    "properties": {
                        "tier": { "type": "string", "enum": ["short", "long", "epistemic"] },
                        "workspace_id": { "type": "string" },
                        "run_id": { "type": "string", "description": "Optional run identifier" },
                        "content": { "type": "string" },
                        "metadata": { "type": "object" }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" }
                    }
                }),
                description: Some(
                    "Records a piece of information into the agent's memory subsystem.".to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "memory.query".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["workspace_id", "query"],
                    "properties": {
                        "workspace_id": { "type": "string" },
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "default": 5 }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string" },
                                    "metadata": { "type": "object" },
                                    "score": { "type": "number" },
                                    "run_id": { "type": "string" }
                                }
                            }
                        }
                    }
                }),
                description: Some(
                    "Queries the agent's short-term memory using vector similarity search."
                        .to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "memory.recall".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["workspace_id", "query"],
                    "properties": {
                        "workspace_id": { "type": "string" },
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "default": 5 }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string" },
                                    "metadata": { "type": "object" },
                                    "score": { "type": "number" },
                                    "run_id": { "type": "string" }
                                }
                            }
                        }
                    }
                }),
                description: Some(
                    "Recalls relevant information from across memory tiers (short and long-term)."
                        .to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "memory.consolidate".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["workspace_id", "run_id"],
                    "properties": {
                        "workspace_id": { "type": "string" },
                        "run_id": { "type": "string" }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "episode": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "summary": { "type": "string" },
                                "timestamp_ms": { "type": "integer" }
                            }
                        }
                    }
                }),
                description: Some(
                    "Consolidates short-term memories from a run into a long-term episode summary."
                        .to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "memory.assemble_context".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["workspace_id", "system_prompt", "task_state", "query"],
                    "properties": {
                        "workspace_id": { "type": "string" },
                        "system_prompt": { "type": "string" },
                        "task_state": { "type": "string" },
                        "query": { "type": "string" },
                        "budget": { "type": "integer", "default": 2048 },
                        "policy": {
                            "type": "object",
                            "properties": {
                                "source_priority": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "messages": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "role": { "type": "string" },
                                    "content": { "type": "string" }
                                }
                            }
                        },
                        "total_estimated_tokens": { "type": "integer" },
                        "dropped_entries_count": { "type": "integer" }
                    }
                }),
                description: Some(
                    "Assembles a budget-fitted context for a model call, incorporating recalled memories."
                        .to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "experiment.close".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["hypothesis_id", "status"],
                    "properties": {
                        "hypothesis_id": { "type": "string" },
                        "status": { "type": "string", "enum": ["confirmed", "falsified"] },
                        "justification": { "type": "string" }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" }
                    }
                }),
                description: Some(
                    "Closes an experiment by recording the outcome of a hypothesis.".to_string(),
                ),
            },
            ToolContract {
                server_name: "kaigents-memory".to_string(),
                tool_name: "experiment.reverify".to_string(),
                version: "v1alpha1".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "required": ["hypothesis_id"],
                    "properties": {
                        "hypothesis_id": { "type": "string" }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" }
                    }
                }),
                description: Some(
                    "Re-opens a falsified hypothesis for deliberate re-verification.".to_string(),
                ),
            },
        ])
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        match tool_name {
            "memory.record" => {
                let record: MemoryRecord = serde_json::from_value(arguments)
                    .map_err(|e| format!("Invalid arguments for memory.record: {}", e))?;

                let status = self.manager.record(record).await?;
                Ok(serde_json::json!({ "status": status }))
            }
            "memory.query" => {
                let workspace_id = arguments["workspace_id"]
                    .as_str()
                    .ok_or_else(|| "workspace_id is required".to_string())?;
                let query = arguments["query"]
                    .as_str()
                    .ok_or_else(|| "query is required".to_string())?;
                let limit = arguments["limit"].as_u64().unwrap_or(5);

                let results = self.manager.search(workspace_id, query, limit).await?;
                Ok(serde_json::json!({ "results": results }))
            }
            "memory.recall" => {
                let workspace_id = arguments["workspace_id"]
                    .as_str()
                    .ok_or_else(|| "workspace_id is required".to_string())?;
                let query = arguments["query"]
                    .as_str()
                    .ok_or_else(|| "query is required".to_string())?;
                let limit = arguments["limit"].as_u64().unwrap_or(5);

                let results = self.manager.recall(workspace_id, query, limit).await?;
                Ok(serde_json::json!({ "results": results }))
            }
            "memory.assemble_context" => {
                let workspace_id = arguments["workspace_id"]
                    .as_str()
                    .ok_or_else(|| "workspace_id is required".to_string())?;
                let system_prompt = arguments["system_prompt"]
                    .as_str()
                    .ok_or_else(|| "system_prompt is required".to_string())?;
                let task_state = arguments["task_state"]
                    .as_str()
                    .ok_or_else(|| "task_state is required".to_string())?;
                let query = arguments["query"]
                    .as_str()
                    .ok_or_else(|| "query is required".to_string())?;
                let budget = arguments["budget"].as_u64().unwrap_or(2048) as u32;
                let policy =
                    serde_json::from_value::<MemoryPolicy>(arguments["policy"].clone()).ok();

                let fitted = self
                    .manager
                    .assemble_context(
                        workspace_id,
                        system_prompt,
                        task_state,
                        query,
                        budget,
                        policy,
                    )
                    .await?;
                Ok(serde_json::to_value(fitted).map_err(|e| e.to_string())?)
            }
            "memory.consolidate" => {
                let workspace_id = arguments["workspace_id"]
                    .as_str()
                    .ok_or_else(|| "workspace_id is required".to_string())?;
                let run_id_str = arguments["run_id"]
                    .as_str()
                    .ok_or_else(|| "run_id is required".to_string())?;
                let run_id = RunId::from_uuid(
                    uuid::Uuid::parse_str(run_id_str)
                        .map_err(|e| format!("Invalid run_id: {}", e))?,
                );

                let episode = self
                    .manager
                    .consolidate_run_memory(workspace_id, &run_id)
                    .await?;
                Ok(serde_json::json!({ "episode": episode }))
            }
            "experiment.close" => {
                let workspace_id = arguments["workspace_id"]
                    .as_str()
                    .ok_or_else(|| "workspace_id is required".to_string())?
                    .to_string();
                let outcome: BeliefOutcome = serde_json::from_value(arguments)
                    .map_err(|e| format!("Invalid arguments for experiment.close: {}", e))?;
                let status = self
                    .manager
                    .close_experiment(&workspace_id, outcome, None)
                    .await?;
                Ok(serde_json::json!({ "status": status }))
            }
            "experiment.reverify" => {
                let hypothesis_id = arguments["hypothesis_id"]
                    .as_str()
                    .ok_or_else(|| "hypothesis_id is required".to_string())?;
                let status = self.manager.reverify_hypothesis(hypothesis_id).await?;
                Ok(serde_json::json!({ "status": status }))
            }
            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn memory_tier_serialization() {
        let short_json = serde_json::to_string(&MemoryTier::Short).unwrap();
        assert_eq!(short_json, "\"short\"");
        let long_json = serde_json::to_string(&MemoryTier::Long).unwrap();
        assert_eq!(long_json, "\"long\"");
        let epistemic_json = serde_json::to_string(&MemoryTier::Epistemic).unwrap();
        assert_eq!(epistemic_json, "\"epistemic\"");

        let parsed: MemoryTier = serde_json::from_str("\"short\"").unwrap();
        assert_eq!(parsed, MemoryTier::Short);
        let parsed: MemoryTier = serde_json::from_str("\"long\"").unwrap();
        assert_eq!(parsed, MemoryTier::Long);
        let parsed: MemoryTier = serde_json::from_str("\"epistemic\"").unwrap();
        assert_eq!(parsed, MemoryTier::Epistemic);
    }

    #[test]
    fn hypothesis_status_serialization() {
        assert_eq!(
            serde_json::to_string(&HypothesisStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&HypothesisStatus::Confirmed).unwrap(),
            "\"confirmed\""
        );
        assert_eq!(
            serde_json::to_string(&HypothesisStatus::Falsified).unwrap(),
            "\"falsified\""
        );

        let parsed: HypothesisStatus = serde_json::from_str("\"falsified\"").unwrap();
        assert_eq!(parsed, HypothesisStatus::Falsified);
    }

    #[test]
    fn memory_record_round_trip() {
        let record = MemoryRecord {
            tier: MemoryTier::Short,
            workspace_id: "ws-1".to_string(),
            run_id: None,
            content: "test content".to_string(),
            metadata: Some(serde_json::json!({"key": "value"})),
            vector: Some(vec![0.1, 0.2, 0.3]),
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: MemoryRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tier, MemoryTier::Short);
        assert_eq!(parsed.workspace_id, "ws-1");
        assert_eq!(parsed.content, "test content");
        assert!(parsed.vector.is_some());
    }

    #[test]
    fn episode_round_trip() {
        let episode = Episode {
            id: Some("ep-1".to_string()),
            workspace_id: "ws-1".to_string(),
            run_id: None,
            summary: "A test episode summary.".to_string(),
            source_content_ids: vec!["src-1".to_string()],
            timestamp_ms: 1234567890,
            origin_workspace_id: None,
            origin_package_id: None,
        };
        let json = serde_json::to_string(&episode).unwrap();
        let parsed: Episode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, Some("ep-1".to_string()));
        assert_eq!(parsed.summary, "A test episode summary.");
        assert_eq!(parsed.timestamp_ms, 1234567890);
    }

    #[test]
    fn hypothesis_round_trip() {
        let hyp = Hypothesis {
            id: Some("hyp-1".to_string()),
            workspace_id: "ws-1".to_string(),
            run_id: None,
            content: "This approach works.".to_string(),
            assumptions: vec!["hyp-0".to_string()],
            confidence: 0.85,
            status: HypothesisStatus::Pending,
            timestamp_ms: 999,
            origin_workspace_id: None,
            origin_package_id: None,
            source_tier: None,
        };
        let json = serde_json::to_string(&hyp).unwrap();
        let parsed: Hypothesis = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, Some("hyp-1".to_string()));
        assert_eq!(parsed.content, "This approach works.");
        assert!((parsed.confidence - 0.85).abs() < 0.001);
        assert_eq!(parsed.status, HypothesisStatus::Pending);
        assert_eq!(parsed.assumptions, vec!["hyp-0".to_string()]);
    }

    #[test]
    fn belief_outcome_round_trip() {
        let outcome = BeliefOutcome {
            hypothesis_id: "hyp-1".to_string(),
            status: HypothesisStatus::Falsified,
            justification: "The test failed.".to_string(),
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let parsed: BeliefOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.hypothesis_id, "hyp-1");
        assert_eq!(parsed.status, HypothesisStatus::Falsified);
        assert_eq!(parsed.justification, "The test failed.");
    }

    #[test]
    fn memory_search_result_round_trip() {
        let result = MemorySearchResult {
            content: "found content".to_string(),
            metadata: Some(serde_json::json!({"type": "short-term"})),
            score: 0.95,
            run_id: Some("run-123".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: MemorySearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, "found content");
        assert!((parsed.score - 0.95).abs() < 0.001);
        assert_eq!(parsed.run_id, Some("run-123".to_string()));
    }

    #[test]
    fn memory_manager_new_with_none_succeeds() {
        let manager = MemoryManager::new(None, None, None, None);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn internal_memory_tool_client_lists_all_tools() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let tools = client.list_tools().await.unwrap();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.tool_name.as_str()).collect();
        assert!(tool_names.contains(&"memory.record"));
        assert!(tool_names.contains(&"memory.query"));
        assert!(tool_names.contains(&"memory.recall"));
        assert!(tool_names.contains(&"memory.assemble_context"));
        assert!(tool_names.contains(&"memory.consolidate"));
        assert!(tool_names.contains(&"experiment.close"));
        assert!(tool_names.contains(&"experiment.reverify"));
        assert_eq!(tools.len(), 7);
    }

    #[tokio::test]
    async fn internal_memory_tool_client_unknown_tool_errors() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let result = client
            .call_tool(
                "nonexistent_tool",
                serde_json::json!({}),
                Duration::from_secs(5),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn memory_record_short_term_without_qdrant_errors() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "tier": "short",
            "workspace_id": "ws-1",
            "content": "test content",
        });
        let result = client
            .call_tool("memory.record", args, Duration::from_secs(5))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Qdrant client not configured"));
    }

    #[tokio::test]
    async fn memory_record_long_term_returns_skip_message() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "tier": "long",
            "workspace_id": "ws-1",
            "content": "test content",
        });
        let result = client
            .call_tool("memory.record", args, Duration::from_secs(5))
            .await
            .unwrap();
        let status = result["status"].as_str().unwrap();
        assert!(status.contains("skipped"));
    }

    #[tokio::test]
    async fn memory_recall_without_qdrant_returns_empty() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "workspace_id": "ws-1",
            "query": "test",
        });
        let result = client
            .call_tool("memory.recall", args, Duration::from_secs(5))
            .await
            .unwrap();
        let results = result["results"].as_array().unwrap();
        assert!(
            results.is_empty(),
            "recall without Qdrant should return empty results"
        );
    }

    #[tokio::test]
    async fn memory_consolidate_without_qdrant_errors() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "workspace_id": "ws-1",
            "run_id": "00000000-0000-0000-0000-000000000001",
        });
        let result = client
            .call_tool("memory.consolidate", args, Duration::from_secs(5))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Qdrant client not configured"));
    }

    #[tokio::test]
    async fn experiment_close_without_rethinkdb_returns_ok() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "workspace_id": "ws-test",
            "hypothesis_id": "hyp-1",
            "status": "falsified",
            "justification": "test failed",
        });
        let result = client
            .call_tool("experiment.close", args, Duration::from_secs(5))
            .await
            .unwrap();
        let status = result["status"].as_str().unwrap();
        assert!(status.contains("Experiment closed"));
    }

    #[tokio::test]
    async fn experiment_reverify_without_rethinkdb_returns_ok() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "hypothesis_id": "hyp-1",
        });
        let result = client
            .call_tool("experiment.reverify", args, Duration::from_secs(5))
            .await
            .unwrap();
        let status = result["status"].as_str().unwrap();
        assert!(status.contains("re-opened"));
    }

    #[tokio::test]
    async fn memory_assemble_context_without_qdrant_succeeds() {
        let manager = MemoryManager::new(None, None, None, None).unwrap();
        let client = InternalMemoryToolClient::new(Arc::new(manager));
        let args = serde_json::json!({
            "workspace_id": "ws-1",
            "system_prompt": "You are AI.",
            "task_state": "working",
            "query": "test",
            "budget": 1000
        });
        let result = client
            .call_tool("memory.assemble_context", args, Duration::from_secs(5))
            .await
            .unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["content"], "You are AI.");
        assert_eq!(messages[1]["content"], "Current task state: working");
    }

    #[tokio::test]
    async fn export_includes_embedding_model_in_manifest() {
        let manager = MemoryManager::new(None, None, None, None)
            .unwrap()
            .with_embedding_model("nomic-embed-text".to_string());

        let bytes = manager.export_memory("ws-1", "pkg-1").await.unwrap();
        let decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut tar = tar::Archive::new(decoder);

        let mut manifest = serde_json::Value::Null;
        for entry in tar.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_path_buf();
            if path.ends_with("manifest.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content).unwrap();
                manifest = serde_json::from_slice(&content).unwrap();
            }
        }

        assert_eq!(manifest["embedding_model"], "nomic-embed-text");
        assert_eq!(manifest["schema_version"], "1");
        assert_eq!(manifest["package_type"], "update");
        assert_eq!(manifest["origin_workspace_id"], "ws-1");
        assert_eq!(manifest["package_id"], "pkg-1");
    }

    #[tokio::test]
    async fn import_warns_on_embedding_model_mismatch() {
        let manager = MemoryManager::new(None, None, None, None)
            .unwrap()
            .with_embedding_model("nomic-embed-text".to_string());

        let bytes = manager.export_memory("ws-source", "pkg-1").await.unwrap();

        let target = MemoryManager::new(None, None, None, None)
            .unwrap()
            .with_embedding_model("bge-m3".to_string());

        let result = target.import_memory("ws-target", &bytes).await;
        assert!(
            result.is_ok(),
            "Import should succeed with warning, not error"
        );
    }

    #[cfg(feature = "rethinkdb")]
    mod integration {
        use super::*;
        use kaigents_core::{HttpOpenAIModelClient, ModelClient, RethinkDbConfig};

        fn env_or_skip(var: &str) -> Option<String> {
            match std::env::var(var) {
                Ok(v) if !v.is_empty() => Some(v),
                _ => None,
            }
        }

        fn require_env() -> bool {
            if std::env::var("KAIGENTS_INTEGRATION_TEST").is_err() {
                eprintln!("Skipping integration test: set KAIGENTS_INTEGRATION_TEST=1 to run");
                return false;
            }
            let required = [
                "KAIGENTS_QDRANT_URL",
                "KAIGENTS_RETHINKDB_HOST",
                "KAIGENTS_MODEL_ENDPOINT_URL",
                "KAIGENTS_EMBEDDING_MODEL",
                "KAIGENTS_MODEL_NAME",
            ];
            for var in &required {
                if env_or_skip(var).is_none() {
                    eprintln!("Skipping integration test: {} not set", var);
                    return false;
                }
            }
            true
        }

        async fn create_manager() -> MemoryManager {
            std::env::set_var("KAIGENTS_MODEL_ENDPOINT_EMBEDDINGS", "true");

            let qdrant_url = env_or_skip("KAIGENTS_QDRANT_URL").unwrap();
            let model_client = HttpOpenAIModelClient::from_env().unwrap();
            let model_client_arc: Arc<dyn ModelClient> = Arc::new(model_client);
            let embedding_model = env_or_skip("KAIGENTS_EMBEDDING_MODEL").unwrap();
            let chat_model = env_or_skip("KAIGENTS_MODEL_NAME").unwrap();

            let mm = MemoryManager::new(
                Some(qdrant_url),
                Some(model_client_arc),
                Some("default".to_string()),
                Some("default".to_string()),
            )
            .unwrap()
            .with_embedding_model(embedding_model)
            .with_chat_model(chat_model);

            let rethink_cfg = RethinkDbConfig::from_env();
            mm.with_rethinkdb(&rethink_cfg).await.unwrap()
        }

        #[tokio::test]
        #[ignore]
        async fn integration_full_memory_flow_m9_to_m12() {
            if !require_env() {
                return;
            }

            let ws = format!(
                "inttest-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );
            let run_id = RunId::from_uuid(uuid::Uuid::new_v4());
            let mm = create_manager().await;

            eprintln!("=== M9: Short-term memory (record + search) ===");

            let record = MemoryRecord {
                tier: MemoryTier::Short,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "The user wants to write an essay about renewable energy sources."
                    .to_string(),
                metadata: None,
                vector: None,
            };
            let result = mm.record(record).await;
            assert!(result.is_ok(), "M9 record failed: {:?}", result);

            let record2 = MemoryRecord {
                tier: MemoryTier::Short,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Solar and wind power are the most cost-effective renewables.".to_string(),
                metadata: None,
                vector: None,
            };
            mm.record(record2).await.unwrap();

            let search_results = mm.search(&ws, "renewable energy", 10).await;
            assert!(
                search_results.is_ok(),
                "M9 search failed: {:?}",
                search_results
            );
            let search_results = search_results.unwrap();
            assert!(
                !search_results.is_empty(),
                "M9: search should find recorded memories"
            );
            eprintln!("M9: search returned {} results", search_results.len());

            eprintln!("=== M10: Long-term memory (consolidate + recall) ===");

            let episode = mm.consolidate_run_memory(&ws, &run_id).await;
            assert!(episode.is_ok(), "M10 consolidate failed: {:?}", episode);
            let episode = episode.unwrap();
            assert!(
                !episode.summary.is_empty(),
                "M10: episode summary should not be empty"
            );
            eprintln!(
                "M10: consolidated episode: {}...",
                &episode.summary[..episode.summary.len().min(80)]
            );

            let recall_results = mm.recall(&ws, "renewable", 10).await;
            assert!(
                recall_results.is_ok(),
                "M10 recall failed: {:?}",
                recall_results
            );
            let recall_results = recall_results.unwrap();
            assert!(
                !recall_results.is_empty(),
                "M10: recall should find results"
            );
            let has_episode = recall_results
                .iter()
                .any(|r| r.content.starts_with("[EPISODE]"));
            assert!(
                has_episode,
                "M10: recall should include consolidated episode"
            );
            eprintln!(
                "M10: recall returned {} results (includes episode)",
                recall_results.len()
            );

            eprintln!("=== M11: Epistemic memory (belief + close_experiment + validate) ===");

            let hyp = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content:
                    "Focusing on solar power is the best approach for renewable energy essays."
                        .to_string(),
                assumptions: vec![],
                confidence: 0.8,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let belief_id = mm.record_belief(hyp).await;
            assert!(
                belief_id.is_ok(),
                "M11 record_belief failed: {:?}",
                belief_id
            );
            let belief_id = belief_id.unwrap();
            eprintln!("M11: recorded belief {}", belief_id);

            let dep_hyp = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Solar panel efficiency data strengthens the renewable energy argument."
                    .to_string(),
                assumptions: vec![belief_id.clone()],
                confidence: 0.7,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let dep_id = mm.record_belief(dep_hyp).await.unwrap();
            eprintln!("M11: recorded dependent belief {}", dep_id);

            let outcome = BeliefOutcome {
                hypothesis_id: belief_id.clone(),
                status: HypothesisStatus::Falsified,
                justification: "Solar power focus was too narrow for the essay scope.".to_string(),
            };
            let close_result = mm.close_experiment(&ws, outcome, None).await;
            assert!(
                close_result.is_ok(),
                "M11 close_experiment failed: {:?}",
                close_result
            );
            eprintln!("M11: closed experiment (falsified) - retraction cascade triggered");

            let violations = mm.validate_approach(&ws, "solar power").await;
            assert!(
                violations.is_ok(),
                "M11 validate_approach failed: {:?}",
                violations
            );
            let violations = violations.unwrap();
            assert!(
                !violations.is_empty(),
                "M11: should find falsified hypothesis for 'solar power'"
            );
            eprintln!(
                "M11: validate_approach found {} falsified hypotheses",
                violations.len()
            );

            eprintln!("=== M12: Knowledge propagation (export + import + dedup) ===");

            let package_id = format!("pkg-{}", uuid::Uuid::new_v4().to_string().get(..8).unwrap());
            let export_result = mm.export_memory(&ws, &package_id).await;
            assert!(
                export_result.is_ok(),
                "M12 export failed: {:?}",
                export_result
            );
            let pkg_bytes = export_result.unwrap();
            assert!(
                !pkg_bytes.is_empty(),
                "M12: exported package should not be empty"
            );
            eprintln!(
                "M12: exported package {} ({} bytes)",
                package_id,
                pkg_bytes.len()
            );

            let target_ws = format!(
                "inttest-target-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );
            let import_result = mm.import_memory(&target_ws, &pkg_bytes).await;
            assert!(
                import_result.is_ok(),
                "M12 import failed: {:?}",
                import_result
            );
            let import_msg = import_result.unwrap();
            assert!(
                import_msg.contains("Imported package"),
                "M12: import result should say 'Imported package': {}",
                import_msg
            );
            eprintln!("M12: first import: {}", import_msg);

            let import_result2 = mm.import_memory(&target_ws, &pkg_bytes).await;
            assert!(
                import_result2.is_ok(),
                "M12 second import failed: {:?}",
                import_result2
            );
            let import_msg2 = import_result2.unwrap();
            assert!(
                import_msg2.contains("skipped"),
                "M12: second import should skip duplicates: {}",
                import_msg2
            );
            eprintln!("M12: second import (dedup): {}", import_msg2);

            eprintln!("=== ALL INTEGRATION TESTS PASSED (M9-M12) ===");
        }

        #[tokio::test]
        #[ignore]
        async fn integration_m11_retraction_cascade() {
            if !require_env() {
                return;
            }

            let ws = format!(
                "inttest-rc-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );
            let run_id = RunId::from_uuid(uuid::Uuid::new_v4());
            let mm = create_manager().await;

            let h1 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Base hypothesis A for cascade test.".to_string(),
                assumptions: vec![],
                confidence: 0.9,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let h1_id = mm.record_belief(h1).await.unwrap();

            let h2 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Dependent hypothesis B depends on A.".to_string(),
                assumptions: vec![h1_id.clone()],
                confidence: 0.8,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let h2_id = mm.record_belief(h2).await.unwrap();

            let h3 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Dependent hypothesis C depends on B.".to_string(),
                assumptions: vec![h2_id.clone()],
                confidence: 0.7,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let _h3_id = mm.record_belief(h3).await.unwrap();

            let outcome = BeliefOutcome {
                hypothesis_id: h1_id.clone(),
                status: HypothesisStatus::Falsified,
                justification: "Base hypothesis A is falsified.".to_string(),
            };
            mm.close_experiment(&ws, outcome, None).await.unwrap();

            let violations = mm.validate_approach(&ws, "hypothesis").await.unwrap();
            assert_eq!(
                violations.len(),
                3,
                "All 3 hypotheses should be falsified by cascade"
            );
            eprintln!(
                "M11 retraction cascade: {} hypotheses falsified (expected 3)",
                violations.len()
            );
        }

        #[tokio::test]
        #[ignore]
        async fn integration_m12_package_scoped_retraction() {
            if !require_env() {
                return;
            }

            let ws = format!(
                "inttest-psr-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );
            let run_id = RunId::from_uuid(uuid::Uuid::new_v4());
            let mm = create_manager().await;

            let pkg_id = format!(
                "pkg-scope-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );

            let h1 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Package-scoped base hypothesis.".to_string(),
                assumptions: vec![],
                confidence: 0.9,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: Some("origin-ws".to_string()),
                origin_package_id: Some(pkg_id.clone()),
                source_tier: Some("core".to_string()),
            };
            let h1_id = mm.record_belief(h1).await.unwrap();

            let h2 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Same-package dependent hypothesis.".to_string(),
                assumptions: vec![h1_id.clone()],
                confidence: 0.8,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: Some("origin-ws".to_string()),
                origin_package_id: Some(pkg_id.clone()),
                source_tier: Some("core".to_string()),
            };
            let h2_id = mm.record_belief(h2).await.unwrap();

            let h3 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Different-package dependent hypothesis.".to_string(),
                assumptions: vec![h1_id.clone()],
                confidence: 0.7,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: Some("origin-ws".to_string()),
                origin_package_id: Some("other-package".to_string()),
                source_tier: Some("core".to_string()),
            };
            let h3_id = mm.record_belief(h3).await.unwrap();

            let outcome = BeliefOutcome {
                hypothesis_id: h1_id.clone(),
                status: HypothesisStatus::Falsified,
                justification: "Base falsified via package removal.".to_string(),
            };
            mm.close_experiment(&ws, outcome, Some(&pkg_id))
                .await
                .unwrap();

            let violations = mm.validate_approach(&ws, "hypothesis").await.unwrap();
            let falsified_ids: Vec<&str> = violations
                .iter()
                .map(|h| h.id.as_deref().unwrap_or(""))
                .collect();
            assert!(
                falsified_ids.contains(&h2_id.as_str()),
                "Same-package dependent should be falsified"
            );
            assert!(
                !falsified_ids.contains(&h3_id.as_str()),
                "Different-package dependent should NOT be falsified"
            );
            eprintln!(
                "M12 package-scoped retraction: {} hypotheses falsified (h2=yes, h3=no)",
                violations.len()
            );
        }

        #[tokio::test]
        #[ignore]
        async fn integration_code_expert_agent_poc() {
            if !require_env() {
                return;
            }

            let ws = format!(
                "codeexpert-{}",
                uuid::Uuid::new_v4().to_string().get(..8).unwrap()
            );
            let run_id = RunId::from_uuid(uuid::Uuid::new_v4());
            let mm = create_manager().await;

            eprintln!("=== Code Expert Agent PoC: Assignment 1 ===");
            eprintln!("Agent attempts to sort a large dataset using bubble sort");

            let h1 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Bubble sort is efficient for sorting large datasets.".to_string(),
                assumptions: vec![],
                confidence: 0.7,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let h1_id = mm.record_belief(h1).await.unwrap();
            eprintln!(
                "Assignment 1: recorded hypothesis {} (bubble sort is efficient)",
                h1_id
            );

            let h2 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id.clone()),
                content: "Bubble sort with early termination handles nearly-sorted data well."
                    .to_string(),
                assumptions: vec![h1_id.clone()],
                confidence: 0.6,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let h2_id = mm.record_belief(h2).await.unwrap();
            eprintln!(
                "Assignment 1: recorded dependent hypothesis {} (early termination variant)",
                h2_id
            );

            eprintln!("Assignment 1: bubble sort timed out on 1M elements — falsifying hypothesis");
            let outcome = BeliefOutcome {
                hypothesis_id: h1_id.clone(),
                status: HypothesisStatus::Falsified,
                justification:
                    "Bubble sort O(n^2) timed out on 1M elements. Use O(n log n) algorithm."
                        .to_string(),
            };
            mm.close_experiment(&ws, outcome, None).await.unwrap();
            eprintln!(
                "Assignment 1: hypothesis falsified, retraction cascade triggered for dependent"
            );

            let violations = mm.validate_approach(&ws, "bubble sort").await.unwrap();
            assert_eq!(
                violations.len(),
                2,
                "Both bubble sort hypotheses should be falsified (base + dependent)"
            );
            eprintln!(
                "Assignment 1: validate_approach found {} falsified hypotheses",
                violations.len()
            );

            let has_base = violations
                .iter()
                .any(|h| h.content.contains("Bubble sort is efficient"));
            let has_dep = violations
                .iter()
                .any(|h| h.content.contains("early termination"));
            assert!(has_base, "Base hypothesis should be falsified");
            assert!(
                has_dep,
                "Dependent hypothesis should be falsified via cascade"
            );

            eprintln!("=== Code Expert Agent PoC: Assignment 2 ===");
            eprintln!("Agent starts a new assignment involving sorting");

            let violations2 = mm.validate_approach(&ws, "sort").await.unwrap();
            assert!(
                !violations2.is_empty(),
                "Assignment 2: validate_approach should surface falsified sorting hypotheses"
            );
            eprintln!(
                "Assignment 2: quality gate found {} falsified hypotheses for 'sort'",
                violations2.len()
            );

            let fitted = mm
                .assemble_context(
                    &ws,
                    "You are a code expert agent. Write efficient code.",
                    "Sort a large dataset of 1M elements.",
                    "sort",
                    4096,
                    None,
                )
                .await;
            assert!(fitted.is_ok(), "assemble_context failed: {:?}", fitted);
            let fitted = fitted.unwrap();

            let has_warning = fitted
                .messages
                .iter()
                .any(|m| m.role == "user" && m.content.contains("Precedence/Belief"));
            assert!(
                has_warning,
                "Assignment 2: assembled context should include falsified belief as precedence signal"
            );
            eprintln!("Assignment 2: context assembled with {} messages, includes falsified belief warning", fitted.messages.len());

            eprintln!("=== Code Expert Agent PoC: Explicit Re-verification ===");
            eprintln!("Agent deliberately re-verifies the bubble sort hypothesis");

            let reverify_result = mm.reverify_hypothesis(&h1_id).await;
            assert!(
                reverify_result.is_ok(),
                "reverify_hypothesis failed: {:?}",
                reverify_result
            );
            eprintln!(
                "Re-verification: {} — hypothesis re-opened for testing",
                reverify_result.unwrap()
            );

            eprintln!("=== Code Expert Agent PoC: Assignment 3 (new approach) ===");
            eprintln!("Agent records a new hypothesis about quicksort");

            let run_id3 = RunId::from_uuid(uuid::Uuid::new_v4());
            let h3 = Hypothesis {
                id: None,
                workspace_id: ws.clone(),
                run_id: Some(run_id3.clone()),
                content: "Quicksort with median-of-three pivot is efficient for large datasets."
                    .to_string(),
                assumptions: vec![],
                confidence: 0.8,
                status: HypothesisStatus::Pending,
                timestamp_ms: 0,
                origin_workspace_id: None,
                origin_package_id: None,
                source_tier: None,
            };
            let h3_id = mm.record_belief(h3).await.unwrap();
            eprintln!(
                "Assignment 3: recorded new hypothesis {} (quicksort)",
                h3_id
            );

            let outcome3 = BeliefOutcome {
                hypothesis_id: h3_id.clone(),
                status: HypothesisStatus::Confirmed,
                justification: "Quicksort sorted 1M elements in 200ms. Approach validated."
                    .to_string(),
            };
            mm.close_experiment(&ws, outcome3, None).await.unwrap();
            eprintln!("Assignment 3: quicksort hypothesis confirmed");

            let violations3 = mm.validate_approach(&ws, "quicksort").await.unwrap();
            assert!(
                violations3.is_empty(),
                "Confirmed hypotheses should not appear in validate_approach violations"
            );
            eprintln!("Assignment 3: validate_approach found 0 violations for confirmed quicksort approach");

            eprintln!("=== Code Expert Agent PoC: Summary ===");
            eprintln!("- Assignment 1: bubble sort hypothesis recorded + falsified (cascade to dependent)");
            eprintln!("- Assignment 2: quality gate surfaced falsified hypotheses, agent avoids repeating");
            eprintln!(
                "- Re-verification: explicit reverify_hypothesis re-opens falsified hypothesis"
            );
            eprintln!("- Assignment 3: new quicksort approach recorded + confirmed, no violations");
            eprintln!("=== CODE EXPERT AGENT PoC PASSED ===");
        }
    }
}
