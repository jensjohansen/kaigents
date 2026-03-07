# Platform Research Log (Draft)

**Project:** AI Customer Agents (AI Agents Platform)
**Status:** Draft (research-in-progress)
**Last Updated:** 2026-03-07

---

## Purpose

Capture research notes as **dated data points** (links + key facts + implications). This is meant to be merged later into:

- ITD register
- architecture doc
- PRD/design docs

---

## 2026-03-07

## Market research plan (scoped)

- Scope: identify external offerings that align with Kaigents’ goals (Kubernetes-native, lightweight/low overhead where possible, commercial-safe OSS preferred, MCP/A2A interop), and decide **adopt vs inspire vs avoid**.
- Sources: prefer primary sources (official docs + GitHub repo + LICENSE text).
- Buckets (finish these, then move on to ITDs/tech design):
  - Agent control plane (already covered: ARK, Kagent, Agentic Layer)
  - Tool plane (already covered: kmcp, ToolHive)
  - Workflow substrate (already covered: Argo/Tekton + embedded DAG libs)
  - UX/designer baselines (already covered: Langflow, Flowise)
  - Remaining market scan gaps:
    - Evaluation/observability (Langfuse, Phoenix, Weave)
    - Agent memory products (Zep, Letta)
    - Platform UI baseline (Backstage, Open WebUI)
    - Tool catalog/connectors benchmark (n8n)

### Data point: Kagent (kagent-dev/kagent)
- Source: https://github.com/kagent-dev/kagent
- License: Apache-2.0
- Claim: Kubernetes controller watches kagent custom resources; includes UI + engine; engine runs agents (mentions ADK).
- Primary-source detail: CRD bases in `go/api/config/crd/bases` include `Agent`, `Memory`, `ModelConfig`, `ModelProviderConfig`, `ToolServer`, and `RemoteMCPServer`.
- Additional primary-source detail:
  - `Agent` CRD includes `a2aConfig` which serves an A2A agent endpoint off the kagent controller service (default `:8083`) at `/api/a2a/<agent-namespace>/<agent-name>`.
  - Agents can be invoked via dashboard chat, `kagent invoke --agent <name> --task <text>`, or directly via HTTP using the agent card at `/api/a2a/<ns>/<agent>/.well-known/agent.json`.
  - `Agent.status.conditions` includes readiness/acceptance-type conditions (ready to serve requests; accepted by the system).
- Observed gap: Unlike ARK’s `Query`, Kagent does not obviously expose a first-class `Query`/`Run` CRD for executions and results; “runs” appear to be handled via dashboard/CLI/A2A responses rather than persisted as Kubernetes objects.
- Implication: Strong candidate for k8s-native agent control plane; must validate “teams” modeling and always-on patterns.

### Data point: kMCP (kagent-dev/kmcp)
- Source: https://github.com/kagent-dev/kmcp
- License: Apache-2.0
- Claim: Provides CRDs + controller for MCP server deployments; transport adapters (HTTP/WS/SSE); scaffolding.
- Primary-source detail: Project positions itself as a toolkit spanning (1) CLI scaffolding and local dev, (2) controller + CRD for MCP servers, (3) a transport adapter providing external traffic routing and multi-transport support without code changes.
- Implication: Strong candidate for Kaigents tool plane, especially if we want a “prototype -> production” workflow and multi-transport support as a first-class concern.

### Data point: Current workspace implementation survey (ai-customer-agents)
- Source: local repo `/home/johnj/CascadeProjects/ai-customer-agents`
- Observed state: This workspace currently contains **docs and planning artifacts**, not an implementation.
- Primary-source detail:
  - Top-level contents: `docs/`, `README.md`, `Makefile`, `requirements.txt`, `VERSION`.
  - No Kubernetes manifests detected (`*.yaml`, `kustomization.yaml`, Helm `Chart.yaml`).
  - No application code detected (no `*.py`, `go.mod`, `package.json`, `Dockerfile` within scan depth).
- Implication: Kaigents work in this repo is at the research/design stage. Implementation will require either:
  - creating new service repos (controller/CLI/dashboard) and linking them from docs, or
  - adding initial skeleton code + deployment artifacts into this repo.

### Data point: Langfuse
- Source: https://github.com/langfuse/langfuse
- License: MIT for most code; explicit carve-out for `ee/` (enterprise) under separate license per repository LICENSE header.
- Primary-source detail: Repository LICENSE states content under `ee/`, `web/src/ee/`, and/or `worker/src/ee/` is licensed per `ee/LICENSE`; content outside those directories is “MIT Expat”.
- Claim: Open source LLM engineering platform (observability/metrics/evals/datasets) with OpenTelemetry integrations.
- Implication: Strong market-aligned candidate for Kaigents **eval + observability plane** with commercial-safe core. Treat EE as optional; ensure Kaigents depends only on MIT-licensed subset if embedding.

