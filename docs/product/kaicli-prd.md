# KaiCLI v2: Product Requirements Document (PRD)

## 1. Overview
KaiCLI is the primary user-facing surface for platform engineers and AI application developers. While the v1 MVP provided basic "plumbing" (kubectl-like resource management), **KaiCLI v2** is a productized interface focused on **velocity, governance, and sovereign operations**.

## 2. Key Requirements

### 2.1 Tool Management (`kaigents mcp`)
- **Integrated Discovery**: `kaigents mcp search [query]` calls the KaiCatalog API to find vetted tools.
- **One-Command Install**: `kaigents mcp add [name]` fetches the manifest, prompts for required secrets (e.g., API keys), and applies it to the cluster.
- **Dependency Resolution**: Automatically installs required sidecars or volume mounts defined in the manifest.

### 2.2 Execution Governance
- **Status Overview**: `kaigents status` provides a cluster-wide view of agent health, tool connectivity, and active work requests.
- **Process Inspection**: `kaigents inspect [work-request-id]` renders a rich, human-readable execution timeline with clickable links to artifacts.
- **Audit Exports**: `kaigents audit [process-name] --output json` generates compliance-ready audit trails.

### 2.3 Sovereign/Air-Gapped Operations
- **Offline Mirroring**: `kaigents mcp mirror` downloads the catalog and pre-fetches all container images to a local registry.
- **Registry Rewriting**: Supports a global `KAIGENTS_REGISTRY_MIRROR` flag to rewrite image paths at install time.

### 2.4 Developer Experience (DX)
- **JSON-First**: Every command supports `--output json` for integration with CI/CD and automation scripts.
- **Interactive Prompts**: Smart prompting for missing configuration or secrets during tool installation.

## 3. Command Menu (v2)

| Command | Description | Status |
| --- | --- | --- |
| `kaigents apply -f` | Apply resource manifests (v1 functionality) | Existing |
| `kaigents run [target]` | Trigger an agent or process (v1 functionality) | Existing |
| `kaigents status` | Cluster health and connectivity overview | **New** |
| `kaigents mcp search` | Search the curated tool catalog | **New** |
| `kaigents mcp add` | Install a tool from the catalog | **New** |
| `kaigents mcp mirror` | Pre-fetch catalog for air-gapped use | **New** |
| `kaigents timeline` | Render run history (v1 functionality) | Existing |
| `kaigents artifact` | Fetch run artifacts (v1 functionality) | Existing |

## 4. Architecture Consistency
- **Persistence**: Queries RethinkDB (via KaiManager/KaiCatalog APIs) for state; does not perform local Git operations.
- **Identity**: Uses OIDC (Keycloak) tokens for all cluster-level operations.
- **Substrate**: Acts as a lightweight client to the `kaigents`, `kaimanager`, and `kaicatalog` services.
