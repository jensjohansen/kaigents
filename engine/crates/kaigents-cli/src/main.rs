//! File: engine/crates/kaigents-cli/src/main.rs
//! Purpose: Kaigents CLI MVP for resource lifecycle, runs, timeline rendering, and artifact fetching.
//! Product/business importance: provides a kubectl-like interface for Kaigents operations.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use clap::{Parser, Subcommand};
use kaigents_core::{
    artifacts_root_dir, default_state_dir, gather_metrics, init_logging, init_metrics, parse_uuid,
    resources::ExecutionContract, timeline_events_path, ArtifactId, ArtifactKind,
    ChatCompletionRequest, ChatMessage, ConsolidationRequest, ContextBudgetStrategy,
    ContextManager, EventType, FileArtifactStore, FileTimelineStore, FileToolContractStore,
    HttpMcpClient, HttpOpenAIModelClient, RunId, StartWorkRequestRequest, TemporalAdapterClient,
    TemporalWorkItemDef, TimelineEvent, ToolPlane, MODEL_TOKENS_TOTAL, RUNS_TOTAL,
    RUN_DURATION_SECONDS, TOOL_INVOCATIONS_TOTAL,
};
use kaigents_memory::MemoryManager;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use kaigents_core::ModelClient;

fn topic_from_run_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(topic) = json.get("topic").and_then(|v| v.as_str()) {
            return topic.to_string();
        }
        if let Some(input_value) = json.get("input").and_then(|v| v.as_str()) {
            return input_value.to_string();
        }
    }

    trimmed.to_string()
}

#[cfg(feature = "rethinkdb")]
use kaigents_core::{RethinkDbArtifactStore, RethinkDbConfig, RethinkDbTimelineStore};

