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
* **ADR-007: Embedded SurrealDB Living Memory**: High-performance in-process ACID persistence for operational items and audit trails.
* **ADR-008: Non-Destructive Git Worktree Sandboxing**: Ephemeral isolated worktrees preventing dirty repository states.
* **ADR-009: Maker-Defined Policy Ownership**: Makers, not the engine, define guard policies, AI steps, and rules in `.exodus/maker_plugins.json`.
* **ADR-010: Configurable Board & Inline Ticket Creation**: Interactive multi-stage flow with zero-dialog inline creation and WIP limits.
* **ADR-011: Goal vs. Execution Failure Diagnostics**: Grounded root cause breakdown and 1-click prompt refinement for failed units.
