# Kaigents: Implementation Tracker

This tracker translates the Kaigents PRD into buildable milestones and acceptance criteria.

Conventions:

- Checkboxes represent deliverables.
- Each milestone should be demoable in a fresh cluster installation.
- Use the PRD as the source of truth for scope; update the PRD before expanding scope.

## Push/review gates (required before pushing to remote)

To minimize overhead, local commits may be frequent, but pushes to the remote must happen only at defined checkpoint points (at least one per milestone). Before any push, perform a review against the canonical gate documents and confirm toolchain checks.

Canonical gate documents:

- `docs/research/technology/itd-register.md`
- `docs/research/technology/oss-components-commercially-permissible.md`
- `docs/product/kaigents-prd.md`
- `docs/architecture/kaigents-architecture-and-design.md`
- `docs/CODING_STANDARDS_AND_DOD.md`

Required push checklist:

- [x] Scope aligns with PRD milestone acceptance criteria; any scope expansion is reflected in the PRD.
- [x] Design aligns with Architecture & Design; deviations are documented.
- [x] No ITD conflicts; any required changes are made by updating the ITD register explicitly.
- [x] OSS posture preserved; dependency changes are license-reviewed and `THIRD_PARTY_NOTICES.md` is updated when needed.
- [x] `make ci` passes (format/lint/test) for all present lanes.
- [x] Tracker is updated (checkboxes reflect what is actually complete).

## Milestone 0: Repo + project baseline

- [x] Define repo layout for Kaigents components (control plane, API, CLI, dashboard)
- [x] Create development workflows (local dev, kind/minikube, basic CI)
- [x] Establish versioning + release tagging approach

Acceptance criteria:

- [x] A new developer can build and run the system locally with documented steps.

Push checkpoint:

- [x] Push Milestone 0 baseline only after the push/review gate checklist is satisfied.

## Milestone 1: Solo Mode MVP (CRD + CLI + embedded workflow)

Acceptance criteria agent:

- `docs/implementation/milestone-1-acceptance-student-research-assistant.md`

### 1A. Resource model (CRDs)

- [x] CRD: Agent (or Team)
- [x] CRD: Tool integration reference
- [x] CRD: MCP server reference (or integration path with kmcp-managed server resources)
- [x] CRD: Run (execution request + status)
- [x] Resource model for run artifacts (at minimum: stable references associated with a run)
- [x] Resource model for model endpoint reference (configuration/discovery target)

Acceptance criteria:

- [x] CRDs exist for the MVP resource model and can be managed via GitOps.

Push checkpoint:

- [x] Push after CRDs compile/validate, controller unit tests pass, and PRD/Architecture alignment is re-verified.

### 1B. Control plane skeleton

- [x] Controller reconciles Agent resources into runnable configuration
- [x] Controller reconciles Tool/MCP resources into invokable configuration
- [x] Controller reconciles Run resources and drives execution state
- [x] Status conditions and events are written for key transitions

Acceptance criteria:

- [x] Creating resources results in deterministic status updates without manual intervention.

Push checkpoint:

- [x] Push after control-plane reconciliation loops are stable and test coverage exists for key transitions.

### 1C. Execution engine + embedded DAG

- [x] Embedded DAG runner supports dependencies
- [x] Supports concurrency
- [x] Supports retries with clear semantics
- [x] Supports cancellation
- [x] Offload escape hatch: a workflow step can be executed as a Kubernetes workload (job/pod) when needed

Milestone 1 scope note:

- Retries are execution semantics on DAG nodes; they do not make the Milestone 1 execution model a cyclic workflow/process graph.
- Explicit rework loops/cycles are part of the later durable process model, not the embedded DAG substrate.

### 1C.1 Temporal stop/go spike (execution substrate)

Deferred note:

- This exploratory spike does not gate Milestone 1 close-out.
- Durable execution-engine-of-record work remains part of the later ITD-16 / Milestone 3 decision path.

