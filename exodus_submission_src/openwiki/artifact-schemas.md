---
okf_version: "0.2"
type: reference
status: planned
sources:
  - .exodus/
  - crates/exodus-core/src/lib.rs
---

# Artifact Schemas

[Index](index.md) | [CLI Reference](cli-reference.md) | [Reproduction Guide](reproduction-guide.md)

Project Exodus records execution evidence and state under `.exodus/`:

## Standard Artifacts
* `.exodus/graph.json`: Serialized ESG with nodes, edges, and risk metadata.
* `.exodus/plan.json`: Topological waves, step dependencies, and approval states.
* `.exodus/debt.json`: List of recorded `MigrationDebt` items with symbol IDs and reasons.
* `.exodus/report.json`: Aggregate metrics (`Verified`, `Compatible`, `Degraded`, `Blocked`).