### Data point: Arize Phoenix
- Source: https://github.com/Arize-ai/phoenix
- License: Elastic License 2.0 (ELv2)
- Primary-source detail: ELv2 explicitly prohibits providing the software “as a hosted or managed service” that gives third parties access to substantial functionality.
- Claim: AI observability and evaluation.
- Implication: Not aligned with “commercial-safe OSS” requirement for Kaigents core (SaaS/managed-service restriction). Treat as **benchmark/inspiration** only, not a dependency.

### Data point: W&B Weave
- Source: https://github.com/wandb/weave
- License: Apache-2.0 (repository LICENSE)
- Claim: Toolkit/platform for LLM app development and evaluation.
- Implication: Commercial-safe OSS baseline for eval/observability workflows; evaluate as inspiration/integration depending on how tightly coupled it is to W&B hosted services.

### Data point: Zep (memory server)
- Source: https://github.com/getzep/zep
- License: Apache-2.0 (repository LICENSE)
- Claim: Memory server / retrieval system for agents (context engineering).
- Implication: Commercial-safe option to study for “memory as a service” patterns; likely an integration candidate rather than Kaigents core dependency.

### Data point: Letta (memory-first agent platform)
- Source: https://github.com/letta-ai/letta and https://github.com/letta-ai/letta-code
- License: Apache-2.0 (letta-code repository LICENSE; verify core Letta repo license separately)
- Claim: Platform for stateful agents with memory-first posture.
- Implication: Worth tracking for product concepts and memory modeling patterns; validate licensing boundaries across repos before considering any reuse.

### Data point: Backstage
- Source: https://github.com/backstage/backstage
- License: Apache-2.0
- Claim: Framework for building developer portals.
- Implication: Strong “platform UI baseline” for Kaigents Platform Mode: Kaigents could ship as a Backstage plugin rather than building a net-new portal; commercial-safe.

### Data point: Open WebUI
- Source: https://github.com/open-webui/open-webui
- License: Custom license with a branding restriction clause (derived from BSD-3 with additional conditions).
- Primary-source detail: LICENSE prohibits altering/removing “Open WebUI” branding except in limited circumstances (including <=50 end users or enterprise license).
- Claim: User-facing AI chat UI.
- Implication: **Avoid as a core dependency** for Kaigents due to branding restriction and license complexity; still useful as a UX benchmark.

### Data point: n8n
- Source: https://docs.n8n.io/sustainable-use-license/
- License: Sustainable Use License (fair-code) + separate proprietary n8n Enterprise License for `.ee.` code.
- Primary-source detail: License limits use to internal business purposes; prohibits white-labeling and hosting n8n as a paid service without an additional commercial agreement (“n8n Embed”).
- Claim: Workflow automation + connector ecosystem.
- Implication: Great market benchmark for connector UX, but **not commercial-safe OSS** for Kaigents core; avoid depending on it.

### Data point: Google ADK (google/adk-python)
- Source: https://github.com/google/adk-python
- License: Apache-2.0
- Claim: Code-first agent toolkit; supports multi-agent; explicitly mentions integration with A2A.
- Implication: Candidate runtime framework, but Python-centric. Consider as optional plugin runtime; do not bake into platform core.

### Data point: A2A protocol
- Source: https://github.com/a2aproject/A2A
- Claim: Open protocol for agent-to-agent interoperability.
- Implication: Consider as optional interop layer for cross-agent-system federation; complementary to MCP.

### Data point: CrewAI
- Source: https://github.com/crewAIInc/crewAI
- License: MIT
- Claim: Python multi-agent orchestration framework.
- Implication: Consider for UX/team-abstraction inspiration; evaluate operational/k8s patterns separately.

### Data point: KServe
- Source: https://github.com/kserve/kserve
- License: Apache-2.0
- Claim: k8s inference serving control plane.
- Implication: Possible model-serving plane; evaluate whether it helps if we standardize model endpoints (even if Lemonade/FLM underneath).

### Data point: Knative Serving
- Source: https://github.com/knative/serving
- License: Apache-2.0
- Claim: request-driven compute, scale-to-zero.
- Implication: Might be useful for bursty tasks; may not align with “mostly always-on agents”.

### Data point: Lemonade
- Source: https://github.com/lemonade-sdk/lemonade
- License: Apache-2.0 (with NOTICE)
- Claim: Local model serving optimized for GPUs + NPUs.
- Implication: Strong fit for AMD Ryzen AI goal; validate k8s deployment and multi-tenant operations.

### Data point: OpenClaw Kubernetes Operator
- Source: https://github.com/openclaw-rocks/k8s-operator
- License: Apache-2.0
- Claim: Operator reconciles a single agent instance custom resource into a managed stack (security, observability, storage, sidecars, etc.). Includes a self-configure mechanism mediated by an allowlist policy.
- Implication: Strong reference implementation for operator/CRD ergonomics and “agent instance lifecycle”. Use as inspiration even if runtime/product scope differs.

