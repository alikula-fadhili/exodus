---
okf_version: "0.2"
type: component
status: planned
sources:
  - crates/exodus-planner/src/lib.rs
  - crates/exodus-graph/src/lib.rs
---

# Migration Planner

[Index](index.md) | [Human Approval Model](human-approval-model.md) | [Graph Algorithms](graph-algorithms.md)

The migration planner (`exodus-planner`) groups symbols into discrete execution waves.

## Wave Structure
* **Wave 0 (Foundation)**: Primitive types, enums, pure data structures, leaf utility functions.
* **Wave 1 (Domain Logic)**: Pure domain entities, business logic classes with resolved leaf dependencies.
* **Wave 2 (Service Layer)**: Orchestration services, repository implementations.
* **Wave 3 (API / Boundary)**: Entrypoints, controllers, network endpoints, CLI handlers.

## Approval Invariants
Any migration step with a high risk score (> 70) or destructive impact requires a human approval checkpoint before proceeding.
