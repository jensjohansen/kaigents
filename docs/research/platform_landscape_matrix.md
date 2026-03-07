# Platform Landscape Matrix (Draft)

**Project:** AI Customer Agents (AI Agents Platform)
**Status:** Draft (research-in-progress)
**Last Updated:** 2026-03-07

---

## Purpose

Survey open-source and commercial platforms/frameworks relevant to building and operating **teams of AI agents** with a focus on:

- Kubernetes-native operations
- Home-lab / low-TCO deployment
- Always-on (or mostly-always-on) agent workloads
- Permissive licensing (goal: MIT for our platform)
- Interop standards (MCP for tools; A2A for agent-to-agent where useful)
- AMD Ryzen AI friendliness (NPU/GPU/CPU hybrid)

This document is intentionally **not** an ITD register. It captures data points and comparisons so we can make better ITDs later.

---

## Branding

Working platform name: **Kaigents**.

---

## Quick Definitions

- **Control plane / platform**: CRDs, operators, UI/API, multi-tenant ops.
- **Runtime / framework**: libraries to implement agent logic and orchestration.
- **Tool plane**: tool integration protocols and deployment mechanisms (e.g., MCP + kMCP).
- **Model serving plane**: serving LLMs/embeddings/rerankers (e.g., Lemonade/FLM, KServe).

---

## Required UX Modalities

Kaigents must support three complementary ways to define and operate agent teams:

- **Declarative (GitOps)**: Kubernetes CRDs as the source of truth.
- **CLI**: developer/operator workflow for scaffolding, applying, validating, and debugging.
- **Web UI Designer**: low-code workflow to create/edit teams, tools, and runs that generates CRDs (or equivalent versioned specs).

Research should evaluate candidates across all three modalities (even if a candidate is strong in only one).

---

## Web UI Variants (Personas)

Kaigents likely needs two Web UI modes with different defaults:

- **Solo Mode (entrepreneur / small team)**
  - Optimize for: guided setup, templates, safe defaults, minimal Kubernetes exposure.
  - Typical deployment: single cluster, single tenant, home-lab or small VPS.

- **Platform Mode (engineering / DevOps / SecOps)**
  - Optimize for: GitOps-first workflows, RBAC, policy/guardrails, observability, multi-tenant operations.
  - Typical deployment: shared cluster, staged environments, compliance constraints.

---

## Matrix

