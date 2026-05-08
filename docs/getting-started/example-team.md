# Example: Kairon Retail Lite Team

In this example, we will deploy a multi-agent team that automates market research and competitor analysis for e-commerce retailers. This "lite" version demonstrates how Kaigents coordinates agents to perform complex web research and data synthesis.

## The Team Structure

The team consists of two agents:

1.  **Market Opportunity Scout**: Uses web research tools to identify high-growth product categories and trending consumer needs.
2.  **Competitor Pricing Analyst**: Scrapes competitor store data to determine price points, shipping terms, and customer sentiment.

## Step 1: Define the Tools

Kaigents agents use the **Model Context Protocol (MCP)** to interact with the world. We'll register the `web-acquisition` toolset.

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: MCPServer
metadata:
  name: web-acquisition
  namespace: kaigents
spec:
  transport: http
  url: "http://web-acquisition-mcp.kaigents.svc.cluster.local/mcp"
---
apiVersion: core.kaigents.io/v1alpha1
kind: Tool
metadata:
  name: web-fetch
  namespace: kaigents
spec:
  mcpServerRef: "web-acquisition"
  toolName: "web.fetch_url"
  description: "Fetch content from a URL"
```

## Step 2: Define the Agents

Create `retail-lite-agents.yaml`:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Agent
metadata:
  name: opportunity-scout
  namespace: kaigents
spec:
  systemPrompt: |
    You are a strategic market researcher. Your goal is to find high-growth e-commerce niches.
    Use web search tools to identify 3 trending product categories for 2026.
  tools:
    - name: web-fetch
---
apiVersion: core.kaigents.io/v1alpha1
kind: Agent
metadata:
  name: pricing-analyst
  namespace: kaigents
spec:
  systemPrompt: |
    You are a retail pricing expert. For a given product category, find the top 3 competitors
    and summarize their pricing and shipping strategy.
  tools:
    - name: web-fetch
```

## Step 3: Define the Team

Create `retail-lite-team.yaml`:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Team
metadata:
  name: retail-strategy-team
  namespace: kaigents
spec:
  agents:
    - name: scout
      agentRef: opportunity-scout
    - name: analyst
      agentRef: pricing-analyst
```

## Step 4: Run the Process

Submit a work request to start the market research:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Run
metadata:
  name: market-research-001
  namespace: kaigents
spec:
  target:
    kind: Team
    name: retail-strategy-team
  input: |
    Perform a market research for the "Sustainable Kitchenware" niche. 
    Find opportunities and analyze the top 3 competitors.
```

## Step 5: Observe and Scale

Track the run progress using the Kaigents Dashboard or `kubectl`:

```bash
kubectl get run market-research-001 -n kaigents -w
```

This "Lite" example is a preview of our **managed Kairon Retail service**, which includes advanced supply chain sourcing, creative asset generation (Flux/ComfyUI), and automated store revitalization.
