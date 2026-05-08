# Installation Guide

Kaigents is designed for enterprise flexibility. You can install all dependencies alongside the platform or connect to existing shared infrastructure.

## 1. Temporal (Durable Execution)

Kaigents requires Temporal to manage long-running agent workflows. 

### Option A: Isolated Temporal (Recommended for Getting Started)

If you want an isolated instance specifically for Kaigents, we recommend using the official Temporal Helm chart.

```bash
helm repo add temporal https://go.temporal.io/helm-charts
helm repo update

# Install a minimal Temporal cluster into the kaigents namespace
helm upgrade --install temporal temporal/temporal \
  --namespace kaigents \
  --create-namespace \
  --set server.replicaCount=1 \
  --set cassandra.enabled=false \
  --set postgresql.enabled=true
```

### Option B: Shared Temporal (Enterprise)

If you already have a shared Temporal cluster, configure the Kaigents Temporal Adapter to connect to it by setting the `TEMPORAL_ADDRESS` environment variable in your `values-override.yaml`.

---

## 2. Infrastructure (S3 and Identity)

Kaigents integrates with industry-standard S3 and OIDC providers. 

### 2.1 Artifact Storage (S3)

Kaigents uses S3 for durable artifact storage.
- **Local/On-Prem**: Use MinIO or Ceph RGW.
- **Cloud**: Connect to AWS S3, Azure Blob (S3-compat), or GCS.

### 2.2 Model Endpoints

Kaigents is model-agnostic but requires OpenAI-compatible endpoints.
- **On-Prem/Edge**: We recommend **Lemonade Server** running on **AMD Ryzen AI** hardware for optimal TCO.
- **Cloud**: Connect to OpenAI, Anthropic (via proxy), or Azure OpenAI.

---

## 3. Kaigents Operator

### Configure Dependencies

Create a `values-override.yaml`. If you are following the Kairon Retail example on-prem, your configuration might look like this:

```yaml
# values-override.yaml

# Connection to the Temporal Adapter (which connects to Temporal)
temporalAdapter:
  url: "http://kaigents-temporal-adapter.kaigents.svc.cluster.local:8080"

# S3 Storage Configuration
storage:
  s3:
    endpoint: "http://minio.storage.svc.cluster.local:9000"
    bucket: "kaigents-artifacts"
    region: "us-east-1"
    accessKey: "YOUR_ACCESS_KEY"
    secretKey: "YOUR_SECRET_KEY"

# Global Model Endpoint Defaults
modelEndpoints:
  default: "http://10.7.0.7:13305/v1" # Point to your local Lemonade Server
```

### Deploy Kaigents

```bash
helm upgrade --install kaigents-operator ./charts/kaigents-operator \
  --namespace kaigents \
  -f values-override.yaml
```

---

## 4. Verification and Health Check

Verify that the operator and adapter are healthy:

```bash
kubectl get pods -n kaigents
```

Check the operator logs to ensure it has successfully connected to Temporal:

```bash
kubectl logs -l app.kubernetes.io/name=kaigents-operator -n kaigents
```

---

## 5. Uninstallation

To remove Kaigents and the example team:

```bash
# Remove the example resources
kubectl delete -f retail-lite-team.yaml -n kaigents
kubectl delete -f retail-lite-agents.yaml -n kaigents

# Remove the platform
helm uninstall kaigents-operator -n kaigents

# Optionally remove the namespace and internal Temporal
kubectl delete namespace kaigents
```
