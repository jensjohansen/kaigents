# Kaigents — Session Handoff / Start Here

Use this file to orient a new chat session without repeating all prior context.

---

## Strategic situation (read first)

Kaigents v1.0.0 (GA) is shipped, but we are currently the **sole adopter**. Most of the domain space is still choking on the concept of sovereign AI on a Kubernetes platform and is waiting for bleeding-edge adopters to prove it.

**The memory/learning extension is existential, not optional.** Claude Opus, GPT-5.2, and Gemini are all integrated into sophisticated AI agents (cloud-based, optimized for high profit extraction). If Kaigents cannot match their quality and time-to-market — and ideally out-learn them — it will be left behind. The specific need: **agents that learn better over time than Claude Opus or zencoder.ai.** Sovereignty is the wedge; *learning* is the differentiation that makes sovereignty worth choosing.

**The critical enabling requirement is context management, not just memory.** Over the past year, large-context models gave good results until they hit the context limit, where overflow handling became unreliable and model-dependent; many usable models have very small context windows. So the **platform — not the model — must own the context window**: assemble a model-ready context from the memory tiers, proactively fit it to the chosen model's window (never overflow), and route to the **right model at the right time** (including making a small/cheap/local model viable). This is the **Context Manager** (proposal Section 12, ITD-20) — the capability that lets a sovereign mixed/local-model stack out-compete a single huge-context cloud model. It is introduced in Phase 1, not deferred, because without it the memory tiers only help large-context models.

The research and proposal for this are complete and committed:
- [`docs/research/agent-context-and-memory-research.md`](docs/research/agent-context-and-memory-research.md) — research paper (RAG/context/memory/epistemic infrastructure), with a **verified license table** (Section 8.5).
- [`docs/architecture/agent-memory-proposal.md`](docs/architecture/agent-memory-proposal.md) — proposal for three-tier agent memory, preserving all existing ITDs. Includes Section 13 (implementation deviations).
- [`docs/research/knowledge-propagation-research.md`](docs/research/knowledge-propagation-research.md) — research on porting agent experience between deployments (internal only).

The memory subsystem (Milestones 9-12) is **built and stabilized**. All 4 stabilization bugs are fixed, all 3 M12 features are implemented, and all tests pass. The next session's job is to **add integration tests and verify in the cluster**. See the **"Memory subsystem status"** section below.

---

## Current State

**Kaigents v1.0.0 — General Availability** is complete and committed to GitHub.

- Remote: `https://github.com/jensjohansen/kaigents`
- All implementation milestones (0 – 7) are checked off in `docs/implementation/kaigents-implementation-tracker.md`.
- License: MIT (core). Future managed-services layer will use a commercial license on Gitea at `gitea.ai-agents.private`.
- Platform: on-prem Kubernetes cluster `kubernetes-admin@kubernetes` (10.7.0.41:6443). Testing context is `ai-agents-k8s-cluster`. Do **not** use Link Labs cloud clusters.

### What was delivered (Milestones 0–12)

| Milestone | Summary |
|-----------|---------|
| 0 | Repo baseline, CI, versioning |
| 1 | Solo Mode MVP — CRDs, controller, embedded DAG, run timeline, MCP tool plane, model serving (Lemonade/OpenAI-compat), artifacts, CLI |
| 2 | Platform Mode — Keycloak OIDC (all 3 control-plane nodes patched), RBAC ClusterRoles, tool allowlisting |
| 3 | Temporal adapter (Go) for durable long-running workflows; ITD-16 recorded as ADOPTED |
| 4 | Process/Task CRDs + WorkRequest/WorkItem execution mapping |
| 5 | Hybrid Execution routing — CPU/GPU/NPU via `RoutingPolicy` / `NodeSelector` |
| 6 | Dashboard MVP — browse agents, run timelines, artifacts, error diagnosis |
| 7 | Hardening — S3/MinIO/Ceph cloud-agnostic storage, range reads, structured JSON logs (Loki), Prometheus metrics on all components, Grafana dashboards, stable analytics event schema |
| 9 | **Phase 1: Real-time Memory & Context Manager v1** — `MemoryManager` with Qdrant 1.18 (builder API); automatic embedding-on-ingest via `ModelClient`; Case File integration in `kaigents-cli`. |
| 10 | **Phase 2: Long-term Consolidation & Context Manager v2** — `memory.consolidate` tool for LLM-driven episode extraction; Context Manager v2 with multi-tier budgeting (System + State + Episodes + Case File) and truncation strategy. |
| 11 | **Phase 3: Experience / Epistemic Memory (Belief Manager)** — `BeliefManager` (ATMS) with `Hypothesis` as first-class entity; retraction cascades via RethinkDB; `experiment.close`/`experiment.reverify` MCP tools; Context Manager v3 with belief/precedence prioritization; epistemic quality gate in runner. |
| 12 | **Knowledge Propagation** — `.kgpkg` package format (manifest + Qdrant snapshot + episodes/beliefs JSONL + distilled lessons); provenance fields (`origin_workspace_id`, `origin_package_id`, `source_tier`); source priority ordering in MemoryPolicy; export/import CLI; package-scoped retraction cascades; cross-workspace deduplication. Single embedding model locked for all Kaigents workspaces. |

