
# Kaigents Implementation Plan

This document translates the Kaigents PRD into an execution-oriented implementation plan.

Source-of-truth references:

- `docs/product/kaigents-prd.md`
- `docs/architecture/kaigents-architecture-and-design.md`
- `docs/research/technology/itd-register.md`
- `docs/implementation/kaigents-implementation-tracker.md`

## Milestone 1 close-out and handoff (Solo Mode MVP)

Milestone 1 is closed as the point where Kaigents can execute a real agent run **in-cluster**, with a durable run timeline and durable artifact plumbing, through the CRD-driven operator and runner path.

Acceptance criteria agent:

- `docs/implementation/milestone-1-acceptance-student-research-assistant.md`

Milestone 1 close-out notes:

- The in-cluster Student Research Assistant acceptance path has been validated through the operator-managed `Run` -> `Job` execution path.
- Timeline, tool/model integration, and artifact plumbing are considered sufficient to close Milestone 1 as the Solo Mode MVP execution baseline.
- Result visibility and authoritative output surfacing on `Run.status` are intentionally deferred rather than used to extend Milestone 1 scope.
- Milestone 1 should be treated as closed so Milestone 2 can start with a clean focus on domain-model hardening and productization.

### Milestone 1 close-out checklist

Use this checklist as the close-out record for Milestone 1.

- **Acceptance criteria agent is actually demoable**
  - Execute the Student Research Assistant acceptance flow end-to-end:
    - web search
    - select 3-5 insights
    - read sources
    - synthesize markdown
    - store a durable artifact
  - Verify the run succeeds in-cluster, not only in a mocked or local-only environment.

- **PRD alignment is preserved**
  - Confirm Milestone 1 remains **Solo Mode MVP (CRD + CLI + embedded workflow)**.
  - Confirm scope still matches the PRD Milestone 1 definition of done:
    - install Kaigents in a cluster
    - define an agent and tools declaratively
    - run a basic multi-step agent workload
    - produce a durable, queryable run timeline
    - fetch artifacts via stable URLs
  - Do not pull Milestone 3 concerns into Milestone 1:
    - no requirement for Temporal adoption
    - no requirement for first-class Process/Task resources
    - no requirement for long-running human waits / bounded rework loops
  - Treat retries as DAG node-execution behavior, not as evidence that Milestone 1 supports cyclic process graphs.

- **Architecture and design constraints are satisfied**
  - `Run` reconciliation drives actual execution from CRDs through terminal completion.
  - Embedded DAG execution remains the Milestone 1 workflow substrate.
  - Tool calls, model calls, workflow-step events, and artifact events all flow into one durable run timeline.
  - Artifact access preserves the intended stable URL / proxy pattern.
  - Model endpoint discovery works for both:
    - in-cluster service DNS
    - developer-local / OS-hosted Lemonade endpoints used by the current environment

- **ITD constraints are respected**
  - ITD-02:
    - Lemonade remains the primary model-serving runtime for Milestone 1 integration.
    - FastFlowLM proprietary kernels are treated as integrate-only and are not bundled into Kaigents.
  - ITD-08:
    - Milestone 1 uses the embedded DAG substrate with retries/cancellation semantics already chosen.
    - Milestone 1 DAG semantics must remain acyclic; explicit rework edges/cycles belong to the later process/workflow graph model.
  - ITD-13 / ITD-14:
    - Artifact behavior remains compatible with S3-style durable artifact storage and private-bucket access patterns, even if the full production storage pattern is deferred.
  - ITD-16:
    - Milestone 1 does not pretend to solve the durable execution-engine-of-record decision.
    - Any exploratory Temporal work must not be required to mark Milestone 1 complete.

- **Observable acceptance evidence exists**
  - A completed demo run shows, in the run timeline:
    - workflow step events
    - tool invocation events
    - model invocation events
    - artifact events
    - stable correlation identifiers
  - Tool failures/timeouts, if exercised, are visible and understandable in the timeline.
  - The final artifact is retrievable from the CLI using the run timeline references.

- **Deferred to early Milestone 2 hardening**
  - Result visibility and authoritative output references on `Run.status` are not treated as Milestone 1 blockers.
  - These are part of the next-step domain-model clarification and control-plane surfacing work.

- **Demo and reproducibility are in place**
  - Provide manifests for the Milestone 1 demo path:
    - Agent
    - Tool / MCP references
    - MCPServer (or equivalent integration resource)
    - ModelEndpoint references
    - Run
  - Document cluster prerequisites and environment assumptions:
    - proxy/network requirements
    - hostname resolution for OS-hosted Lemonade endpoints
    - any required secrets or credentials
  - A fresh developer/operator can follow the documented steps and reproduce the Milestone 1 demo.

- **Coding standards and Definition of Done are met**
  - Review `docs/CODING_STANDARDS_AND_DOD.md` before push.
  - Run `make ci` and fix format/lint/test failures across all present lanes.
  - Ensure any new dependency or packaging choice remains compatible with the OSS posture and licensing guidance.
  - Ensure tests and validation cover the new/changed Run execution path and timeline/event behavior.

- **Tracker and gate documents are updated to reflect reality**
  - Update `docs/implementation/kaigents-implementation-tracker.md` checkboxes only for work that is actually complete.
  - Re-check alignment against:
    - `docs/product/kaigents-prd.md`
    - `docs/architecture/kaigents-architecture-and-design.md`
    - `docs/research/technology/itd-register.md`
    - `docs/research/technology/oss-components-commercially-permissible.md`
    - `docs/CODING_STANDARDS_AND_DOD.md`
  - Resolve any conflicts in docs before pushing.