### Data point: Dify
- Source: https://github.com/langgenius/dify
- License: "Dify Open Source License" (modified Apache 2.0 with additional conditions)
- Claim: Visual workflow canvas + agent capabilities + model integrations; self-host or cloud.
- Critical license constraints:
  - Prohibits operating a multi-tenant environment without explicit authorization.
  - Disallows removing/modifying logo/copyright in the frontend.
  - Producer can tighten/relax license terms; contributor code can be used commercially.
- Implication: Avoid as a dependency for Kaigents core (license conflicts with a permissive, multi-tenant platform). Safe only as inspiration.

### Data point: Flowise
- Source: https://github.com/FlowiseAI/Flowise
- License: Apache-2.0 for most code, but enterprise directory and some files are commercial-only.
- Claim: Visual builder for agents/workflows.
- Implication: Potential UI inspiration; if considering reuse, validate we only depend on Apache-licensed subset.

### Data point: Langflow
- Source: https://github.com/langflow-ai/langflow
- License: MIT
- Claim: Low-code tool for building and deploying AI agents and workflows.
- Implication: Good UI inspiration with permissive license; not itself a Kubernetes control plane.

### Data point: UX modality baseline — Kagent CLI + Dashboard
- Source: https://kagent.dev/docs/kagent/getting-started/quickstart and https://kagent.dev/docs/kagent/concepts/architecture
- License: Apache-2.0
- Claim: Kagent supports both a CLI and a web dashboard as first-class entry points.
- Primary-source detail:
  - CLI includes: `kagent dashboard`, `kagent get`, `kagent invoke`, `kagent install`.
  - Architecture docs describe: Go controller, Python engine (conversation loop) built on ADK, CLI connecting to engine, and dashboard for managing/working with agents.
- Implication: Strong reference for Kaigents UX layering: CRD/operator core with both CLI and dashboard; CLI is not just “kubectl wrappers” but includes interactive invocation.

### Data point: Argo Workflows
- Source: https://github.com/argoproj/argo-workflows
- License: Apache-2.0
- Claim: Kubernetes-native workflow engine for DAGs and step-based workflows.
- Implication: Great substrate for batch-y platform tasks (eval pipelines, dataset ingestion, indexing, tool execution jobs). Not a full agent platform by itself.

### Data point: Temporal
- Source: https://github.com/temporalio/temporal
- License: MIT
- Claim: Durable workflow orchestration service.
- Implication: Strong option for long-running, reliable orchestration across failures/retries/timeouts. Likely too heavy for earliest Kaigents MVP, but a strong Phase 2 candidate.

### Data point: Argo Workflows
- Source: https://github.com/argoproj/argo-workflows and https://argoproj.github.io/workflows/
- License: Apache-2.0
- Claim: Kubernetes-native workflow engine implemented as Kubernetes CRDs; supports step-based workflows and DAG dependencies where each step is a container.
- Implication: Good default “k8s-native DAG substrate” when we need parallel job orchestration (eval pipelines, indexing, tool jobs). Likely heavier than an embedded DAG library, but is a known-good, standard K8s integration path.

### Data point: Tekton Pipelines
- Source: https://github.com/tektoncd/pipeline and https://tekton.dev/docs/pipelines/
- License: Apache-2.0
- Claim: Kubernetes-native pipeline engine with primitives like `Task`, `TaskRun`, `Pipeline`, `PipelineRun`, and `Run`.
- Implication: Strong for CI/CD-style pipelines and “pipeline runs as CRDs”. For Kaigents, could be used for deterministic platform automation, but may be more machinery than needed if the goal is a minimal, fast DAG substrate.

### Data point: Dagger
- Source: https://github.com/dagger/dagger
- License: Apache-2.0
- Claim: Local-first automation engine to build/test/ship codebases; programmable execution engine with SDKs, containerized steps, caching, and OpenTelemetry tracing.
- Implication: Great inspiration for developer-experience, caching, and observability patterns. Not Kubernetes-native CRD-first by default; likely better as an integration option (run a Dagger module inside a Kaigents job) than as the core workflow substrate.

### Data point: lyra (Go DAG task orchestration library)
- Source: https://github.com/sourabh-kumar2/lyra
- License: AGPL-3.0
- Claim: Type-safe DAG task orchestration library for Go with automatic concurrency.
- Implication: License is not commercial-safe for Kaigents core. Avoid adopting as a dependency; conceptually useful as a “minimal embedded DAG executor” reference.

### Data point: oxigdal-workflow (Rust DAG library)
- Source: https://docs.rs/crate/oxigdal-workflow/latest
- License: Apache-2.0 (confirmed via docs.rs crate docs)
- Claim: Pure-Rust DAG-based workflow engine library; advertises scheduling, parallel execution planning, and additional workflow features.
- Primary-source detail: docs.rs lists repository/homepage as https://github.com/cool-japan/oxigdal (appears to be a monorepo for the OxiGDAL ecosystem).
- Implication: Potential inspiration for a Rust-first embedded workflow substrate. Despite permissive license, it is domain-associated (geospatial) and brings a larger dependency surface; treat as an inspiration/reference until maturity and non-domain fit are validated.

