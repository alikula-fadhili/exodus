---
okf_version: "0.2"
type: decision
status: approved
sources:
  - crates/exodus-core/src/lib.rs
  - crates/exodus-graph/src/lib.rs
tags:
  - adr
  - esg
  - language-neutral
  - okf
---

# ADR-001: Exodus Semantic Graph (ESG) Model Expressiveness for Multi-Language Targets

[Index](../../../../../openwiki/index.md) | [Decisions](../decisions.md) | [System Architecture](../../../../../openwiki/system-architecture.md)

## Context
Phase 1 migration targeted Python 3.11 $\to$ Rust 2021. To enable multi-language modernization (TypeScript $\to$ Go, Java $\to$ Rust, etc.), the core semantic representation must decouple concrete AST syntax from semantic dependency relationships without losing behavioral contracts or structural typing.

## Decision
We establish the **Exodus Semantic Graph (ESG)** as a strictly language-neutral directed multigraph in `exodus-graph`:
1. Nodes represent canonical semantic constructs: `Function`, `Method`, `Class`, `Interface`, `TypeAlias`, `Module`, `Global`.
2. Edges represent semantic dependencies: `Calls`, `Instantiates`, `Implements`, `Extends`, `TypeReference`, `Imports`.
3. Graph algorithms (Tarjan strongly connected components, Feedback Arc Set cycle breaking, topological wave grouping) operate exclusively on symbol-agnostic structural graphs.
4. Language-specific details reside inside `exodus-parser` Tree-sitter adapters and `exodus-transform`.

## Consequences
- **Positive**: Adding a new source or target language requires only a parser adapter and codegen emitter; the graph engine, cycle breaker, and wave planner remain completely unchanged.
- **Positive**: Validated by TypeScript-to-Go integration suite passing 4/4 tests without modifications to `exodus-graph`.
- **Negative**: Language-specific constructs (e.g. Go goroutines, Rust lifetimes, Python decorators) must be mapped to normalized semantic node metadata.
