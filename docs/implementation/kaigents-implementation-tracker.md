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
- [ ] Controller reconciles Run resources and drives execution state
- [x] Status conditions and events are written for key transitions

Acceptance criteria:

- [x] Creating resources results in deterministic status updates without manual intervention.

Push checkpoint:

- [ ] Push after control-plane reconciliation loops are stable and test coverage exists for key transitions.

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

- [ ] Tool invocations are observable, auditable, and show up consistently in the run timeline.

Push checkpoint:

- [ ] Push after MCP tool invocation and contract snapshotting are implemented and recorded in the run timeline.

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

- [ ] Production artifact **byte storage** backed by S3-compatible object store (Ceph RGW) (ITD-13)
- [ ] Production artifact access via private-bucket signing/proxy semantics (or equivalent deployment pattern), preserving Range + cache headers (ITD-14)

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

## Milestone 3: Durable process execution engine decision + integration (ITD-16)

This milestone establishes the durable execution engine of record for long-running Work Requests.

Key constraint:

- Temporal (or any engine) concepts must remain hidden behind Kaigents product/domain terms.

### 3A. Temporal stop/go spike (execution engine of record)

- [ ] Validate self-hosted Temporal operational footprint on the on-prem baseline (CPU/mem/storage + required backing services)
- [ ] Define a minimal Kaigents **Process/Task** definition representation suitable for POC validation (may be code or JSON; CRDs are not required in this milestone)
- [ ] Implement a minimal “compiler”/mapping from Process/Task graph semantics to the durable engine execution model, including:
- [ ] explicit rework edges (cycles)
- [ ] bounded rework limits/escalation
- [ ] at least one human approval/wait gate
- [ ] Implement a thin Temporal adapter service (Go) that exposes Kaigents-native operations:
- [ ] Start execution (WorkRequest)
- [ ] Signal execution (human-in-loop / rework)
- [ ] Query execution state
- [ ] Implement a minimal Go Worker that can:
- [ ] Run a representative workflow that blocks on a Signal and then completes
- [ ] Execute one Activity that represents a Kaigents WorkAttempt
- [ ] Validate the integration boundary (Rust backend calls adapter; no Temporal SDK usage from Rust)
- [ ] Demonstrate that a Work Request execution produces a reconstructable history that maps cleanly to:
- [ ] WorkRequest state transitions
- [ ] WorkItem state transitions
- [ ] WorkAttempt attempts (including retries/rework)

Exit criteria (decision checkpoint):

- [ ] Ops footprint is acceptable for baseline on-prem cluster assumptions
- [ ] Integration boundary is acceptable (Rust <-> adapter stable, minimal surface)
- [ ] Developer experience is acceptable (workflow determinism constraints are manageable)
- [ ] The Process/Task definition model remains simple-first and does not collapse into engine-specific workflow code
- [ ] Record outcome as ITD-16: Adopt Temporal backend vs Build custom durable engine

Push checkpoint:

- [ ] Push only after ITD-16 is recorded and the POC code is reproducible.

## Milestone 4: Process definition model (Process/Task) + execution mapping

This milestone makes the refined product domain model real in the control plane and runtime surfaces.

### 4A. Definition model (CRD + CLI first)

- [ ] CRD: Process definition
- [ ] CRD: Task definition
- [ ] Validation: simple-first graph constraints (explicit rework edges; bounded rework semantics declared)
- [ ] CLI: create/apply/update Process/Task resources

### 4B. Execution model mapping

- [ ] Map WorkRequest/WorkItem/WorkAttempt concepts to underlying execution engine without leaking engine-specific concepts
- [ ] Timeline events and query surfaces align to WorkRequest/WorkItem/WorkAttempt

Acceptance criteria:

- [ ] A user can define a simple-first process and execute it as a Work Request.

Push checkpoint:

- [ ] Push after Process/Task definitions are validated and a Work Request produces a queryable execution history.

## Milestone 5: Hybrid Execution routing (CPU/GPU/NPU)

- [ ] Operator-visible routing configuration
- [ ] Policy surfaces for execution preferences/constraints (CPU/GPU/NPU)
- [ ] Timeline/telemetry exposes what routing was requested vs what occurred

Acceptance criteria:

- [ ] Workloads can be routed according to policy and observed in telemetry.

Push checkpoint:

- [ ] Push after routing policy is operator-visible and correlation is demonstrated in both timeline and telemetry.

## Milestone 6: Dashboard MVP

- [ ] Browse agents/teams
- [ ] Trigger runs
- [ ] View run timelines and traces
- [ ] View errors and retry reasons
- [ ] Browse/preview artifacts (where feasible)

Acceptance criteria:

- [ ] Operators can diagnose failures via UI without needing direct DB access.

Push checkpoint:

- [ ] Push after dashboard can render run timelines reliably and failures are diagnosable without backdoor access.