### Data point: dagx (Rust embedded async DAG executor)
- Source: https://docs.rs/dagx/latest/dagx/
- License: MIT
- Claim: Minimal, runtime-agnostic async DAG executor emphasizing performance; documents an inline execution fast-path for sequential layers.
- Primary-source detail: docs.rs lists homepage as https://github.com/swaits/dagx.
- Implication: Good candidate reference for an embedded “fast DAG” substrate if Kaigents chooses to run deterministic dependency graphs inside a service. Still requires Kaigents to supply persistence/durability and K8s integration.

### Data point: async_dag (Rust embedded async DAG scheduler)
- Source: https://docs.rs/async_dag/latest/async_dag/
- License: MIT OR Apache-2.0
- Claim: Async task scheduling utility that runs tasks at maximum possible parallelism given DAG dependencies; includes fail-fast variant (`TryGraph`).
- Primary-source detail: docs.rs links to repository https://github.com/chubei-oppen/async_dag.
- Implication: Another lightweight embedded DAG reference option; useful when the goal is “parallelize dependency graphs inside a runtime” rather than run each step as a separate Kubernetes workload.

### Data point: KubeRay
- Source: https://github.com/ray-project/kuberay
- License: Apache-2.0
- Claim: Kubernetes operator and toolkit to run Ray applications on Kubernetes.
- Implication: Useful if Kaigents needs Ray-style distributed execution (mostly Python-centric). Potential mismatch with Rust-first efficiency goals; keep as an option.

### Data point: Crossplane
- Source: https://github.com/crossplane/crossplane
- License: Apache-2.0
- Claim: Framework for building cloud-native control planes.
- Implication: Not an agent platform, but strong inspiration for how to build/ship a Kubernetes control plane (packaging, compositions, extension model).

### Data point: AgentField
- Source: https://github.com/Agent-Field/agentfield
- License: Apache-2.0
- Claim: Control plane that treats AI agents like microservices.
- Implication: Candidate control-plane alternative to evaluate. Key question is whether it can be made CRD/GitOps-first and whether its runtime/SDK approach fits Kaigents.

### Data point: Kubernetes SIG agent-sandbox
- Source: https://github.com/kubernetes-sigs/agent-sandbox
- License: Apache-2.0
- Claim: Sandbox CRD and controller for isolated, stateful, singleton workloads; positioned as useful for AI agent runtimes.
- Primary-source detail: Google Open Source blog describes core CRDs: `Sandbox`, `SandboxTemplate`, `SandboxClaim`; plus extensions like warm pools for sub-second startup and shutdown time for automated cleanup; supports multiple isolation backends (e.g., gVisor, Kata).
- Implication: Strong inspiration for safe execution environments and for Kaigents CRD design patterns around “claiming” an execution environment, fast startup, and deterministic teardown.

### Data point: K8sGPT Operator
- Source: https://github.com/k8sgpt-ai/k8sgpt-operator
- License: Apache-2.0
- Claim: Kubernetes operator that manages a K8sGPT analysis workload via a `K8sGPT` custom resource; outputs surfaced as `Result` custom resources.
- Implication: Useful CRD/operator reference pattern for Kaigents: reconcile a spec-driven analysis/run config into a managed workload and emit first-class result objects (for UI, CLI, and GitOps visibility).

### Data point: Agentic Layer Agent Runtime Operator
- Source: https://github.com/agentic-layer/agent-runtime-operator
- License: Apache-2.0
- Claim: Kubernetes operator providing a unified, framework-agnostic way to deploy and manage agentic workloads; abstracts framework-specific wiring (observability stack, model routers, etc.).
- Primary-source detail:
  - Docs provide install via raw Kubernetes YAML and a Flux GitOps install path (OCIRepository + Kustomization).
  - CRD bases include: `Agent`, `AgenticWorkforce`, `ToolServer`, `AgentRuntimeConfiguration`, plus gateway and gateway-class CRDs for AI/Tool/Agent gateways.
  - Agents communicate via A2A protocol (docs show `protocols: - type: A2A`).
  - `AgenticWorkforce` models “teams” as entry-point agents with operator-discovered transitive agents and tools (surfaced in `.status`).
- Additional primary-source detail: `AgenticWorkforce.status` includes `transitiveAgents` (cluster or remote agents, including remote URL to an agent card) and `transitiveTools` discovered from those agents.
- Observed gap: In the CRD bases inspected, there is no obvious first-class `Query`/`Run`/`Result`-style CRD for representing executions and outputs in Kubernetes objects (execution likely occurs externally via A2A calls and/or a separate component).
- Implication: This is one of the closest matches so far to Kaigents’ desired shape (CRD/operator-first + team modeling + interop protocol). Need to validate MCP/tooling story and whether the gateway CRDs align with Kaigents’ desired tool plane (kmcp).