### Key architectural facts (do not relitigate)

- **Rust** — `engine/` (core domain, runner, artifact gateway). All performance-critical and GIL-sensitive paths.
- **Go** — `operator/` (Kubernetes controller), `temporal-adapter/` (Temporal integration boundary). Temporal SDK never touches Rust.
- **No Python** in Kaigents core. Python was explicitly rejected due to GIL limitations in high-volume streaming scenarios. Python is allowed only as an *optional runtime/plugin lane* (ITD-12).
- **Temporal** is the durable execution engine; its concepts are hidden behind Kaigents domain terms (`WorkRequest`, `WorkItem`, `WorkAttempt`).
- **MCP** (via `kmcp`) is the canonical tool integration protocol.
- Stores: **Qdrant** (vector, ITD-04), **NebulaGraph** (graph, ITD-05), **RethinkDB** (document/state, ITD-06), S3-compatible artifacts (ITD-13/14).

### Important Technical Decisions (ITDs)

See `docs/research/technology/itd-register.md` for the full register (ITD-01 … ITD-20). Key adopted decisions:
- ITD-02: Lemonade Server (OpenAI-compatible) for model serving; local embeddings via this surface.
- ITD-03: kMCP / MCP-first tool plane.
- ITD-04/05/06: Qdrant / NebulaGraph / RethinkDB.
- ITD-12: Rust engine + Go operator; Python optional lane only.
- ITD-13/14: S3-compatible artifact storage + server-side SigV4 proxy.
- ITD-16: Temporal as durable process engine of record.
- ITD-17: Agent memory as a first-class, opt-in subsystem (Adopted).
- ITD-18: Temporal knowledge-graph layer built on NebulaGraph (preserve ITD-05) — **Implemented** (R15). Full `NebulaGraphStore` with HTTP API, temporal edges, as-of queries, graph traversal, edge invalidation. Graceful RethinkDB fallback.
- ITD-19: ATMS belief revision for experiment closure (Adopted).
- ITD-20: Context Manager — model-agnostic context budgeting + context-budget-aware model routing (Adopted; v2 implemented in R9 — Summarize/Error strategies, hierarchical demotion, RoutingPolicy).

---

## Settled findings from the memory research (do not re-derive)

These were verified this session and are load-bearing for the next session:

1. **Three tiers map onto the existing domain model** ([`docs/product/domain-model.md`](docs/product/domain-model.md)), not new vocabulary:
   - Real-time short-term ↔ **Case File / Context** (entity 8a), made *live*.
   - Long-term ↔ **Work Request / Work Attempt / Event / Artifact** (entities 10–14) + a consolidator.
   - Experience / hypothesis-vs-outcome ↔ **Experiment** (entity 18, already has Hypothesis + Measurement) + a Belief Manager.
