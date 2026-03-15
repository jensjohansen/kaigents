# Process Engine Evaluation for Kaigents (Draft)

## Purpose
Kaigents is a platform for designing and operating business processes executed by Agents and Teams of Agents.

After refining the product domain model (Process/Task definitions and WorkRequest/WorkItem/WorkAttempt executions), we need to decide whether Kaigents should:
- Build and maintain its own process execution engine,
- Adopt an off-the-shelf workflow/process engine,
- Or adopt a hybrid approach (use an engine for durability/orchestration while retaining Kubernetes-native execution for certain steps).

This document surveys candidate engines and classifies them as:
- **Adopt**: a viable core engine of record for process execution.
- **Inspire**: borrow ideas/patterns, but not a core dependency.
- **Avoid**: poor fit, excessive complexity, or licensing risk for an OSS platform.

## Non-negotiables and evaluation criteria
- **Kubernetes-first**: fits a K8s-based platform architecture.
- **OSS-friendly licensing**: acceptable for an open-source Kaigents platform.
- **Clear definition vs execution split**: supports Process definition evolution while preserving immutable execution history.
- **Durable execution**: supports long-running work (minutes to days/weeks) with waiting on humans/external systems.
- **Human-in-the-loop**: supports approvals/reviews/assignments.
- **Auditability**: strong event/history model suitable for compliance.

## Constraints learned from the product domain model exercise

### Simple-first process modeling (avoid BPMN failure modes)
Kaigents process definition must be simple-first:
- People who are not process/BPMN experts must be able to define and interpret a process correctly.
- The process graph must be understandable “at a glance”, both as a definition and as a live view of current work.

This strongly suggests:
- Kaigents should own a simplified process definition model and graph UI.
- BPMN is a source of inspiration, not a required user-facing modeling language.

### Processes are not DAGs (rework introduces cycles)
Because Kaigents explicitly supports rework, the process graph cannot be restricted to a DAG.

Kaigents processes should be treated as a directed graph that may contain cycles (rework loops), with bounded semantics:
- Rework edges must be explicit.
- Rework must be bounded (attempt limits, time limits, or escalation policies).

### Efficiency constraints (on-prem cluster economics)
Kaigents targets on-prem Kubernetes clusters where compute is expensive and reserved primarily for models and tools.

The orchestration and process engine must be efficient enough that it does not overwhelm the recommended hardware:
- Orchestration overhead must remain small relative to model/tool workloads.
- It must support using lightweight models for orchestration/persona while reserving large models for heavy tasks.

## Domain model mapping (Kaigents → workflow concepts)
Kaigents product-domain terms:
- **Process**: reusable definition of a business procedure.
- **Task**: step/activity definition inside a process.
- **WorkRequest**: execution instance of a process.
- **WorkItem**: instance of a task created within a WorkRequest.
- **WorkAttempt**: one attempt to perform a WorkItem.

Common workflow-engine equivalents:
- Process/Workflow **Definition**
- Process/Workflow **Execution** (instance)
- Task/Activity (step)
- Activity execution attempt / retry

## Shortlist

### Temporal
- **Class**: Durable workflow engine
- **License**: Apache 2.0 (verify exact components used for self-hosting)
- **Kubernetes fit**: strong (runs well on K8s; worker model maps naturally)

**Why it may be a core fit**
- Strong separation of **workflow definition** vs **workflow execution**.
- First-class long-running semantics: timers, retries, cancellation, signals.
- Designed for correctness under failure; complete history enables audit.

**Mapping to Kaigents domain**
- WorkRequest ≈ Workflow Execution
- Task/WorkItem ≈ workflow state + one or more Activities
- WorkAttempt ≈ Activity attempt / execution with retry

**Open questions**
- Best product UX for process modeling if core engine is code-first.
- Human task UX: build Tasklist-like surface vs integrate existing systems.

**Notes for Kaigents constraints**
- Temporal is not BPMN-first and does not require exposing BPMN to end users.
- Temporal workflows are defined in code, which can be a good execution substrate if Kaigents owns a simplified graph model and compiles it to workflows/activities.
- Temporal provides strong durability/correctness, but introduces an operational dependency (Temporal server + persistence).

**Preliminary classification**: **Adopt (strong contender)**

---

### Argo Workflows
- **Class**: Kubernetes-native DAG/step orchestrator
- **License**: Apache 2.0
- **Kubernetes fit**: excellent

**Why it may be valuable**
- Great for containerized steps and compute-heavy workloads.
- Strong fit for “offload steps to Kubernetes” patterns.

**Risks / gaps**
- Definition vs instance can be blurred depending on whether using Workflow vs WorkflowTemplate.
- Long-lived, human-waiting processes are possible but not a primary strength.

**Preliminary classification**: **Inspire / Complement**

---

### Camunda 8 / Zeebe
- **Class**: BPMN-first process orchestration platform
- **License**: licensing and source-availability restrictions are a primary concern; must be validated for any OSS-distribution assumptions.
- **Kubernetes fit**: yes (self-managed), but ecosystem weight is significant.

