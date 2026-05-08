<p align="center">
  <img src="assets/kaigents.png" alt="Kaigents" width="720" />
</p>

# Kaigents Platform (GA Release)

Kaigents is a **production-ready**, Kubernetes-native platform for building, running, and operating AI agents in enterprise environments. It is optimized for low total cost of ownership (TCO) with a strong focus on AMD Ryzen AI hardware.

**Current Version**: 1.0.0 (General Availability)

This repository is the Kaigents **platform** (distinct from any future marketing/community site).

## Canonical docs

- [`docs/product/kaigents-prd.md`](docs/product/kaigents-prd.md)
  - Product goals, MVP scope, functional requirements, UX requirements (run timeline), and milestones.
- [`docs/architecture/kaigents-architecture-and-design.md`](docs/architecture/kaigents-architecture-and-design.md)
  - Canonical system design: boundaries, data flows, and the role of tool plane, run timeline, artifacts, and RAG.
- [`docs/implementation/kaigents-implementation-tracker.md`](docs/implementation/kaigents-implementation-tracker.md)
  - Milestone tracker and push/review gates (when we are allowed to push working code).
- [`docs/CODING_STANDARDS_AND_DOD.md`](docs/CODING_STANDARDS_AND_DOD.md)
  - Coding standards, CI quality gates, and definition of done.
- [`docs/research/technology/itd-register.md`](docs/research/technology/itd-register.md)
  - Important Technical Decisions (ITDs) that constrain implementation choices.
- [`docs/research/technology/oss-components-commercially-permissible.md`](docs/research/technology/oss-components-commercially-permissible.md)
  - OSS due diligence list and licensing posture (redistribute vs integrate-only vs exclude).

## Production Hardened

Kaigents 1.0.0 is built for stability and enterprise operations:
- **Durable Execution**: Long-running workflows survive component restarts.
- **Observability**: Full Prometheus metrics and JSON structured logs (Loki) across all components.
- **Enterprise Storage**: Cloud-agnostic S3 support with large-object streaming.
- **Identity**: Built-in OIDC (Keycloak) and Kubernetes RBAC integration.

## Features

- **Kubernetes-native**: Built on CRDs, standard RBAC, and GitOps-friendly workflows.
- **Enterprise Identity**: Full OIDC integration with Keycloak for platform-wide SSO.
- **Durable Execution**: Powered by Temporal for long-running, human-gateable agent workflows.
- **Hybrid Execution**: Declarative hardware pinning (CPU/GPU/NPU) via `RoutingPolicy`.
- **Observable**: Structured JSON logging (Loki-ready) and Prometheus metrics.
- **Cloud-Agnostic Storage**: S3-compatible artifact storage (AWS, MinIO, Ceph).

## License and OSS posture

- Kaigents is MIT-licensed.
- Core dependencies must remain commercial-safe (redistribution-safe).
- Integrate-only components (user-supplied) are allowed only when clearly separated and documented.

## Getting Started

Follow the [Getting Started Guide](docs/getting-started/index.md) to install Kaigents and deploy your first AI agent team.

## Managed AI Teams

You can build any agent team on Kaigents yourself. You can also skip the build — we operate a growing catalog of production-ready AI agent teams as fully managed services. See [docs/product/managed-services.md](docs/product/managed-services.md) for an overview of available teams and how to get access.

## Development

This repo uses a minimal Makefile to run formatting, linting, and tests when relevant toolchains are present.

```bash
make fmt
make lint
make test
```

## Operations

- [Temporal installation](docs/ops/temporal-installation.md)

## Project status

Implementation scope, milestones, and acceptance criteria are tracked in `docs/implementation/kaigents-implementation-tracker.md`.
