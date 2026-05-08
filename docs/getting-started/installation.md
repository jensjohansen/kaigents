# Installation Guide

Kaigents is designed to be flexible. You can install all dependencies alongside the platform or connect to existing enterprise infrastructure.

## 1. Temporal (Durable Execution)

Kaigents requires Temporal to manage long-running agent workflows.

### Option A: Install Temporal for Kaigents

If you don't have Temporal, you can install it into the `kaigents` namespace. We recommend using the official Temporal Helm chart.

```bash
helm repo add temporal https://go.temporal.io/helm-charts
helm repo update

# Install a minimal Temporal cluster
helm upgrade --install temporal temporal/temporal \
  --namespace kaigents \
  --create-namespace \
  --set server.replicaCount=1 \
  --set cassandra.enabled=false \
  --set postgresql.enabled=true
```

### Option B: Connecting to Existing Temporal

If you already have Temporal running (e.g., in a `temporal-system` namespace), configure Kaigents to use it:

```yaml
# values.yaml
temporal:
  address: temporal-frontend.temporal-system.svc.cluster.local:7233
  namespace: default # Temporal namespace
```

---

## 2. Shared Infrastructure (S3 and Identity)

Kaigents integrates with your existing S3-compatible storage and OIDC providers.

### 2.1 Artifact Storage (S3)

Kaigents uses S3 for durable artifact storage.

- **Option A: MinIO (Development)**: If you don't have S3, you can install MinIO into your cluster.
- **Option B: Enterprise S3 (Production)**: Connect to AWS S3, Ceph RGW, or Google Cloud Storage.

### 2.2 Identity (OIDC)

- **Option A: Development Mode**: Disable OIDC to use the platform without authentication (not recommended for production).
- **Option B: Keycloak/Okta**: Connect Kaigents to your existing OIDC provider for RBAC and SSO.

---

## 3. Kaigents Operator

The operator manages the Kaigents lifecycle and CRDs.

### Configure Dependencies

Create a `values-override.yaml` to point Kaigents to your storage and OIDC providers:

```yaml
# values-override.yaml
temporalAdapter:
  url: "http://kaigents-temporal-adapter.kaigents.svc.cluster.local:8080"

# Storage Configuration
storage:
  s3:
    endpoint: "http://minio.storage.svc.cluster.local:9000"
    bucket: "kaigents-artifacts"
    region: "us-east-1"
    accessKey: "..."
    secretKey: "..."

# OIDC Configuration (Optional)
oidc:
  enabled: true
  issuerUrl: "https://keycloak.example.com/realms/kaigents"
  clientId: "kaigents-platform"
```

### Deploy Kaigents

```bash
helm upgrade --install kaigents-operator ./charts/kaigents-operator \
  --namespace kaigents \
  -f values-override.yaml
```

## 3. Verification

Verify that the operator and runner pods are healthy:

```bash
kubectl get pods -n kaigents
```

You should see:
- `kaigents-operator-*`
- `kaigents-temporal-adapter-*` (if enabled)
- `kaigents-runner-*`
