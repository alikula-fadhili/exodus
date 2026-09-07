---
okf_version: "0.2"
type: component
status: planned
sources:
  - crates/exodus-graph/src/lib.rs
  - openwiki/semantic-graph-specification.md
---

# Graph Algorithms

[Index](index.md) | [Semantic Graph Specification](semantic-graph-specification.md) | [Migration Planner](migration-planner.md)

`exodus-graph` implements deterministic graph algorithms:

## 1. Cycle Detection & Breaking
* **Tarjan's SCC Algorithm**: Identify strongly connected components.
* **Feedback Arc Set**: Compute minimal edge cuts to break mutual recursion or circular imports, converting cycle edges into dynamic interface boundaries or trait pointers.

## 2. Topological Sorting
* **Kahn's Algorithm**: Order DAG components into sequential migration layers (leaves first).

## 3. Risk Weighting
* **Composite Risk Metric**: Combines fan-in, fan-out, cyclomatic complexity, dynamic reflection usage, and external library surface area into a 1-100 risk score per symbol.
