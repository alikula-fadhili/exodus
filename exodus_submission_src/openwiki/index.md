---
okf_version: "0.2"
type: overview
status: draft
sources:
  - openwiki/INSTRUCTIONS.md
  - Cargo.toml
---

# Project Exodus Knowledge Base

Welcome to the Project Exodus Knowledge Base, structured in accordance with the **Open Knowledge Format v0.2 (OKF v0.2)**.

Project Exodus is a graph-guided, agent-assisted legacy code migration engine designed to systematically migrate legacy codebases into modern languages with verifiable behavioral parity.

## Core Knowledge Structure

### 1. Fundamentals & Architecture
* [Project Overview](project-overview.md) — Mission, core principles, and migration philosophy.
* [User and Migration Bottleneck](user-and-migration-bottleneck.md) — Analysis of legacy modernization friction and human workflows.
* [System Architecture](system-architecture.md) — End-to-end multi-stage pipeline architecture.
* [Architectural Decisions](architectural-decisions.md) — Foundational design choices and ADRs.

### 2. Graph & Program Analysis
* [Semantic Graph Specification](semantic-graph-specification.md) — Schema and semantics of the Exodus Semantic Graph (ESG).
* [Parsing Architecture](parsing-architecture.md) — Tree-sitter AST extraction and symbol table ingestion.
* [Graph Algorithms](graph-algorithms.md) — Dependency ordering, cycle breaking, and risk weighting.

### 3. Planning & Agent Boundaries
* [Migration Planner](migration-planner.md) — Wave-based migration scheduling and topological ordering.
* [Agent Boundaries](agent-boundaries.md) — Bounded execution, strict token limits, and LLM sandboxing.
* [Human Approval Model](human-approval-model.md) — Checkpoints, diff inspection, and authority invariants.

### 4. Transformation & Verification
* [Transformation Engine](transformation-engine.md) — Deterministic mapping and neural transformation.
* [Fallback and Repair Layer](fallback-and-repair-layer.md) — Graceful degradation, explicit stubs, and migration debt logging.
* [Verification Lifecycle](verification-lifecycle.md) — Multi-tier verification: formatting, compilation, and behavior validation.
* [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md) — Unit boundaries, contract provenance/oracle hierarchy, the per-unit gate, verification hierarchy, worktree commit behavior, and Case Engine integration.

### 5. Standards, Operations & Reference
* [Artifact Schemas](artifact-schemas.md) — JSON schemas for execution records in `.exodus/`.
* [CLI Reference](cli-reference.md) — Command-line interface and subcommand documentation.
* [Evaluation Methodology](evaluation-methodology.md) — Empirical scoring and outcome classification.
* [Reproduction Guide](reproduction-guide.md) — Pinned toolchains and build reproducibility instructions.
* [Security Boundaries](security-boundaries.md) — Code execution sandboxing, secrets isolation, and privacy.
* [Known Limitations](known-limitations.md) — Explicitly documented unsupported constructs and migration bottlenecks.
* [Glossary](glossary.md) — Standard taxonomy and domain terms.