- **Push/review checkpoint for Milestone 1**
  - Milestone 1 has been closed only after:
    - the acceptance workflow was demoable end-to-end
    - the `Run` reconciler drove real execution
    - the run timeline was durable and queryable
    - artifacts were fetchable from the CLI
    - `make ci` passed
    - the tracker and gate documents were updated to reflect the actual implementation state

Milestone 1 scope note:

- Milestone 1 uses a **Run + embedded DAG** execution substrate (ITD-08).
- Milestone 1 is not expected to provide full process semantics (human-in-the-loop waits over long durations, bounded rework loops, or a first-class Process/Task definition model).

## Milestone 2+ plan

Once Milestone 1 is stable and demoable, the plan shifts to making the refined product domain model real and introducing a durable execution engine of record.

### First step after Milestone 1: productize and harden the Milestone 1 path

This is the **must improve immediately** list. These items exist to prevent Milestone 1 shortcuts from becoming the de facto architecture.

- **Replace env-var-heavy runner handoff with a clearer execution contract**
  - Keep Kubernetes job env vars as a transport mechanism when useful, but do not let them become the source of truth for execution semantics.
  - Move toward a Kaigents-owned execution request contract derived from the resource/domain model.

- **Preserve the domain model as authoritative**
  - `Agent`, `Tool`, `MCPServer`, `ModelEndpoint`, and `Run` resources remain the source of truth.
  - Do not hardcode specific tool names or single-model assumptions into the runner as a long-term design.

- **Introduce capability-aware model/tool selection**
  - Kaigents must be able to reason over available model endpoints and tool capabilities instead of relying on one preselected synthesis model and a fixed pair of tool names.
  - Support differentiated model roles such as synthesis, embeddings, reranking, and coding/planning where appropriate.

- **Surface execution outputs back onto the control-plane resources**
  - Populate `Run.status` with authoritative output references and execution summary information.
  - Reduce dependence on ad hoc filesystem knowledge or out-of-band inspection to understand run results.

### Immediate next steps for tomorrow morning

- **Spike 1: speed up GPT-OSS-20B on `llai03:8000`**
  - Reproduce the successful acceleration approach already used for `Qwen3-Coder-30B` on `jc01:8000`.
  - Capture the deployment/configuration changes in source-controlled docs or manifests rather than relying on environment-only fixes.
  - Revalidate the acceptance path against the faster serving configuration.

- **Spike 2: add a file management MCP tool and standardize on Markdown outputs**
  - Prioritize a file management capability for Student Research Assistant and CodeKnowl before broader document-suite integrations.
  - Prefer Markdown artifacts and file-management workflows over Google Docs or LibreOffice for the next increment.

- **Start the Milestone 2 research/deep dive**
  - Clarify the next-step domain model for results, artifact visibility, and execution summaries.
  - Turn the Milestone 1 must-have improvements into a concrete M2 work plan.
  - Identify which additional MCP tools are worth adding first for Student Research Assistant and CodeKnowl after file management.

- **Harden persistence and retrieval paths**
  - Ensure the default supported Milestone 1 deployment path uses a durable timeline/artifact backend appropriate for in-cluster runs.
  - Avoid runner-local filesystem behavior being mistaken for a durable production path.

- **Reduce acceptance-path hardcoding**
  - The Student Research Assistant flow is a valid acceptance path, but it should not define the permanent execution architecture.
  - Refactor acceptance-agent-specific assumptions into configurable or resource-derived behavior.

- **Prepare the transition to the expanded domain model without skipping Milestone boundaries**
  - Milestone 1 remains `Run + Agent + embedded DAG`.
  - The next implementation step should prepare for the richer Process/Task and Work Request / Work Item / Work Attempt model rather than fight it.
  - Do not backfit Milestone 3 durable-process semantics into Milestone 1 code as a hidden rewrite.

### Milestone 2: Platform Mode essentials (identity + policy)

Focus:

- OIDC authentication for API/UI
- authorization for core resources
- tool allowlisting enforcement
- audit trail of user actions

### Milestone 3: Durable process execution engine decision + integration (ITD-16)

Focus:

- Run the stop/go POC for Temporal as the durable execution substrate
- Decide and record ITD-16
- If adopting Temporal:
  - define the Kaigents execution-engine interface boundary (Rust core calls adapter; do not leak Temporal types into Kaigents domain primitives)
  - define a minimal Kaigents **Process/Task** definition representation for the POC (code or JSON; CRDs are not required in this milestone)
  - demonstrate one representative process compiled from that definition model, with:
    - at least one bounded rework loop
    - at least one human approval gate
    - reconstructable history mapped into Work Request / Work Item / Work Attempt + timeline events

### Milestone 4: Process definition model + UX surface sequencing

Focus:

- Introduce first-class Process/Task definition resources (CRD + CLI first)
- Ensure definition vs execution separation is preserved
- Add a minimal “process graph view” representation suitable for later dashboard rendering

### Milestone 5: Hybrid Execution routing (CPU/GPU/NPU)

Focus:

- Operator-visible routing policies and observability surfaces
- Correlation in timeline and telemetry

### Milestone 6: Dashboard MVP

Focus:

- browse agents/processes/work requests
- trigger executions
- render timeline/history consistently

