<p align="center">
  <img src="assets/kaigents.png" alt="Kaigents" width="720" />
</p>

# Kaigents Platform

Kaigents is a Kubernetes-native platform for building, running, and operating AI agents in production environments, optimized for low total cost of ownership (TCO) with a strong focus on AMD Ryzen AI hardware.

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

## License and OSS posture

- Kaigents is MIT-licensed.
- Core dependencies must remain commercial-safe (redistribution-safe).
- Integrate-only components (user-supplied) are allowed only when clearly separated and documented.

## Development

This repo uses a minimal Makefile to run formatting, linting, and tests when relevant toolchains are present.

```bash
make fmt
make lint
make test
```

## Project status

Implementation scope, milestones, and acceptance criteria are tracked in `docs/implementation/kaigents-implementation-tracker.md`.