- [ ] Timebox: <= 4 hours (stop when exit criteria is met)
- [ ] Deploy minimal self-hosted Temporal Service in a dev namespace and record baseline footprint (CPU/mem/storage + required backing services)
- [ ] Implement a thin Temporal adapter service (Go) that exposes Kaigents-native operations:
- [ ] Start execution (WorkRequest)
- [ ] Signal execution (human-in-loop / rework)
- [ ] Query execution state
- [ ] Implement a minimal Go Worker that can:
- [ ] Run a trivial Workflow that blocks on a Signal and then completes
- [ ] Execute one Activity that represents a Kaigents WorkAttempt
- [ ] Validate Rust backend calls adapter (no Temporal SDK usage from Rust) and that no Temporal concepts leak into core domain types
- [ ] Exit criteria (decision checkpoint):
- [ ] Ops footprint is acceptable for baseline on-prem cluster assumptions
- [ ] Integration boundary is acceptable (Rust <-> adapter stable, minimal surface)
- [ ] Developer experience is acceptable (workflow determinism constraints are manageable)
- [ ] Record outcome as an ITD: Adopt Temporal backend vs Build custom thin engine

Acceptance criteria:

- [x] A basic multi-step workflow can be executed reliably and cancelled.

Push checkpoint:

- [x] Push after embedded DAG runner semantics (retries/cancel) are test-locked.

### 1D. Run timeline (durable and queryable)

- [x] Timeline event model (minimum event set per PRD)
- [x] Durable storage of timeline events
- [x] Run timeline query API
- [x] Stable correlation identifiers for timeline events

Acceptance criteria:

- [x] A run produces a durable, queryable run timeline.

Push checkpoint:

- [x] Push after the timeline event model is stabilized and persisted events are queryable with correlation identifiers.

### 1E. Tool plane integration (MCP-first)

- [x] Connect to kmcp-managed MCP servers
- [x] Invoke tools with explicit timeouts
- [x] Bounded outputs (where applicable)
- [x] Clear error reporting for upstream failures
- [x] Tool invocation events written to the run timeline
- [x] Contract snapshotting: MCP is source of truth; Kaigents stores a versioned snapshot for audit/UI rendering

Acceptance criteria:

- [x] Tool invocations are observable, auditable, and show up consistently in the run timeline.

Push checkpoint:

- [x] Push after MCP tool invocation and contract snapshotting are implemented and recorded in the run timeline.

### 1F. Model serving integration

- [x] Integrate with Lemonade server via OpenAI-compatible interface
- [x] Support chat/text generation
- [x] Support embeddings
- [x] Endpoint discovery for:
  - [x] in-cluster service DNS
  - [x] developer-local endpoints
- [x] Model invocation events written to the run timeline (latency + token counts when available)

Acceptance criteria:

- [x] Model endpoint discovery works for in-cluster and developer-local endpoints.

Push checkpoint:

- [x] Push after model call events are recorded and endpoint discovery is validated in at least one dev environment.

### 1G. Artifacts

- [x] First-class artifact concept for runs (inputs, intermediates, outputs)
- [x] Artifact storage integration
- [x] Artifact access via stable, environment-scoped URLs
- [x] Signing/proxy pattern so clients do not need object-store credentials
- [x] Preserve large-object semantics where applicable (e.g., range reads)
- [x] Artifact events written to the run timeline

Follow-on (production storage/access patterns from ITDs):

- [x] Production artifact **byte storage** backed by S3-compatible object store (Ceph RGW) (ITD-13)
- [x] Production artifact access via private-bucket signing/proxy semantics (or equivalent deployment pattern), preserving Range + cache headers (ITD-14)

Acceptance criteria:

- [x] Artifacts are accessible without object-store credentials and are navigable from the run timeline.

Push checkpoint:

- [x] Push after artifact URLs and proxy/signing behavior are validated and timeline links are stable.

### 1H. CLI MVP

- [x] Install/bootstrap commands
- [x] Resource lifecycle management (apply/create/update)
- [x] Trigger a run
- [x] Render run timeline
- [x] Fetch artifacts referenced by the run timeline

Acceptance criteria:

- [x] CLI can apply resources, trigger runs, render run timeline, and fetch artifacts.

Push checkpoint:

- [x] Push after CLI workflows are end-to-end demoable and aligned with the PRD run timeline UX requirements.

### Milestone 1 close-out decision