### Data point: ToolHive (Stacklok)
- Source: https://github.com/stacklok/toolhive
- License: Apache-2.0
- Claim: Secure-by-default deployment and management for MCP servers across desktop/CLI/Kubernetes; includes a Kubernetes Operator.
- Primary-source detail:
  - Operator CRDs include `MCPServer` and `MCPRegistry`.
  - `MCPServer` reconciliation creates a workload + Service and configures permissions/settings; architecture diagrams show a ToolHive-Proxy mediating communication, including stdio attach mode via Kubernetes API.
  - `MCPRegistry` supports syncing registry data (e.g., ConfigMap, Git), runs a registry API for discovery, and performs image validation.
- Implication: ToolHive is a serious alternative tool plane with strong emphasis on secure-by-default deployment + registry/discovery. There is real overlap with kmcp; likely decision is adopt one as the primary “MCP control plane” and borrow concepts (e.g., registry, permission profiles, proxy patterns) from the other.

### Data point: ARK (Agents at Scale)
- Source: https://github.com/mckinsey/agents-at-scale-ark and https://mckinsey.github.io/agents-at-scale-ark/
- License: Apache-2.0
- Claim: Kubernetes runtime environment to host AI agents with built-in CRDs for agents, models, tools, memory, and evaluation; includes CLI and dashboard.
- Primary-source detail:
  - Quickstart installs CLI via npm (`@agents-at-scale/ark`), then `ark install`, optional `ark models create default`, and `ark dashboard`.
  - Reference docs list core resources including `Models`, `Agents`, `Teams`, `Queries`, `Tools`, `MCPServers`, `Memories`, plus `A2AServer`, `A2ATask`.
  - `Team` supports strategies: sequential, round-robin, selector, graph, and `selector + graph` with explicit edge constraints.
  - `Query` is a first-class resource representing execution; status includes phase, per-target responses, start/completion timestamps, and optional A2A metadata (`contextId`, `taskId`).
- Implication: One of the closest end-to-end “agent platform on Kubernetes” candidates (CRDs + controller + dashboard). Needs deeper evaluation for licensing stability, extensibility, and fit with Kaigents’ MCP-first tool plane and AMD-optimized model serving.

### Data point: UX modality baseline — ARK CLI + Dashboard + dev mode
- Source: https://mckinsey.github.io/agents-at-scale-ark/quickstart/ and https://mckinsey.github.io/agents-at-scale-ark/
- License: Apache-2.0
- Claim: ARK provides a CLI (`ark`) and dashboard (`ark dashboard`) as part of the standard install.
- Primary-source detail:
  - Quickstart flow: `npm install -g @agents-at-scale/ark`, `ark install`, optional `ark models create default`, `ark dashboard`.
  - Docs call out ARK is a **Technical Preview** with a disclaimer.
  - Local dev mode: `devspace dev` and `devspace run routes` for service routes (including ark-dashboard).
- Implication: Useful reference for “CLI drives install + dashboard access + dev-mode live reload”. Risk: tech preview status; we should emulate patterns, not depend on it.

### Data point: UX modality baseline — Langflow visual editor + Playground + Share/MCP
- Source: https://docs.langflow.org/concepts-overview and https://docs.langflow.org/
- License: MIT
- Claim: Visual editor for building flows; includes a Playground for running/chatting with flows and viewing tool calls.
- Primary-source detail:
  - Playground supports running a flow, viewing inputs/outputs, and (for agent components) displaying tool calls and outputs.
  - Share options include export/import, API access snippets, embedding, and exposing flows as an **MCP Server**.
- Implication: Very strong inspiration for Kaigents’ eventual low-code designer UX: (1) a “playground” that shows tool invocations, (2) simple share/export, and (3) MCP-native packaging of flows.

### Data point: UX modality baseline — Flowise visual editor capabilities
- Source: https://docs.flowiseai.com
- License: Apache-2.0 for the repo with some commercial-only enterprise parts (verify before reuse).
- Claim: Visual editor for building assistants/agent flows; documents broad capabilities (orchestration, logs/debugging, MCP integration, RBAC/SSO).
- Primary-source detail:
  - Flowise UX concepts: `Assistant`, `Chatflow`, and `Agentflow` (superset) for building single/multi-agent systems.
  - Docs claim execution logs, visual debugging, and MCP client/server nodes.
- Implication: Good inspiration for feature checklist of a designer UI, but licensing and “enterprise-only parts” concerns mean we should treat it primarily as inspiration.

### Data point: Kubernetes SIG kube-agentic-networking
- Source: https://github.com/kubernetes-sigs/kube-agentic-networking
- License: Apache-2.0
- Claim: Standardized APIs for secure, governed communication between agents and tools (and potentially LLMs), designed around user intent rather than specific protocols; explicitly considers MCP and A2A.
- Primary-source detail: Tool Authorization proposal defines CRDs like `Backend` and `AccessPolicy` to enforce MCP `tools/call` authorization, but is explicitly marked **PROVISIONAL** and says do not implement/use in production.
- Implication: Strong inspiration for Kaigents Platform Mode governance/RBAC/policy layer around tools. Track this SIG’s direction, but do not depend on provisional APIs.

