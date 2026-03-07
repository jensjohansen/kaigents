# Technology Research Seed: Commercially-permissible OSS components (Kaigents)

## Purpose
This document is the seed for Kaigents “technology research” and buy-vs-build decisions.

It focuses on OSS components that are commonly used in commercial contexts, **but highlights licensing constraints** that matter if Kaigents is:

- open source (target: permissive license)
- a commercial product (including paid support/hosting)

This is not legal advice; it is a technical due diligence checklist.

## How to read this document
“Commercially-permissible” here generally means:

- the component’s license is commonly considered compatible with commercial use and redistribution (e.g., MIT, Apache-2.0), **or**
- the component can be integrated as an optional external dependency that customers install and run themselves, **or**
- the component is permissible for internal use but may be problematic to redistribute or bundle.

## OSS publication and redistribution policy (permissive-friendly)
Kaigents’ goal is to be safe and legal to publish as open source under a permissive license.

Accordingly, for every third-party component we consider, we must clearly classify:

- **Redistribute (bundle)**
  - Safe to include in the Kaigents repository and/or in Kaigents release artifacts.
  - Typical fit: MIT / Apache-2.0 / BSD-2-Clause / BSD-3-Clause.

- **Integrate-only (user-supplied)**
  - Kaigents can support invoking it (CLI/API) and ingesting outputs, but **we do not bundle or redistribute**.
  - Typical triggers: source-available licenses, copyleft constraints we don’t want to inherit, “terms and conditions” licenses, or branding restrictions.

- **Exclude**
  - Not supported in Kaigents OSS offering due to licensing/terms risk.

Licensing permissiveness is an explicit, valid reason to push back on or revise an ITD choice, even if a tool is technically strong.

### Policy summary (table)

| Classification | What it means for Kaigents OSS |
| --- | --- |
| Redistribute (bundle) | Safe to include in repository and/or release artifacts |
| Integrate-only (user-supplied) | Supported by invocation/ingestion, but Kaigents does not bundle |
| Exclude | Not supported due to licensing/terms risk |

## 1) Tooling & workflow automation platforms — licensing caveats (benchmarks)

### 1.1 n8n — Sustainable Use License (fair-code)
- **Primary source:** https://docs.n8n.io/sustainable-use-license/
- **License posture:** Exclude (core)
- **Why:** License restricts use to internal business purposes and disallows paid hosting/white-labeling without a commercial agreement.
- **Kaigents note:** Treat as a benchmark for connector UX, not a dependency.

### 1.2 Open WebUI — branding restriction clause
- **Primary source (license):** https://github.com/open-webui/open-webui/blob/main/LICENSE
- **License posture:** Exclude (core)
- **Why:** License forbids altering/removing “Open WebUI” branding except in limited circumstances; adds compliance complexity.
- **Kaigents note:** Treat as a UX benchmark only.

## 2) Workflow substrates (Kubernetes-native vs embedded)

### 2.1 Argo Workflows
- **Primary source:** https://github.com/argoproj/argo-workflows
- **License posture:** Redistribute (bundle)
- **Why:** Apache-2.0; Kubernetes-native workflow substrate.

### 2.2 Tekton Pipelines
- **Primary source:** https://github.com/tektoncd/pipeline
- **License posture:** Redistribute (bundle)
- **Why:** Apache-2.0; Kubernetes-native pipeline substrate.

### 2.3 Embedded DAG libraries (Rust)
- dagx
  - **Primary source:** https://docs.rs/dagx/latest/dagx/
  - **License posture:** Redistribute (bundle)
  - **License:** MIT
- async_dag
  - **Primary source:** https://docs.rs/async_dag/latest/async_dag/
  - **License posture:** Redistribute (bundle)
  - **License:** MIT OR Apache-2.0
- oxigdal-workflow
  - **Primary source:** https://docs.rs/crate/oxigdal-workflow/latest
  - **License posture:** Redistribute (bundle)
  - **License:** Apache-2.0
  - **Note:** validate maturity and dependency footprint before adopting.

## 3) Evaluation / observability tools

### 3.1 Langfuse
- **Primary source (license):** https://github.com/langfuse/langfuse/blob/main/LICENSE
- **License posture:** Redistribute (bundle) with carve-out awareness
- **Why:** License states most of repo is MIT, with explicit carve-out for `ee/` directories under separate license.
- **Kaigents note:** Use as an OSS baseline for eval/observability; ensure we don’t depend on EE-only modules.

### 3.2 W&B Weave
- **Primary source (license):** https://github.com/wandb/weave/blob/master/LICENSE
- **License posture:** Redistribute (bundle)
- **Why:** Apache-2.0.

### 3.3 Arize Phoenix
- **Primary source (license):** https://github.com/Arize-ai/phoenix/blob/main/LICENSE
- **License posture:** Exclude (core)
- **Why:** Elastic License 2.0 (hosted/managed-service restriction) is not aligned with “commercial-safe OSS” for a platform.

## 4) Memory products (agent state / long-term memory)

### 4.1 Zep
- **Primary source (license):** https://github.com/getzep/zep/blob/main/LICENSE
- **License posture:** Redistribute (bundle)
- **Why:** Apache-2.0.
- **Kaigents note:** likely an integration option rather than core.

### 4.2 Letta
- **Primary source (one repo license):** https://github.com/letta-ai/letta-code/blob/main/LICENSE
- **License posture:** Integrate-only (until full license boundary is verified)
- **Why:** Apache-2.0 in letta-code; verify licensing across Letta repos before reuse.

## 5) Platform UI baselines

### 5.1 Backstage
- **Primary source (license):** https://github.com/backstage/backstage/blob/master/LICENSE
- **License posture:** Redistribute (bundle)
- **Why:** Apache-2.0.
- **Kaigents note:** candidate for Platform Mode UI via plugins.
