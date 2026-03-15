# Temporal Installation (Customer Guide)

This guide shows how to install Temporal on Kubernetes for use with Kaigents.

Kaigents can run against:
- A Temporal cluster you already operate, or
- A Temporal cluster you install using the official Temporal Helm chart (shown below)

## Install Temporal via Helm (Kubernetes)

### Prerequisites

- A Kubernetes cluster
- `kubectl` configured for the cluster
- `helm` installed
- A namespace to install into (example: `kaigents`)

Create the namespace (if needed):

```bash
kubectl create namespace kaigents
```

### Add the Temporal Helm repo

```bash
helm repo add temporal https://go.temporal.io/helm-charts
helm repo update
```

### Install Temporal (minimal)

This is the simplest starting point using the official chart defaults:

```bash
helm upgrade --install temporal temporal/temporal \
  --namespace kaigents
```

### Verify

```bash
kubectl -n kaigents get pods
kubectl -n kaigents get svc
```

### Access the Temporal UI

Port-forward the UI service:

```bash
kubectl -n kaigents port-forward svc/temporal-web 8233:8080
```

Then open:

- http://localhost:8233

### Uninstall

```bash
helm -n kaigents uninstall temporal
```

## Customizing the installation

If you need to tune Temporal for your operational requirements, use a values file.

Export the chart defaults:

```bash
helm show values temporal/temporal > temporal.values.yaml
```

Edit `temporal.values.yaml`, then install/upgrade:

```bash
helm upgrade --install temporal temporal/temporal \
  --namespace kaigents \
  -f temporal.values.yaml
```

Common customizations:
- Reduce replicas for smaller clusters
- Set CPU/memory requests/limits
- Configure persistence to use your existing PostgreSQL
- Configure ingress / exposure for the UI
- Tune PodDisruptionBudgets (PDBs) to match replica counts

## Configure Kaigents to use Temporal

Configure Kaigents to point at your Temporal Frontend gRPC endpoint (example service name if installed into `kaigents`):

- `temporal-frontend.kaigents.svc.cluster.local:7233`

Set the Temporal address wherever Kaigents runtime configuration is defined for your deployment method.
