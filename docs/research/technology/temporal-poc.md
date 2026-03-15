# Temporal POC for Kaigents Process Execution (Draft)

## Purpose
This proof-of-concept (POC) evaluates whether Temporal can serve as Kaigents’ process execution engine **without** forcing end users to learn technical workflow concepts and **without** imposing unacceptable overhead for on-prem Kubernetes deployments.

The result of this POC is intended to become a firm ITD:
- **ITD: Use Temporal as the Kaigents process execution engine**, or
- **ITD: Build a Kaigents-specific process engine**.

## Background (why a POC is necessary)
Many workflow/process engines are replaced after initial adoption because:
- They require end users to understand the workflow domain at a technical level.
- Their modeling primitives encourage incorrect usage by non-experts.
- Operational footprint and overhead are misaligned with lightweight on-prem deployments.

Kaigents has explicit product constraints:
- **Simple-first process definition** (avoid BPMN failure modes).
- **Non-DAG process graphs** (rework loops introduce cycles).
- **High efficiency**: orchestration overhead must be small compared to model/tool workloads.

## What “success” means (POC acceptance criteria)
### A) Product-model alignment
- Kaigents can keep a **simple-first** process graph model.
- A process can include:
  - at least one **rework loop** (cycle)
  - at least one **approval gate** (human-in-the-loop)
  - at least one **quality gate** that can trigger rework
- Temporal concepts remain hidden behind Kaigents terms:
  - WorkRequest
  - WorkItem
  - WorkAttempt

### B) Observability and audit
- Every WorkRequest produces a durable history sufficient to reconstruct:
  - WorkRequest state timeline
  - WorkItem state timeline
  - WorkAttempt attempts (including retries/rework)
  - artifacts produced/operated on
  - events (audit trail)

### C) Flow monitoring (LEAN-inspired)
From Temporal history (or derived Kaigents events), we can compute and display:
- WIP per process step
- blocked vs starved steps
- rework rate
- cycle time per step
- bottleneck identification via queue depth and latency

### D) Efficiency / footprint
- Worker processes can be implemented with a lightweight runtime.
- Temporal server footprint is acceptable for an on-prem cluster where the majority of resources must be reserved for model serving and tool execution.

## POC scope
### In scope
- One representative Kaigents process (simple graph) executed end-to-end.
- One UI-free “graph view” output (e.g., renderable JSON) sufficient for a future dashboard.
- A minimal human-in-the-loop mechanism.
- A minimal Kaigents **Process/Task** definition representation and a mapping/"compiler" to the durable execution substrate (CRDs are not required for the POC).

### Out of scope (for this POC)
- A full process designer UI.
- Full multi-tenant RBAC.
- Full tool/model plane integration.

## Proposed representative process: “Research Report with Review + Rework”
A compact process that still exercises the hard requirements:

- **Task: Intake**
  - Create a CaseFile from request inputs.
- **Task: Research**
  - Gather sources.
- **Quality Gate: Citation check**
  - If fail → rework to Research (bounded attempts).
- **Task: Draft report**
- **Approval Gate: Human review**
  - Approve → publish
  - Request changes → rework to Draft report (bounded attempts)
- **Task: Publish**

This is intentionally simple-first and maps well to your earlier “Student Research Assistant” context.

## Temporal mapping hypothesis (execution substrate)
The POC evaluates whether Temporal can serve as a substrate where:
- **WorkRequest** maps to a Temporal Workflow Execution.
- **WorkItem** maps to a Kaigents-tracked unit of work that may correspond to one or multiple Temporal activities.
- **WorkAttempt** maps to an activity attempt or an application-level attempt recorded as events.

The key test is that Kaigents can keep its own simple-first process graph and execution model and only use Temporal for durability, scheduling, retries, and waiting.

Additional key test (definition model):

- Kaigents can define a minimal Process/Task representation (code or JSON) and compile it to the durable execution substrate without exposing Temporal concepts to end users.

## Human-in-the-loop approach (POC)
Minimal viable human-in-loop:
- A workflow step transitions to “waiting for approval”.
- An external signal/command supplies the approval decision.

Success criteria:
- The work request remains durable while waiting.
- Resuming is reliable after worker restarts.

## Rework approach (POC)
Rework must be explicit and bounded.

POC will include:
- One rework loop triggered by a quality gate.
- One rework loop triggered by human review.

Success criteria:
- Rework is visible as repeated WorkAttempts and/or explicit “returned” WorkItem state.
- The system enforces a max rework attempt count and escalates/fails predictably.

## Metrics to capture
- **Temporal server**: CPU/memory under a small test load.
- **Workers**: CPU/memory; concurrency behavior.
- **Latency**: time to schedule and execute steps.
- **Recovery**: restart workers mid-execution and verify deterministic resume.
- **History size**: rough growth characteristics for a typical WorkRequest.

## Deliverables
- A short report that answers:
  - Does Temporal satisfy the product constraints without exposing workflow complexity?
  - Is the operational footprint acceptable for the target on-prem economics?
  - Do rework loops and human gates feel natural or awkward?
- A recommendation for the ITD:
  - Use Temporal, or
  - build a Kaigents-specific engine.

## Risks and failure modes to watch for
- Modeling drift: Kaigents “simple-first graph” collapses into Temporal-specific workflow code.
- Rework loops become unnatural or hard to observe.
- Operational footprint (datastores, services) is too heavy for the target on-prem baseline.
- Human-in-loop requires too much bespoke infrastructure.

## Next steps (if POC is successful)
- Define the Kaigents process graph schema and “compiler” mapping to Temporal.
- Define the event model for dashboard overlays (WIP/capacity/starvation/rework).
- Draft the ITD and update the PRD/design accordingly.
