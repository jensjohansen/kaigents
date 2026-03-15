# ITD Register — Kaigents (Important Technical Decisions)

## Purpose
This document tracks the major technical decisions (ITDs) for Kaigents and the status of each decision.

Status values:

- Completed: Decision made / direction chosen
- Pending: Not yet decided

## At a glance

| Topic area | ITDs |
| --- | --- |
| Control plane + runtime | ITD-01, ITD-07 |
| Models and inference | ITD-02 |
| Tool plane | ITD-03 |
| Storage | ITD-04, ITD-05, ITD-06, ITD-13, ITD-14 |
| Workflow substrate | ITD-08, ITD-16 |
| Product surfaces | ITD-09 |
| Identity and access | ITD-10 |
| Operations | ITD-11, ITD-15 |
| Development stack | ITD-12 |

| ID | Decision area | Status |
| --- | --- | --- |
| ITD-01 | Agent control plane baseline | Completed |
| ITD-02 | Model serving + hardware optimization | Completed |
| ITD-03 | MCP server management | Completed |
| ITD-04 | Vector database | Completed |
| ITD-05 | Graph database | Completed |
| ITD-06 | Document/state store | Completed |
| ITD-07 | Agent runtime framework strategy | Completed |
| ITD-08 | Workflow substrate | Completed |
| ITD-09 | UX modality sequencing (CRD + CLI + UI designer) | Completed |
| ITD-10 | AuthN/AuthZ integration | Completed |
| ITD-11 | Observability baseline | Completed |
| ITD-12 | Primary implementation languages/tooling | Completed |
| ITD-13 | Artifact blob storage (S3-compatible object store) | Completed |
| ITD-14 | Secure artifact access pattern for private buckets | Completed |
| ITD-15 | Harbor image pull robot naming convention | Completed |
| ITD-16 | Durable process execution engine of record | Pending |

## ITDs

### ITD-01 — Agent control plane baseline: Kagent + ARK patterns
- Status: Completed
- Business problem:
  - We need a Kubernetes-native control plane for agents/teams and execution, with commercial-safe licensing and a clear operator UX.
- Options considered:
  - Kagent (chosen as baseline pattern)
  - ARK (chosen as baseline pattern)
  - Agentic Layer Agent Runtime Operator
- Chosen option:
  - Use **Kagent + ARK** as reference baselines for Kaigents’ control plane model and UX.
- Primary reason:
  - Both are K8s-native and align with CRD + CLI + dashboard modality; ARK demonstrates a first-class `Query` execution resource; Kagent demonstrates CLI and dashboard entry points.
- Impacts:
  - Kaigents should preserve a CRD-first, GitOps-friendly approach and invest in CLI/dashboard parity early.

### ITD-02 — Model serving + hardware optimization
- Status: Completed
- Business problem:
  - Kaigents wants a credible AMD Ryzen AI optimization story while still supporting standard K8s operations.
- Options considered:
  - Lemonade Server (Apache-2.0) as an OpenAI-compatible server with multiple backends (CPU/Vulkan/ROCm; Windows NPU backends)
  - Lemonade Server with FastFlowLM backend for Ryzen AI NPU (note: proprietary binary license terms for accelerated kernels)
  - KServe as a Kubernetes-native serving control plane (standardized model endpoints, rollout/scale/policy)
  - Ollama as a developer baseline for local iteration
- Chosen option:
  - Use **Lemonade Server** as the primary serving runtime because it supports **hybrid execution across NPU + GPU + CPU** through backends.
  - Treat the **FastFlowLM NPU acceleration kernels** as **integrate-only (user-supplied)** due to proprietary binary licensing terms.
  - Keep **KServe** as a **compatible, optional control-plane integration** for clusters that want standardized KServe semantics (not the primary baseline).
- Primary reason:
  - Lemonade provides a single OpenAI-compatible server surface while still enabling AMD optimization paths across CPU/GPU/NPU.
  - FastFlowLM’s NPU acceleration relies on **proprietary binaries** with **redistribution restrictions** and a **revenue threshold**, which conflicts with “commercial-safe OSS core” and predictable redistribution; therefore Kaigents cannot bundle/redistribute those kernels.
  - KServe alone does not address hybrid execution requirements; it is useful as an optional standardized integration layer rather than the baseline runtime.
