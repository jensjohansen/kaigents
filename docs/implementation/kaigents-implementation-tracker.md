# Kaigents: Implementation Tracker

This tracker translates the Kaigents PRD into buildable milestones and acceptance criteria.

Conventions:

- Checkboxes represent deliverables.
- Each milestone should be demoable in a fresh cluster installation.
- Use the PRD as the source of truth for scope; update the PRD before expanding scope.

## Milestone 0: Repo + project baseline

- [ ] Define repo layout for Kaigents components (control plane, API, CLI, dashboard)
- [ ] Create development workflows (local dev, kind/minikube, basic CI)
- [ ] Establish versioning + release tagging approach

Acceptance criteria:

- [ ] A new developer can build and run the system locally with documented steps.

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

### 1B. Control plane skeleton

- [ ] Controller reconciles Agent resources into runnable configuration
- [ ] Controller reconciles Tool/MCP resources into invokable configuration
- [ ] Controller reconciles Run resources and drives execution state
- [ ] Status conditions and events are written for key transitions

Acceptance criteria:

- [ ] Creating resources results in deterministic status updates without manual intervention.

### 1C. Execution engine + embedded DAG

- [ ] Embedded DAG runner supports dependencies
- [ ] Supports concurrency
- [ ] Supports retries with clear semantics
- [ ] Supports cancellation
- [ ] Offload escape hatch: a workflow step can be executed as a Kubernetes workload (job/pod) when needed

Acceptance criteria:

- [ ] A basic multi-step workflow can be executed reliably and cancelled.

### 1D. Run timeline (durable and queryable)

- [ ] Timeline event model (minimum event set per PRD)
- [ ] Durable storage of timeline events
- [ ] Run timeline query API
- [ ] Stable correlation identifiers for timeline events

Acceptance criteria:

- [ ] A run produces a durable, queryable run timeline.

### 1E. Tool plane integration (MCP-first)

- [ ] Connect to kmcp-managed MCP servers
- [ ] Invoke tools with explicit timeouts
- [ ] Bounded outputs (where applicable)
- [ ] Clear error reporting for upstream failures
- [ ] Tool invocation events written to the run timeline
- [ ] Contract snapshotting: MCP is source of truth; Kaigents stores a versioned snapshot for audit/UI rendering

Acceptance criteria:

- [ ] Tool invocations are observable, auditable, and show up consistently in the run timeline.

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

### 1G. Artifacts

- [ ] First-class artifact concept for runs (inputs, intermediates, outputs)
- [ ] Artifact storage integration
- [ ] Artifact access via stable, environment-scoped URLs
- [ ] Signing/proxy pattern so clients do not need object-store credentials
- [ ] Preserve large-object semantics where applicable (e.g., range reads)
- [ ] Artifact events written to the run timeline

Acceptance criteria:

- [ ] Artifacts are accessible without object-store credentials and are navigable from the run timeline.

### 1H. CLI MVP

- [ ] Install/bootstrap commands
- [ ] Resource lifecycle management (apply/create/update)
- [ ] Trigger a run
- [ ] Render run timeline
- [ ] Fetch artifacts referenced by the run timeline

Acceptance criteria:

- [ ] CLI can apply resources, trigger runs, render run timeline, and fetch artifacts.

## Milestone 2: Platform Mode essentials (identity + policy)

### 2A. Authentication

- [ ] Keycloak OIDC integration for UI/API
- [ ] Audit trail of user actions

Acceptance criteria:

- [ ] Users authenticate via SSO-compatible mechanism.

### 2B. Authorization + tool allowlisting

- [ ] RBAC for key resources (at minimum: view vs execute)
- [ ] Tool allowlisting policy enforced at invocation time
- [ ] Denied tool calls show clear reasons in run timeline

Acceptance criteria:

- [ ] Unauthorized users cannot invoke restricted tools.

## Milestone 3: Hybrid Execution routing (CPU/GPU/NPU)

- [ ] Operator-visible routing configuration
- [ ] Policy surfaces for execution preferences/constraints (CPU/GPU/NPU)
- [ ] Timeline/telemetry exposes what routing was requested vs what occurred

Acceptance criteria:

- [ ] Workloads can be routed according to policy and observed in telemetry.

## Milestone 4: Dashboard MVP

- [ ] Browse agents/teams
- [ ] Trigger runs
- [ ] View run timelines and traces
- [ ] View errors and retry reasons
- [ ] Browse/preview artifacts (where feasible)

Acceptance criteria:

- [ ] Operators can diagnose failures via UI without needing direct DB access.