### Data point: SpiffWorkflow
- Source: https://github.com/sartography/SpiffWorkflow
- License: LGPL-3.0 (COPYING header is GNU LGPL v3)
- Claim: Workflow engine implemented in pure Python; supports BPMN/DMN processing and is the workflow library underlying SpiffArena.
- Implication: Strong source of inspiration for BPMN modeling semantics and “business process” UX, but LGPL licensing and Python runtime performance make it a poor fit as a Kaigents core dependency.

### Data point: SpiffArena
- Source: https://github.com/sartography/spiff-arena
- License: LGPL-2.1 (LICENSE header is GNU LGPL v2.1)
- Claim: Web-based platform around SpiffWorkflow for building/running/monitoring executable BPMN diagrams.
- Implication: Useful UX reference for low-code BPMN-based process design, but LGPL licensing likely blocks direct reuse in Kaigents core.

### Data point: dagrs
- Source: https://github.com/dagrs-dev/dagrs
- License: Dual MIT + Apache-2.0 (repo lists LICENSE-MIT and LICENSE-APACHE)
- Claim: High-performance asynchronous DAG task orchestration framework in Rust.
- Implication: If Kaigents decides “DAG is enough” for most agent-business-process workflows, a Rust DAG substrate like this is closer to the desired performance profile than BPMN engines. Needs evaluation for durability/state persistence semantics (vs just in-process execution) and k8s control-plane integration.

---

## Decision memo (draft): ARK vs Kagent vs Agentic Layer for Kaigents control plane

### Observed modeling differences

- **ARK**
  - **Team modeling:** First-class `Team` CRD with execution strategies including `graph` and `selector + graph`.
  - **Run/results modeling:** First-class `Query` CRD with `status` capturing responses, timestamps, phase, and A2A metadata.
  - **Implication:** Strongest primary-source reference for a CRD-first platform where executions are GitOps-visible and “workflow-ish teams” are explicit.

- **Kagent**
  - **Team modeling:** Not yet evidenced as a first-class “Team” CRD (current evidence is Agent-centric).
  - **Run/results modeling:** Execution is primarily request/response via dashboard chat, `kagent invoke`, and controller-served A2A endpoints (no obvious `Query`/`Run` CRD persisted as Kubernetes objects).
  - **Implication:** Strong packaging reference (operator + UI + A2A serving + CRD-based agent/tool/model config). If Kaigents requires GitOps-visible runs/results, this likely needs an additional CRD layer.

- **Agentic Layer Agent Runtime Operator**
  - **Team modeling:** First-class `AgenticWorkforce` CRD with operator-discovered `.status.transitiveAgents` and `.status.transitiveTools` (including remote agent card URLs).
  - **Run/results modeling:** No obvious `Query`/`Run`/`Result` CRD in inspected CRD bases; execution appears external via A2A and/or separate components.
  - **Implication:** Strong reference for “workforce inventory / discovery / topology” and gateway-based platform engineering patterns, but likely needs a Kaigents execution object layer.

### Provisional guidance (adopt vs emulate)

- **Prefer emulating ARK’s `Query` pattern** (execution as CRD with results in `status`) even if we do not adopt ARK wholesale.
- **Prefer emulating ARK’s `Team` strategy surface** (especially `graph` and `selector + graph`) if we want lightweight workflow semantics without adopting a full BPMN engine.
- **Borrow Kagent’s UI/CLI/A2A serving patterns** as reference for operator + dashboard ergonomics.
- **Borrow Agentic Layer’s discovery pattern** (`transitiveAgents`/`transitiveTools`) to support Platform Mode inventory, governance, and “what can this workforce do?” UX.

### Minimum Kaigents CRD set (composable approach)

- **`Agent`**
  - Desired shape: declarative agent deployment/runtime + references to model config and tool references.
- **`Workforce`/`Team`**
  - Desired shape: team membership + strategy (`sequential`, `round-robin`, `selector`, `graph`, `selector+graph`) + optional graph constraints.
- **`ModelConfig`**
  - Desired shape: model endpoint/config + secret refs; compatible with multiple providers and/or KServe/Lemonade endpoints.
- **`MCPServer`** and **`Tool`**
  - Desired shape: tool definitions that reference MCP servers, with transport/address and auth/secret references.
- **`Run`/`Query`**
  - Desired shape: first-class execution object referencing an `Agent` or `Team`, capturing input + parameters, with results and timing in `status`.
- **`Result`/`Artifact`** (optional but useful)
  - Desired shape: model “run outputs” as separate CRs when results are large or need lifecycle/retention policies.
- **`Policy` (future / track SIG)**
  - Desired shape: governed tool access (inspired by kube-agentic-networking `Backend`/`AccessPolicy`), but avoid depending on provisional APIs.

---

## Decision memo (draft): workflow substrate for Kaigents (simple / lightweight / fast)

### What “lightweight” likely means for Kaigents