- Impacts:
  - Kaigents should standardize on an **OpenAI-compatible HTTP interface** for model endpoints.
  - Kaigents should explicitly classify model runtimes:
    - **Core-supported (redistributable)**: permissive OSS runtimes suitable for Kubernetes packaging.
    - **Integrate-only (user-supplied)**: FastFlowLM NPU acceleration kernels, where users bring their own installer/binaries and accept the license terms.
  - Hardware optimization story becomes a layered message:
    - **AMD GPUs on Linux/Windows**: permissive stacks (e.g., ROCm/Vulkan backends) under the KServe control plane.
    - **Ryzen AI NPU (XDNA2)**: Lemonade + FastFlowLM integration for edge/local deployments (user-supplied kernels); not required for cluster MVP.

### ITD-03 — MCP server management: kMCP (kmcp)
- Status: Completed
- Business problem:
  - We need a Kubernetes-native way to deploy/manage MCP servers with a clean dev-to-prod path.
- Options considered:
  - kMCP (kmcp) (chosen)
  - ToolHive
  - Custom MCP deployment
- Chosen option:
  - kMCP (kmcp)
- Primary reason:
  - CRD + controller approach with multi-transport support and scaffolding aligns with Kaigents tool plane goals.
- Impacts:
  - Kaigents tool plane should be MCP-first and integrate with kmcp semantics.

### ITD-04 — Vector database: Qdrant
- Status: Completed
- Business problem:
  - We need a commercial-safe vector store for retrieval workloads.
- Options considered:
  - Qdrant (chosen)
  - Pinecone
- Chosen option:
  - Qdrant
- Primary reason:
  - Apache-2.0 licensing and strong operational fit.
- Impacts:
  - Retrieval and embedding strategy will need to align with Qdrant collection management.

### ITD-05 — Graph database: NebulaGraph
- Status: Completed
- Business problem:
  - We need a commercial-safe graph store candidate for knowledge graphs and relationship-heavy modeling.
- Options considered:
  - NebulaGraph (chosen)
  - Neo4j
- Chosen option:
  - NebulaGraph
- Primary reason:
  - Apache-2.0 licensing and distributed scalability.
- Impacts:
  - Graph schema and queries will need to map to nGQL.

### ITD-06 — Document/state store
- Status: Completed
- Business problem:
  - We need a durable store for runs, artifacts, and agent state that works well in Kubernetes.
- Options considered:
  - RethinkDB
  - Postgres
- Chosen option:
  - RethinkDB
- Primary reason:
  - For a JSON document store workload, RethinkDB is materially more responsive under load than PostgreSQL for indexed document queries on sparse data.
- Impacts:
  - Kaigents’ persisted run/event documents should be modeled as JSON-first records with secondary indexes designed around the query patterns used by CLI/UI and observability.

### ITD-07 — Agent runtime framework strategy
- Status: Completed
- Business problem:
  - We need a runtime loop for agents/teams that is reliable, observable, and compatible with platform goals.
- Options considered:
  - Google ADK (Python)
  - LangGraph (Python)
  - “runtime-agnostic” execution engines (ARK-style)
- Chosen option:
  - Use a **runtime-agnostic execution engine** (ARK-style) with **pluggable runtimes**, and treat ADK/LangGraph as **optional runtime implementations** rather than hard platform dependencies.
- Primary reason:
  - Preserves Kubernetes-native control plane semantics while avoiding tight coupling to a single Python framework; keeps the door open for Go/Rust execution engines and multiple agent runtime implementations.
- Impacts:
  - Kaigents CRDs should target stable execution semantics (inputs, tool-calls, outputs, status) while allowing different runtime implementations behind an execution-engine interface.

### ITD-08 — Workflow substrate
- Status: Completed
- Business problem:
  - We need a simple, lightweight and fast workflow substrate for batch-y DAGs and integration tasks.
- Options considered:
  - Embedded DAG libraries (Rust)
  - Argo Workflows
  - Tekton Pipelines
- Chosen option:
  - Use an **embedded Rust DAG substrate** as the default workflow engine for Kaigents.
  - Provide an escape hatch for DAG nodes to **offload** work into Kubernetes workloads (e.g., `Job`/`Pod`) when isolation or resource scheduling demands it.
  - Treat Argo/Tekton as **optional integration targets**, not the default substrate.
