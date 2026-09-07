---
okf_version: "0.2"
type: data-model
status: planned
sources:
  - crates/exodus-graph/src/lib.rs
  - crates/exodus-core/src/lib.rs
---

# Semantic Graph Specification

[Index](index.md) | [Graph Algorithms](graph-algorithms.md) | [System Architecture](system-architecture.md)

The **Exodus Semantic Graph (ESG)** is a language-neutral graph capturing symbol definitions, relationships, and invariants.

## Node Schema
* `id`: Unique global symbol identifier (e.g. `python::app.services.user::UserService.get_by_id`).
* `name`: Identifier name.
* `kind`: Symbol category (`module`, `class`, `function`, `struct`, `interface`, `variable`).
* `file_path`: Source file relative path.
* `language`: Source language enum.
* `complexity_score`: Calculated cyclomatic and semantic complexity.

## Edge Schema
* `from`: Source node ID.
* `to`: Target node ID.
* `relationship`:
  * `Calls`: Direct function/method invocation.
  * `Imports`: Module dependency.
  * `Inherits`: Class inheritance.
  * `Implements`: Interface / trait implementation.
  * `TypeDependency`: Parameter or return type reference.

## Distinction from Documentation Graph
The ESG models strictly program semantics, whereas OpenWiki models documentation and architectural concepts. The two graphs are independent.