- **Fast startup and low idle overhead** (prefer in-cluster controllers that are small, or embedded libraries).
- **DAG-first** semantics (dependencies + concurrency) before BPMN/durable saga semantics.
- **Execution visibility** (at least “Run -> Results” objects or well-structured event logs) without requiring a large control-plane stack.

### Candidate buckets

- **Kubernetes-native DAG engines (CRD-based):**
  - **Argo Workflows**: proven K8s-native DAG engine; good default when we want jobs-as-pods with clear orchestration and K8s primitives.
  - **Tekton Pipelines**: strong “runs as CRDs” model and supply-chain ecosystem; may be overkill for a minimal agent-workflow substrate.

- **Embedded DAG libraries (Rust/Go):**
  - Best for performance and minimal overhead *inside* a Kaigents runtime, but require Kaigents to provide durability, persistence, and K8s execution integration.
  - Licensing can be a blocker (e.g., `lyra` is AGPL).
  - Permissive embedded candidates worth tracking include `dagx` (MIT) and `async_dag` (MIT OR Apache-2.0).

- **Developer-first automation engines (not CRD-first):**
  - **Dagger**: strong caching + observability patterns; likely an integration target rather than the core substrate.

### Provisional recommendation

- **MVP path:** Use **Argo Workflows** as the default K8s-native DAG substrate for “batch-y” workflows (eval, indexing, tool jobs), while keeping the agent runtime control plane separate.
- **Kaigents-native path:** Define a minimal **`Run`/`Query`** CRD pattern (similar to ARK `Query`) for agent/team executions; optionally back complex deterministic flows with Argo.
- **Phase 2:** Revisit “embedded Rust DAG” only if Argo’s operational footprint is too heavy or we need tighter in-process orchestration; validate permissive-licensed crates and add persistence/durability semantics.

### Choice matrix (draft)

| Option | Overhead (idle/runtime) | Execution visibility | Durability | K8s integration effort | License risk | Best fit |
|---|---|---|---|---|---|---|
| **Argo Workflows (CRDs)** | Medium | High (CRDs, status) | Medium (workflow CR history; persistence depends on setup) | Low | Low | Batch-y workflows where each step is a pod/container (eval, indexing, ETL, tool jobs) |
| **Tekton Pipelines (CRDs)** | Medium | High (TaskRun/PipelineRun CRDs) | Medium (run CR history; persistence depends on setup) | Low | Low | CI/CD-style deterministic pipelines; supply-chain heavy environments |
| **Embedded DAG (dagx/async_dag inside service)** | Low | Low by default (must add events/logs) | Low by default (must persist state) | Medium–High | Low | Fast in-process orchestration of small deterministic graphs; “micro-workflows” within an agent runtime |
| **Embedded DAG (oxigdal-workflow)** | Medium (bigger crate surface) | Medium (library + optional server features) | Unknown (claims state persistence; needs validation) | Medium–High | Low | Potential reference for richer embedded workflow runtime; validate maturity and non-domain fit |
| **Dagger (engine + SDKs)** | Medium | Medium–High (OTel tracing) | Low–Medium (not CRD-native; depends on usage) | Medium | Low | Developer automation modules executed in CI/jobs; inspiration for caching/observability |

### Recommended path for “simple, lightweight, fast”

- **MVP (platform-level workflows):** Use **Argo Workflows** for batch-y DAGs that naturally map to pods/containers.
- **MVP (agent/team execution):** Implement Kaigents’ own **`Run/Query` CRD** so every execution has GitOps-visible status/results.
- **Fast path (micro-workflows inside runtime):** If we need ultra-low overhead orchestration inside the agent runtime, prototype with **`dagx`** (MIT) as the embedded DAG executor and emit structured events that can also be attached back to the `Run/Query.status`.

---

## Proposal (draft): minimal Kaigents execution CRDs (Run/Query + Artifact)

This section sketches a minimal execution object model for Kaigents that is:

- **GitOps-visible** (execution intent in `spec`, results in `status`)
- **Lightweight** (small CRDs; large payloads stored externally)
- **Composable** (works whether execution is in-process DAG, A2A call, or Argo workflow)

### CRD 1: `Query` (or `Run`)

**Purpose:** represent a single execution against an `Agent` or `Team`.

**Key design rules:**

- `spec` is the immutable-ish intent.
- `status` is the authoritative output + progress.
- large outputs go into `Artifact` objects (or object storage) referenced by `status`.

#### Suggested `spec`

- `target`:
  - `type`: `agent` | `team`
  - `name`
  - `namespace` (optional; defaults to same namespace)
- `input`:
  - `type`: `text` | `messages` | `ref`
  - `text` (optional)
  - `messages` (optional; array)
  - `ref` (optional; ConfigMap/Secret/Artifact reference)
- `parameters` (optional): free-form map used for templating/variable expansion.
- `timeoutSeconds` (optional)
- `stream` (optional): request streaming output.
- `conversationId` (optional): stable conversation/thread key.
- `retention` (optional):
  - `ttlSecondsAfterFinished`
  - `maxArtifacts`