- Primary reason:
  - Embedded DAGs avoid the operational and latency overhead of “workflow-as-a-controller plus step-as-a-pod” for the common case of many small steps and high fan-out.
  - Rust DAG libraries are permissively licensed and enable a tight integration with Kaigents’ run/state model, observability events, and hybrid hardware scheduling.
  - The offload escape hatch preserves step isolation and Kubernetes scheduling when needed without forcing every step into that model.
- Impacts:
  - Kaigents must implement persistence for DAG state (node status, retries, timing, outputs) in the document/state store so runs are durable across process restarts.
  - Retries do not change the execution topology; Milestone 1 remains a DAG substrate rather than a general workflow graph with cycles.
  - Kaigents must implement first-class execution visibility (events, logs, traces) since we are not inheriting Argo UI/CRD status semantics.
  - Kaigents can still support Argo/Tekton by generating workflows or running them as an integration path, but the platform core does not depend on them.

 Milestone scope clarification:

 - This decision describes the **Milestone 1** embedded workflow substrate for short-lived, batch-y DAG execution.
 - Retries and cancellation are execution semantics on DAG nodes; they do not imply rework loops or cyclic process graphs.
 - Longer-lived process semantics (human-in-the-loop waiting, durable resumability across restarts over days/weeks, bounded rework loops) are tracked separately as ITD-16.

### ITD-09 — UX modality sequencing (CRD + CLI + UI)
- Status: Completed
- Business problem:
  - Kaigents must support declarative CRDs, a CLI, and an eventual low-code designer.
- Options considered:
  - CRD + CLI first (Kagent/ARK patterns)
  - UI-first (Langflow/Flowise patterns)
- Chosen option:
  - Build **CRD + CLI + lightweight dashboard** first; treat low-code designer (Langflow/Flowise-style) as a later integration surface.
- Primary reason:
  - CRD+CLI first preserves GitOps workflows and unblocks early adopters; UI-first systems tend to introduce early coupling and licensing/packaging complexity.
- Impacts:
  - Initial UX should focus on:
    - `kubectl`-compatible CRDs
    - a Kaigents CLI that supports both resource management and interactive invocation
    - a dashboard that can render run history and tool-call traces

### ITD-10 — AuthN/AuthZ integration
- Status: Completed
- Business problem:
  - Platform Mode requires secure access controls aligned with enterprise IdPs.
- Options considered:
  - Keycloak (OIDC)
  - OpenLDAP
  - Database-backed auth (local users)
- Chosen option:
  - Keycloak
- Primary reason:
  - Keycloak supports a broad set of AuthN/AuthZ strategies, fits commercial-safe OSS requirements, and provides a durable path to SSO without locking Kaigents into a proprietary IdP.
- Impacts:
  - Kaigents should standardize on OIDC for platform authentication, with a migration-friendly path from simpler deployments (DB auth) to enterprise SSO.

### ITD-11 — Observability baseline
- Status: Completed
- Business problem:
  - Kaigents requires reliable logs/metrics/traces for agent runs, tools, and workflows.
- Options considered:
  - OpenTelemetry baseline + Prometheus/Grafana
  - Langfuse as optional eval/observability platform
  - Arize Phoenix (benchmark; license restricted)
- Chosen option:
  - Use **OpenTelemetry** as the primary instrumentation standard (traces + metrics + logs where applicable).
  - Use **Prometheus + Grafana** as the default cluster-level metrics/visualization baseline.
  - Treat **Langfuse** as an optional higher-level LLM observability/eval product (ensure we integrate only against MIT-licensed core).
- Primary reason:
  - OpenTelemetry provides a vendor-neutral, language-agnostic standard for tracing and context propagation across distributed components (agent runtime, tool plane, workflow execution, model serving).
  - Prometheus provides a proven pull-based metrics pipeline with strong Kubernetes ergonomics; Grafana provides flexible visualization and alerting on top of those metrics.
  - This baseline is technically aligned with Kaigents’ need to correlate “runs” with tool calls, model calls, and workflow steps via a shared trace context and structured events.
  - Phoenix is not suitable as a dependency for a commercial-safe platform core due to its ELv2 license constraints.
