//! File: engine/crates/kaigents-cli/src/main.rs
//! Purpose: Kaigents CLI MVP for resource lifecycle, runs, timeline rendering, and artifact fetching.
//! Product/business importance: provides a kubectl-like interface for Kaigents operations.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use clap::{Parser, Subcommand};
use kaigents_core::{
    artifacts_root_dir, default_state_dir, parse_uuid, timeline_events_path, ArtifactId,
    ArtifactKind, CancellationToken, ChatCompletionRequest, ChatMessage, DAGExecutor, EventType,
    FileArtifactStore, FileTimelineStore, FileToolContractStore, HttpMcpClient,
    HttpOpenAIModelClient, Node, NodeId, RunId, StepType, TimelineEvent, ToolPlane, DAG,
};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;

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
    },
    /// Trigger a run
    Run {
        /// Agent name
        agent: String,
        /// Input message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Show run timeline
    Timeline {
        /// Run ID
        run_id: String,
    },
    /// Fetch an artifact
    Artifact {
        /// Artifact ID
        artifact_id: String,
        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Bootstrap/install (placeholder)
    Bootstrap,

    /// Execute a Run inside a Kubernetes Job (runner entrypoint)
    Runner,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let state_dir = default_state_dir();
    let store_backend = std::env::var("KAIGENTS_STORE").unwrap_or_else(|_| "file".to_string());

    let timeline_store = FileTimelineStore::new(timeline_events_path(&state_dir))?;
    let artifact_store = FileArtifactStore::new(artifacts_root_dir(&state_dir))?;

    match cli.command {
        Commands::Apply { file } => {
            println!("Applying resource from: {}", file);
            // Placeholder: parse and apply resource
            // For MVP, we just echo
        }
        Commands::Run { agent, message } => {
            let run_id = RunId::new();
            println!("Triggering run for agent: {} (Run ID: {})", agent, run_id);

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

            // Optional: invoke a configured MCP tool (Milestone 1E smoke path)
            // KAIGENTS_MCP_SERVER_URL: full JSON-RPC HTTP endpoint
            // KAIGENTS_MCP_SERVER_NAME: logical name used for contract routing
            // KAIGENTS_MCP_TOOL: tool name to call
            if store_backend != "rethinkdb" {
                if let (Ok(server_url), Ok(tool_name)) = (
                    std::env::var("KAIGENTS_MCP_SERVER_URL"),
                    std::env::var("KAIGENTS_MCP_TOOL"),
                ) {
                    let server_name = std::env::var("KAIGENTS_MCP_SERVER_NAME")
                        .unwrap_or_else(|_| "mcp".to_string());
                    let contracts_path = state_dir.join("tool_contracts.jsonl");
                    let contract_store = FileToolContractStore::new(contracts_path)?;

                    let mut tool_plane = ToolPlane::new(Arc::new(timeline_store.clone()))
                        .with_contract_sink(Arc::new(contract_store));
                    tool_plane.register_client(
                        server_name.clone(),
                        Box::new(HttpMcpClient::new(server_name.clone(), server_url)),
                    );
                    tool_plane.refresh_contracts().await?;

                    let tool_args = match tool_name.as_str() {
                        "searxng_web_search" => {
                            let query = message.clone().unwrap_or_else(|| agent.clone());
                            serde_json::json!({"query": query, "pageno": 1})
                        }
                        "web_url_read" => {
                            let url = message
                                .clone()
                                .unwrap_or_else(|| "https://example.com".to_string());
                            serde_json::json!({"url": url})
                        }
                        _ => serde_json::json!({"agent": agent, "message": message.clone()}),
                    };

                    match tool_plane
                        .invoke_tool(
                            run_id.clone(),
                            &tool_name,
                            tool_args,
                            Duration::from_secs(20),
                        )
                        .await
                    {
                        Ok(output) => println!("MCP tool output: {}", output),
                        Err(err) => eprintln!("MCP tool error: {}", err),
                    }
                }
            }

            // Create a simple DAG with one node
            let mut dag = DAG::new();
            let node_id = NodeId::new();
            dag.add_node(Node {
                id: node_id.clone(),
                name: format!("run-{}", agent),
                step_type: StepType::Inline,
                dependencies: vec![],
            });
            let executor = DAGExecutor::new(3);
            let cancel = CancellationToken::new();

            // Execute DAG
            match executor.execute(&dag, cancel).await {
                Ok(_results) => {
                    println!("Run completed successfully.");

                    // Store a simple output artifact
                    let output = message.unwrap_or_else(|| format!("Ran agent {}", agent));

                    let (artifact, record) = if store_backend == "rethinkdb" {
                        #[cfg(feature = "rethinkdb")]
                        {
                            let cfg = RethinkDbConfig::from_env();
                            let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                            let artifact_store = RethinkDbArtifactStore::new(
                                cfg.database.clone(),
                                "artifacts".to_string(),
                                artifacts_root_dir(&state_dir),
                            )?;
                            artifact_store.ensure_schema(&mut session).await?;
                            let (artifact, record) = artifact_store.store_bytes(
                                run_id.clone(),
                                "output.txt".to_string(),
                                ArtifactKind::Output,
                                "text/plain".to_string(),
                                output.into_bytes(),
                                HashMap::new(),
                            )?;
                            artifact_store
                                .upsert_index_record(&mut session, &record)
                                .await?;
                            (artifact, record)
                        }

                        #[cfg(not(feature = "rethinkdb"))]
                        {
                            return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                        }
                    } else {
                        artifact_store.store_bytes(
                            run_id.clone(),
                            "output.txt".to_string(),
                            ArtifactKind::Output,
                            "text/plain".to_string(),
                            output.into_bytes(),
                            HashMap::new(),
                        )?
                    };

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
                    if store_backend == "rethinkdb" {
                        #[cfg(feature = "rethinkdb")]
                        {
                            let cfg = RethinkDbConfig::from_env();
                            let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                            let timeline = RethinkDbTimelineStore::default();
                            timeline.ensure_schema(&mut session).await?;
                            timeline.append(&mut session, &produced).await?;
                            timeline
                                .append(
                                    &mut session,
                                    &TimelineEvent::new(run_id.clone(), EventType::RunFinished),
                                )
                                .await?;
                        }
                        #[cfg(not(feature = "rethinkdb"))]
                        {
                            return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                        }
                    } else {
                        timeline_store.append(produced)?;
                        timeline_store
                            .append(TimelineEvent::new(run_id.clone(), EventType::RunFinished))?;
                    }

                    println!("Output artifact stored: {}", artifact.id.as_uuid());
                }
                Err(e) => {
                    eprintln!("Run failed: {}", e);
                    let finished = TimelineEvent::new(run_id.clone(), EventType::RunFinished)
                        .with_payload("status".to_string(), "failed".to_string())
                        .with_payload("error".to_string(), e);

                    if store_backend == "rethinkdb" {
                        #[cfg(feature = "rethinkdb")]
                        {
                            let cfg = RethinkDbConfig::from_env();
                            let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                            let timeline = RethinkDbTimelineStore::default();
                            timeline.ensure_schema(&mut session).await?;
                            timeline.append(&mut session, &finished).await?;
                        }
                        #[cfg(not(feature = "rethinkdb"))]
                        {
                            return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                        }
                    } else {
                        timeline_store.append(finished)?;
                    }
                }
            }
            println!("Run ID: {}", run_id);
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
        Commands::Bootstrap => {
            println!("Bootstrap/Install: placeholder");
            // Placeholder: install CRDs, set up namespace, etc.
        }

        Commands::Runner => {
            let run_id_raw = std::env::var("KAIGENTS_RUN_ID")
                .map_err(|_| "KAIGENTS_RUN_ID is required for runner mode")?;
            let run_id = RunId::from_uuid(parse_uuid(&run_id_raw)?);

            let run_input = std::env::var("KAIGENTS_RUN_INPUT")
                .or_else(|_| std::env::var("KAIGENTS_TOPIC"))
                .map_err(|_| "KAIGENTS_RUN_INPUT (topic) is required for runner mode")?;
            let topic = topic_from_run_input(&run_input);
            if topic.trim().is_empty() {
                return Err(other_error(
                    "KAIGENTS_RUN_INPUT did not provide a usable topic".to_string(),
                ));
            }

            let mcp_server_url = std::env::var("KAIGENTS_MCP_SERVER_URL")
                .map_err(|_| "KAIGENTS_MCP_SERVER_URL is required for runner mode")?;
            let mcp_server_name =
                std::env::var("KAIGENTS_MCP_SERVER_NAME").unwrap_or_else(|_| "mcp".to_string());
            let search_tool_name = std::env::var("KAIGENTS_SEARCH_TOOL_NAME")
                .unwrap_or_else(|_| "searxng_web_search".to_string());
            let read_tool_name = std::env::var("KAIGENTS_READ_TOOL_NAME")
                .unwrap_or_else(|_| "web_url_read".to_string());
            let system_prompt = std::env::var("KAIGENTS_AGENT_SYSTEM_PROMPT").unwrap_or_else(|_| {
                "You are a Student Research Assistant. Given a topic, perform web searches, read sources, and synthesize a short Markdown essay with a sources section.".to_string()
            });

            let timeline_store = FileTimelineStore::new(timeline_events_path(&state_dir))?;
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

            let contracts_path = state_dir.join("tool_contracts.jsonl");
            let contract_store = FileToolContractStore::new(contracts_path)?;
            let mut tool_plane = ToolPlane::new(Arc::new(timeline_store.clone()))
                .with_contract_sink(Arc::new(contract_store));
            tool_plane.register_client(
                mcp_server_name.clone(),
                Box::new(HttpMcpClient::new(mcp_server_name.clone(), mcp_server_url)),
            );
            tool_plane.refresh_contracts().await?;

            let model_client = HttpOpenAIModelClient::from_env()?;

            let search_results = tool_plane
                .invoke_tool(
                    run_id.clone(),
                    &search_tool_name,
                    serde_json::json!({"query": topic, "pageno": 1}),
                    Duration::from_secs(30),
                )
                .await?;

            let mut urls: Vec<String> = Vec::new();
            if let Some(results) = search_results.get("results").and_then(|v| v.as_array()) {
                for item in results.iter().take(3) {
                    if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                        urls.push(url.to_string());
                    }
                }
            }

            let mut source_texts: Vec<String> = Vec::new();
            for url in &urls {
                let read_output = tool_plane
                    .invoke_tool(
                        run_id.clone(),
                        &read_tool_name,
                        serde_json::json!({"url": url}),
                        Duration::from_secs(30),
                    )
                    .await?;
                let text = read_output
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                source_texts.push(format!("URL: {}\n{}", url, text));
            }

            let endpoint_name = std::env::var("KAIGENTS_MODEL_ENDPOINT_NAME")
                .unwrap_or_else(|_| "default".to_string());
            let model_name =
                std::env::var("KAIGENTS_MODEL_NAME").unwrap_or_else(|_| "gpt-oss-20b".to_string());

            let prompt = format!(
                "{system_prompt}\n\nWrite a short markdown essay about the topic: '{topic}'.\n\nUse the following sources (may be partial):\n\n{}\n\nOutput only markdown with a title, intro, 3-5 insight paragraphs, conclusion, and a Sources section listing the URLs.",
                source_texts.join("\n\n---\n\n")
            );

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
            let response = match model_client
                .chat_completion(
                    &endpoint_name,
                    ChatCompletionRequest {
                        model: model_name,
                        messages: vec![ChatMessage {
                            role: "user".to_string(),
                            content: prompt,
                        }],
                        max_tokens: Some(1200),
                        temperature: Some(0.4),
                        stream: true,
                    },
                    model_timeout,
                )
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let failed = TimelineEvent::new(
                        run_id.clone(),
                        EventType::ModelFailed { error: e.clone() },
                    )
                    .with_correlation(correlation_id);
                    if store_backend == "rethinkdb" {
                        #[cfg(feature = "rethinkdb")]
                        {
                            let cfg = RethinkDbConfig::from_env();
                            let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                            let timeline = RethinkDbTimelineStore::default();
                            timeline.ensure_schema(&mut session).await?;
                            timeline.append(&mut session, &failed).await?;
                        }
                        #[cfg(not(feature = "rethinkdb"))]
                        {
                            return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                        }
                    } else {
                        timeline_store.append(failed)?;
                    }
                    return Err(other_error(e));
                }
            };
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
            }

            if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                    let timeline = RethinkDbTimelineStore::default();
                    timeline.ensure_schema(&mut session).await?;
                    timeline.append(&mut session, &finished).await?;
                }
                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                timeline_store.append(finished)?;
            }

            let essay = response
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_else(|| "# Essay\n\n(no content)".to_string());

            let (artifact, record) = if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                    let artifact_store = RethinkDbArtifactStore::new(
                        cfg.database.clone(),
                        "artifacts".to_string(),
                        artifacts_root_dir(&state_dir),
                    )?;
                    artifact_store.ensure_schema(&mut session).await?;
                    let (artifact, record) = artifact_store.store_bytes(
                        run_id.clone(),
                        "essay.md".to_string(),
                        ArtifactKind::Output,
                        "text/markdown".to_string(),
                        essay.into_bytes(),
                        HashMap::new(),
                    )?;
                    artifact_store
                        .upsert_index_record(&mut session, &record)
                        .await?;
                    (artifact, record)
                }
                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                artifact_store.store_bytes(
                    run_id.clone(),
                    "essay.md".to_string(),
                    ArtifactKind::Output,
                    "text/markdown".to_string(),
                    essay.into_bytes(),
                    HashMap::new(),
                )?
            };

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
            if store_backend == "rethinkdb" {
                #[cfg(feature = "rethinkdb")]
                {
                    let cfg = RethinkDbConfig::from_env();
                    let mut session = RethinkDbTimelineStore::connect_session(&cfg).await?;
                    let timeline = RethinkDbTimelineStore::default();
                    timeline.ensure_schema(&mut session).await?;
                    timeline.append(&mut session, &produced).await?;
                    timeline
                        .append(
                            &mut session,
                            &TimelineEvent::new(run_id, EventType::RunFinished),
                        )
                        .await?;
                }
                #[cfg(not(feature = "rethinkdb"))]
                {
                    return Err("KAIGENTS_STORE=rethinkdb requires building kaigents-cli with --features rethinkdb".into());
                }
            } else {
                timeline_store.append(produced)?;
                timeline_store.append(TimelineEvent::new(run_id, EventType::RunFinished))?;
            }
        }
    }
    Ok(())
}

fn other_error(message: String) -> Box<dyn std::error::Error> {
    Box::new(io::Error::other(message))
}