#[derive(Parser)]
#[command(name = "kaigents")]
#[command(about = "Kaigents CLI - Manage agents, runs, and artifacts")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply a resource (create/update)
    Apply {
        /// Resource file (YAML/JSON)
        file: String,
        /// Namespace (defaults to current context)
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Trigger a run
    Run {
        /// Target name (Agent or Process)
        target: String,
        /// Target kind (Agent or Process)
        #[arg(short, long, default_value = "Agent")]
        kind: String,
        /// Input message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Show run timeline
    Timeline {
        /// Run ID
        run_id: String,
    },
    /// Manage MCP tools
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
    /// Manage Personas
    Persona {
        #[command(subcommand)]
        command: PersonaCommands,
    },
    /// Show cluster status
    Status,
    /// Fetch an artifact
    Artifact {
        /// Artifact ID
        artifact_id: String,
        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Manage memory
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },
    /// Bootstrap/install (placeholder)
    Bootstrap,

    /// Execute a Run inside a Kubernetes Job (runner entrypoint)
    Runner,
}

#[derive(Subcommand)]
enum McpCommands {
    /// Search the curated tool catalog
    Search { query: String },
    /// Install a tool from the catalog
    Add { name: String },
    /// Pre-fetch catalog for air-gapped use
    Mirror,
}

#[derive(Subcommand)]
enum PersonaCommands {
    /// List available personas
    List,
    /// Create a new persona
    Create { file: String },
    /// Inspect a persona version
    Inspect {
        name: String,
        version: Option<String>,
    },
    /// Activate a specific persona version
    Activate { name: String, version: String },
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Export memory to a package
    Export {
        /// Workspace ID
        workspace: String,
        /// Package ID
        package: String,
        /// Output file (.kgpkg)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import memory from a package
    Import {
        /// Target workspace ID
        workspace: String,
        /// Package file (.kgpkg)
        file: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    init_metrics();
    let cli = Cli::parse();

    let state_dir = default_state_dir();
    let store_backend = std::env::var("KAIGENTS_STORE").unwrap_or_else(|_| "file".to_string());

    let timeline_store = FileTimelineStore::new(timeline_events_path(&state_dir))?;
    let artifact_store = FileArtifactStore::new(artifacts_root_dir(&state_dir))?;

    match cli.command {
        Commands::Apply { file, namespace } => {
            info!("Applying resource from: {}", file);
            let content = std::fs::read_to_string(&file)?;
            let yaml: serde_json::Value = serde_yaml::from_str(&content)?;

            let client = kube::Client::try_default().await?;
            let ns = namespace.unwrap_or_else(|| client.default_namespace().to_string());

            let kind = yaml
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or("missing kind")?;
            let name = yaml
                .get("metadata")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .ok_or("missing name")?;
            let api_version = yaml
                .get("apiVersion")
                .and_then(|v| v.as_str())
                .ok_or("missing apiVersion")?;

            // Simple-first: use DynamicObject and patch
            let parts: Vec<&str> = api_version.split('/').collect();
            let gvk = if parts.len() == 2 {
                kube::api::GroupVersionKind::gvk(parts[0], parts[1], kind)
            } else {
                kube::api::GroupVersionKind::gvk("", parts[0], kind)
            };
            let ar = kube::discovery::ApiResource::from_gvk(&gvk);

            let api: kube::Api<kube::api::DynamicObject> =
                kube::Api::namespaced_with(client, &ns, &ar);
            let patch = kube::api::Patch::Apply(&yaml);
            let params = kube::api::PatchParams::apply("kaigents-cli").force();

            api.patch(name, &params, &patch).await?;
            info!("Resource {}/{} applied in namespace {}", kind, name, ns);
        }
        Commands::Run {
            target,
            kind,
            message,
        } => {
            let run_id = RunId::new();
            info!(
                "Triggering run for {}: {} (Run ID: {})",
                kind, target, run_id
            );

            let client = kube::Client::try_default().await?;
            let ns = client.default_namespace().to_string();
            let gvk = kube::api::GroupVersionKind::gvk("core.kaigents.io", "v1alpha1", "Run");
            let ar = kube::discovery::ApiResource::from_gvk(&gvk);
            let runs: kube::Api<kube::api::DynamicObject> =
                kube::Api::namespaced_with(client, &ns, &ar);

            let run_json = serde_json::json!({
                "apiVersion": "core.kaigents.io/v1alpha1",
                "kind": "Run",
                "metadata": {
                    "name": format!("{}-run", target.to_lowercase().replace("_", "-")),
                    "generateName": format!("{}-", target.to_lowercase().replace("_", "-")),
                },
                "spec": {
                    "target": {
                        "kind": kind,
                        "name": target
                    },
                    "input": message.unwrap_or_default()
                }
            });

            let params = kube::api::PostParams::default();
            let created = runs
                .create(&params, &serde_json::from_value(run_json)?)
                .await?;
            let created_name = created.metadata.name.unwrap_or_default();

            println!("Run resource created: {}", created_name);
            println!("Run ID: {}", run_id);
        }
        Commands::Mcp { command } => {
            let catalog_url = std::env::var("KAICATALOG_URL")
                .unwrap_or_else(|_| "http://kaicatalog.kaicatalog.svc.cluster.local".to_string());
            match command {
                McpCommands::Search { query } => {
                    let client = reqwest::Client::new();
                    let res = client
                        .get(format!("{}/api/v1/catalog/search", catalog_url))
                        .query(&[("q", &query)])
                        .send()
                        .await?
                        .json::<serde_json::Value>()
                        .await?;
                    println!("{:<20} {:<40} {:<15}", "NAME", "DESCRIPTION", "POSTURE");
                    if let Some(entries) = res.as_array() {
                        for entry in entries {
                            let name = entry["metadata"]["name"].as_str().unwrap_or_default();
                            let desc = entry["spec"]["description"].as_str().unwrap_or_default();
                            let posture =
                                entry["spec"]["runtimePosture"].as_str().unwrap_or_default();
                            println!("{:<20} {:<40} {:<15}", name, desc, posture);
                        }
                    }
                }
                McpCommands::Add { name } => {
                    let client = reqwest::Client::new();
                    let res = client
                        .get(format!("{}/api/v1/catalog/entries/{}", catalog_url, name))
                        .send()
                        .await?;
                    if res.status().is_success() {
                        let entry = res.json::<serde_json::Value>().await?;
                        println!("Adding MCP tool: {}", name);
                        if let Some(manifest) = entry["manifest"].as_str() {
                            println!("Manifest found. Use 'kaigents apply' to install it.");
                            println!("---");
                            println!("{}", manifest);
                        } else {
                            println!("Metadata: {}", serde_json::to_string_pretty(&entry)?);
                        }
                    } else {
                        error!("Tool not found in catalog: {}", name);
                    }
                }
                McpCommands::Mirror => {
                    println!("Mirroring catalog to local registry (placeholder)...");
                }
            }
        }
        Commands::Persona { command } => {
            let manager_url = std::env::var("KAIMANAGER_URL")
                .unwrap_or_else(|_| "http://kaimanager.kaimanager.svc.cluster.local".to_string());
            match command {
                PersonaCommands::List => {
                    let client = reqwest::Client::new();
                    let res = client
                        .get(format!("{}/api/v1/personas", manager_url))
                        .send()
                        .await?
                        .json::<serde_json::Value>()
                        .await?;
                    println!(
                        "{:<20} {:<40} {:<10} {:<10}",
                        "NAME", "DESCRIPTION", "VERSION", "PHASE"
                    );
                    if let Some(personas) = res.as_array() {
                        for p in personas {
                            let name = p["metadata"]["name"].as_str().unwrap_or_default();
                            let desc = p["spec"]["description"].as_str().unwrap_or_default();
                            let version = p["status"]["version"].as_str().unwrap_or_default();
                            let phase = p["status"]["phase"].as_str().unwrap_or_default();
                            println!("{:<20} {:<40} {:<10} {:<10}", name, desc, version, phase);
                        }
                    }
                }
                PersonaCommands::Create { file } => {
                    let content = std::fs::read_to_string(&file)?;
                    let persona: serde_json::Value = serde_yaml::from_str(&content)?;
                    let client = reqwest::Client::new();
                    let res = client
                        .post(format!("{}/api/v1/personas", manager_url))
                        .json(&persona)
                        .send()
                        .await?;
                    if res.status().is_success() {
                        println!("Persona created successfully.");
                    } else {
                        error!("Failed to create persona: {}", res.text().await?);
                    }
                }
                PersonaCommands::Inspect { name, version } => {
                    let client = reqwest::Client::new();
                    let url = if let Some(_v) = version {
                        format!("{}/api/v1/personas/{}/versions", manager_url, name)
                    // Simplified, usually search version in history
                    } else {
                        format!("{}/api/v1/personas/{}", manager_url, name)
                    };

                    let res = client.get(url).send().await?;
                    if res.status().is_success() {
                        let body = res.json::<serde_json::Value>().await?;
                        if body.is_array() {
                            // If versions were requested, show list
                            println!("{:<20} {:<10}", "VERSION", "PHASE");
                            for v in body.as_array().unwrap() {
                                println!(
                                    "{:<20} {:<10}",
                                    v["status"]["version"].as_str().unwrap_or_default(),
                                    v["status"]["phase"].as_str().unwrap_or_default()
                                );
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&body)?);
                        }
                    } else {
                        error!("Failed to inspect persona: {}", res.text().await?);
                    }
                }
                PersonaCommands::Activate { name, version } => {
                    let client = reqwest::Client::new();
                    let payload = serde_json::json!({ "version": version });
                    let res = client
                        .post(format!("{}/api/v1/personas/{}/activate", manager_url, name))
                        .json(&payload)
                        .send()
                        .await?;
                    if res.status().is_success() {
                        println!("Persona {} activated with version {}.", name, version);
                    } else {
                        error!("Failed to activate persona: {}", res.text().await?);
                    }
                }
            }
        }
        Commands::Status => {
            println!("Kaigents Cluster Status (placeholder)");
        }
        Commands::Timeline { run_id } => {
            let run_id = RunId::from_uuid(parse_uuid(&run_id)?);
            println!("Timeline for run: {}", run_id);

            let events = if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                    let timeline = RethinkDbTimelineStore::default();
                    timeline.ensure_schema(&mut session).await?;
                    timeline.query_by_run(&mut session, &run_id).await?
                }
                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                timeline_store.query_by_run(&run_id)?
            };
            if events.is_empty() {
                println!("No events found for this run.");
            } else {
                for event in events {
                    println!("{}: {:?}", event.timestamp_ms, event.event_type);
                    if !event.payload.is_empty() {
                        println!("  payload: {}", serde_json::to_string(&event.payload)?);
                    }
                }
            }
        }
        Commands::Artifact {
            artifact_id,
            output,
        } => {
            let artifact_id = ArtifactId::from_uuid(parse_uuid(&artifact_id)?);
            println!("Fetching artifact: {}", artifact_id.as_uuid());

            let bytes = if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let artifact_store = RethinkDbArtifactStore::new(
                        cfg.database.clone(),
                        "artifacts".to_string(),
                        artifacts_root_dir(&state_dir),
                    )?;
                    artifact_store.retrieve_bytes(&artifact_id)?
                }
                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                artifact_store.retrieve_bytes(&artifact_id)?
            };
            match output {
                Some(output_path) => {
                    std::fs::write(&output_path, &bytes)?;
                    println!("Wrote {} bytes to {}", bytes.len(), output_path);
                }
                None => {
                    // Print as UTF-8 if possible, else show size.
                    match String::from_utf8(bytes) {
                        Ok(text) => print!("{}", text),
                        Err(e) => println!("Artifact is binary ({} bytes)", e.into_bytes().len()),
                    }
                }
            }
        }
        Commands::Memory { command } => {
            let qdrant_url = std::env::var("KAIGENTS_QDRANT_URL").ok();
            let embedding_model = std::env::var("KAIGENTS_EMBEDDING_MODEL").ok();
            let chat_model = std::env::var("KAIGENTS_MODEL_NAME").ok();
            let mut mm = MemoryManager::new(qdrant_url, None, None, None)?;
            if let Some(model) = embedding_model {
                mm = mm.with_embedding_model(model);
            }
            if let Some(model) = chat_model {
                mm = mm.with_chat_model(model);
            }
            #[cfg(feature = "rethinkdb")]
            if store_backend == "rethinkdb" {
                let rethink_cfg = RethinkDbConfig::from_env();
                mm = mm.with_rethinkdb(&rethink_cfg).await?;
            }

            match command {
                MemoryCommands::Export {
                    workspace,
                    package,
                    output,
                } => {
                    let bytes = mm.export_memory(&workspace, &package).await?;
                    let out_path = output.unwrap_or_else(|| format!("{}.kgpkg", package));
                    std::fs::write(&out_path, bytes)?;
                    println!("Memory exported to {}", out_path);
                }
                MemoryCommands::Import { workspace, file } => {
                    let bytes = std::fs::read(&file)?;
                    let res = mm.import_memory(&workspace, &bytes).await?;
                    println!("{}", res);
                }
            }
        }
        Commands::Bootstrap => {
            println!("Bootstrap/Install: placeholder");
            // Placeholder: install CRDs, set up namespace, etc.
        }

        Commands::Runner => {
            let metrics_port =
                std::env::var("KAIGENTS_METRICS_PORT").unwrap_or_else(|_| "9090".to_string());
            let server = tiny_http::Server::http(format!("0.0.0.0:{}", metrics_port)).unwrap();
            std::thread::spawn(move || {
                for request in server.incoming_requests() {
                    let response = tiny_http::Response::from_string(gather_metrics());
                    let _ = request.respond(response);
                }
            });

            let run_timer = RUN_DURATION_SECONDS.start_timer();

            // Load Execution Contract
            let json_str = std::env::var("KAIGENTS_EXECUTION_CONTRACT")
                .map_err(|_| "KAIGENTS_EXECUTION_CONTRACT is required")?;
            let contract = serde_json::from_str::<ExecutionContract>(&json_str)
                .map_err(|e| format!("Failed to parse KAIGENTS_EXECUTION_CONTRACT: {}", e))?;

            let run_id = RunId::from_uuid(parse_uuid(&contract.run_id)?);
            let target_kind = contract.target_kind.clone();
            let target_name = contract.target_name.clone();
            let run_input = contract.input.clone();

            let model_client = HttpOpenAIModelClient::from_contract(&contract)?;
            let model_client_arc: Arc<dyn kaigents_core::ModelClient> =
                Arc::new(model_client.clone());

            let qdrant_url = std::env::var("KAIGENTS_QDRANT_URL").ok();
            let embedding_endpoint = contract.model_endpoint_name.clone();
            let chat_endpoint = contract.model_endpoint_name.clone();
            let embedding_model = std::env::var("KAIGENTS_EMBEDDING_MODEL")
                .unwrap_or_else(|_| embedding_endpoint.clone().unwrap_or_default());
            let chat_model = contract
                .model_name
                .clone()
                .unwrap_or_else(|| std::env::var("KAIGENTS_MODEL_NAME").unwrap_or_else(|_| "gpt-oss-20b".to_string()));
            let memory_manager = {
                #[allow(unused_mut)]
                let mut mm = MemoryManager::new(
                    qdrant_url,
                    Some(model_client_arc),
                    embedding_endpoint,
                    chat_endpoint,
                )?
                .with_embedding_model(embedding_model)
                .with_chat_model(chat_model);

                #[cfg(feature = "rethinkdb")]
                if store_backend == "rethinkdb" {
                    let rethink_cfg = kaigents_core::rethinkdb_store::RethinkDbConfig::from_env();
                    mm = mm.with_rethinkdb(&rethink_cfg).await?;
                }
                mm
            };

            let memory_manager = Arc::new(memory_manager);
            let context_manager = ContextManager::new();

            let temporal_adapter_url = std::env::var("KAIGENTS_TEMPORAL_ADAPTER_URL").ok();
            if temporal_adapter_url.is_some() {
                let memory_api_port = std::env::var("KAIGENTS_MEMORY_API_PORT")
                    .unwrap_or_else(|_| "8090".to_string())
                    .parse::<u16>()
                    .unwrap_or(8090);
                serve_memory_api(memory_manager.clone(), memory_api_port);
                info!("Temporal consolidation enabled — memory API server started for workflow activities");
            }

            RUNS_TOTAL
                .with_label_values(&[&target_kind, "started"])
                .inc();

            let client = kube::Client::try_default().await?;
            let ns = client.default_namespace();

            let steps = if target_kind == "Process" {
                let processes: kube::Api<kaigents_core::resources::Process> =
                    kube::Api::namespaced(client.clone(), ns);
                let process = processes.get(&target_name).await?;

                let mut steps = Vec::new();
                let tasks: kube::Api<kaigents_core::resources::Task> =
                    kube::Api::namespaced(client.clone(), ns);

                for step_def in process.spec.steps {
                    let task = tasks.get(&step_def.task_ref).await?;
                    steps.push(TemporalWorkItemDef {
                        work_item_id: format!("{}-{}", run_id.as_uuid(), step_def.id),
                        step_name: step_def.name,
                        agent_name: task.spec.agent_name,
                        prompt: task.spec.prompt.map(|p| p.replace("{{input}}", &run_input)),
                        requires_gate: task.spec.requires_gate,
                    });
                }
                steps
            } else {
                // Default Agent behavior (1 step)
                vec![TemporalWorkItemDef {
                    work_item_id: format!("{}-exec", run_id.as_uuid()),
                    step_name: "execute".to_string(),
                    agent_name: Some(target_name.clone()),
                    prompt: Some(run_input.clone()),
                    requires_gate: None,
                }]
            };

            let timeline_store = FileTimelineStore::new(timeline_events_path(&state_dir))?;
            // ... existing logic ...
            let artifact_store = FileArtifactStore::new(artifacts_root_dir(&state_dir))?;

            if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                    let timeline = RethinkDbTimelineStore::default();
                    timeline.ensure_schema(&mut session).await?;
                    timeline
                        .append(
                            &mut session,
                            &TimelineEvent::new(run_id.clone(), EventType::RunStarted),
                        )
                        .await?;
                }

                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                timeline_store.append(TimelineEvent::new(run_id.clone(), EventType::RunStarted))?;
            }

            if let Ok(adapter_url) = std::env::var("KAIGENTS_TEMPORAL_ADAPTER_URL") {
                let adapter = TemporalAdapterClient::new(adapter_url);
                let req = StartWorkRequestRequest {
                    work_request_id: run_id.as_uuid().to_string(),
                    process_name: Some(target_name.clone()),
                    steps,
                };
                adapter.start_work_request(req).await?;
                info!("WorkRequest started via Temporal adapter.");

                // Poll for completion (simple-first for MVP)
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    match adapter
                        .query_work_request(&run_id.as_uuid().to_string())
                        .await
                    {
                        Ok(state) => {
                            info!(
                                "WorkRequest state: {} (Step: {})",
                                state.phase,
                                state.current_step.unwrap_or_default()
                            );
                            if state.phase == "Succeeded" {
                                timeline_store.append(TimelineEvent::new(
                                    run_id.clone(),
                                    EventType::RunFinished,
                                ))?;
                                info!("Run completed successfully.");
                                RUNS_TOTAL
                                    .with_label_values(&[&target_kind, "succeeded"])
                                    .inc();
                                run_timer.observe_duration();
                                break;
                            }
                            if state.phase == "Failed" {
                                let error =
                                    state.message.unwrap_or_else(|| "unknown error".to_string());
                                RUNS_TOTAL
                                    .with_label_values(&[&target_kind, "failed"])
                                    .inc();
                                run_timer.observe_duration();
                                timeline_store.append(
                                    TimelineEvent::new(run_id.clone(), EventType::RunFinished)
                                        .with_payload("status".to_string(), "failed".to_string())
                                        .with_payload("error".to_string(), error.clone()),
                                )?;
                                return Err(other_error(error));
                            }
                        }
                        Err(e) => {
                            error!("Error querying WorkRequest: {}", e);
                        }
                    }
                }
                return Ok(());
            }

            // Fallback: Solo Mode execution (embedded logic)
            // Supports any Agent persona — tool configuration is driven by the execution contract.
            if target_kind == "Agent" {
                let topic = topic_from_run_input(&run_input);
                let mcp_server_url = contract.mcp_server_url.clone();
                let mcp_server_name = contract
                    .mcp_server_name
                    .clone()
                    .unwrap_or_else(|| "mcp".to_string());
                let search_tool_name = contract
                    .search_tool_name
                    .clone()
                    .unwrap_or_else(|| "searxng_web_search".to_string());
                let read_tool_name = contract
                    .read_tool_name
                    .clone()
                    .unwrap_or_else(|| "web_url_read".to_string());
                let system_prompt = contract.system_prompt.clone().unwrap_or_else(|| {
                    "You are a Kaigents AI Agent. Complete the task as specified in the input.".to_string()
                });

                let mcp_timeout_ms: u64 = std::env::var("KAIGENTS_MCP_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(30000);
                let mcp_timeout = Duration::from_millis(mcp_timeout_ms);

                let contracts_path = state_dir.join("tool_contracts.jsonl");
                let contract_store = FileToolContractStore::new(contracts_path)?;
                let mut tool_plane = ToolPlane::new(Arc::new(timeline_store.clone()))
                    .with_contract_sink(Arc::new(contract_store));

                if let Some(ref mcp_url) = mcp_server_url {
                    tool_plane.register_client(
                        mcp_server_name.clone(),
                        Box::new(HttpMcpClient::new(mcp_server_name.clone(), mcp_url.clone())),
                    );
                }
                // Always register memory tool client
                tool_plane.register_client(
                    "kaigents-memory".to_string(),
                    Box::new(kaigents_memory::InternalMemoryToolClient::new(
                        memory_manager.clone(),
                    )),
                );
                tool_plane.refresh_contracts().await?;

                let mut source_texts: Vec<String> = Vec::new();

                if mcp_server_url.is_some() {
                    let search_results = tool_plane
                        .invoke_tool(
                            run_id.clone(),
                            &search_tool_name,
                            serde_json::json!({"query": topic, "pageno": 1}),
                            mcp_timeout,
                        )
                        .await?;
                    TOOL_INVOCATIONS_TOTAL
                        .with_label_values(&[&search_tool_name, "succeeded"])
                        .inc();

                    let mut urls: Vec<String> = Vec::new();
                    if let Some(results) = search_results.get("results").and_then(|v| v.as_array()) {
                        for item in results.iter().take(3) {
                            if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                                urls.push(url.to_string());
                            }
                        }
                    }

                    for url in &urls {
                        let read_output = tool_plane
                            .invoke_tool(
                                run_id.clone(),
                                &read_tool_name,
                                serde_json::json!({"url": url}),
                                mcp_timeout,
                            )
                            .await?;
                        TOOL_INVOCATIONS_TOTAL
                            .with_label_values(&[&read_tool_name, "succeeded"])
                            .inc();
                        let text = read_output
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        source_texts.push(format!("URL: {}\n{}", url, text));
                    }
                }

                let endpoint_name = contract
                    .model_endpoint_name
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let model_name = contract
                    .model_name
                    .clone()
                    .unwrap_or_else(|| "gpt-oss-20b".to_string());

                let workspace_id = std::env::var("KAIGENTS_WORKSPACE_ID")
                    .unwrap_or_else(|_| "default".to_string());
                let historical_memories = memory_manager
                    .recall(&workspace_id, &topic, 5)
                    .await
                    .unwrap_or_default();

                let mut case_file_entries: Vec<String> = Vec::new();
                let mut episodes: Vec<String> = Vec::new();
                let mut beliefs: Vec<String> = Vec::new();

                for m in historical_memories {
                    let mem_type = m
                        .metadata
                        .as_ref()
                        .and_then(|v| v.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("short-term");

                    match mem_type {
                        "long-term" => episodes.push(m.content),
                        "epistemic" => beliefs.push(m.content),
                        _ => case_file_entries.push(m.content),
                    }
                }

                // Add current source texts to case file for budgeting
                case_file_entries.extend(source_texts);

                let task_state = format!("Writing an essay about '{}'.", topic);
                let budget = contract.context_window_size.unwrap_or(4096);

                // Epistemic Quality Gate: Check for falsified approaches matching the topic
                let violations = memory_manager
                    .validate_approach(&workspace_id, &topic)
                    .await
                    .unwrap_or_default();
                let mut fitted = context_manager.fit_to_budget(
                    &system_prompt,
                    &task_state,
                    episodes,
                    case_file_entries,
                    beliefs,
                    budget,
                    ContextBudgetStrategy::Truncate,
                );

                if !violations.is_empty() {
                    warn!("Epistemic Quality Gate triggered: {} falsified hypotheses found for topic '{}'", violations.len(), topic);
                    let mut warning_text = "WARNING: The following approaches have been PREVIOUSLY FALSIFIED for this topic:\n".to_string();
                    for v in violations {
                        warning_text
                            .push_str(&format!("- {} (Confidence: {})\n", v.content, v.confidence));
                    }
                    warning_text.push_str(
                        "DO NOT repeat these failed approaches. Pivot to a different strategy.",
                    );

                    fitted.messages.insert(
                        1,
                        ChatMessage {
                            role: "system".to_string(),
                            content: warning_text,
                        },
                    );
                }

                timeline_store.append(TimelineEvent::new(
                    run_id.clone(),
                    EventType::ContextAssembled {
                        budget,
                        total_tokens: fitted.total_estimated_tokens,
                        dropped_count: fitted.dropped_entries_count,
                    },
                ))?;

                let model_timeout_secs: u64 = std::env::var("KAIGENTS_MODEL_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(600);
                let model_timeout = Duration::from_secs(model_timeout_secs);
                let correlation_id = format!("chat-{}", uuid::Uuid::new_v4());
                let invoked = TimelineEvent::new(
                    run_id.clone(),
                    EventType::ModelInvoked {
                        endpoint: endpoint_name.clone(),
                    },
                )
                .with_correlation(correlation_id.clone())
                .with_payload("model".to_string(), model_name.clone())
                .with_payload(
                    "timeout_ms".to_string(),
                    model_timeout.as_millis().to_string(),
                );
                timeline_store.append(invoked)?;

                let model_start = std::time::Instant::now();
                let response = model_client
                    .chat_completion(
                        &endpoint_name,
                        ChatCompletionRequest {
                            model: model_name.clone(),
                            messages: fitted.messages,
                            max_tokens: Some(1200),
                            temperature: Some(0.4),
                            stream: true,
                        },
                        model_timeout,
                    )
                    .await
                    .map_err(other_error)?;

                let latency_ms = model_start.elapsed().as_millis().to_string();

                let mut finished = TimelineEvent::new(run_id.clone(), EventType::ModelFinished)
                    .with_correlation(correlation_id)
                    .with_payload("latency_ms".to_string(), latency_ms);
                if let Some(usage) = &response.usage {
                    finished = finished
                        .with_payload("prompt_tokens".to_string(), usage.prompt_tokens.to_string())
                        .with_payload(
                            "completion_tokens".to_string(),
                            usage.completion_tokens.to_string(),
                        )
                        .with_payload("total_tokens".to_string(), usage.total_tokens.to_string());

                    MODEL_TOKENS_TOTAL
                        .with_label_values(&[&model_name, "prompt"])
                        .inc_by(usage.prompt_tokens as u64);
                    MODEL_TOKENS_TOTAL
                        .with_label_values(&[&model_name, "completion"])
                        .inc_by(usage.completion_tokens as u64);
                }
                timeline_store.append(finished)?;

                let essay = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_else(|| "# Essay\n\n(no content)".to_string());

                // Record the essay to memory
                let memory_record = kaigents_memory::MemoryRecord {
                    tier: kaigents_memory::MemoryTier::Short,
                    workspace_id: workspace_id.clone(),
                    run_id: Some(run_id.clone()),
                    content: format!("Essay about '{}':\n{}", topic, essay),
                    metadata: None,
                    vector: None, // Will be generated by MemoryManager
                };
                if let Err(e) = memory_manager.record(memory_record).await {
                    error!("Failed to record essay to memory: {}", e);
                }

                let (artifact, record) = artifact_store.store_bytes(
                    run_id.clone(),
                    "essay.md".to_string(),
                    ArtifactKind::Output,
                    "text/markdown".to_string(),
                    essay.into_bytes(),
                    HashMap::new(),
                )?;

                let produced = TimelineEvent::new(
                    run_id.clone(),
                    EventType::ArtifactProduced {
                        artifact_id: artifact.id.as_uuid().to_string(),
                    },
                )
                .with_correlation(format!("artifact-{}", artifact.id.as_uuid()))
                .with_payload("name".to_string(), record.name)
                .with_payload("mime_type".to_string(), record.mime_type)
                .with_payload("size_bytes".to_string(), record.size_bytes.to_string())
                .with_payload("blob_path".to_string(), record.blob_path);

                timeline_store.append(produced)?;

                // Consolidate short-term memories from this run into a long-term episode.
                // If a Temporal adapter is configured, trigger the durable consolidation workflow.
                // Otherwise, fall back to in-process consolidation.
                if let Some(adapter_url) = &temporal_adapter_url {
                    let adapter = TemporalAdapterClient::new(adapter_url);
                    let cons_req = ConsolidationRequest {
                        workspace_id: workspace_id.clone(),
                        run_id: run_id.as_uuid().to_string(),
                    };
                    match adapter.start_consolidation(cons_req).await {
                        Ok(state) => {
                            info!(
                                "Temporal consolidation started: {} (phase: {})",
                                state.consolidation_id, state.phase
                            );
                            let episode_id = state.episode_id.unwrap_or_default();
                            let consolidated = TimelineEvent::new(
                                run_id.clone(),
                                EventType::MemoryConsolidated { episode_id },
                            )
                            .with_correlation(format!("consolidation-{}", run_id.as_uuid()));
                            timeline_store.append(consolidated)?;
                        }
                        Err(e) => {
                            warn!(
                                "Temporal consolidation failed, falling back to in-process: {}",
                                e
                            );
                            match memory_manager
                                .consolidate_run_memory(&workspace_id, &run_id)
                                .await
                            {
                                Ok(episode) => {
                                    let episode_id = episode.id.unwrap_or_default();
                                    info!("In-process consolidation succeeded: episode {}", episode_id);
                                    let consolidated = TimelineEvent::new(
                                        run_id.clone(),
                                        EventType::MemoryConsolidated { episode_id },
                                    )
                                    .with_correlation(format!("consolidation-{}", run_id.as_uuid()));
                                    timeline_store.append(consolidated)?;
                                }
                                Err(e2) => {
                                    warn!("In-process consolidation also failed: {}", e2);
                                }
                            }
                        }
                    }
                } else {
                    match memory_manager
                        .consolidate_run_memory(&workspace_id, &run_id)
                        .await
                    {
                        Ok(episode) => {
                            let episode_id = episode.id.unwrap_or_default();
                            info!("Consolidated run {} into episode {}", run_id, episode_id);
                            let consolidated = TimelineEvent::new(
                                run_id.clone(),
                                EventType::MemoryConsolidated { episode_id },
                            )
                            .with_correlation(format!("consolidation-{}", run_id.as_uuid()));
                            timeline_store.append(consolidated)?;
                        }
                        Err(e) => {
                            warn!("Failed to consolidate run memory into episode: {}", e);
                        }
                    }
                }

                timeline_store.append(TimelineEvent::new(run_id, EventType::RunFinished))?;

                info!("Solo Mode execution completed.");
                RUNS_TOTAL
                    .with_label_values(&[&target_kind, "succeeded"])
                    .inc();
                run_timer.observe_duration();
                return Ok(());
            }

            return Err(other_error(format!(
                "No execution path for {}/{}",
                target_kind, target_name
            )));
        }
    }
    Ok(())
}

