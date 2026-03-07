# Kaigents Platform

Kaigents is a Kubernetes-native platform for building, running, and operating AI agents in production environments, optimized for low total cost of ownership (TCO) with a strong focus on AMD Ryzen AI hardware.

This repository is the Kaigents **platform** (distinct from any future marketing/community site).

## Canonical docs

- `docs/product/kaigents-prd.md`
- `docs/architecture/kaigents-architecture-and-design.md`
- `docs/implementation/kaigents-implementation-tracker.md`
- `docs/CODING_STANDARDS_AND_DOD.md`

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
