# Replacing nginx‑Ingress with Envoy

This document outlines the steps required to migrate an existing Kubernetes cluster from the deprecated **nginx‑Ingress** controller to the **Envoy Ingress Controller**.  It also includes a concise checklist you can use to track progress.

## High‑level migration plan
1. **Install Envoy Ingress Controller** – Deploy via Helm or a plain manifest into a dedicated namespace.
2. **Migrate TLS secrets** – Copy any TLS `Secret`s used by nginx‑Ingress into the Envoy namespace.
3. **Translate Ingress resources** – Convert existing `Ingress` objects (and their annotations) into the Gateway API (`Gateway`, `VirtualService`, `DestinationRule`, optional `EnvoyFilter`).
4. **Validate routing and TLS** – Ensure services are reachable, TLS terminates correctly, and path rewrites work.
5. **Confirm health checks & metrics** – Verify Envoy health checks and expose required metrics.
6. **Remove nginx‑Ingress controller** – Delete the old controller and clean up associated resources.
7. **Update CI/CD & documentation** – Adjust deployment pipelines and internal docs to reference Envoy.

## Detailed steps
> **Note**: All commands assume you are in the repository root `/home/johnj/CascadeProjects/ai-customer-agents` and have `kubectl` and `helm` installed.

### 1. Install Envoy
```bash
helm repo add envoy https://istio-release.storage.googleapis.com/charts
helm repo update
helm install envoy envoy/envoy \
  --namespace envoy-system \
  --create-namespace \
  --set replicaCount=2 \
  --set resources.requests.cpu=100m \
  --set resources.requests.memory=200Mi
```

### 2. Migrate TLS secrets
```bash
# Example: copy a secret named `web-tls` from namespace `default` to `envoy-system`
kubectl get secret web-tls -n default -o yaml | \
  sed 's/namespace: default/namespace: envoy-system/' | \
  kubectl apply -f -
```

### 3. Translate Ingress to Gateway API
* Create a `Gateway` per host.
* Create a `VirtualService` per Ingress, translating annotations:
  * `nginx.ingress.kubernetes.io/ssl-redirect` → `tls` in `Gateway`
  * `nginx.ingress.kubernetes.io/rewrite-target` → `rewrite`
  * `nginx.ingress.kubernetes.io/cors-allow-origin` → `cors`

Example `Gateway`:
```yaml
apiVersion: gateway.networking.k8s.io/v1beta1
kind: Gateway
metadata:
  name: example-gateway
  namespace: envoy-system
spec:
  gatewayClassName: envoy
  listeners:
    - name: https
      port: 443
      protocol: HTTPS
      tls:
        mode: Terminate
        credentialName: web-tls
      hostname: example.com
```

Example `VirtualService`:
```yaml
apiVersion: gateway.networking.k8s.io/v1beta1
kind: VirtualService
metadata:
  name: example-virtualservice
  namespace: envoy-system
spec:
  gateways:
    - example-gateway
  hosts:
    - example.com
  http:
    - match:
        - uri:
            prefix: /api
      rewrite:
        uri: /v1
      route:
        - destination:
            host: api-service
            port:
              number: 80
```

### 4. Verify routing and TLS
```bash
# Replace <envoy-ip> with the external IP of the Envoy LoadBalancer
curl -k -H "Host: example.com" https://<envoy-ip>/api/health
```

### 5. Health checks & metrics
Add `health_checks` to `Gateway` or `EnvoyFilter` if needed, and expose Envoy stats via Prometheus annotations.

### 6. Remove nginx‑Ingress
```bash
kubectl delete deployment nginx-ingress-controller -n <namespace>
kubectl delete service nginx-ingress-service -n <namespace>
# Clean up any Ingress resources not yet converted
```

### 7. Update CI/CD & docs
* Adjust Helm values or kustomize overlays.
* Update any scripts that reference `nginx‑Ingress`.
* Commit this markdown file and update the project wiki.

## Checklist
- [ ] Install Envoy Ingress Controller
- [ ] Migrate TLS secrets
- [ ] Translate Ingress resources
- [ ] Verify routing, TLS, and health checks
- [ ] Confirm metrics exposure
- [ ] Remove nginx‑Ingress controller
- [ ] Update CI/CD pipelines
- [ ] Update documentation

---

**Author:** Cline
**Date:** 2026-02-17
