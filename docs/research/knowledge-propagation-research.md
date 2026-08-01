# Knowledge Propagation Between Kaigents Agents: Porting Experience Without Re-Computing Ingestion

*Research Date: July 21, 2026*
*Status: Internal use only. Not for external distribution.*

## 0. Purpose

This paper investigates how the accumulated experience of one Kaigents agent or team can be ported to another — ideally without repeating the full compute cycle of ingestion, embedding, consolidation, and belief formation. It identifies what is structurally difficult about such knowledge propagation and evaluates approaches within the existing Kaigents architecture.

The practical driver is the "redeployable team" product concept: a pre-trained agent team packaged with domain knowledge, vectors, embeddings, and weights that an enterprise can deploy, ingest their corpora into, and start using immediately. For that product to work, we need to understand what is portable, what is not, and why.

## 1. What constitutes "experience" in Kaigents

An experienced Kaigents agent has accumulated state across three memory tiers, each with different portability characteristics.

### 1.1 Short-term memory (Qdrant vectors)

Every `memory.record` call upserts a point into a per-workspace Qdrant collection (`workspace-{workspace_id}`). Each point contains:

- An **embedding vector** (produced by the configured embedding model at ingest time)
- A **payload** with the original content text, metadata, and `run_id` provenance

This is the most compute-expensive layer to regenerate (re-embedding requires access to source content + model inference) and the most model-dependent (vectors are only meaningful relative to the model that produced them).

### 1.2 Long-term memory (RethinkDB episodes)

After each run, `consolidate_run_memory` extracts a summary episode from the short-term memories of that run. Episodes are stored in the `memory_episodes` table with:

- `summary` — a natural-language description of what was learned
- `source_content_ids` — back-links to the original Qdrant points
- `workspace_id`, `run_id`, `timestamp_ms` — provenance

Episodes are more portable than vectors because they are text, not model-specific embeddings. They are less portable than they appear because `source_content_ids` reference Qdrant points that may not exist in the target workspace.

### 1.3 Epistemic memory (RethinkDB beliefs)

The `memory_beliefs` table holds hypotheses and their outcomes:

- `content` — the hypothesis statement
- `assumptions` — IDs of hypotheses this belief depends on
- `confidence` — a numeric confidence value
- `status` — `pending`, `confirmed`, or `falsified`
- `workspace_id`, `run_id`, `timestamp_ms` — provenance

Beliefs are the most valuable and the most dangerous to transfer. They encode *what worked and what didn't* — the core of "experience." But they are also context-bound: a hypothesis that was falsified in one workspace (e.g., "this API supports pagination") may be true in another.

### 1.4 Configuration and policy

Beyond data, an experienced agent has tuned configuration:

- `MemoryPolicy` CRD — retention rules, consolidation triggers, context budget
- System prompts (refined through use)
- Tool allowlists (which MCPs the agent has proven useful)
- Context manager strategy preferences

