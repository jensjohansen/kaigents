# KaiCLI: Product Requirements Document (PRD)

## 1. Overview

KaiCLI is the **developer and operator experience layer** for Kaigents — a redesigned, progressive command-line interface that makes the platform approachable for new adopters while remaining powerful for experienced platform engineers.

Kaigents v1.0.0 ships a functional CLI (Milestone 1H) that covers the basics: install/bootstrap, resource lifecycle management, triggering runs, viewing run timelines, and fetching artifacts. That CLI is built for platform engineers who already understand Kubernetes, CRDs, and the Kaigents domain model.

KaiCLI evolves the CLI in two directions simultaneously:
1. **Simpler on-ramp**: new commands for onboarding, catalog integration, and process scaffolding that lower the barrier for first-time adopters.
2. **More powerful operations**: structured output modes, health and status commands, and a richer interactive experience for platform engineers managing production deployments.

### 1.1 At a glance

| Area | Summary |
| --- | --- |
| Primary value proposition | A CLI that makes Kaigents feel like `apt install` and `helm install` — powerful and obvious at the same time |
| Primary users | AI application developers (new adopters), platform engineers (day-to-day operations) |
| Builds on | Kaigents v1.0.0 CLI (Milestone 1H): resource lifecycle, run trigger, run timeline, artifact fetch |
| Net new | Catalog integration (`mcp add`), onboarding wizard, platform status, process scaffolding, structured output modes |
| KaiCatalog dependency | `kaigents mcp add` is the CLI surface for KaiCatalog; the two are designed together but the CLI ships independently |

---

## 2. Problem Statement

The Kaigents v1.0.0 CLI was designed and built as an execution tool for platform engineers. It works, but it creates friction in two important scenarios:

**The new adopter scenario**: A developer hears about Kaigents, clones the repo, and tries to get a first agent running. They encounter: raw CRD YAML authoring, manual KMCP manifest configuration, `kubectl`-style commands with no guided workflow, and no easy way to discover what MCP tools are available. The experience is close to what you'd expect from a mature internal tool — which is appropriate for a platform team, but not for driving adoption.

**The catalog scenario**: When a developer wants to add a new tool capability to their agent team, today the workflow is: find an MCP server somewhere, understand its KMCP deployment requirements, write a manifest, apply it, verify it. There is no `kaigents mcp add github` equivalent. KaiCatalog provides the catalog, but without a good CLI surface, catalog value is unrealized.

**The operational scenario**: Platform engineers managing production deployments need quick answers: "Is the platform healthy?", "What agents are running?", "What's the current tool allowlist for this agent?", "Give me run timeline output I can pipe to jq." Today these require combining multiple commands with raw JSON output.

---

## 3. Goals and Non-Goals

### 3.1 Goals

- Provide a guided **onboarding wizard** (`kaigents onboard`) that walks a new user through installation, cluster connectivity, model endpoint configuration, and first agent deployment.
- Provide a **platform status command** (`kaigents status`) that gives an immediate health overview of all core platform components.
- Provide a **catalog integration command group** (`kaigents mcp`) that integrates with KaiCatalog for MCP server discovery, install, and management.
- Provide a **process scaffolding command** (`kaigents process new`) that generates a valid, annotated Process + Task CRD skeleton for a user-described use case.
- Provide **structured output modes** across all commands: human-readable table (default), JSON, and YAML, with consistent `--output` flag.
- Provide a **team operations command group** (`kaigents team`) with simplified commands for common team management tasks without requiring raw `kubectl apply`.
- Maintain full backward compatibility with the v1.0.0 CLI command surface.

### 3.2 Non-Goals

- KaiCLI is not a GUI or TUI. It is a command-line tool.
- KaiCLI does not build or manage the MCP catalog itself; it consumes it. KaiCatalog is a separate product.
- KaiCLI does not replace `kubectl` for low-level CRD inspection. Power users can always drop to `kubectl` for anything not covered by KaiCLI.
- KaiCLI does not provide interactive chat with agents. Agent invocation is a run trigger, not a conversational interface.
- KaiCLI does not manage KaiManager entities (Personas, QualityGates, etc.) in v1; those commands are delivered as part of KaiManager.

---

## 4. Target Users

- **AI application developers (new adopters)**: Want to get a working agent team deployed in under 30 minutes. Comfortable with command-line tools but not Kubernetes experts.
- **Platform engineers**: Manage production Kaigents deployments; need fast, scriptable, structured-output commands for automation and monitoring.
- **Process owners (limited CLI use)**: May use `kaigents run`, `kaigents run timeline`, and `kaigents status` to monitor work without needing dashboard access.

---

## 5. Design Principles

### 5.1 Progressive disclosure

Simple commands should work with no flags. Power is available through flags, not required at the start. Example:

```
# Simple (works immediately)
kaigents mcp add github

# Advanced (available when needed)
kaigents mcp add github --namespace security-team --version 2.1.0 --dry-run
```

### 5.2 Consistent output contract

Every command that returns data supports `--output` with three modes:
- `table` (default): human-readable formatted output, not designed for piping.
- `json`: machine-readable JSON, stable schema, safe to pipe to `jq`.
- `yaml`: Kubernetes-resource-compatible YAML for commands that return CRD-representable data.

### 5.3 Fail loudly and clearly

Error messages must:
- State what went wrong in plain English.
- Identify the specific resource or configuration involved.
- Suggest a concrete next step or point to relevant documentation.

Never silently succeed when something is wrong. Never print a stack trace as the primary error surface.

### 5.4 Verbs first, nouns second

Command structure follows `kaigents <verb> <noun>` where possible:

```
kaigents add mcp github         # Less good
kaigents mcp add github         # Good: noun group, then verb
kaigents run list               # Good
kaigents process new            # Good
```

Noun-grouped commands (`kaigents mcp`, `kaigents run`, `kaigents team`) keep related operations discoverable via `kaigents <noun> --help`.

---

## 6. Command Surface

### 6.1 Onboarding

#### `kaigents onboard`

An interactive wizard for first-time setup. Runs a guided sequence:

1. Verify Node/toolchain prerequisites.
2. Verify Kubernetes cluster connectivity and context.
3. Install Kaigents CRDs and operator if not present (optionally: `--skip-install` for existing deployments).
4. Configure model endpoint (Lemonade or external OpenAI-compatible).
5. Configure MCP catalog source.
6. Deploy a minimal "hello world" agent run to verify end-to-end connectivity.
7. Print a summary of what was configured and next steps.

Flags:
- `--skip-install`: skip CRD/operator installation; assume Kaigents is already installed.
- `--namespace <ns>`: target namespace (default: `kaigents`).
- `--non-interactive`: accept defaults for all prompts; suitable for CI/scripted environments.

---

### 6.2 Platform status

#### `kaigents status`

Prints a health overview of all core Kaigents platform components. Output includes component name, status (healthy/degraded/unreachable), and a brief diagnostic message where relevant.

Components checked:
- Kaigents operator (controller) pod health.
- Temporal adapter service reachability.
- Model endpoint reachability (configured endpoint).
- Artifact store reachability (S3-compatible endpoint).
- MCP servers registered and their health status.
- Keycloak OIDC endpoint reachability (Platform Mode only).

Example output (table mode):
```
COMPONENT               STATUS      MESSAGE
kaigents-operator       healthy     1/1 pods running
temporal-adapter        healthy     responding at cluster.local:8080
model-endpoint          healthy     lemonade-server v1.2.0
artifact-store          healthy     ceph-rgw (s3-compatible)
mcp/github              healthy     github-mcp v0.4.1
mcp/slack               degraded    timeout on last health check
keycloak-oidc           healthy     ai-agents realm
```

Flags:
- `--output json|yaml|table`
- `--watch`: refresh every 30 seconds until interrupted.

---

### 6.3 MCP catalog operations

#### `kaigents mcp add <name>`

Installs an MCP server from KaiCatalog into the cluster.

- Fetches the KMCP manifest from the catalog.
- Applies the `MCPServer` CRD to the target namespace.
- Waits for the MCP server to become ready.
- Prints a success message with the tool capabilities registered.

Flags:
- `--namespace <ns>`: target namespace.
- `--version <v>`: pin to a specific catalog entry version.
- `--dry-run`: print the KMCP manifest that would be applied without applying it.
- `--output json|yaml|table`

Example:
```
$ kaigents mcp add github
Fetching catalog entry: github (v0.4.1)
Applying MCPServer manifest to namespace: kaigents
Waiting for github-mcp to become ready... done
Tools registered: create_issue, list_prs, get_file, search_code (4 tools)
```

#### `kaigents mcp list`

Lists all MCP servers currently installed in the cluster with their health status and registered tool count.

#### `kaigents mcp search <query>`

Searches the KaiCatalog for MCP servers matching the query. Returns name, description, tool count, license, and catalog version.

Example:
```
$ kaigents mcp search email
NAME            DESCRIPTION                         TOOLS   LICENSE     VERSION
gmail           Read, draft, and send Gmail          8       MIT         0.3.0
smtp-send       Send email via SMTP relay            2       Apache-2.0  0.1.2
outlook-mcp     Microsoft Outlook integration        6       MIT         0.2.1
```

#### `kaigents mcp remove <name>`

Removes an MCP server from the cluster (unregisters from KMCP, deletes the `MCPServer` CRD resource).