fn other_error(message: String) -> Box<dyn std::error::Error> {
    Box::new(io::Error::other(message))
}

fn serve_memory_api(mm: Arc<MemoryManager>, port: u16) {
    let server = match tiny_http::Server::http(format!("0.0.0.0:{}", port)) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to start memory API server on port {}: {}", port, e);
            return;
        }
    };
    info!("Memory API server listening on port {}", port);

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime for memory API: {}", e);
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().as_str().to_string();

            if url == "/api/v1/memory/record" && method == "POST" {
                let mut body = String::new();
                if request.as_reader().read_to_string(&mut body).is_err() {
                    let _ = request.respond(tiny_http::Response::from_string("Invalid body"));
                    continue;
                }

                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(params) => {
                        let workspace_id = params["workspace_id"].as_str().unwrap_or("default");
                        let content = params["content"].as_str().unwrap_or("");
                        let tier = params["tier"].as_str().unwrap_or("short");

                        let record = kaigents_memory::MemoryRecord {
                            tier: if tier == "long" {
                                kaigents_memory::MemoryTier::Long
                            } else {
                                kaigents_memory::MemoryTier::Short
                            },
                            workspace_id: workspace_id.to_string(),
                            run_id: None,
                            content: content.to_string(),
                            metadata: None,
                            vector: None,
                        };

                        let mm_clone = mm.clone();
                        let result = runtime.block_on(async move {
                            mm_clone.record(record).await
                        });

                        match result {
                            Ok(id) => {
                                let _ = request.respond(tiny_http::Response::from_string(
                                    serde_json::json!({ "id": id, "status": "recorded" }).to_string(),
                                ).with_header(tiny_http::Header::from_bytes(
                                    b"Content-Type", b"application/json"
                                ).unwrap()));
                            }
                            Err(e) => {
                                let _ = request.respond(tiny_http::Response::from_string(
                                    serde_json::json!({ "error": e }).to_string(),
                                ).with_status_code(500));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = request.respond(tiny_http::Response::from_string(
                            serde_json::json!({ "error": format!("Invalid JSON: {}", e) }).to_string(),
                        ).with_status_code(400));
                    }
                }
            } else if url == "/api/v1/memory/query" && method == "POST" {
                let mut body = String::new();
                if request.as_reader().read_to_string(&mut body).is_err() {
                    let _ = request.respond(tiny_http::Response::from_string("Invalid body"));
                    continue;
                }

                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(params) => {
                        let workspace_id = params["workspace_id"].as_str().unwrap_or("default");
                        let query = params["query"].as_str().unwrap_or("");
                        let limit = params["limit"].as_u64().unwrap_or(10);

                        let mm_clone = mm.clone();
                        let result = runtime.block_on(async move {
                            mm_clone.recall(workspace_id, query, limit).await
                        });

                        match result {
                            Ok(results) => {
                                let json_results: Vec<serde_json::Value> = results
                                    .iter()
                                    .map(|r| serde_json::json!({
                                        "content": r.content,
                                        "score": r.score,
                                        "run_id": r.run_id,
                                    }))
                                    .collect();
                                let _ = request.respond(tiny_http::Response::from_string(
                                    serde_json::json!({ "results": json_results }).to_string(),
                                ).with_header(tiny_http::Header::from_bytes(
                                    b"Content-Type", b"application/json"
                                ).unwrap()));
                            }
                            Err(e) => {
                                let _ = request.respond(tiny_http::Response::from_string(
                                    serde_json::json!({ "error": e }).to_string(),
                                ).with_status_code(500));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = request.respond(tiny_http::Response::from_string(
                            serde_json::json!({ "error": format!("Invalid JSON: {}", e) }).to_string(),
                        ).with_status_code(400));
                    }
                }
            } else {
                let _ = request.respond(tiny_http::Response::from_string(
                    serde_json::json!({ "error": "Not found" }).to_string(),
                ).with_status_code(404));
            }
        }
    });
}
