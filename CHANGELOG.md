# Changelog (Kaigents)

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-08

### Added
- **Milestone 7: Hardening & Production Readiness**:
    - Cloud-agnostic S3 artifact storage implementation in `kaigents-core`.
    - Support for large-object range reads in Artifact Proxy for streaming.
    - Standardized Prometheus metrics across Rust (Engine) and Go (Dashboard/Adapter) components.
    - Integrated structured JSON logging (Loki-ready) stack-wide.
    - Sample Grafana dashboard for platform overview.
- **Milestone 6: Dashboard MVP**: Lightweight Go service for monitoring agents, processes, and active runs.
- **Milestone 5: Hybrid Execution**: `RoutingPolicy` support in `Agent` and `Run` resources, allowing `NodeSelector` injection for GPU/NPU pinning.
- **Milestone 4: Process & Task Model**: First-class `Process` and `Task` CRDs, and mapping to Temporal WorkRequest/WorkItem durable execution.

### Changed
- Refactored `kaigents-cli` to use `tracing` for structured JSON logging.
- Updated `RunReconciler` to support `RoutingPolicy` for hardware pinning.

## [0.1.0] - 2026-05-08

### Added
- **Milestone 1: Solo Mode MVP**: Core CRDs (Agent, Tool, Run), embedded DAG engine, and CLI.
- **Milestone 2: Platform Mode Essentials**: RBAC and Keycloak OIDC integration.
- **Milestone 3: Durable Execution substrate**: Temporal adapter POC and durable execution decision.
