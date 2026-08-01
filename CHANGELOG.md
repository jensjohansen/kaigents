# Changelog (Kaigents)

All notable changes to this project will be documented in this file.

## [1.3.0] - 2026-08-01

### Added
- **NebulaGraph temporal graph layer (R15/R12)**: Full `NebulaGraphStore` implementation using NebulaGraph's HTTP API (port 19669) via reqwest. Schema initialization (space, `entity` tag, `depends_on`/`consolidated_from` edges with `valid_from`/`valid_to`/`transaction_time` temporal fields). `with_nebula()` connects and initializes schema with graceful fallback to RethinkDB on connection failure. `record_belief` inserts entity vertices and `depends_on` temporal edges for assumptions. `close_experiment` uses `traverse_dependents_recursive` for graph-traversal-based retraction cascades. `consolidate_run_memory` inserts `consolidated_from` temporal edges. As-of queries, edge invalidation, and recursive graph traversal all implemented. 4 new unreachable tests.
- **K8sOffload pod submission (R15/G6)**: `submit_k8s_offload` in `dag.rs` creates real Kubernetes Pods via the `kube` crate, polls for Succeeded/Failed with 600s timeout, cleans up pods, returns `ExecutionResult`.
- **Task, Process, MemoryPolicy controllers (R15/G7)**: Three new Go reconcilers following the `AgentReconciler` pattern, registered in `setup.go`.
- **Temporal durability tests (R15/R11)**: 4 tests verifying all Temporal adapter HTTP calls return "unreachable" errors when the adapter is down.
- **Semantic similarity dedup (R15/M12)**: `check_semantic_duplicate` generates embeddings and searches Qdrant (score > 0.95) before falling back to exact text match in `import_memory`.

### Fixed
- **CRD YAML schema drift (R15/G11)**: `memorypolicies.yaml`, `agents.yaml`, `runs.yaml` updated to match Go struct definitions (added `conditions` to status, `contextBudgetStrategy`/`preferredContextWindow` to routing, `outputs` to run status).

### Changed
- All implementation deviations in `agent-memory-proposal.md` §13 are now resolved (ITD-18, ITD-19, ITD-20, consolidation durability).
- Test coverage: 53 core tests (including 4 NebulaGraph + 4 Temporal durability) + 19 memory unit tests + 4 integration tests = 76 total, all passing. Both default and rethinkdb builds, zero warnings.
- No deferred or future items remain. The codebase delivers completely to the PRD, tech design, and implementation plan.

## [1.2.0] - 2026-08-01

### Added
- **Context Manager v2 (R9)**: `Summarize` strategy with sync `simple_compress` + async `SummaryProvider` for LLM-backed summarization; `Error` strategy with `budget_exceeded` flag; `ContextTier` enum (Core/Recall/Archival) with hierarchical demotion; `RoutingPolicy` with `select_model_for_context` for context-budget-aware model routing; `fit_to_budget_tiered` for explicit tier-based context assembly. 12 new tests.
- **Code Expert Agent PoC (R10)**: Integration test demonstrating 3 assignments with falsification, retraction cascade, quality gate, context assembly with warnings, re-verification, and confirmed approach.
- **Temporal-based consolidation (R11)**: `ConsolidationRequest`/`ConsolidationState` types and `start_consolidation`/`query_consolidation` methods on `TemporalAdapterClient`; `serve_memory_api` HTTP server with `/api/v1/memory/record` and `/api/v1/memory/query` endpoints; runner triggers Temporal consolidation when `KAIGENTS_TEMPORAL_ADAPTER_URL` is set, with in-process fallback. 4 new tests.
- **DAG dependency ordering (R13-G5)**: Rewrote `DAGExecutor::execute` to enforce that nodes only spawn after all dependencies complete, while preserving concurrent execution of independent nodes.
- **Artifact retrieval (R13-G4)**: `ArtifactPlane::retrieve_artifact` implemented with in-memory index mapping `ArtifactId` to `ArtifactStorageRef`.
- **Run status outputs (R13-G8)**: `RunStatus.Outputs` field added to Run CRD; run controller reads ConfigMap `{run-name}-outputs` and populates outputs.
- **MCP timeout enforcement (R13-G12)**: `HttpMcpClient::call_tool` wraps JSON-RPC with `tokio::time::timeout`; configurable via `KAIGENTS_MCP_TIMEOUT_MS` env var.
- **Package format enhancement (R14)**: `policy.yaml` and `distilled-lessons.md` now included in `.kgpkg` package.

### Fixed
- **Qdrant scroll pagination in consolidation (R13-G3)**: Added pagination loop to `consolidate_run_memory` to fetch all points, not just the first batch.
- **Solo runner persona support (R13-G9)**: Removed hardcoded "Research" check; MCP tools and search/read steps are now optional based on contract configuration; any Agent persona can run.
- **Model activity failure propagation (R13-G10)**: `ExecuteWorkItem` activity now returns error on model failure, allowing Temporal to retry and workflow to record the failure.

### Changed
- `agent-memory-proposal.md` §13.3 (Context Manager v2) and §13.4 (Temporal consolidation) deviations resolved.
- Implementation plan updated with R9-R14 completion notes.
- Test coverage: 44 core tests + 19 memory tests + 4 integration tests = 63 total, all passing.

## [1.1.0] - 2026-08-01