- `execution` (optional): hints/constraints
  - `mode`: `inprocess` | `a2a` | `argo`
  - `engineRef` (optional)

#### Suggested `status`

- `phase`: `pending` | `running` | `done` | `failed` | `canceled` | `timedout`
- `conditions`: standard K8s conditions (`Accepted`, `Running`, `Succeeded`).
- `startTime`, `completionTime`
- `responses`: list of per-target or per-step outputs:
  - `content` (small text)
  - `artifactRef` (for large output)
  - `metadata` (token usage, model, tool invocations summary)
- `error`:
  - `code`
  - `message`
  - `retryable` (bool)
- `a2a` (optional): `contextId`, `taskId` (when executed via A2A)
- `engine` (optional):
  - `argoWorkflowRef` (when backed by Argo)
  - `executorPodRef` (when executed in a dedicated pod)

#### YAML skeleton

```yaml
apiVersion: kaigents.io/v1alpha1
kind: Query
metadata:
  name: example-query
spec:
  target:
    type: agent
    name: example-agent
  input:
    type: text
    text: "Summarize the incident and propose next steps."
  timeoutSeconds: 120
  retention:
    ttlSecondsAfterFinished: 86400
```

### CRD 2 (optional but recommended): `Artifact`

**Purpose:** store large outputs and intermediate artifacts out-of-band (object store, database, etc.) while keeping CRDs small.

#### Suggested `spec`

- `ownerQueryRef` (optional): link back to the Query/Run.
- `mimeType`
- `sizeBytes` (optional)
- `storage`:
  - `type`: `inline` | `s3` | `http` | `db`
  - for `s3`: `bucket`, `key`, `endpoint`, `region` (optional)
  - for `http`: `url`
---

## Decision memo (draft): UX modalities for Kaigents (CRD + CLI + Dashboard + Designer)

### Findings (primary-source inspired)

- **Kagent** demonstrates a clean separation of:
  - **controller + CRDs** (platform core)
  - **engine** (runtime loop)
  - **CLI** (including invoke)
  - **dashboard** (web chat + management)

- **ARK** demonstrates a “CLI as the installer + launcher” pattern and provides a dashboard, plus a dev-mode workflow.

- **Langflow/Flowise** demonstrate that the core value of a low-code designer is not just drawing graphs:
  - a **Playground** that shows tool calls and outputs
  - simple **Share/Export** and “turn it into an API/tool” paths
  - MCP-native integration points

### Provisional sequencing recommendation

- **Phase 1 (minimum lovable platform UX):**
  - CRDs + reconciliation (GitOps-first)
  - A Kaigents CLI that covers:
    - install/uninstall
    - list/get resources
    - invoke/run (with structured output)

- **Phase 1.5 (operator UX):**
  - A small dashboard focused on:
    - browsing agents/teams/tools/models
    - running queries and viewing outputs/events

- **Phase 2+ (designer UX):**
  - Low-code designer with:
    - graph editing
    - playground that surfaces tool calls
    - export/share to CRDs and/or MCP tools

### UX patterns to emulate (concrete)

- **CLI “invoke” as first-class** (Kagent): not just `kubectl apply`, but a supported interactive execution entry point.
- **Dashboard accessible via CLI** (Kagent/ARK): a consistent `kaigents dashboard` command that handles port-forwarding/routes.
- **Playground with tool-call trace** (Langflow): make tool invocation visibility a first-order concept.
- **Export/share** (Langflow): generate artifacts that can be deployed as CRDs and/or exposed as MCP tools.

- `hash` (optional): content hash for integrity.
- `expiresAt` (optional)

#### Suggested `status`

- `phase`: `pending` | `ready` | `failed`
- `conditions`
- `resolvedUrl` (optional): pre-signed URL or internal URL

#### YAML skeleton

```yaml
apiVersion: kaigents.io/v1alpha1
kind: Artifact
metadata:
  name: example-artifact
spec:
  mimeType: text/html
  storage:
    type: s3
    bucket: kaigents-artifacts
    key: runs/example-query/report.html
```

### Retention / GC conventions (minimal)

- Prefer TTL-based GC on `Query` and `Artifact` via a controller (or `ttlSecondsAfterFinished` semantics similar to Jobs).
- The controller should:
  - delete `Artifact` objects when their owning `Query` expires
  - optionally delete underlying object-store keys when deleting an `Artifact` (configurable)

### Status conventions for UI/CLI

- `phase` is always set once accepted.
- `completionTime` must be set for terminal phases: `done`, `failed`, `canceled`, `timedout`.
- `responses` should remain bounded (store full transcripts in artifacts when needed).

## Open questions

- What existing platform offers: team designer + ops control plane + permissive license?
- How much of Kagent’s engine can be swapped for a Rust runtime while keeping CRDs/UX?
- What is the minimal set of CRDs required for “team + tool + model + run”?
- Can we treat model serving as a separate plane (KServe vs Lemonade vs both)?
