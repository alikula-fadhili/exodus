# RFC-001: The Enterprise Agentic Execution Fabric

## Executive Thesis: Bridging Software Execution and Business Intent

In enterprise environments, engineering, customer operations, and product strategy are frequently treated as disconnected silos with incompatible tooling:

* **Software engineering** prioritizes runtime stability, compiler correctness, and continuous delivery.
* **Commercial operations** prioritizes customer service-level agreements (SLAs), revenue integrity, and customer relationship management (CRM) hygiene.
* **Product teams** prioritize market feedback, customer satisfaction (CSAT) harmonization, and feature demand signals.

When evaluated through an architectural lens, these seemingly disparate tasks share an identical underlying bottleneck: **the semantic gap between business requirements and execution mechanics**. Whether modifying an abstract syntax tree (AST) to resolve an unhandled null exception, mutating a customer tier in an enterprise CRM under strict discount governance, or reclassifying hundreds of free-form survey responses into a unified ontology, each workflow requires:

1. Translating unstructured business intent into a structured semantic model.
2. Executing changes inside an isolated sandbox with zero risk to the operational system.
3. Enforcing deterministic, non-negotiable verification constraints before deployment.
4. Securing explicit human authorization through transparent diffs and impact analysis.
5. Capturing the resulting resolution into reusable organizational memory to prevent repeated failure.

Project Exodus is therefore not a collection of fragmented automation bots. It is a **Unified Agentic Execution Fabric**: a single, repeatable operating plane embedded directly within the enterprise software ecosystem that coordinates complex, cross-functional workflows with mathematical rigor and continuous organizational learning.

---

## The Universal Governance Framework: The 6-Stage Closed Loop

Every operational event entering the system—regardless of whether it originates as a broken build, a sales escalation, or a raw survey batch—is processed through an identical six-stage agentic state machine:

```
Event Ingestion (#tags)
  -> Context Materialization (SurrealDB Graph)
  -> Sandboxed Execution (Worktree / Shadow State)
  -> Contract Verification (Unit Harness / Policy Rules)
  -> Governed HITL Gate (Review UI / Diff Inspection)
  -> Knowledge Promotion (Case Engine / Case Fixtures)
```

| Pipeline Stage | Architectural Function | Engineering Domain (`#prod-bug`) | Commercial Domain (`#crm-request`) | Product Domain (`#survey-mapping`) |
| --- | --- | --- | --- | --- |
| **1. Intent Ingestion** | Captures incoming operational payload and assigns domain routing tags. | Ingests CI failure logs, stack traces, and commit IDs. | Ingests contract change, discount request, or tier upgrade tickets. | Ingests raw CSV/JSON exports from survey platforms. |
| **2. Graph Contextualization** | Maps the event into embedded SurrealDB to establish upstream and downstream relations. | Traces error lines to AST symbol nodes and dependent caller files. | Links request to account nodes, ARR tiers, and contract compliance policies. | Links survey items to canonical product taxonomy vertices. |
| **3. Sandboxed Execution** | Spawns an isolated execution boundary to prevent production state corruption. | Creates an isolated Git worktree (`.exodus/worktrees/<id>`) on a dedicated branch. | Spawns a staged transaction workspace simulating CRM state mutations. | Generates candidate schema alignment mappings in a shadow partition. |
| **4. Contract Verification** | Runs deterministic validation rules; the agent cannot declare success on opinion alone. | Runs unit-level test harness (`cargo test`, `kotlinc`) to verify localized fix. | Executes declarative business rules (e.g., max discount threshold, VP approval rules). | Verifies bidirectional coverage, foreign key consistency, and zero unmapped keys. |
| **5. Governed HITL Gate** | Halts automated execution and presents an interactive diff for human sign-off. | Visual side-by-side Git diff with test pass logs in Web UI; one-click merge. | Proposed CRM field diffs and compliance impact summary in Web UI; one-click sign-off. | Interactive visual taxonomy mapping matrix in Web UI; one-click schema sync. |
| **6. Knowledge Promotion** | Converts the verified resolution into a permanent, reusable case (`CASE-XXXX`). | Stores failing subgraph and fix strategy in SurrealDB; creates regression test. | Records discount authorization precedent and account SLA exception rules. | Adds validated survey terminology mappings to the persistent ontology graph. |

---

## Cross-Functional Domain Realization

### 1. Engineering Operations (`#prod-bug`): Preserving Software Health

* **The Business Need:** Production downtime, broken CI/CD pipelines, and regressive compiler errors directly consume costly engineering capacity and delay sprint roadmaps.
* **The Systematic Agent Process:**
  * Rather than relying on developers to manually parse multi-thousand-line compiler logs, the system isolates the failing symbol boundary in the Exodus Semantic Graph (ESG).
  * It checks out the failing commit in an isolated Git worktree, applies historical repair patterns retrieved from SurrealDB (e.g., null checking, error re-wrapping), and executes isolated unit-level contract tests.
  * Upon passing local verification, it generates a production-ready Pull Request complete with root cause analysis and test pass telemetry, transforming hours of manual triage into a 3-minute human review.

