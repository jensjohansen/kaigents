//! File: engine/crates/kaigents-cli/src/main.rs
//! Purpose: Kaigents CLI MVP for resource lifecycle, runs, timeline rendering, and artifact fetching.
//! Product/business importance: provides a kubectl-like interface for Kaigents operations.
//!
//! Copyright (c) 2026 John K Johansen
//! License: MIT (see LICENSE)

use clap::{Parser, Subcommand};
use kaigents_core::{
    artifacts_root_dir, default_state_dir, parse_uuid, timeline_events_path, ArtifactId,
    ArtifactKind, CancellationToken, DAGExecutor, EventType, FileArtifactStore, FileTimelineStore,
    FileToolContractStore, HttpMcpClient, Node, NodeId, RunId, StepType, TimelineEvent, ToolPlane,
    DAG,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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

                    let _ = tool_plane
                        .invoke_tool(
                            run_id.clone(),
                            &tool_name,
                            serde_json::json!({"agent": agent, "message": message.clone()}),
                            Duration::from_secs(20),
                        )
                        .await;
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
    }
    Ok(())
}
