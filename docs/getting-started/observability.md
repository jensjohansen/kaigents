# Observability in Kaigents

Kaigents is built for enterprise operations, providing deep visibility into agent behaviors and system health.

## 1. The Kaigents Dashboard

The Dashboard provides a real-time view of agent teams, run timelines, and artifacts.

### Accessing the Dashboard
If you haven't exposed the dashboard via an Ingress, use port-forwarding:

```bash
kubectl port-forward -n kaigents svc/kaigents-dashboard 3000:80
```
Open [http://localhost:3000](http://localhost:3000).

### Key Features
- **Team Browser**: View all deployed agent teams and their status.
- **Run Timeline**: Drill into a specific `Run` to see the exact sequence of agent thoughts, tool calls, and results.
- **Artifact Gallery**: Browse and download files (images, PDFs, datasets) produced by agents.

---

## 2. Metrics (Prometheus & Grafana)

Kaigents exports standard Prometheus metrics for all components.

### Pre-configured Dashboards
If you have Grafana installed, you can import the pre-configured Kaigents dashboards from `deploy/grafana/dashboards/`:
- **Kaigents Overview**: Platform-wide health and throughput.
- **Agent Performance**: Latency and success rates per agent.
- **Hardware Utilization**: Monitor CPU/GPU/NPU usage via the `RoutingPolicy`.

### Key Metrics to Watch
- `kaigents_run_total`: Total number of agent runs.
- `kaigents_task_duration_seconds`: Histogram of task execution times.
- `kaigents_tool_call_errors_total`: Count of tool failures.

---

## 3. Logs (Loki)

All Kaigents components output structured JSON logs, optimized for ingestion by Grafana Loki.

### Querying Logs
Use LogQL in Grafana to filter logs by namespace or component:

```logql
{namespace="kaigents", app="kaigents-operator"} |= "error"
```

### Trace IDs
Every `Run` is assigned a unique `trace_id` that is propagated through the operator, adapter, and runner. Use this ID to correlate logs across the entire execution path.
