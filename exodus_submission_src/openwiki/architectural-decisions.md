---
okf_version: "0.2"
type: decision
status: planned
sources:
  - Cargo.toml
  - openwiki/INSTRUCTIONS.md
---

# Architectural Decisions

[Index](index.md) | [System Architecture](system-architecture.md) | [Project Overview](project-overview.md)

Key Architectural Decision Records (ADRs) established for Project Exodus:

* **ADR-001: Rust as Core Implementation Language**: Chosen for memory safety, performance, zero-cost abstractions, and robust compiler diagnostics.
* **ADR-002: MVP Demonstration Lane**: Python 3.11 source to Rust 2021 target.
* **ADR-003: Source Parsing with Tree-Sitter**: Fast, error-tolerant incremental parsing with concrete syntax trees.
* **ADR-004: In-Memory Owned Semantic Graph**: High-speed traversal and deterministic topological ordering.
* **ADR-005: Portable JSON Artifact Storage**: `.exodus/` stores inspectable and machine-readable evidence files.
* **ADR-006: OpenWiki with OKF v0.2**: Separation of program semantics (ESG) from documentation knowledge graph.
