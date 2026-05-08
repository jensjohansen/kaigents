# Changelog (Kaigents)

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-05-08

### Added
- **Milestone 4: Process & Task Model**: First-class `Process` and `Task` CRDs, and mapping to Temporal WorkRequest/WorkItem durable execution.
- **Milestone 5: Hybrid Execution**: `RoutingPolicy` support in `Agent` and `Run` resources, allowing `NodeSelector` injection for GPU/NPU pinning.
- **Milestone 6: Dashboard MVP**: Lightweight Go service for monitoring agents, processes, and active runs.
- **Milestone 7: Hardening (In Progress)**:
    - Cloud-agnostic S3 artifact storage implementation in `kaigents-core`.
    - Structured JSON logging helper in `kaigents-core` and conversion of CLI.
    - Observability requirements added to PRD and Architecture.

### Changed
- Refactored `kaigents-cli` to use `tracing` for structured JSON logging.
- Updated `RunReconciler` to support `RoutingPolicy` for hardware pinning.

## [0.1.0] - 2026-05-08

### Added
- **Milestone 1: Solo Mode MVP**: Core CRDs (Agent, Tool, Run), embedded DAG engine, and CLI.
- **Milestone 2: Platform Mode Essentials**: RBAC and Keycloak OIDC integration.
- **Milestone 3: Durable Execution substrate**: Temporal adapter POC and durable execution decision.