| Candidate | Category | License | Kubernetes-native (CRDs/operator) | MCP | A2A | Always-on fit | Notes | Adopt vs Inspire vs Avoid |
|---|---|---|---|---|---|---|---|---|
| Kagent | Control plane (agents) | Apache-2.0 | Yes | Adjacent (ecosystem) | Yes (A2A endpoint) | Likely | CRDs include `Agent`, `Memory`, `ModelConfig`, `ModelProviderConfig`, `ToolServer`, `RemoteMCPServer`; Go controller + Python engine (ADK) + CLI (`get`, `invoke`, `dashboard`) + web dashboard; execution invoked via dashboard chat/CLI/A2A responses (no obvious first-class `Query`/`Run` CRD) | TBD |
| kMCP (kmcp) | Control plane (tools) | Apache-2.0 | Yes | Yes | N/A | Yes | MCP server CRDs + controller + transport adapter; CLI scaffolding (FastMCP + MCP Go SDK); built-in transports (HTTP/WS/SSE); one-command deploy workflow | Adopt |
| Google ADK | Runtime/framework | Apache-2.0 | No (library) | Adjacent | Yes (integration) | Depends | Code-first agent framework; treat as optional runtime | TBD |
| LangGraph | Runtime/framework | MIT | No (library) | Adjacent | No | Depends | Workflow/state machine for agents | TBD |
| LangChain | Runtime/framework | MIT | No (library) | Adjacent | No | Depends | Tooling + adapters + ecosystem | TBD |
| CrewAI | Runtime/framework | MIT | No (library) | Adjacent | No | Depends | Team abstractions; Python-centric | TBD |
| KServe | Control plane (model serving) | Apache-2.0 | Yes | N/A | N/A | Mixed | Great for model endpoints; often paired w/ Knative | TBD |
| Knative Serving | Control plane (serverless serving) | Apache-2.0 | Yes | N/A | N/A | Maybe | Good for bursty workloads; may conflict with always-on assumption | TBD |
| Lemonade + FastFlowLM | Model serving/runtime | Apache-2.0 | Deployable on k8s | N/A | N/A | Yes | AMD Ryzen AI NPU focus; validate k8s deployment model | TBD |
| OpenClaw k8s Operator | Control plane (agent instances) | Apache-2.0 | Yes | Adjacent | Unknown | Likely | Operator pattern reference for CRD ergonomics, lifecycle, security, observability | Inspire |
| Langfuse | Observability + evals | MIT (+ separate EE license for `ee/` dirs) | Deployable | Adjacent | Adjacent | Yes | OSS LLM observability/evals/datasets; repo license explicitly carves out EE directories under separate license | Inspire / Integrate |
| Arize Phoenix | Observability + evals | Elastic License 2.0 (ELv2) | Deployable | Adjacent | Adjacent | Yes | ELv2 forbids offering as a hosted/managed service providing substantial functionality; not commercial-safe OSS for platform core | Avoid (core) |
| W&B Weave | Observability + evals | Apache-2.0 | Deployable | Adjacent | Adjacent | Mixed | Commercial-safe OSS license; evaluate coupling to W&B hosted platform vs self-managed | Inspire |
| Zep | Memory server (agent memory) | Apache-2.0 | Deployable | Adjacent | Adjacent | Yes | Memory server / retrieval system for agent context; likely an integration option rather than platform core | Inspire / Integrate |
| Letta | Stateful agent platform (memory-first) | Apache-2.0 (verify per repo) | Deployable | Adjacent | Adjacent | Mixed | Memory-first agent runtime + APIs; validate licensing boundaries and fit before reuse | Inspire |
| Dify | Platform product (designer + runtime) | Modified Apache-2.0 w/ restrictions | Yes (deployable) | N/A/adjacent | N/A/adjacent | Mixed | License restricts multi-tenant usage + logo removal; producer may change license terms | Avoid (core) |
| Flowise | Platform product (designer) | Apache-2.0 + commercial-only enterprise parts | Deployable | Adjacent | No | Mixed | Good UI inspiration; docs emphasize visual debugging/execution logs, MCP integration, and RBAC/SSO features but verify what is in Apache-only subset | Inspire |
| Langflow | Platform product (designer) | MIT | Deployable | Adjacent | No | Mixed | Low-code builder UX inspiration; includes Playground w/ tool-call visibility; Share/export and MCP server/client integration points; not a k8s control plane | Inspire |
| Backstage | Platform UI baseline | Apache-2.0 | Deployable | N/A | N/A | Yes | Developer portal framework; Kaigents could ship as plugins for Platform Mode rather than building a net-new portal | Inspire |
| Open WebUI | Chat UI baseline | Custom license w/ branding restriction clause | Deployable | Adjacent | Adjacent | Mixed | License restricts removing/altering “Open WebUI” branding; treat as UX benchmark only | Avoid (core) |
| n8n | Workflow automation benchmark | Sustainable Use License (+ proprietary EE) | Deployable | Adjacent | N/A | Mixed | Fair-code license restricts internal-use and forbids paid hosting/white-labeling without agreement; not commercial-safe OSS | Avoid (core) |
| Argo Workflows | Workflow engine (k8s-native) | Apache-2.0 | Yes | N/A | N/A | Mixed | Great for DAGs/batch workflows (eval, indexing, tool runs); not an always-on agent runtime by itself | Inspire |
| Tekton Pipelines | Workflow engine (k8s-native) | Apache-2.0 | Yes | N/A | N/A | Mixed | CRD-first pipeline engine (`TaskRun`, `PipelineRun`, `Run`) with strong supply-chain/CI/CD ecosystem; may be more machinery than needed for a minimal Kaigents DAG substrate | TBD |
| Temporal | Workflow engine (durable) | MIT | Deployable | N/A | N/A | Mixed | Durable orchestration substrate for long-running processes; adds ops complexity; may be Phase 2 | Inspire |
| Dagger | Workflow/automation engine (dev-first) | Apache-2.0 | Deployable (not CRD-first) | N/A | N/A | Mixed | Local-first containerized automation with caching + OpenTelemetry; likely an integration option (run inside jobs) rather than Kaigents’ core substrate | Inspire |
| KubeRay | Control plane (distributed compute) | Apache-2.0 | Yes | N/A | N/A | Mixed | Useful if we need distributed Python compute; may conflict with Rust-first efficiency goal | TBD |
| Crossplane | Control plane framework | Apache-2.0 | Yes | N/A | N/A | Yes | Control-plane patterns (compositions, packages); could inspire Kaigents packaging/distribution | Inspire |
| AgentField | Control plane (agents as services) | Apache-2.0 | Deployable | N/A/adjacent | N/A/adjacent | Mixed | Control plane concept for agents-as-microservices; evaluate alignment with CRD/GitOps approach | TBD |
| Kubernetes SIG agent-sandbox | Control plane (sandbox pods) | Apache-2.0 | Yes | N/A | N/A | Likely | Sandbox primitive for isolated stateful singleton workloads; CRDs include `Sandbox`, `SandboxTemplate`, `SandboxClaim`; warm pools for fast startup; shutdown time for cleanup | Inspire |
| K8sGPT Operator | Control plane (cluster analysis) | Apache-2.0 | Yes | N/A | N/A | Yes | CRD-driven managed analysis workload; `K8sGPT` config CR + `Result` outputs pattern is useful reference for Kaigents “Run -> Results” modeling | Inspire |
| Agentic Layer Agent Runtime Operator | Control plane (agent workloads) | Apache-2.0 | Yes | Adjacent/unknown | Yes (A2A) | Likely | Framework-agnostic operator focused on deploying agents and discovering “workforces”; `AgenticWorkforce.status` exposes `transitiveAgents` and `transitiveTools`; no obvious first-class `Query`/`Run`/`Result` CRD in bases (execution appears external via A2A) | TBD |
| ToolHive (Stacklok) | Control plane (tools/MCP servers) | Apache-2.0 | Yes (operator) | Yes | N/A | Yes | Operator CRDs: `MCPServer`, `MCPRegistry`; deploys MCP server + ToolHive-Proxy (transport-dependent, incl. stdio attach); supports permission profiles; registry sync + image validation; overlaps with kmcp (compare) | Inspire |
| ARK (Agents at Scale) | Control plane (agent runtime) | Apache-2.0 | Yes (CRDs) | Yes (explicit) | Yes (explicit) | Likely | CRDs include `Agent`, `Team`, `Tool`, `MCPServer`, `Model`, `Memory`, `Query`, `A2AServer`, `A2ATask`, plus evaluator/evaluation and execution-engine CRDs; `Team` supports strategies including `graph` and `selector+graph`; `Query` is a first-class “run” with status responses + timing; CLI drives install + dashboard (`ark dashboard`) but project is tech preview | TBD |
| Kubernetes SIG kube-agentic-networking | Control plane (networking/governance) | Apache-2.0 | Yes (APIs/proposals) | Yes (explicit) | Adjacent | N/A | Defines standardized APIs for governed agent/tool comms; includes provisional MCP tool authorization proposal (do not implement yet) | Inspire |
| SpiffWorkflow | Workflow engine (BPMN/DMN) | LGPL-3.0 | Deployable | N/A | N/A | Mixed | Pure-Python workflow engine underlying SpiffArena; strong low-code BPMN capabilities but LGPL and Python performance concerns | Inspire (concepts) |
| SpiffArena | Workflow platform (BPMN + UI) | LGPL-2.1 | Deployable | N/A | N/A | Mixed | Full platform around SpiffWorkflow; licensing likely blocks use as Kaigents core dependency; useful for UX inspiration for business-process modeling | Inspire |
| dagrs | Workflow engine (DAG, Rust) | MIT OR Apache-2.0 | Library (embed) | N/A | N/A | Yes | Rust async DAG execution framework; lighter than BPMN engines; candidate inspiration for Kaigents “workflow substrate” if DAG semantics are sufficient | TBD |
| lyra | Workflow engine (DAG, Go) | AGPL-3.0 | Library (embed) | N/A | N/A | Yes | Lightweight embedded DAG library concept, but license is not commercial-safe for Kaigents core | Avoid (core) |
| dagx | Workflow engine (DAG, Rust) | MIT | Library (embed) | N/A | N/A | Yes | Minimal runtime-agnostic async DAG executor emphasizing performance; candidate “embedded fast path” substrate | TBD |
| async_dag | Workflow engine (DAG, Rust) | MIT OR Apache-2.0 | Library (embed) | N/A | N/A | Yes | Lightweight async DAG scheduler; useful reference for maximizing parallelism given dependencies inside a runtime | TBD |
| oxigdal-workflow | Workflow engine (DAG, Rust) | Apache-2.0 | Library (embed) | N/A | N/A | Yes | Pure-Rust DAG engine library; license confirmed via docs.rs but still needs maturity review and upstream repo confirmation; potential inspiration for a Rust-first workflow substrate | TBD |