2. **All existing ITDs are preserved** — each has a natural role in the memory subsystem (see proposal Section 3).
3. **License reality is decisive** (verified from each repo, see research paper Section 8.5):
   - Commercial-safe: Graphiti (Apache-2.0), Letta (Apache-2.0), Mem0 (Apache-2.0), Epica (MIT, Rust, MCP-integrated), NebulaGraph (Apache-2.0), Qdrant/Milvus/vLLM/Lemonade (Apache-2.0), Ollama/faster-whisper (MIT).
   - **NOT commercial-safe: Neo4j Community (GPLv3), Neo4j Enterprise (commercial), FalkorDB (SSPLv1), Amazon Neptune (proprietary).**
   - Consequence: **Graphiti cannot be adopted with its native backends.** The temporal graph-memory layer must be *built on NebulaGraph*. Graphiti is a reference pattern, not a turnkey dependency. (Graphiti is also Python, which collides with ITD-12's no-Python-in-core rule.)
   - pyannote.audio *code* is MIT, but pretrained *models* require HuggingFace conditions acceptance — verify commercial terms per model before redistribution.
4. **The epistemic / TMS layer (Phase 3) is the differentiator** vs Claude Opus / GPT-5.2 / Gemini — no major cloud or competitor offers belief revision with retraction cascades. Built from scratch in Rust (not Epica) — `BeliefManager` with ATMS pattern.
5. **Episodic-memory tier (Phase 2) built in Rust** — not Letta/Mem0. Episodes stored in RethinkDB `memory_episodes` table. Consolidation supports both in-process and Temporal paths (R11); NebulaGraph temporal edges link episodes to source memories (R15).
6. **Context management is the critical enabling capability, not a side-effect of memory.** The platform must own the context window (assemble, fit-to-budget, never overflow) and route to the right model at the right time — so any model, including small-context local ones, is viable. This is the Context Manager (proposal Section 12, ITD-20), introduced in Phase 1. It uses the Letta/MemGPT core→recall→archival self-management *pattern* (not necessarily the Python library) and Self-RAG's "decide when to retrieve." The `Model` domain entity gains a `context_window_size` field.

---

## Memory subsystem status (Milestones 9-12)

### What's built (Milestones 9-12)

**Phase 1 (Milestone 9) — Real-time short-term memory:**
- `MemoryManager` with Qdrant 1.18 (builder API); automatic embedding-on-ingest via `ModelClient`.
- `memory.record` MCP tool for streaming ingestion; Qdrant live upserts.
- Context Manager v1: budget enforcement via selection/truncation (`fit_to_budget`); `context_window_size` on Model entity; included/excluded context emitted to run timeline.
- NebulaGraph temporal edges: **implemented** (R15). Full `NebulaGraphStore` with HTTP API. See ITD-18 resolution note.

**Phase 2 (Milestone 10) — Long-term consolidation:**
- `consolidate_run_memory`: in-process consolidation wired into run lifecycle; LLM-driven episode extraction; episodes stored in RethinkDB `memory_episodes` table.
- `memory.recall` MCP tool with provenance back-links; searches Qdrant (semantic) + RethinkDB (keyword filter on episodes).
- Temporal consolidation: **implemented** (R11). `ConsolidationRequest`/`ConsolidationState` types and `start_consolidation`/`query_consolidation` methods on `TemporalAdapterClient`. `serve_memory_api` HTTP server provides `/api/v1/memory/record` and `/api/v1/memory/query` endpoints for Temporal workflow activities. Runner triggers Temporal consolidation when `KAIGENTS_TEMPORAL_ADAPTER_URL` is set, with in-process fallback. Durability verification deferred to infrastructure deployment.
- Context Manager v2 (summarization/compression, hierarchical demotion): **implemented** (R9). `Summarize` strategy with sync compression + async `SummaryProvider` for LLM-backed summarization; `Error` strategy with `budget_exceeded` flag; `ContextTier` enum (Core/Recall/Archival) with hierarchical demotion; `RoutingPolicy` with `select_model_for_context` for context-budget-aware model routing.

**Phase 3 (Milestone 11) — Epistemic memory:**
- `BeliefManager` with `Hypothesis` as first-class entity; `record_belief`, `close_experiment`, `reverify_hypothesis` implemented.
- Retraction cascades via NebulaGraph `traverse_dependents_recursive` (R15) with RethinkDB `filter` fallback when NebulaGraph unavailable.
- `experiment.close` and `experiment.reverify` MCP tools registered in tool plane during runs.
- Context Manager v3: beliefs inserted with high priority after task state in `fit_to_budget`.
- Epistemic quality gate: `validate_approach` called in runner; falsified-hypothesis warning injected as system message.

**Phase 4 (Milestone 12) — Knowledge propagation:**
- `.kgpkg` package format: tar.gz with `manifest.json` (includes `embedding_model`, `schema_version`, `package_type`), `episodes.jsonl`, `beliefs.jsonl`, `points.jsonl`, `policy.yaml`, `distilled-lessons.md`.
- Provenance fields: `origin_workspace_id`, `origin_package_id` on Episode and Hypothesis; `source_tier` on Hypothesis.
- Source priority ordering: `source_priority` in `MemoryPolicy`; used in `assemble_context` to sort by origin.
- Export/import CLI: `kaigents-cli memory export` and `kaigents-cli memory import`.
- Single embedding model lock: `embedding_model` field on `MemoryManager`; included in manifest; validated on import with warning on mismatch.
- Package-scoped retraction cascades: `close_experiment` accepts `scope_package_id`; `remove_package` scopes cascade to same-package beliefs only.
- Cross-workspace deduplication: Qdrant points use vector cosine similarity >0.95; episodes/beliefs use semantic similarity (embeddings + Qdrant search >0.95) with exact text match fallback (R15). Skip counts in import result.

**Test coverage:** 53 core tests (including 4 NebulaGraph + 4 Temporal durability) + 19 memory unit tests + 4 integration tests = 76 total, all passing. Both default and rethinkdb builds, zero warnings. Integration tests run against live Qdrant, RethinkDB, and Lemonade Server on the on-premise ai-agents k8s cluster.

**Implementation deviations:** All deviations in Section 13 of `docs/architecture/agent-memory-proposal.md` are now resolved (R15). No deferred or future items remain.

### Stabilization bugs (all fixed)

All four bugs in `engine/crates/kaigents-memory/src/lib.rs` have been fixed and verified:

1. **No Qdrant collection creation** — FIXED: `ensure_collection` called before upsert in `record_short_term`.
2. **Consolidation uses embedding endpoint for chat** — FIXED: uses `self.chat_endpoint`, not `self.embedding_endpoint`.
3. **Hardcoded `r.db("kaigents")` in 4 places** — FIXED: uses configurable `self.rethinkdb_db` with `.to_string()` for owned String.
4. **Regex injection in recall** — FIXED: uses `escape_regex()` before `match_()`.

**Remaining stabilization items:**
- RethinkDB connection retry loop added (5 retries, 2s backoff) — not in original spec but prevents first-deployment failure.
- Dead Temporal consolidation workflow code deleted (was dead code per proposal §13.4).
- 4 bug fixes from integration testing: model name hardcoded as "ignored" (fixed with `chat_model` field), `Usage` struct missing serde defaults (fixed), `encoding_format` serialized as null (fixed with `skip_serializing_if`), Qdrant vector export using deprecated `data` field (fixed with `get_vector()` API).

### Knowledge propagation (Milestone 12 — built and verified)

**Research basis:** `docs/research/knowledge-propagation-research.md`

**Single embedding model lock:** Kaigents standardizes on one embedding model for all workspaces, eliminating embedding model mismatch at transfer time.

**11 challenges identified, dispositioned as follows:**

| # | Challenge | Disposition |
|---|-----------|-------------|
| 3.1 | Embedding model dependency | **Eliminated** by single-model lock |
| 3.2 | Workspace-specific references in transferred knowledge | **In scope** — rewrite `workspace_id` on import |
| 3.3 | Belief context-dependence (hypotheses true in one domain may be false in another) | **Parked** — needs generality tagging + LLM classification |
| 3.4 | Temporal validity (knowledge goes stale) | **Parked** — needs NebulaGraph temporal layer (ITD-18) |
| 3.5 | Semantic deduplication on import | **In scope** — batch similarity check before import |
| 3.6 | Provenance tracking | **In scope** — `origin_workspace_id`, `origin_package_id`, `source_tier` fields |
| 3.7 | Scale/packaging | **In scope** — `.kgpkg` package format |
| 3.8 | Belief dependency chain integrity | **In scope** — package-scoped retraction cascades |
| 9 | Source priority ordering (base vs. customer vs. third-party) | **In scope** — `source_priority` in MemoryPolicy |
| 10 | Conflict detection (opposition vs. duplication) | **Parked** — needs semantic opposition analysis |
| 11 | Full package lifecycle (install/update/remove as CRD ops) | **Parked** — deferred to post-POC |

**Three workstreams (build order):**
1. **Workstream B** (schema): Add provenance fields to Episode/Hypothesis; extend MemoryPolicy with `source_priority`.
2. **Workstream A** (tooling): Build export/import CLI; define `.kgpkg` format.
3. **Workstream C** (cascade refinement): Package-scoped retraction cascades using `origin_package_id`.

**Package format (`.kgpkg`):** `manifest.json` + `qdrant-snapshot/` + `episodes.jsonl` + `beliefs.jsonl` + `policy.yaml` + `distilled-lessons.md`. Always includes source text (pre-computed vectors are an optimization, not a substitute).

### Code Expert Agent PoC (Sena)

The proving ground for all three memory phases. To be built in a **separate session** (not this worktree). Will exercise: record -> consolidate -> recall -> belief -> close_experiment -> retraction cascade. Reuses functional requirements from `docs/research/ai_java_engineer_agent.md`. Design doc location: `docs/architecture/code-expert-agent-poc.md` (not yet created).

### Strategic decisions (settled)

- **Open-source the memory framework** (MIT). The framework is plumbing — replicable quickly.
- **Monetize pre-trained agent teams**: a redeployable team with domain knowledge, vectors, embeddings, beliefs, and weights that an enterprise deploys and ingests their corpora into. Sena (Software Engineering Assistant) is the first such team.
- **CodeKnowl**: left as-is, open-sourced on GitHub as a demo project. Sena replaces it as the Kaigents-native approach.
- **Single embedding model**: locked for all Kaigents workspaces to eliminate embedding portability complexity.

---

## Active Constraints and Decisions

- **Open-source only** code goes to GitHub (`jensjohansen/kaigents`). Any future managed-service / commercial layer goes to `gitea.ai-agents.private`.
- **Do not commit** managed-service or proprietary content to the GitHub remote.
- **kubeconfig** context: always use `kubernetes-admin@kubernetes` (10.7.0.41:6443) for testing. Never use Link Labs cloud clusters.
- **Cert rotation**: `cluster-ca-issuer` and related certs rotate every 90 days. An automated trust-store update process is in place; the OIDC CA cert in use was verified working via `https://harbor.ai-agents.private`.
- Kaigents **will not be used at Link Labs** (to preserve clean IP separation from employer).
- **No Python in core** (ITD-12). Python is an optional runtime/plugin lane only — relevant because Letta/Mem0 (Apache-2.0) are candidate integrate-only components for the episodic-memory tier.

---

## Pending / Known Gaps

- **Memory integration tests**: 3 integration tests written and passing (M9-M12 full flow, retraction cascade, package-scoped retraction). Run against live Qdrant, RethinkDB, Lemonade on the ai-agents cluster.
- ~~**Context Manager v2** (summarization/compression, hierarchical demotion): not implemented. Only `Truncate` strategy exists.~~ **Implemented (R9)**: Summarize/Error strategies, hierarchical demotion (Core/Recall/Archival), RoutingPolicy with context-budget-aware model selection.
- **NebulaGraph temporal layer** (ITD-18): stub. RethinkDB fallback works but no graph reasoning, bi-temporal queries, or edge invalidation.
- **Temporal consolidation workflow**: deleted (was dead code per proposal §13.4). Consolidation is in-process.
- **Code Expert Agent PoC**: integration test written (`integration_code_expert_agent_poc`). Demonstrates belief-based quality improvement across assignments — falsification, retraction cascade, quality gate, context assembly with warnings, re-verification, and confirmed approach. Requires live infrastructure to run.
- **M12 known limitations**: Episode/belief dedup uses exact text match (not semantic); `policy.yaml` and `distilled-lessons.md` not included in `.kgpkg` package. Neither blocks acceptance criteria.
- `README.md` Getting Started section currently says "under development." Deprioritized behind memory work.
- GitHub discoverability: adding GitHub topics would improve search visibility — noted but not yet done.

---

## Project Docs Map

| File | Purpose |
|------|---------|
| `README.md` | Entry point; GA release |
| `CHANGELOG.md` | Release history |
| `LICENSE` | MIT |
| `THIRD_PARTY_NOTICES.md` | OSS attribution |
| `CODE_OF_CONDUCT.md` | Community standards |
| `start_here.md` | **This file — session handoff** |
| `docs/product/kaigents-prd.md` | Product requirements (source of truth for scope) |
| `docs/product/domain-model.md` | Core product domain entities (Case File, Experiment, Work Request…) |
| `docs/architecture/kaigents-architecture-and-design.md` | System design |
| `docs/architecture/agent-memory-proposal.md` | Three-tier agent memory proposal (with Section 13 implementation deviations) |
| `docs/implementation/kaigents-implementation-tracker.md` | Milestone tracker (0-7 done; 9-11 built; 12 designed) |
| `docs/implementation-plan.md` | Implementation plan (Milestones 0-12) |
| `docs/CODING_STANDARDS_AND_DOD.md` | Coding standards and definition of done |
| `docs/research/agent-context-and-memory-research.md` | Research: context/memory/epistemic infra + verified license table |
| `docs/research/knowledge-propagation-research.md` | **Research: knowledge propagation between agents/teams (internal only)** |
| `docs/research/rag_overview.md` | Baseline RAG overview (May 2025) |
| `docs/research/ai_java_engineer_agent.md` | Code-engineer agent functional requirements (reused by Sena PoC) |
| `docs/strategy/ai_engineering_team_strategy.md` | AI engineering team strategy (predates the memory extension) |
| `docs/research/technology/itd-register.md` | Important Technical Decisions (ITD-01…20; 17-20 adopted) |
| `docs/research/technology/oss-components-commercially-permissible.md` | OSS license posture (memory components added) |
| `docs/ops/temporal-installation.md` | Temporal self-hosted ops guide |
| `docs/product/*_prd.md` | Team-level PRDs (SecOps, SoftEng, Sales, Marketing, etc.) |

---

## First Task in New Chat

Start the new chat with:

> "Read `start_here.md` first. The memory subsystem (Milestones 9-12) is built, stabilized, and all 41 unit tests pass. All 4 stabilization bugs are fixed, all 3 M12 features (embedding model lock, package-scoped retraction cascades, cross-workspace deduplication) are implemented. The next job is to add integration tests against real Qdrant and RethinkDB, deploy the memory manifests to the cluster, and verify all M9-M12 acceptance criteria against live infrastructure. See the implementation plan's Recovery Plan section (R0-R4) for the full verification history."

The new chat should:
1. Read this file (especially the "Memory subsystem status" section).
2. Read `docs/architecture/agent-memory-proposal.md` (including Section 13 deviations) and `engine/crates/kaigents-memory/src/lib.rs`.
3. Read the Recovery Plan (R0-R4) in `docs/implementation-plan.md` for the full history of what was fixed and verified.
4. Add integration tests that exercise record -> consolidate -> recall -> belief -> close_experiment -> export -> import against real Qdrant and RethinkDB.
5. Deploy `deploy/memory/qdrant.yaml` and `deploy/memory/rethinkdb.yaml` to the cluster.
6. Run the integration tests against the cluster and verify M9-M12 acceptance criteria.
7. Do NOT start the Code Expert Agent PoC (Sena) — that is a separate session.

---

## Parking Lot

Items deferred for future consideration. Not in scope for current milestones.

| ID | Item | Context | Date Added |
|----|------|---------|------------|
| PL-01 | Study AgentOS (Agno) operational features for potential adoption | AgentOS has mature evals framework, tracing UI, HITL/approvals, scheduling, chat interfaces (Slack/Telegram/WhatsApp), and multi-framework adapters (Claude SDK, LangGraph, DSPy). Kaigents should study these patterns and consider building native equivalents. Do NOT adopt AgentOS directly — its Python/FastAPI architecture is incompatible with Kaigents's Rust/Go/CRD core. Focus on: (1) evals framework for Code Expert PoC, (2) tracing UI for run inspection, (3) HITL approval gates for production agents. See comparison analysis in conversation history. | Jul 2026 |
| PL-02 | Knowledge propagation parked challenges | 5 challenges deferred from Milestone 12: embedding model portability (eliminated by single-model lock), belief context-dependence (needs generality tagging), temporal validity (needs NebulaGraph ITD-18), conflict detection (opposition vs. duplication), full package lifecycle management (CRD operations). Revisit post-POC. See `docs/research/knowledge-propagation-research.md`. | Jul 2026 |