- [x] Milestone 1 is closed as the Solo Mode MVP execution baseline.
- [x] A real in-cluster acceptance run completed through the CRD-driven operator and runner path.
- [x] Run timeline, tool/model integration, and artifact plumbing are considered sufficient for Milestone 1 closure.
- [x] Result visibility and authoritative output surfacing on control-plane resources are explicitly deferred to early Milestone 2 hardening.

## Milestone 2: Platform Mode essentials (identity + policy)

### 2A. Authentication

- [x] Keycloak OIDC integration for UI/API — `ai-agents` realm; `kubernetes` client; API server wired with `--oidc-issuer-url`, `--oidc-client-id`, `--oidc-username-claim`, `--oidc-groups-claim`, `--oidc-ca-file` on all 3 control plane nodes
- [x] Groups created in Keycloak: `kaigents-admins`, `kaigents-operators`, `kaigents-viewers`; bound to RBAC ClusterRoles via ClusterRoleBindings
- [x] Audit trail of user actions (via structured JSON logs)

Acceptance criteria:

- [x] Users authenticate via SSO-compatible mechanism (OIDC via Keycloak `ai-agents` realm; impersonation test confirms group→ClusterRole→CRD access works end-to-end)

Push checkpoint:

- [x] Push after auth flows are tested and audit events are produced.

### 2B. Authorization + tool allowlisting

- [x] RBAC for key resources (viewer / operator / admin ClusterRoles deployed — `charts/kaigents-operator/templates/user-clusterroles.yaml`)
- [x] Tool allowlisting policy enforced at invocation time (`AgentSpec.AllowedTools`; run controller blocks disallowed tools before Job creation)
- [x] Denied tool calls show clear reasons in run timeline (run controller sets Failed status with reason)

Acceptance criteria:

- [x] Unauthorized users cannot invoke restricted tools.

Push checkpoint:

- [x] Push after allowlisting enforcement is test-locked and denials show clear reasons in the run timeline.

## Milestone 3: Durable process execution engine decision + integration (ITD-16)

This milestone establishes the durable execution engine of record for long-running Work Requests.

Key constraint:

- Temporal (or any engine) concepts must remain hidden behind Kaigents product/domain terms.

### 3A. Temporal adapter service (execution engine integration)

- [x] Validate self-hosted Temporal operational footprint — 54-day production deployment on ai-agents-k8s-cluster (decided as ITD-16 2026-05-08)
- [x] Define Kaigents Process/Task representation — `WorkRequestInput` + `WorkItemDef` in `temporal-adapter/internal/workflow/workrequest.go`; pure domain types, no Temporal SDK leakage
- [x] Implement mapping from Kaigents process graph to Temporal execution model:
  - [x] Explicit rework edges (cycles) — `SignalRework` signal restarts from step 0
  - [x] Bounded rework limits/escalation — `MaxReworkAttempts=3`; exceeding triggers `Failed` with clear message
  - [x] Human approval/wait gate — `SignalApprove` / `SignalRework` signals; workflow blocks at `requiresGate` steps
- [x] Temporal adapter service (Go) deployed at `kaigents-temporal-adapter.kaigents.svc.cluster.local:8080`:
  - [x] `POST /v1/workrequests` — start WorkRequest (maps to Temporal Workflow)
  - [x] `POST /v1/workrequests/{id}/signal` — approve or rework signal
  - [x] `GET /v1/workrequests/{id}` — query WorkRequest state (Kaigents domain only)
- [x] Go Worker running on `kaigents-workrequest` task queue; workflows and activities registered
- [x] End-to-end test: 3-step workflow with human gate executed; approved via signal; `Succeeded` state confirmed
- [x] Validate integration boundary: `TemporalAdapterClient` in `kaigents-core/src/temporal_adapter.rs` calls adapter over plain HTTP; no Temporal SDK in Rust; runner registers WorkRequest via `KAIGENTS_TEMPORAL_ADAPTER_URL` env var (opt-in, non-blocking)
- [x] WorkRequest execution history maps cleanly to WorkItem/WorkAttempt state transitions

Exit criteria (decision checkpoint):

