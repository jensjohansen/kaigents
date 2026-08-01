# Changelog (Kaigents)

All notable changes to this project will be documented in this file.

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