### 2. Commercial Operations (`#crm-request`): Safe Business Execution

* **The Business Need:** Frontline sales and support teams routinely handle requests that require mutations across operational databases and CRMs (e.g., custom payment terms, enterprise license additions, SLA adjustments). Unchecked direct modifications risk billing errors, contract breaches, and compliance audits.
* **The Systematic Agent Process:**
  * Inbound requests are treated as structured operational transactions rather than loose conversational prompts.
  * The agent evaluates the request against company policies stored in SurrealDB, checking customer entitlement, contract bounds, and authorized discount schedules.
  * It stages the proposed mutations in an isolated transaction layer and presents an auditable impact preview to finance or sales operations in the Web UI. Once approved, it commits the changes across CRM APIs, maintaining an immutable audit log.

### 3. Product & Market Operations (`#survey-mapping`): Unifying Voice of Customer

* **The Business Need:** Product teams collect thousands of survey responses across disconnected tools (Typeform, Qualtrics, NPS trackers, CSAT widgets). Manual data cleaning is slow, subjective, and prone to losing critical user sentiment regarding platform bugs or feature requests.
* **The Systematic Agent Process:**
  * The agent breaks unstructured feedback into atomic semantic statements and aligns them against the central product taxonomy graph in SurrealDB.
  * If customer responses reference a specific platform failure or latency concern, the agent can cross-link the survey feedback directly to relevant symbols in the engineering codebase graph.
  * Product managers inspect and approve the mapping clusters in the Web UI, generating immediate, structured backlog intelligence backed by verified customer data.

---

## Compounding Value: The Organizational Immunology Model

The core architectural breakthrough of this unified platform is that **learning is compound and multi-directional**:

* **Cross-Pollination of Insights:** A customer survey entry (`#survey-mapping`) complaining about an authentication timeout can be correlated by the SurrealDB graph directly with an active CI failure or bug report (`#prod-bug`) on the auth service module.
* **Reusable Solution Repository:** When an edge-case configuration bug is solved and approved under `#prod-bug`, the underlying structural lesson is stored in SurrealDB as a verified case (`CASE-XXXX`). Future pipeline failures across different services can query this knowledge base to resolve identical structural issues instantaneously.
* **Guaranteed Non-Regression:** Every promoted case automatically generates a persistent fixture in `.exodus/knowledge/cases/`. Running `exodus cases test` continuously proves that past lessons remain intact across software updates, turning ad-hoc firefighting into an expanding organizational test suite.

---

## Architectural Mandate: Unified Business-to-Execution Control Plane

1. **Unified Event Model:**
   * Every task (engineering bug, CRM request, or survey mapping) is an `OperationalItem` moving through an identical, auditable lifecycle:
     `Captured -> Sandboxed -> Contract-Verified -> Human-Approved -> Promoted`.

2. **Deterministic Verification Over Heuristics:**
   * For `#prod-bug`: Verification is enforced via isolated unit compilation and test execution inside Git worktrees.
   * For `#crm-request`: Verification is enforced via policy graphs and schema constraints in SurrealDB.
   * For `#survey-mapping`: Verification is enforced via taxonomy graph completeness and foreign key integrity.

3. **Cross-Functional Web UI:**
   * The embedded UI (Axum + static web assets) serves as the universal Human-in-the-Loop (HITL) gate.
   * It provides side-by-side Git diffs for engineering, policy-impact matrices for commercial operations, and visual taxonomy alignment for product teams.
   * Approval actions must explicitly promote verified resolutions into SurrealDB to expand the system's organizational memory.

4. **Alignment with Existing Infrastructure:**
   * Zero external server dependencies: Embedded SurrealDB (SurrealKV) runs entirely local to the project.
   * Works natively with standard Git repositories, standard toolchains (Cargo, JDK), and standard browser environments.

---

## Strategic Hackathon Positioning

* **For Rubric Criterion 1 (Problem & User Value — 15 pts):** Solves an organizational problem that causes friction across engineering, product, and operations teams daily.
* **For Rubric Criterion 2 (Agent Engineering — 30 pts):** Demonstrates a multi-domain agentic architecture featuring sandboxed Git worktrees, AST traversal, embedded multi-model graph persistence, and strict verification loops.
* **For Rubric Criterion 6 (Architectural Lessons — 5 pts):** Provides a compelling counter-narrative: *AI agents achieve production reliability not by expanding conversational prompt windows, but by operating within a strictly governed, cross-functional execution harness backed by deterministic verification and compounding memory*.
