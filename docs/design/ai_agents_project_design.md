# AI Agents Project Design

**Status:** Superseded by `docs/architecture/kaigents-architecture-and-design.md`.

## 1. Overview

This document outlines the architecture and workflow for a **team of software AI agents** that will autonomously understand a legacy codebase, triage and resolve Jira tickets, follow a solid Software Development Life Cycle (SDLC), run automated tests, and commit changes into a CI/CD pipeline for quality assurance and deployment.

The design is intentionally **code‑free**; it focuses on concepts, data flows, and governance rather than implementation details.

## 2. Project Scope & Objectives

| Objective | Description |
|---|---|
| **Legacy Code Understanding** | Build a knowledge base that captures the structure, semantics, and intent of the existing code. |
| **Ticket Management** | Automate the intake, triage, and assignment of Jira tickets to the appropriate AI agent. |
| **Feature & Maintenance Delivery** | Agents produce code changes that satisfy ticket requirements while preserving existing functionality. |
| **SDLC Compliance** | Enforce a defined SDLC that includes design, implementation, testing, review, and deployment stages. |
| **CI/CD Integration** | Commit changes to the repository and trigger the existing CI/CD pipeline for build, test, and deployment. |
| **Governance & Monitoring** | Ensure auditability, security, and compliance throughout the process. |

## 3. AI Agent Personas

| Agent | Primary Responsibility | Key Skills |
|---|---|---|
| **Knowledge Curator** | Extracts and maintains a vector‑based representation of the legacy code. | Static analysis, AST parsing, embedding generation |
| **Ticket Triage** | Parses Jira tickets, determines priority, and assigns the correct agent. | NLP, Jira API, rule engine |
| **Feature Implementer** | Generates code changes that satisfy ticket requirements. | LLM code generation, diff creation |
| **QA Tester** | Generates and runs automated tests, validates regression. | Test generation, unit/integration testing frameworks |
| **Commit & CI/CD Orchestrator** | Commits changes, triggers pipelines, monitors results. | Git, CI/CD APIs, webhook handling |
| **Governance & Security Auditor** | Reviews changes for compliance, security, and style. | Static analysis, policy enforcement |

These personas map closely to the roles described in our existing product strategy documents, e.g. [docs/strategy/technical_support_agent_strategy.md](docs/strategy/technical_support_agent_strategy.md) and [docs/strategy/software_engineering_ai_strategy.md](docs/strategy/software_engineering_ai_strategy.md).

## 4. Legacy Code Knowledge Extraction Pipeline

1. **Static Analysis** – Parse the entire codebase to build an Abstract Syntax Tree (AST) and extract metadata (classes, functions, modules, dependencies). |
2. **Semantic Embedding** – Convert AST nodes and comments into vector embeddings using a language model. |
3. **Vector Store** – Persist embeddings in a vector database (e.g., Pinecone, Weaviate) for fast similarity search. |
4. **Knowledge Graph** – Build a graph that links code entities, documentation, and test cases. |
5. **Continuous Refresh** – On each commit, re‑run the pipeline for affected modules only.

The Knowledge Curator agent orchestrates this pipeline and exposes a query API that other agents use.

## 5. Jira Integration & Ticket Triage

- **Webhook Listener** – Receives events from Jira (issue created/updated). |
- **NLP Parser** – Extracts intent, affected components, and acceptance criteria. |
- **Priority Engine** – Applies business rules (e.g., severity, sprint backlog) to rank tickets. |
- **Assignment Logic** – Matches tickets to the appropriate agent persona based on component and complexity.

The Ticket Triage agent publishes a *ticket job* to a message queue (e.g., RabbitMQ) that the Feature Implementer or QA Tester consumes.

## 6. SDLC Workflow

```
Ticket → Triage → Feature Implementer → QA Tester → Commit & CI/CD Orchestrator → Deployment
```

Each stage is a **micro‑service** that can be scaled independently. Agents communicate via a **state machine** that tracks ticket status and enforces guard conditions (e.g., tests must pass before commit).

## 7. Automated Testing Strategy

- **Unit Test Generation** – Feature Implementer produces unit tests for new/changed functions using an LLM. |
- **Integration Test Harness** – QA Tester runs integration tests against a sandbox environment. |
- **Regression Suite** – A curated set of tests that run on every commit to detect unintended side effects. |
- **Test Coverage Analysis** – Static tools report coverage; low‑coverage areas trigger additional test generation.

All test artifacts are stored in the repository and tracked by the CI/CD pipeline.

## 8. CI/CD Integration

1. **Commit** – The Commit & CI/CD Orchestrator pushes a new branch with the changes. |
2. **Pipeline Trigger** – A webhook or API call starts the existing CI/CD pipeline (e.g., GitHub Actions, Jenkins). |
3. **Build & Test** – The pipeline runs linting, unit, integration, and security scans. |
4. **Approval** – If all checks pass, the orchestrator merges the branch into `main` and triggers a deployment. |
5. **Rollback** – In case of failure, the orchestrator reverts the branch and notifies stakeholders.

## 9. Governance & Monitoring

- **Audit Trail** – Every agent action is logged with timestamps, user context, and decision rationale. |
- **Policy Engine** – Enforces coding standards, security rules, and compliance checks before commits. |
- **Metrics Dashboard** – Tracks throughput (tickets per day), success rate, test coverage, and deployment frequency. |
- **Alerting** – Slack/Teams notifications for failures, security findings, or SLA breaches.

## 10. High‑Level Architecture Diagram

```
+----------------+      +----------------+      +----------------+
|  Jira System  | ---> |  Ticket Triage | ---> | Feature Agent |
+----------------+      +----------------+      +----------------+
          |                        |                    |
          v                        v                    v
+----------------+      +----------------+      +----------------+
| Knowledge Cur- | <--- |  QA Tester     | <--- | Commit Agent  |
| ator           |      +----------------+      +----------------+
+----------------+                |                    |
          |                        |                    |
          v                        v                    v
+----------------+      +----------------+      +----------------+
|  Vector Store |      |  CI/CD System  |      |  Monitoring    |
+----------------+      +----------------+      +----------------+
```

(For a visual representation, see the diagram in the repository’s `docs/architecture` folder.)

## 11. Implementation Roadmap

1. **Prototype Knowledge Curator** – Build a lightweight static analyzer and embedder. |
2. **Set Up Vector Store** – Deploy a vector database and ingest the legacy code. |
3. **Develop Ticket Triage Service** – Connect to Jira and implement basic parsing. |
4. **Create Feature Implementer Prototype** – Use an LLM to generate a simple code change. |
5. **Integrate CI/CD Trigger** – Hook the orchestrator into the existing pipeline. |
6. **Add Governance Layer** – Implement policy checks and audit logging. |
7. **Roll Out Incrementally** – Start with a single component, then expand coverage. |

## 12. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| **Incorrect code generation** | Use a review step and automated tests before merge. |
| **Data privacy** | Store embeddings in a secure, access‑controlled vector store. |
| **Jira API limits** | Cache ticket data and batch processing. |
| **Pipeline failures** | Implement retry logic and fallback branches. |
| **Governance gaps** | Continuously update policy engine with new rules. |

## 13. Next Steps

- Finalize the architecture diagram in the `docs/architecture` folder. |
- Draft the detailed API contracts for each agent. |
- Prepare a proof‑of‑concept for the Knowledge Curator. |
- Schedule a review meeting with the product and engineering teams.

---

*Prepared by the AI Agents Design Team.*
