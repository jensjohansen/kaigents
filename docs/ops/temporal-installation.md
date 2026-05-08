# Temporal Installation (Operations Guide)

Temporal provides the durable execution substrate for Kaigents. This guide outlines how to deploy Temporal specifically for Kaigents or connect to an existing enterprise instance.

## Deployment Strategies

Kaigents is unopinionated about how Temporal is deployed, as long as the **Temporal Frontend** gRPC service is reachable.

### Strategy 1: Isolated (Sidecar-style)
Deploy a dedicated Temporal cluster in the same namespace as Kaigents (`kaigents`). This is recommended for evaluation and projects requiring strict isolation.

### Strategy 2: Shared (Enterprise)
Connect to an existing shared Temporal cluster managed by your platform or DevOps team. This is recommended for production environments to centralize durability management.

---

## 1. Deploying Isolated Temporal (Helm)

We recommend using the official Temporal Helm chart.

### 1.1 Add the Repository
```bash
helm repo add temporal https://go.temporal.io/helm-charts
helm repo update
```

### 1.2 Minimal Configuration for Kaigents
Create a `temporal-kaigents-values.yaml` for a minimal, single-replica deployment:

```yaml
server:
  replicaCount: 1
cassandra:
  enabled: false
postgresql:
  enabled: true # Uses a sidecar PostgreSQL for persistence
```

### 1.3 Install
```bash
helm upgrade --install temporal temporal/temporal \
  --namespace kaigents \
  --create-namespace \
  -f temporal-kaigents-values.yaml
```

---

## 2. Connecting to External Temporal

If you already have Temporal running in another namespace (e.g., `temporal-system`), configure Kaigents to point to the external address.

### 2.1 Configuration
Update your Kaigents `values-override.yaml`:

```yaml
# Point Kaigents to the external Temporal Frontend
temporalAdapter:
  env:
    - name: TEMPORAL_ADDRESS
      value: "temporal-frontend.temporal-system.svc.cluster.local:7233"
    - name: TEMPORAL_NAMESPACE
      value: "default"
```

---

## 3. Verification

### 3.1 Check Pod Status
```bash
kubectl get pods -n kaigents -l app.kubernetes.io/instance=temporal
```

### 3.2 Access the UI
```bash
kubectl port-forward -n kaigents svc/temporal-web 8233:8080
```
Open [http://localhost:8233](http://localhost:8233) to monitor agent workflows.

---

## 4. Maintenance and Uninstallation

### 4.1 Upgrading
```bash
helm upgrade temporal temporal/temporal -n kaigents --reuse-values
```

### 4.2 Uninstallation
```bash
helm uninstall temporal -n kaigents
# Note: Persistence (PVs) may remain and should be cleaned up manually if needed.
```