---

## UX Modality Evaluation Criteria

### Declarative / CRD

- **CRD design quality**: composable, versioned, minimal sharp edges.
- **GitOps story**: works cleanly with ArgoCD/Flux.
- **Extensibility**: can add new tool providers/model providers/runtime backends.

### CLI

- **Scaffold**: can generate a new agent/team/tool project quickly.
- **Local dev loop**: run/debug locally with the same contracts as in-cluster.
- **Debug**: fetch logs/traces/status, validate specs, replay runs.

### Web UI Designer

- **Spec-first UI**: UI edits produce versioned specs (CRDs or generated manifests).
- **Diff/preview**: safe changes with a preview, and an audit trail.
- **Guardrails**: validate quotas, security constraints, and tool access.

---

## Research Questions (to answer before committing)

### Control Plane
- What is the most “Kubernetes-native” way to define **teams** (not just agents)?
- Do we want CRDs for `Team`, `Agent`, `ToolServer`, `ModelEndpoint`, `Run`?
- Which platforms already model teams explicitly?

### Runtime
- Which runtimes allow an **efficient always-on loop** with bounded memory + predictable concurrency?
- How hard is it to replace a Python runtime (ADK/LangGraph/CrewAI) with a Rust runtime while keeping spec compatibility?

### Interop
- Minimum viable commitment:
  - MCP for tool integration?
  - A2A for remote agent federation?

### Model Serving
- Is KServe worth adopting for standardized inference endpoints even if we use Lemonade/FLM underneath?
- Can Lemonade/FLM be wrapped as a KServe runtime (or should it remain standalone)?

---

## Next Steps

- Expand candidates list (OSS + commercial).
- Fill in each row with primary-source links (repo/docs) and concrete evidence.
- Add a separate section for “commercial platforms” with careful licensing notes.
