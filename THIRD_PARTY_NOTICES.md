# Third-Party Notices (Kaigents)

Kaigents is MIT-licensed. This file is used to track third-party software included in Kaigents releases and to capture required attributions and license/NOTICE obligations.

Policy:

- Kaigents core dependencies must remain commercial-safe and redistribution-safe.
- If a dependency introduces additional obligations (e.g., Apache-2.0 NOTICE propagation), we record the required attributions here and include any required license/NOTICE texts in release artifacts.
- Integrate-only components (user-supplied) are not bundled or redistributed by Kaigents and must be clearly separated and documented.

## How to update

When adding or updating a dependency:

- Record the dependency name, version, license, and homepage/repository.
- If the upstream project includes a NOTICE file or attribution requirement, capture it here and ensure the NOTICE requirement is satisfied in distribution.

## Notices

### Rust crates (engine)

The Rust execution engine crates depend on the following third-party components. Licenses are permissive (MIT / Apache-2.0 / BSD-family) per the project OSS posture.

- **tokio** — async runtime
  - **License**: MIT
  - **Homepage**: https://tokio.rs/
- **uuid** — UUID generation/parsing
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/uuid
- **serde_json** — JSON serialization
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/serde_json
- **async-trait** — async traits
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/async-trait
- **sha2** — SHA-2 hashing
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/sha2
- **clap** — CLI argument parsing
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/clap
- **reqwest** — HTTP client
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://crates.io/crates/reqwest
- **aws-sdk-s3** — AWS SDK for S3
  - **License**: Apache-2.0
  - **Homepage**: https://github.com/awslabs/aws-sdk-rust
- **tracing** — application-level tracing and logging
  - **License**: MIT
  - **Homepage**: https://tracing.rs/
- **kube** — Kubernetes client
  - **License**: Apache-2.0
  - **Homepage**: https://kube.rs/
- **k8s-openapi** — Kubernetes API types
  - **License**: Apache-2.0
  - **Homepage**: https://github.com/Arnavion/k8s-openapi
- **prometheus** — Prometheus instrumentation library
  - **License**: Apache-2.0
  - **Homepage**: https://github.com/prometheus/client_rust
- **tiny_http** — ultra-lightweight HTTP server
  - **License**: Apache-2.0 OR MIT
  - **Homepage**: https://github.com/tiny-http/tiny-http

### Rust crates (optional RethinkDB backend)

The RethinkDB backend is optional and is enabled via a Cargo feature flag. It uses the following third-party component:

- **unreql** — Unofficial RethinkDB driver for Rust
  - **License**: MIT
  - **Homepage**: https://crates.io/crates/unreql

### Go modules (temporal-adapter)

The `temporal-adapter` service depends on the following third-party Go modules. All are commercially safe and redistribution-safe per the project OSS posture.

- **go.temporal.io/sdk** — Temporal Go SDK; used to implement the WorkRequest workflow and WorkItem activity
  - **License**: MIT
  - **Homepage**: https://github.com/temporalio/sdk-go
  - **Apache-2.0 transitive deps**: `google.golang.org/grpc`, `google.golang.org/protobuf`, `google.golang.org/genproto` — Apache-2.0; no NOTICE propagation obligation for binary distribution in this use case (API/server use), but noted here for completeness
- **go.uber.org/zap** — Structured logging
  - **License**: MIT
  - **Homepage**: https://github.com/uber-go/zap
- **go.temporal.io/api** — Temporal API protobuf definitions (indirect via SDK)
  - **License**: MIT
  - **Homepage**: https://github.com/temporalio/api-go
