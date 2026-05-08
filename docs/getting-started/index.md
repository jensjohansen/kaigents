# Getting Started with Kaigents

This guide walks you through installing Kaigents on a Kubernetes cluster and deploying your first AI agent team.

## Overview

We will follow these steps:

1.  **Prepare the Environment**: Ensure your Kubernetes cluster is ready and shared dependencies are available.
2.  **Install Temporal**: Set up the durable execution engine (or connect to an existing one).
3.  **Install Kaigents**: Deploy the Kaigents operator and runner.
4.  **Deploy the Expense Report Approver Team**: Create a 3-agent team to automate corporate expense approvals.
5.  **Run and Observe**: Execute a work request and monitor it via the dashboard and observability stack.
6.  **Cleanup**: Remove the example resources and platform.

## Prerequisites

Before starting, ensure you have:

-   **Kubernetes Cluster**: A running cluster (v1.26+) with `kubectl` access.
-   **Helm**: v3.x installed.
-   **Cert-Manager**: Installed for certificate management.
-   **Ingress Controller**: (Optional but recommended) e.g., NGINX Ingress.
-   **OIDC Provider**: (Optional) e.g., Keycloak. Kaigents can run without OIDC in development mode.
-   **S3-Compatible Storage**: (Optional) e.g., MinIO or Ceph RGW. Local storage can be used for evaluation.

---

## Phase 1: Shared Dependencies

Kaigents relies on several industry-standard components. You can choose to deploy these specifically for Kaigents or use existing instances in your cluster.

### 1.1 Temporal (Durable Execution)

Temporal provides the substrate for long-running, resilient agent workflows.

-   If you have a shared Temporal cluster: [Connecting to existing Temporal](installation.md#connecting-to-existing-temporal)
-   If you need to install Temporal: [Installing Temporal for Kaigents](installation.md#installing-temporal)

### 1.2 Storage and Identity

-   **Storage**: Kaigents uses S3-compatible storage for artifacts.
-   **Identity**: Keycloak is the preferred OIDC provider.

---

## Phase 2: Installing Kaigents

Once the dependencies are ready, install the Kaigents operator:

```bash
helm upgrade --install kaigents-operator ./charts/kaigents-operator \
  --namespace kaigents \
  --create-namespace
```

See the [Installation Guide](installation.md) for detailed configuration options.

---

## Phase 3: Your First Team (Expense Report Approver)

We will build a team that classifies expense reports, checks them against company policy, and routes them for approval.

-   **Classifier Agent**: Extracts data from receipts.
-   **Policy Checker Agent**: Validates against `policy.pdf`.
-   **Approval Router**: Determines if human intervention is needed.

[Build the Expense Report Approver Team](example-team.md)

---

## Phase 4: Validation and NFRs

As part of this guide, we will validate the durability of the platform by simulating component failures during a run.

[Temporal NFR Validation](temporal-validation.md)