### Added
- **Milestone 9: Short-Term Memory (Live Case File)**:
    - `MemoryManager` with Qdrant 1.18 builder API; automatic embedding-on-ingest via `ModelClient`.
    - `memory.record` MCP tool for streaming ingestion; Qdrant live upserts.
    - Context Manager v1: budget enforcement via selection/truncation (`fit_to_budget`); `context_window_size` on Model entity; included/excluded context emitted to run timeline.
    - NebulaGraph temporal edges: **deferred** (stub). See ITD-18 deviation note.
- **Milestone 10: Long-Term Memory (Cross-Request Recall)**:
    - `consolidate_run_memory`: in-process consolidation wired into run lifecycle; LLM-driven episode extraction; episodes stored in RethinkDB `memory_episodes` table.
    - `memory.recall` MCP tool with provenance back-links; searches Qdrant (semantic) + RethinkDB (keyword filter on episodes).
    - Temporal consolidation workflow: deleted (was dead code per proposal Section 13.4).
    - Context Manager v2 (summarization/compression, hierarchical demotion): **not implemented** (documented deviation per proposal Section 13.3).
- **Milestone 11: Epistemic Memory (Belief Manager)**:
    - `BeliefManager` with `Hypothesis` as first-class entity; `record_belief`, `close_experiment`, `reverify_hypothesis` implemented.
    - Retraction cascades via RethinkDB `filter` on `assumptions` array (not NebulaGraph graph traversals as designed).
    - `experiment.close` and `experiment.reverify` MCP tools registered in tool plane during runs.
    - Context Manager v3: beliefs inserted with high priority after task state in `fit_to_budget`.
    - Epistemic quality gate: `validate_approach` called in runner; falsified-hypothesis warning injected as system message.
- **Milestone 12: Knowledge Propagation**:
    - `.kgpkg` package format: tar.gz with `manifest.json` (includes `embedding_model`, `schema_version`, `package_type`), `episodes.jsonl`, `beliefs.jsonl`, `points.jsonl`.
    - Provenance fields: `origin_workspace_id`, `origin_package_id` on Episode and Hypothesis; `source_tier` on Hypothesis.
    - Source priority ordering: `source_priority` in `MemoryPolicy`; used in `assemble_context` to sort by origin.
    - Export/import CLI: `kaigents-cli memory export` and `kaigents-cli memory import`.
    - Single embedding model lock: `embedding_model` field on `MemoryManager`; included in manifest; validated on import with warning on mismatch.
    - Package-scoped retraction cascades: `close_experiment` accepts `scope_package_id`; `remove_package` scopes cascade to same-package beliefs only.
    - Cross-workspace deduplication: Qdrant points use vector cosine similarity >0.95; episodes/beliefs use exact text match in RethinkDB. Skip counts in import result.
- **Integration tests**: 3 integration tests against live Qdrant, RethinkDB, and Lemonade Server (full M9-M12 flow, retraction cascade, package-scoped retraction).

### Fixed
- Model name hardcoded as "ignored" in LLM requests — added `chat_model` field and `with_chat_model()` builder.
- `Usage` struct missing `#[serde(default)]` — Lemonade server omits `completion_tokens` from embeddings response.
- `encoding_format` serialized as `null` — added `#[serde(skip_serializing_if = "Option::is_none")]`.
- Qdrant vector export using deprecated `data` field — switched to `VectorsOutput::get_vector()` for Qdrant server v1.17 compatibility.
- 4 stabilization bugs: no Qdrant collection creation, consolidation using wrong endpoint, hardcoded `r.db("kaigents")` in 4 places, regex injection in recall.

### Changed
- `kaigents-memory` crate added to engine workspace.
- `MemoryPolicy` CRD added with `source_priority` field.
- `Agent` CRD extended with `context_window_size` field.
- `start_here.md` updated with M9-M12 status and test coverage.
- Version bumped from 1.0.0 to 1.1.0.

## [0.2.0] - 2026-05-08

### Added
- **Milestone 7: Hardening & Production Readiness**:
    - Cloud-agnostic S3 artifact storage implementation in `kaigents-core`.
    - Support for large-object range reads in Artifact Proxy for streaming.
    - Standardized Prometheus metrics across Rust (Engine) and Go (Dashboard/Adapter) components.
    - Integrated structured JSON logging (Loki-ready) stack-wide.
    - Sample Grafana dashboard for platform overview.
- **Milestone 6: Dashboard MVP**: Lightweight Go service for monitoring agents, processes, and active runs.
- **Milestone 5: Hybrid Execution**: `RoutingPolicy` support in `Agent` and `Run` resources, allowing `NodeSelector` injection for GPU/NPU pinning.
- **Milestone 4: Process & Task Model**: First-class `Process` and `Task` CRDs, and mapping to Temporal WorkRequest/WorkItem durable execution.

### Changed
- Refactored `kaigents-cli` to use `tracing` for structured JSON logging.
- Updated `RunReconciler` to support `RoutingPolicy` for hardware pinning.

## [0.1.0] - 2026-05-08

### Added
- **Milestone 1: Solo Mode MVP**: Core CRDs (Agent, Tool, Run), embedded DAG engine, and CLI.
- **Milestone 2: Platform Mode Essentials**: RBAC and Keycloak OIDC integration.
- **Milestone 3: Durable Execution substrate**: Temporal adapter POC and durable execution decision.
