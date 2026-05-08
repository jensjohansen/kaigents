# Example: Expense Report Approver Team

In this example, we will deploy a multi-agent team that automates the processing of employee expense reports.

## The Team Structure

The team consists of three agents:

1.  **Receipt Classifier**: Extracts structured data (vendor, date, amount, category) from receipt images or text.
2.  **Policy Compliance Auditor**: Compares extracted data against the `Corporate Expense Policy` and identifies violations.
3.  **Approval Router**: Decides if the report can be auto-approved, auto-rejected, or requires human review based on amount and compliance status.

## Step 1: Define the Agents

Create `expense-team-agents.yaml`:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Agent
metadata:
  name: receipt-classifier
  namespace: kaigents
spec:
  systemPrompt: |
    You are an expert financial clerk. Extract JSON data from receipts.
    Include fields: vendor, date, amount, currency, category.
---
apiVersion: core.kaigents.io/v1alpha1
kind: Agent
metadata:
  name: policy-auditor
  namespace: kaigents
spec:
  systemPrompt: |
    You are a corporate compliance officer. 
    Audit expenses against the policy:
    - Meals: < $50
    - Travel: Must have business purpose
    - Alcohol: Not reimbursable
---
apiVersion: core.kaigents.io/v1alpha1
kind: Agent
metadata:
  name: approval-router
  namespace: kaigents
spec:
  systemPrompt: |
    Route the expense report.
    - If total < $100 and COMPLIANT -> AUTO_APPROVE
    - If total >= $100 or NON_COMPLIANT -> HUMAN_REVIEW
```

Apply the agents:

```bash
kubectl apply -f expense-team-agents.yaml
```

## Step 2: Define the Team

Create `expense-team.yaml`:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Team
metadata:
  name: finance-automation
  namespace: kaigents
spec:
  coordinationModel: sequential
  members:
    - agentRef: receipt-classifier
    - agentRef: policy-auditor
    - agentRef: approval-router
```

Apply the team:

```bash
kubectl apply -f expense-team.yaml
```

## Step 3: Run the Process

Submit a work request with a sample expense:

```yaml
apiVersion: core.kaigents.io/v1alpha1
kind: Run
metadata:
  name: expense-run-001
  namespace: kaigents
spec:
  target:
    kind: Team
    name: finance-automation
  input: |
    Receipt: Starbucks, 2026-05-10, $15.50, Coffee with client.
```

Apply the run:

```bash
kubectl apply -f expense-run.yaml
```

## Step 4: Observe

Track the run progress:

```bash
kubectl get run expense-run-001 -n kaigents -o yaml
```

You can also view the **Run Timeline** in the Kaigents Dashboard to see the sequence of agent interactions and artifacts produced.