- Impacts:
  - Kaigents needs a stable event schema for runs/tool calls/workflow nodes that can be rendered in CLI/UI and exported as OTel spans and metrics.
  - Kaigents should propagate trace context across:
    - agent runtime loop
    - MCP tool invocations
    - workflow DAG node execution
    - model serving requests
  - Kaigents should expose “run timelines” that can be viewed both in-platform (CLI/UI) and in external tooling (Grafana/trace backend).

### ITD-12 — Primary implementation languages/tooling
- Status: Completed
- Business problem:
  - Kaigents wants to balance performance (Rust/Go) with ecosystem leverage (Python) while staying K8s-native.
- Options considered:
  - Go operator + Python runtimes (Kagent pattern)
  - Go operator + Rust runtimes
  - Python-first
- Chosen option:
  - Default to **Rust** for the Kaigents execution engine components where low overhead and concurrency matter (embedded DAG substrate, run coordinator, scheduling, eventing).
  - Use **Go** where it is the pragmatic choice for Kubernetes controllers/operators and CLIs.
  - Allow **Python** as an optional runtime/plugin lane (e.g., ADK/LangGraph-compatible execution engines) without making the platform core dependent on Python.
- Primary reason:
  - Rust enables a lightweight, high-throughput always-on runtime with strong concurrency and predictable resource usage.
  - Go remains best-in-class for Kubernetes-native controller development and operational tooling in the K8s ecosystem.
  - Keeping Python optional preserves access to the agent framework ecosystem while avoiding a Python-only platform architecture.
- Impacts:
  - Kaigents should define clear component boundaries so the Rust execution engine can evolve independently of the Go control plane.
  - Kaigents must define a stable “execution engine” contract (inputs/outputs/events/status) so multiple runtimes (Rust, Python) can be supported.

### ITD-13 — Artifact blob storage (S3-compatible object store)
- Status: Completed
- Business problem:
  - We need a durable, scalable, cost-effective storage substrate for large/binary artifacts that is decoupled from the indexed document/state store.
  - We want to support cluster deployments using Ceph RGW while staying compatible with S3 semantics.
- Options considered:
  - Store artifact bytes directly in the document/state store (not suitable for large blobs)
  - Store artifact bytes on local disk/PVC (simple but less portable; harder multi-node scaling)
  - Store artifact bytes in an S3-compatible object store (Ceph RGW) (chosen)
- Chosen option:
  - Use **S3-compatible object storage** (Ceph RGW in-cluster) as the preferred production storage for artifact bytes.
  - Keep indexed metadata (artifact records, timeline events) in the document/state store; artifact records reference object storage locations.
- Primary reason:
  - Object storage is the right durability/performance/cost fit for large media/files; indexed stores are optimized for metadata and query.
- Impacts:
  - Kaigents artifact metadata should support a storage reference that can point to object storage (e.g., `s3://bucket/key`).
  - Kaigents should adopt stable key-prefix conventions (tenant/project/agent) so artifacts can be listed and lifecycle-managed without storing every run immutably.

### ITD-14 — Secure artifact access pattern for private buckets
- Status: Completed
- Business problem:
  - We need browser- and CLI-friendly artifact access without making buckets public.
  - We need correct streaming behavior for media (Range requests) and predictable headers for caches and clients.
- Options considered:
  - Public bucket policies / static website hosting (rejected)
  - Client-side presigned URLs exposed directly to users (acceptable in some deployments, but not the baseline)
  - Server-side SigV4 signing proxy to private buckets (chosen baseline)
- Chosen option:
  - Use a **server-side SigV4 proxy pattern** to access private S3 buckets, preserving important HTTP semantics.
- Primary reason:
  - Keeps buckets private while still enabling simple URLs and browser access patterns.
- Impacts:
  - Kaigents (or a companion gateway) should preserve: `Range`, `ETag`, `Last-Modified`, `Accept-Ranges`, `Content-Range`, and `Cache-Control` headers when proxying.
  - Kaigents should avoid coupling core requirements to any specific gateway product scope; this is an implementation pattern that can be applied by multiple services.

### ITD-15 — Harbor image pull robot naming convention
- Status: Completed
- Business problem:
  - Kaigents (and other platform workloads) run in Kubernetes namespaces and pull private images from Harbor.
  - Harbor robot accounts are often created as project-scoped credentials for image pulls.
  - If robot names are not discoverable, operators waste time correlating Harbor robot credentials to Kubernetes namespaces.