**Why it may be attractive**
- BPMN modeling and human task patterns are well-trodden.

**Risks / gaps**
- Licensing may be incompatible with an OSS platform dependency.
- Potentially heavy/over-engineered relative to Kaigents needs.

**Preliminary classification**: **Avoid as core dependency; Inspire for semantics**

---

### Flowable
- **Class**: BPMN engine (Java)
- **License**: generally positioned as open source; confirm exact OSS license and any restrictions.

**Pros**
- BPMN-native concepts align to business process modeling.

**Cons**
- Java BPM suite introduces ecosystem weight and operational complexity.

**Preliminary classification**: **Inspire (possible adopt if BPMN-first is a product requirement)**

---

### jBPM / Kogito
- **Class**: business automation toolkit (process + decisions)
- **License**: Apache 2.0 (jBPM stated as Apache 2.0; verify Kogito specifics)

**Pros**
- Strong BPMN/DMN pedigree; Kogito aims for cloud-native.

**Cons**
- Ecosystem and modeling approach may dominate Kaigents architecture.

**Preliminary classification**: **Inspire; evaluate if BPMN/DMN is first-class UX goal**

---

### Netflix Conductor
- **Class**: microservice orchestration
- **License**: Apache 2.0

**Pros**
- Useful orchestration patterns; established.

**Cons**
- Less directly aligned to the durable workflow ergonomics that match business-process + human-in-loop requirements.

**Preliminary classification**: **Inspire**

---

### StackStorm
- **Class**: event-driven automation platform
- **License**: Apache 2.0

**Pros**
- Good integration concepts and operational automation patterns.

**Cons**
- Oriented toward ops/runbook automation more than durable business processes.

**Preliminary classification**: **Inspire**

---

### Kestra
- **Class**: declarative workflow orchestrator
- **License**: Apache 2.0 (per vendor FAQ; verify repo license)

**Pros**
- Declarative, plugin ecosystem.

**Cons**
- Often positioned toward data/ops orchestration; evaluate fit for human-in-loop business processes.

**Preliminary classification**: **Inspire**

## Recommended decision path
1. Decide if Kaigents is **BPMN-first** (business analysts model in BPMN) or **durable-workflow-first** (engine for reliability; modeling layer is a Kaigents product concern).
2. If durable-workflow-first:
   - Deep evaluate **Temporal** as engine of record.
   - Use Kubernetes-native orchestrators (e.g., Argo) only where they are the best substrate.
3. If BPMN-first:
   - Evaluate **Flowable** or **jBPM/Kogito** for OSS feasibility and operational complexity.
   - Treat Camunda/Zeebe as high-risk unless licensing is clearly acceptable.

## Implications for the existing Milestone 1 execution engine
- If adopting a durable engine (Temporal):
  - Keep runner/tool-plane/model/timeline/artifacts concepts.
  - Replace the “process execution/orchestration core” with the engine’s workflow semantics.
- If adopting a BPMN engine:
  - Kaigents becomes the integration layer around BPMN definitions, identity/policy, and agent/tool/model bindings.

## Next research items
- Confirm licensing and OSS-distribution implications for Camunda 8 / Zeebe.
- Validate Temporal self-hosted components + operational footprint on Kubernetes.
- Decide whether Kaigents must provide BPMN as a user-facing modeling artifact, or whether Kaigents provides its own process model UX.

## Temporal proof-of-concept (POC) to drive a firm ITD

This POC is intended to produce a firm ITD:
- **ITD: Adopt Temporal as Kaigents process execution engine**, or
- **ITD: Build a Kaigents-specific process engine**.

### POC goal
Validate that Temporal can support Kaigents’ requirements without forcing end users to learn a technical process domain, and without imposing unacceptable overhead for on-prem deployments.

### What the POC must demonstrate
- Kaigents can keep a **simple-first process graph model** and compile/execute it on Temporal.
- A process graph with **rework loops** (cycles) can be executed with bounded rework semantics.
- A work request produces a durable, queryable history sufficient to drive:
  - WorkRequest state
  - WorkItem state
  - WorkAttempt history
  - Timeline/events and artifacts
- The system can represent **human-in-the-loop** as first-class waiting/approval steps.
- The system can compute dashboard overlays for:
  - WIP per step
  - capacity constraints
  - starvation/blockage
  - rework rate

### POC implementation shape (minimal)
- Define one representative Kaigents process (simple graph with at least one rework loop and one approval gate).
- Implement workers with a lightweight runtime (prefer Go for efficiency).
- Execute a small set of WorkRequests and capture:
  - throughput and resource consumption (CPU/mem of workers and server)
  - time-to-recover from worker restart
  - ability to pause/wait/resume for human input

### Pass/fail criteria
Temporal is a viable core engine if:
- The compiled execution model remains simple and does not require exposing Temporal concepts to end users.
- Rework loops are implementable and observable without awkward workarounds.
- Operational footprint is acceptable for a “small on-prem cluster” target.

If any of the above fail, prefer a Kaigents-specific engine tailored to the domain model.