#### `kaigents mcp inspect <name>`

Prints full details for an installed or catalog MCP server: description, all tools with their input/output schemas, license, version, and security classification.

---

### 6.4 Run operations (evolution of v1.0.0 CLI)

The existing run commands are extended with consistent output modes and additional convenience operations.

#### `kaigents run trigger <process-name> [--input <json-or-file>]`

Triggers a Work Request for the named Process. Replaces the lower-level v1.0.0 trigger command with a process-oriented interface.

#### `kaigents run list [--process <name>] [--status <status>]`

Lists Work Requests with filtering by Process and status. Supports `--output json|yaml|table`.

#### `kaigents run timeline <work-request-id>`

Renders the run timeline for a Work Request. Output is human-readable by default; `--output json` emits the raw timeline event JSON for piping.

#### `kaigents run approve <work-request-id> [--rework]`

Sends an approval or rework signal to a Work Request waiting on a human gate. Replaces manual signal API calls.

#### `kaigents run artifacts <work-request-id>`

Lists and optionally downloads artifacts produced by a Work Request.

---

### 6.5 Process scaffolding

#### `kaigents process new`

An interactive command that scaffolds a Process + Task CRD skeleton.

The wizard prompts for:
1. Process name and objective.
2. Number and names of tasks.
3. For each task: responsible actor type (AI or human), required tools (offered as a search against installed MCP servers), and success criteria.
4. Whether any tasks require human approval gates.

Output: a generated, annotated YAML file saved to the current directory. The user reviews and applies it with `kaigents apply -f <file>` or `kubectl apply -f <file>`.

The generated YAML includes inline comments explaining each field, making it a learning artifact as well as a starting point.

---

### 6.6 Team operations

#### `kaigents team list`

Lists all Teams defined in the cluster with their member agents and last Work Request status.

#### `kaigents team show <team-name>`

Shows full detail for a Team: member agents, Persona assignments, tool allowlist, and recent Work Request history.

#### `kaigents team run <team-name> <process-name> [--input <json-or-file>]`

A combined command for triggering a Process execution with a specific Team. A common pattern for managed AI teams that the user wants to invoke without knowing the underlying Work Request mechanics.

---

### 6.7 Agent operations

#### `kaigents agent list`

Lists all Agents defined in the cluster.

#### `kaigents agent show <agent-name>`

Shows full detail for an Agent: type (AI/human), Persona, allowed tools, current Work Request assignment if any.

---

## 7. Functional Requirements

### 7.1 Compatibility

- All v1.0.0 CLI commands remain functional and unchanged. KaiCLI is an additive, backward-compatible evolution.
- KaiCLI commands must work against any Kaigents v1.0.0+ cluster. New features (e.g., `mcp add`) degrade gracefully if KaiCatalog integration is not configured.

### 7.2 Output stability

- `--output json` output schemas are versioned and stable across patch releases. Breaking changes to JSON output schema require a minor version bump.
- All JSON output includes a `apiVersion` field indicating the schema version.

### 7.3 Authentication

- KaiCLI inherits authentication from the existing Kaigents CLI: OIDC token from kubeconfig or `KAIGENTS_TOKEN` environment variable.
- Commands that mutate state require operator-level RBAC. Commands that read state are accessible to viewers.

### 7.4 Catalog integration

- Catalog source is configured via `kaigents config set catalog-url <url>` or `KAIGENTS_CATALOG_URL` environment variable.
- Default catalog URL points to the official `kaigents/mcp-catalog` repository release artifacts.
- Catalog operations work offline for `mcp list` (from local cache); require network for `mcp add`, `mcp search`, and cache refresh.

---

## 8. Quality Attributes

- **Time to first agent**: a developer with a running Kubernetes cluster should be able to run `kaigents onboard` and have a working agent run visible in under 15 minutes.
- **Error quality**: every error message includes: what failed, which resource was involved, and what to do next.
- **Scriptability**: all commands are safe to use in CI/CD pipelines; exit codes are meaningful; `--output json` is always available.
- **Performance**: `kaigents status` must complete in under 5 seconds on a healthy cluster. `kaigents mcp add` must complete in under 30 seconds for a simple MCP server.

---

## 9. Milestones (proposed)

| Milestone | Deliverables |
| --- | --- |
| KaiCLI-1 | `--output json|yaml|table` across all existing v1.0.0 commands; `kaigents status`; `kaigents run approve` |
| KaiCLI-2 | `kaigents mcp add|list|remove|inspect`; catalog source configuration; `kaigents mcp search` |
| KaiCLI-3 | `kaigents onboard` wizard; `kaigents process new` scaffolding |
| KaiCLI-4 | `kaigents team` and `kaigents agent` command groups; `kaigents run trigger` process-oriented interface |