- Options considered:
  - Free-form robot naming (inconsistent; hard to correlate)
  - Encode repository or chart name into the robot (still ambiguous when namespaces change)
  - Standardize on a *reference implementation* convention for our deployments, without imposing it on Kaigents users (chosen)
- Chosen option:
  - Kaigents **does not require** a specific Kubernetes namespace naming scheme, registry choice, or IdP choice.
  - For Kaigents’ **default reference implementation** (including Keycloak-backed Harbor where local Harbor users may be disabled), standardize on:
    - Kubernetes namespace: `kaigents`
    - Harbor robot account: `robot$kaigents+kaigents`
  - This convention is recommended for our internal environments and examples because it is searchable and easy to correlate, but it is not a Kaigents platform requirement.
- Primary reason:
  - Keeps our docs/runbooks consistent while preserving customer freedom to choose namespaces and auth models.
- Impacts:
  - The cluster operator runbook should standardize on a single pull-secret name per namespace (e.g. `harbor-regcred`) whose credentials map to a Harbor robot chosen by the cluster operator.
  - Our reference implementation runbook should use the defaults above; if a deployment uses a different namespace, the robot/secret should be created accordingly.

### ITD-16 — Durable process execution engine of record
- Status: Pending

Decision statement:

- Decide the **durable process execution engine of record** for Kaigents Work Requests that require long-running durability (hours to days/weeks), human-in-the-loop waits, bounded rework loops (cycles), cancellation, and a reconstructable history for audit.

Scope boundary (relationship to ITD-08):

- ITD-08 covers the **Milestone 1** embedded DAG substrate for short-lived, batch-y workflows.
- ITD-16 governs the **long-running durable execution path** and must support waiting on humans/external systems and resuming safely across restarts.
- ITD-16 is about a broader **process/workflow graph** model that may include explicit rework edges (cycles); it is not a claim that Milestone 1 DAG execution already supports those semantics.
- This ITD does not require replacing the embedded DAG substrate; both may coexist.

Non-negotiables / acceptance criteria:

- **Product-model alignment (substrate + definition model):**
  - Kaigents can define a minimal **Process/Task** model (code or JSON is acceptable for the POC; CRDs are not required to decide this ITD).
  - Kaigents can compile/map that model into the durable engine execution model without exposing engine-native concepts to end users.
  - The model supports:
    - at least one explicit **rework loop** (cycle)
    - bounded rework semantics (attempt limits, time limits, and/or escalation)
    - at least one **human approval/wait** gate
- **Durability and recovery:**
  - Work Requests remain durable while waiting.
  - Resumption is reliable after worker restarts.
  - Cancellation and retries have predictable, observable semantics.
- **Audit/history:**
  - A Work Request produces a durable history sufficient to reconstruct:
    - WorkRequest state timeline
    - WorkItem state timeline
    - WorkAttempt attempts (including retries/rework)
    - key events required for the Kaigents run/work-request timeline
- **Operational footprint:**
  - The server footprint (CPU/mem/storage + required backing services) is acceptable for the on-prem baseline where most resources are reserved for models/tools.
- **Commercial-safe OSS posture:**
  - Dependencies required for the durable engine-of-record must be redistributable and compatible with Kaigents’ OSS posture.

Options considered:

- Adopt **Temporal** as the durable execution engine of record.
- Build a **Kaigents-specific** durable process engine.

Decision inputs:

- `docs/research/technology/process-engine-evaluation.md`
- `docs/research/technology/temporal-poc.md`

Decision record (to be completed when finalized):

- Chosen option: TBD
- Primary reason: TBD

Impacts (regardless of chosen engine):

- The product domain model becomes authoritative:
  - Process/Task (definitions)
  - Work Request / Work Item / Work Attempt (executions)
- Kaigents must define a stable execution-engine interface that:
  - hides engine-specific concepts behind Kaigents terminology
  - emits a consistent event stream to the run/work-request timeline
  - supports human-in-the-loop waits and bounded rework semantics
- Milestone 1 embedded DAG is not invalidated; it remains a lightweight substrate for short-lived workflows, while ITD-16 governs the long-running durable execution path.