This layer is trivially portable (it's YAML/config) but often the most valuable, because it encodes human-curated decisions about what the agent should do.

## 2. Transfer scenarios

### Scenario A: Clone for a new customer (same domain)

A retail strategy team has been operating for months in workspace `ws-retail-001`. A new customer signs up. We want to create `ws-retail-002` with the same domain knowledge but none of the first customer's proprietary data.

**What transfers**: domain episodes ("competitor pricing patterns in sustainable kitchenware"), confirmed hypotheses ("Lighthouse SEO scores correlate with conversion rate"), tuned system prompts, tool configurations.

**What doesn't transfer**: customer-specific episodes ("customer X's inventory has these gaps"), customer-specific beliefs, raw ingested vectors (they contain customer data).

### Scenario B: Cross-domain structural transfer

A software engineering agent (Sena) has learned effective patterns: "read test files before implementation files," "run the type checker after each edit." We want to port these *structural* lessons to a different engineering agent working in a different language ecosystem.

**What transfers**: structural/epistemic beliefs ("approach X is effective for problem class Y").

**What doesn't transfer**: domain-specific episodes ("this codebase uses Framework Z v3.2"), specific code embeddings.

### Scenario C: Team-level collective transfer

A team of agents (scout + analyst + writer) has developed shared working knowledge through multiple runs. We want to port the *team's collective experience* to a new team with the same roles but different members.

**What transfers**: inter-agent coordination patterns (encoded in episodes), shared beliefs about effective workflows.

**What doesn't transfer**: individual agent-specific memories that were never consolidated into episodes.

### Scenario D: Pre-trained package deployment

The "redeployable team" product: a team is pre-trained on public domain data, packaged with its full memory state, and deployed to an enterprise environment. The enterprise ingests their private corpora on top.

**What transfers**: everything — vectors, episodes, beliefs, config — as a baseline.

**What gets added**: new episodes and beliefs from the enterprise's own runs, layered on top of the pre-trained base.

## 3. What is challenging about knowledge propagation

### 3.1 Embedding model dependency

**Problem**: Qdrant vectors are only meaningful relative to the embedding model that produced them. If the target agent uses a different embedding model (e.g., a different Lemonade-served model, or a different quantization), the transferred vectors are in a different vector space. Cosine similarity between a query vector (from model B) and a stored vector (from model A) is meaningless.

**Severity**: Critical. This is the single biggest barrier to vector-level transfer.

**Mitigation options**:
- Pin the embedding model as part of the package metadata. Reject transfer if models don't match. (Simple but rigid.)
- Re-embed source content using the target model. (Requires access to original text, which Qdrant payloads do contain — so this is feasible but compute-expensive.)
- Use a model-bridging technique (e.g., Procrustes alignment) to map between vector spaces. (Research-grade, unreliable for production.)

### 3.2 Workspace-specific references

**Problem**: Episodes and beliefs carry `workspace_id`, `run_id`, and `source_content_ids` that reference entities in the source workspace. In the target workspace, these IDs either don't exist or point to different data. The `recall` function filters by `workspace_id`, so transferred episodes would be invisible unless their `workspace_id` is rewritten.

**Severity**: High. Without ID remapping, transferred knowledge is unreachable.

**Mitigation**: Rewrite `workspace_id` to the target workspace. Replace `run_id` and `source_content_ids` with null or a synthetic "imported" provenance marker. This breaks the back-link chain (you can't trace a transferred episode back to its original run) but makes the knowledge reachable.

### 3.3 Belief context-dependence

**Problem**: A hypothesis that was falsified in one context may be valid in another. "This library's API supports streaming" may be true for Library v2 and false for v1. Transferring a `falsified` belief without its context could cause the target agent to avoid a valid approach.

The current ATMS implementation has no notion of *context scope* — beliefs are workspace-scoped, not domain-scoped or context-scoped. A belief is either falsified or it isn't; there's no "falsified in context X but untested in context Y."

**Severity**: High for epistemic transfer. This is the core risk of propagating experience.

**Mitigation options**:
- Transfer beliefs as `pending` (not `confirmed` or `falsified`) so the target agent re-verifies them. Safer but loses the value of prior testing.
- Add a `context_scope` field to beliefs (e.g., "language: Python", "domain: retail") and only transfer beliefs whose scope matches the target. Requires schema change and retroactive tagging.
- Transfer beliefs with their evidence (the episodes that confirmed/falsified them) so the target agent can reason about applicability. Most robust but requires transferring the full dependency chain.

### 3.4 Temporal validity

**Problem**: Some knowledge is time-sensitive. "This API endpoint returns JSON" may be true today and false after a deprecation. Without temporal metadata (`valid_from`/`valid_to`), we can't distinguish timeless knowledge ("recursion is a valid approach for tree traversal") from time-bound knowledge ("this service is up and responding").

The deferred NebulaGraph temporal layer (ITD-18) was designed to solve this. Without it, all knowledge is implicitly "valid as of when it was recorded," which is correct for the source workspace but potentially stale for the target.

**Severity**: Medium for short-term transfer (same time period). High for long-term transfer or pre-trained packages that age.

**Mitigation**: Use `timestamp_ms` as a staleness signal. Add a `validity_duration` or `expires_at` field to episodes and beliefs. The target agent can then re-verify stale knowledge through its own runs.

### 3.5 Semantic deduplication

**Problem**: If the target agent already has independently learned similar knowledge, transferring creates duplicates. Two episodes with slightly different summaries but the same semantic content waste context budget and can create conflicting beliefs.

Deduplication requires semantic comparison (vector similarity between episode summaries), which is expensive at scale and inexact (what threshold counts as "duplicate"?).

**Severity**: Medium. Accumulates over multiple transfers.

**Mitigation**: Before transfer, embed episode summaries using the target's embedding model and check similarity against existing episodes. Merge or skip above a threshold. This is a batch operation, not a streaming one.

### 3.6 Provenance and auditability

**Problem**: Transferred knowledge needs to track its origin. If a transferred belief causes the target agent to make a bad decision, we need to know: "this belief was learned by agent X in workspace Y under conditions Z." Without provenance, debugging is impossible and trust erodes.

The current schema has `workspace_id` and `run_id` but no `origin_workspace_id` or `transfer_batch_id`. Once `workspace_id` is rewritten for the target, the origin is lost.

**Severity**: Medium for single transfer. High for chained transfers (A → B → C).

**Mitigation**: Add an `imported_from` field to episodes and beliefs. Preserve the original `workspace_id` and `run_id` in a nested `provenance` object while setting the top-level `workspace_id` to the target.

### 3.7 Scale and packaging

**Problem**: A pre-trained team package must include Qdrant collection snapshots, RethinkDB table exports, configuration files, and metadata about embedding models and schema versions. This is a multi-gigabyte artifact that needs to be versioned, distributed, and applied atomically.

Qdrant supports collection snapshots (`/collections/{collection}/snapshots`), which is the right primitive for vector-level transfer. RethinkDB has no native snapshot mechanism — we'd need to export tables to JSON and re-import.

**Severity**: Medium. Engineering work, not research.

**Mitigation**: Use Qdrant snapshots for vectors. Export RethinkDB tables to JSON dumps. Package everything in a tarball with a `manifest.json` that records schema version, embedding model, and creation date.

### 3.8 Belief dependency chain integrity

**Problem**: Beliefs form dependency chains via the `assumptions` array. If belief B depends on hypothesis A, and we transfer B without A, the target agent has a belief with a dangling dependency. The retraction cascade logic (`close_experiment` → filter on `assumptions`) would fail to cascade correctly.

**Severity**: High for partial transfers. Not an issue for full transfers.

**Mitigation**: Transfer beliefs as a connected graph, not individual records. Before transfer, compute the transitive closure of dependencies for each selected belief and include all ancestors. Alternatively, flatten dependencies: if A is `confirmed` and B depends on A, transfer B with `assumptions: []` and a note that its dependency was pre-validated.

## 4. Approaches evaluated

### 4.1 Snapshot-and-restore (full clone)

**Method**: Export the entire Qdrant collection and RethinkDB tables from the source workspace. Import into the target workspace with `workspace_id` rewritten.

**Pros**: Complete fidelity. No knowledge loss. Simple to implement (Qdrant snapshots + JSON export).

**Cons**: Transfers everything including irrelevant or stale knowledge. No deduplication. Embedding model must match. Customer-specific data leaks if not filtered.

**Best for**: Scenario D (pre-trained package deployment) where the entire baseline is meant to transfer.

### 4.2 Curated transfer (selective)

**Method**: A human or meta-agent selects specific episodes and beliefs to transfer. Only selected records are exported, with dependency chains resolved.

**Pros**: Precise control. Can filter out domain-specific or stale knowledge. Can downgrade `confirmed`/`falsified` beliefs to `pending` for re-verification.

**Cons**: Labor-intensive. Requires domain expertise to curate. Doesn't scale to large knowledge bases.

**Best for**: Scenario B (cross-domain structural transfer) where only a subset of knowledge is relevant.

### 4.3 Re-embedding bridge (model-independent transfer)

**Method**: Export episode summaries and belief content as text (not vectors). In the target workspace, re-embed the text using the target's embedding model and upsert as new Qdrant points. Episodes and beliefs are inserted into RethinkDB with rewritten IDs.

**Pros**: Model-independent. Works across different embedding models. Text is human-readable and auditable.

**Cons**: Loses the original vector fidelity (re-embedding introduces quantization differences). Compute-expensive for large knowledge bases. Loses `source_content_ids` back-links.

**Best for**: Scenario A (clone for new customer) where the embedding model may differ and only domain knowledge (not customer data) should transfer.

### 4.4 Distillation (lossy but portable)

**Method**: Use the experienced source agent to generate a "lessons learned" document — a structured summary of confirmed beliefs, falsified approaches, and effective strategies. The target agent ingests this document as a Case File, giving it the distilled experience without the raw memory.

**Pros**: Maximally portable (it's just text). Model-independent. Human-reviewable. Compact. Works across any agent architecture, not just Kaigents.

**Cons**: Lossy — the nuance of individual episodes and the confidence values of beliefs are flattened into prose. The target agent doesn't get the structured belief graph, just a narrative. Can't do retraction cascades on distilled knowledge.

**Best for**: Scenario B (cross-domain) and as a complement to other approaches. Also the most practical for the "pre-trained team" product: ship a distilled lessons document alongside the full memory snapshot, so the target agent can use either layer.

### 4.5 Layered transfer (generality-tiered)

**Method**: Tag each episode and belief with a generality level:
- **Structural** — domain-independent patterns ("read tests before implementation")
- **Domain** — domain-specific but customer-independent ("retail pricing follows cost-plus models")
- **Instance** — specific to a workspace/customer ("customer X's API returns these fields")

Transfer only `structural` and `domain` layers. Skip `instance` layers.

**Pros**: Clean separation of what should and shouldn't transfer. Automatic once tagging is in place.

**Cons**: Requires retroactive tagging of existing knowledge. The current schema has no generality field. Tagging accuracy depends on the consolidation LLM's ability to classify generality, which is unreliable without fine-tuning.

**Best for**: All scenarios. This is the ideal long-term approach but requires schema changes and a tagging pipeline.

### 4.6 Delta transfer (incremental)

**Method**: After an initial full transfer, subsequent transfers only include episodes and beliefs created or updated since the last transfer. The target agent merges the delta into its existing knowledge base.

**Pros**: Efficient for ongoing knowledge sharing between long-running agents. Avoids re-transferring unchanged knowledge.

**Cons**: Requires tracking transfer history (what was sent, when). Merge conflicts if the target has independently learned conflicting beliefs. Deduplication still needed for overlapping deltas.

**Best for**: Scenario C (team-level collective transfer) where teams share knowledge over time, and for maintaining pre-trained packages that receive updates.

## 5. What exists in Kaigents today vs. what's missing

### Already available

| Capability | Location | Transfer utility |
|---|---|---|
| Per-workspace Qdrant collections | `workspace-{id}` naming | Cloneable via Qdrant snapshot API |
| RethinkDB episodes with `workspace_id` filter | `memory_episodes` table | Exportable via `filter` query |
| RethinkDB beliefs with `assumptions` array | `memory_beliefs` table | Exportable with dependency graph |
| Consolidation produces text summaries | `consolidate_run_memory` | Summaries are model-independent and portable |
| `MemoryPolicy` CRD | K8s API | Trivially portable (YAML) |
| `recall` filters by `workspace_id` | `kaigents-memory/src/lib.rs` | Rewriting `workspace_id` makes transferred data reachable |

### Missing

| Gap | Impact | Build effort |
|---|---|---|
| No export/import tooling | Cannot transfer without manual DB operations | Medium — CLI command wrapping Qdrant snapshots + RethinkDB export |
| No embedding model metadata on collections | Can't detect model mismatch at transfer time | Small — store model ID in Qdrant payload metadata |
| No `origin_workspace_id` / provenance tracking on transferred records | Lost audit trail after transfer | Small — add field to Episode and Hypothesis structs |
| No generality tagging on episodes/beliefs | Can't distinguish structural from instance-level knowledge | Medium — requires LLM classification at consolidation time |
| No `context_scope` on beliefs | Can't scope belief validity to domains/languages | Medium — schema change + retroactive tagging |
| No temporal validity (`valid_from`/`valid_to`) | Can't detect stale knowledge | Large — requires the deferred NebulaGraph temporal layer (ITD-18) |
| No cross-workspace deduplication | Merges create duplicates | Medium — batch semantic comparison before import |
| No package manifest format | Can't version or validate transfer artifacts | Small — JSON schema for manifest with model IDs, schema version, counts |
| No delta/incremental transfer tracking | Can't do efficient ongoing sync | Medium — transfer log table + watermark tracking |

## 6. Recommended approach for the pre-trained team product

For the "redeployable team" product (Scenario D), the pragmatic path is a **hybrid of snapshot-and-restore + distillation**:

1. **Full snapshot** of the pre-trained agent's Qdrant collection and RethinkDB tables, packaged with:
   - Embedding model ID and version
   - Schema version
   - Creation timestamp
   - Episode count, belief count, vector count
   - A `manifest.json` declaring all of the above

2. **Distilled lessons document** generated by the source agent, included alongside the snapshot. This gives the deploying enterprise:
   - A human-readable summary of what the team knows
   - A fallback if the embedding model doesn't match (they can ingest the distilled document instead of restoring the snapshot)

3. **Restore pipeline** that:
   - Validates embedding model compatibility (reject or offer re-embedding)
   - Creates the target Qdrant collection from snapshot
   - Imports RethinkDB records with `workspace_id` rewritten and `imported_from` provenance set
   - Marks transferred beliefs as `pending` (re-verification) or preserves status based on a policy flag

4. **Post-restore warm-up**: the target agent runs a verification pass — it re-checks a sample of transferred beliefs against its own environment and flags any that fail. This catches context-dependence issues without requiring a full re-test of every belief.

This approach is buildable with the current architecture plus the export/import CLI and package manifest. It doesn't require the generality tagging or temporal validity layers, which can be added incrementally.

## 7. Open questions for future research

1. **Can consolidation be tuned for transfer-readiness?** If the consolidation LLM is prompted to classify each episode as structural/domain/instance during consolidation, the generality tag comes for free — but does this degrade consolidation quality?

2. **What is the half-life of agent knowledge?** Empirically, how long do beliefs remain valid before they need re-verification? This varies by domain (software APIs decay faster than mathematical truths) and determines the staleness policy.

3. **Can belief dependency chains be compressed for transfer?** Instead of transferring the full graph, can we compute a "closure certificate" — a summary of which assumptions are confirmed and which are falsified — that preserves cascade semantics without the full graph?

4. **Is cross-model vector alignment feasible at our scale?** For collections under 100K points, Procrustes alignment between two embedding models' vector spaces might be accurate enough to avoid re-embedding. This would make snapshot transfer work across model changes.

5. **How does team-level knowledge differ from agent-level knowledge?** Do teams develop collective beliefs that no individual agent holds, or is team knowledge always a union of individual knowledge? This determines whether team transfer is just N individual transfers or requires a separate abstraction.

## 8. References

- Kaigents memory architecture: `docs/architecture/agent-memory-proposal.md`
- Memory research basis: `docs/research/agent-context-and-memory-research.md`
- Implementation tracker (Milestones 9-11): `docs/implementation/kaigents-implementation-tracker.md`
- ITD-18 (NebulaGraph temporal layer): `docs/research/technology/itd-register.md`
- Qdrant collection snapshots: https://qdrant.tech/documentation/concepts/snapshots/
- RethinkDB data export: https://rethinkdb.com/docs/dump-restore/
- Procrustes alignment for cross-model vector spaces: https://arxiv.org/abs/2104.08555
- Belief revision (AGM theory): Alchourrón, Gärdenfors, Makinson (1985)
- Assumption-based TMS (de Kleer): https://dekleer.org/Publications/An%20Assumption-Based%20TMS.pdf