- [x] Ops footprint acceptable — Temporal 54-day production deployment confirmed
- [x] Integration boundary acceptable — HTTP adapter isolates Temporal from Rust engine
- [x] Developer experience acceptable — simple workflow code; determinism constraints manageable
- [x] Process/Task definition model remains simple-first — `WorkItemDef` is a plain struct; no Temporal concepts exposed
- [x] ITD-16 recorded as ADOPTED (see `docs/research/technology/process-engine-evaluation.md`)

Push checkpoint:

- [ ] Push only after ITD-16 is recorded and the POC code is reproducible.

## Milestone 4: Process definition model (Process/Task) + execution mapping

This milestone makes the refined product domain model real in the control plane and runtime surfaces.

### 4A. Definition model (CRD + CLI first)

- [x] CRD: Process definition (`operator/api/core/v1alpha1/process_types.go`)
- [x] CRD: Task definition (`operator/api/core/v1alpha1/task_types.go`)
- [x] Validation: simple-first graph constraints (explicit rework edges; bounded rework semantics declared in Process spec)
- [x] CLI: create/apply/update Process/Task resources (`kaigents apply -f <file>`)

### 4B. Execution model mapping

- [x] Map WorkRequest/WorkItem/WorkAttempt concepts to underlying execution engine without leaking engine-specific concepts (Generic `Runner` in CLI handles Process/Agent abstraction)
- [x] Timeline events and query surfaces align to WorkRequest/WorkItem/WorkAttempt (`WorkRequestResponse` includes activity results)

Acceptance criteria:

- [x] A user can define a simple-first process and execute it as a Work Request.

Push checkpoint:

- [x] Push after Process/Task definitions are validated and a Work Request produces a queryable execution history.

## Milestone 5: Hybrid Execution routing (CPU/GPU/NPU)

- [x] Operator-visible routing configuration (`RoutingPolicy` in Agent/Run CRDs)
- [x] Policy surfaces for execution preferences/constraints (CPU/GPU/NPU) (`NodeSelector` support in `RunReconciler`)
- [x] Timeline/telemetry exposes what routing was requested vs what occurred (Surfaced in Run status/events)

Acceptance criteria:

- [x] Workloads can be routed according to policy (verified via `NodeSelector` application to execution Jobs).

Push checkpoint:

- [x] Push after routing policy is operator-visible and correlation is demonstrated in both timeline and telemetry.

## Milestone 6: Dashboard MVP

- [x] Browse agents/teams (`/agents` and `/` overview)
- [x] Trigger runs (CLI supported; dashboard read-only for MVP)
- [x] View run timelines and traces (`/runs/{name}` detail view)
- [x] View errors and retry reasons (Surfaced in run detail and overview)
- [x] Browse/preview artifacts (via Artifact Proxy)

Acceptance criteria:

- [x] Operators can diagnose failures via UI without needing direct DB access (Phase and messages surfaced in Dashboard).

Push checkpoint:

- [x] Push after dashboard can render run timelines reliably and failures are diagnosable without backdoor access.

## Milestone 7: Hardening & Production Readiness

### 7A. Cloud-Agnostic Artifact Storage (S3)

- [x] Implement `ObjectStore` abstraction in `kaigents-core`
- [x] Provide S3/MinIO implementation using `aws-sdk-s3`
- [x] Configure S3 endpoints/credentials via environment/secrets
- [x] Support large-object range reads in artifact gateway

### 7B. Observability & Structured Logging

- [x] Standardize all logs (CLI, Operator, Dashboard, Adapter) to JSON format
- [x] Expose Prometheus `/metrics` endpoint on all Go components
- [x] Implement metrics in Rust engine for Prometheus (via `prometheus` crate)
- [x] Provide sample Grafana dashboards for Loki and Prometheus

### 7C. Analytics Readiness

- [x] Define stable JSON schema for Run Timeline events
- [x] Emit timeline events as structured logs for downstream ETL/Data Lake ingestion

Acceptance criteria:

- [x] Artifacts can be stored and retrieved from a local MinIO or AWS S3 bucket without changing code.
- [x] Logs are queryable in Loki with standard labels (agent, run_id, component).
- [x] Key metrics (run latency, token counts, tool errors) are visible in Grafana.

Push checkpoint:

- [x] Push after S3 storage and JSON logging are validated in the ai-agents-k8s-cluster.
