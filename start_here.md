# Kaigents — Session Handoff / Start Here

Use this file to orient a new chat session without repeating all prior context.

---

## Current State

**Kaigents v1.0.0 — General Availability** is complete and committed to GitHub.

- Remote: `https://github.com/jensjohansen/kaigents`
- All implementation milestones (0 – 7) are checked off in `docs/implementation/kaigents-implementation-tracker.md`.
- License: MIT (core). Future managed-services layer will use a commercial license on Gitea at `gitea.ai-agents.private`.
- Platform: on-prem Kubernetes cluster `kubernetes-admin@kubernetes` (10.7.0.41:6443). Testing context is `ai-agents-k8s-cluster`. Do **not** use Link Labs cloud clusters.

### What was delivered

| Milestone | Summary |
|-----------|---------|
| 0 | Repo baseline, CI, versioning |
| 1 | Solo Mode MVP — CRDs, controller, embedded DAG, run timeline, MCP tool plane, model serving (Lemonade/OpenAI-compat), artifacts, CLI |
| 2 | Platform Mode — Keycloak OIDC (all 3 control-plane nodes patched), RBAC ClusterRoles, tool allowlisting |
| 3 | Temporal adapter (Go) for durable long-running workflows; ITD-16 recorded as ADOPTED |
| 4 | Process/Task CRDs + WorkRequest/WorkItem execution mapping |
| 5 | Hybrid Execution routing — CPU/GPU/NPU via `RoutingPolicy` / `NodeSelector` |
| 6 | Dashboard MVP — browse agents, run timelines, artifacts, error diagnosis |
| 7 | Hardening — S3/MinIO/Ceph cloud-agnostic storage, range reads, structured JSON logs (Loki), Prometheus metrics on all components, Grafana dashboards, stable analytics event schema |

### Key architectural facts (do not relitigate)

- **Rust** — `engine/` (core domain, runner, artifact gateway). All performance-critical and GIL-sensitive paths.
- **Go** — `operator/` (Kubernetes controller), `temporal-adapter/` (Temporal integration boundary). Temporal SDK never touches Rust.
- **No Python** in Kaigents core. Python was explicitly rejected due to GIL limitations in high-volume streaming scenarios.
- **Temporal** is the durable execution engine; its concepts are hidden behind Kaigents domain terms (`WorkRequest`, `WorkItem`, `WorkAttempt`).
- **MCP** (via `kmcp`) is the canonical tool integration protocol.

### Important Technical Decisions (ITDs)

See `docs/research/technology/itd-register.md` for the full register. Key adopted decisions:
- ITD-13: S3-compatible object store (Ceph RGW / MinIO / AWS S3) for artifact byte storage.
- ITD-14: Private-bucket signing/proxy for artifact access (clients never get object-store credentials).
- ITD-16: Temporal adopted as durable process engine.

---

## What We Were About to Start

**Getting Started guide** — a new document (or set of documents) in the repo aimed at enterprise adopters.
The guide should walk an implementer through:
1. Installing Kaigents on a Kubernetes cluster.
2. Defining and deploying an AI agent team using a concrete, realistic example.
3. Running the team and observing it through the dashboard, Prometheus/Grafana, and Loki.

### Candidate example team / use case (decision pending)

Three were proposed. We had not yet chosen one. The chosen example must **not** compete with planned commercial products and should resonate with enterprise buyers:

| Candidate | Notes |
|-----------|-------|
| **Corporate Travel & Expenses Manager** | Familiar pain point; no Kaigents product conflicts. Finance-adjacent team. |
| **Corporate Software License Manager** | Strong enterprise IT appeal; ties to governance/compliance angle. No product conflicts. |
| **Product Research Assistant** | Good for showing RAG and model serving; ties to the `data_science_ai_team_prd.md`. May overlap future TeamKnowl positioning. |

**Recommendation to evaluate in new chat**: the **Corporate Travel & Expenses Manager** is the safest choice — it is a well-understood enterprise pain point, maps cleanly to the multi-agent team model (approval gates, human-in-the-loop, artifact outputs), and does not conflict with any of the planned products (TeamKnowl, CodeKnowl, or vertical-market teams in the PRDs).

Alternatively, consider a **SecOps Alert Triage** team, which is specified in `docs/product/security_operations_ai_team_prd.md`, would be a strong signal to enterprise security buyers, and directly demonstrates Kaigents' human-in-the-loop and audit capabilities.

---

## Active Constraints and Decisions

- **Open-source only** code goes to GitHub (`jensjohansen/kaigents`). Any future managed-service / commercial layer goes to `gitea.ai-agents.private`.
- **Do not commit** managed-service or proprietary content to the GitHub remote.
- **kubeconfig** context: always use `kubernetes-admin@kubernetes` (10.7.0.41:6443) for testing. Never use Link Labs cloud clusters.
- **Cert rotation**: `cluster-ca-issuer` and related certs rotate every 90 days. An automated trust-store update process is in place; the OIDC CA cert in use was verified working via `https://harbor.ai-agents.private`.
- Kaigents **will not be used at Link Labs** (to preserve clean IP separation from employer).

---

## Pending / Known Gaps

- `README.md` Getting Started section currently says "under development." This will be resolved by the Getting Started guide.
- Milestone 3 push checkpoint (`[ ]` in tracker) — the box was left unchecked in the tracker because it was superseded by the Milestone 3 full implementation. This is cosmetic; the work is done and the ITD is recorded.
- GitHub discoverability: the repo at `https://github.com/jensjohansen/kaigents` is published but not easily found by search. No topics/tags were added. Adding GitHub topics (`kubernetes`, `ai-agents`, `mlops`, `temporal`, `amd-ryzen-ai`, `rust`) and a short repo description would improve discoverability — this was noted but not yet done.

---

## Project Docs Map

| File | Purpose |
|------|---------|
| `README.md` | Entry point; GA release |
| `CHANGELOG.md` | Release history |
| `LICENSE` | MIT |
| `THIRD_PARTY_NOTICES.md` | OSS attribution |
| `CODE_OF_CONDUCT.md` | Community standards |
| `docs/product/kaigents-prd.md` | Product requirements (source of truth for scope) |
| `docs/architecture/kaigents-architecture-and-design.md` | System design |
| `docs/implementation/kaigents-implementation-tracker.md` | Milestone tracker |
| `docs/CODING_STANDARDS_AND_DOD.md` | Coding standards and definition of done |
| `docs/research/technology/itd-register.md` | Important Technical Decisions |
| `docs/research/technology/oss-components-commercially-permissible.md` | OSS license posture |
| `docs/ops/temporal-installation.md` | Temporal self-hosted ops guide |
| `docs/product/*_prd.md` | Team-level PRDs (SecOps, SoftEng, Sales, Marketing, etc.) |

---

## First Task in New Chat

Start the new chat with:

> "I want to build the Getting Started guide for Kaigents. Read `start_here.md` first, then let's choose the example team and plan the guide."

The new chat should:
1. Read this file.
2. Pick the example team (Travel & Expenses or SecOps are top candidates).
3. Plan the guide structure (cluster prerequisites, Helm install, CRD authoring, running a team, observing it).
4. Implement the guide as `docs/getting-started/` with sub-pages as needed.
5. Update `README.md` Getting Started section to point to it.
6. Commit and push to GitHub.
