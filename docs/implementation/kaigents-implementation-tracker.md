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

- [ ] Scope aligns with PRD milestone acceptance criteria; any scope expansion is reflected in the PRD.
- [ ] Design aligns with Architecture & Design; deviations are documented.
- [ ] No ITD conflicts; any required changes are made by updating the ITD register explicitly.
- [ ] OSS posture preserved; dependency changes are license-reviewed and `THIRD_PARTY_NOTICES.md` is updated when needed.
- [ ] `make ci` passes (format/lint/test) for all present lanes.
- [ ] Tracker is updated (checkboxes reflect what is actually complete).

## Milestone 0: Repo + project baseline

- [ ] Define repo layout for Kaigents components (control plane, API, CLI, dashboard)
- [ ] Create development workflows (local dev, kind/minikube, basic CI)
- [ ] Establish versioning + release tagging approach

Acceptance criteria:

- [ ] A new developer can build and run the system locally with documented steps.

Push checkpoint:

- [ ] Push Milestone 0 baseline only after the push/review gate checklist is satisfied.

## Milestone 1: Solo Mode MVP (CRD + CLI + embedded workflow)

### 1A. Resource model (CRDs)

- [ ] CRD: Agent (or Team)
- [ ] CRD: Tool integration reference
- [ ] CRD: MCP server reference (or integration path with kmcp-managed server resources)
- [ ] CRD: Run (execution request + status)
- [ ] Resource model for run artifacts (at minimum: stable references associated with a run)
- [ ] Resource model for model endpoint reference (configuration/discovery target)

Acceptance criteria:

- [ ] CRDs exist for the MVP resource model and can be managed via GitOps.

Push checkpoint:

- [ ] Push after CRDs compile/validate, controller unit tests pass, and PRD/Architecture alignment is re-verified.

### 1B. Control plane skeleton

- [ ] Controller reconciles Agent resources into runnable configuration
- [ ] Controller reconciles Tool/MCP resources into invokable configuration
- [ ] Controller reconciles Run resources and drives execution state
- [ ] Status conditions and events are written for key transitions

Acceptance criteria:

- [ ] Creating resources results in deterministic status updates without manual intervention.

Push checkpoint:

- [ ] Push after control-plane reconciliation loops are stable and test coverage exists for key transitions.

### 1C. Execution engine + embedded DAG

- [ ] Embedded DAG runner supports dependencies
- [ ] Supports concurrency
- [ ] Supports retries with clear semantics
- [ ] Supports cancellation
- [ ] Offload escape hatch: a workflow step can be executed as a Kubernetes workload (job/pod) when needed

Acceptance criteria:

- [ ] A basic multi-step workflow can be executed reliably and cancelled.

Push checkpoint:

- [ ] Push after embedded DAG runner semantics (retries/cancel) are test-locked.

### 1D. Run timeline (durable and queryable)

- [ ] Timeline event model (minimum event set per PRD)
- [ ] Durable storage of timeline events
- [ ] Run timeline query API
- [ ] Stable correlation identifiers for timeline events

Acceptance criteria:

- [ ] A run produces a durable, queryable run timeline.

Push checkpoint:

- [ ] Push after the timeline event model is stabilized and persisted events are queryable with correlation identifiers.

### 1E. Tool plane integration (MCP-first)

- [ ] Connect to kmcp-managed MCP servers
- [ ] Invoke tools with explicit timeouts
- [ ] Bounded outputs (where applicable)
- [ ] Clear error reporting for upstream failures
- [ ] Tool invocation events written to the run timeline
- [ ] Contract snapshotting: MCP is source of truth; Kaigents stores a versioned snapshot for audit/UI rendering

Acceptance criteria:

- [ ] Tool invocations are observable, auditable, and show up consistently in the run timeline.

Push checkpoint:

- [ ] Push after MCP tool invocation and contract snapshotting are implemented and recorded in the run timeline.

### 1F. Model serving integration

- [ ] Integrate with Lemonade server via OpenAI-compatible interface
- [ ] Support chat/text generation
- [ ] Support embeddings
- [ ] Endpoint discovery for:
  - [ ] in-cluster service DNS
  - [ ] developer-local endpoints
- [ ] Model invocation events written to the run timeline (latency + token counts when available)

Acceptance criteria:

- [ ] Model endpoint discovery works for in-cluster and developer-local endpoints.

Push checkpoint:

- [ ] Push after model call events are recorded and endpoint discovery is validated in at least one dev environment.

### 1G. Artifacts

- [ ] First-class artifact concept for runs (inputs, intermediates, outputs)
- [ ] Artifact storage integration
- [ ] Artifact access via stable, environment-scoped URLs
- [ ] Signing/proxy pattern so clients do not need object-store credentials
- [ ] Preserve large-object semantics where applicable (e.g., range reads)
- [ ] Artifact events written to the run timeline

Acceptance criteria:

- [ ] Artifacts are accessible without object-store credentials and are navigable from the run timeline.

Push checkpoint:

- [ ] Push after artifact URLs and proxy/signing behavior are validated and timeline links are stable.

### 1H. CLI MVP

- [ ] Install/bootstrap commands
- [ ] Resource lifecycle management (apply/create/update)
- [ ] Trigger a run
- [ ] Render run timeline
- [ ] Fetch artifacts referenced by the run timeline

Acceptance criteria:

- [ ] CLI can apply resources, trigger runs, render run timeline, and fetch artifacts.

Push checkpoint:

- [ ] Push after CLI workflows are end-to-end demoable and aligned with the PRD run timeline UX requirements.

## Milestone 2: Platform Mode essentials (identity + policy)

### 2A. Authentication

- [ ] Keycloak OIDC integration for UI/API
- [ ] Audit trail of user actions

Acceptance criteria:

- [ ] Users authenticate via SSO-compatible mechanism.

Push checkpoint:

- [ ] Push after auth flows are tested and audit events are produced.

### 2B. Authorization + tool allowlisting

- [ ] RBAC for key resources (at minimum: view vs execute)
- [ ] Tool allowlisting policy enforced at invocation time
- [ ] Denied tool calls show clear reasons in run timeline

Acceptance criteria:

- [ ] Unauthorized users cannot invoke restricted tools.

Push checkpoint:

- [ ] Push after allowlisting enforcement is test-locked and denials show clear reasons in the run timeline.

## Milestone 3: Hybrid Execution routing (CPU/GPU/NPU)

- [ ] Operator-visible routing configuration
- [ ] Policy surfaces for execution preferences/constraints (CPU/GPU/NPU)
- [ ] Timeline/telemetry exposes what routing was requested vs what occurred

Acceptance criteria:

- [ ] Workloads can be routed according to policy and observed in telemetry.

Push checkpoint:

- [ ] Push after routing policy is operator-visible and correlation is demonstrated in both timeline and telemetry.

## Milestone 4: Dashboard MVP

- [ ] Browse agents/teams
- [ ] Trigger runs
- [ ] View run timelines and traces
- [ ] View errors and retry reasons
- [ ] Browse/preview artifacts (where feasible)

Acceptance criteria:

- [ ] Operators can diagnose failures via UI without needing direct DB access.

Push checkpoint:

- [ ] Push after dashboard can render run timelines reliably and failures are diagnosable without backdoor access.
