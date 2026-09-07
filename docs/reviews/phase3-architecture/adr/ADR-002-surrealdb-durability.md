---
okf_version: "0.2"
type: decision
status: approved
sources:
  - crates/exodus-store/src/lib.rs
  - crates/exodus-store/src/operational_store.rs
tags:
  - adr
  - persistence
  - surrealdb
  - okf
---

# ADR-002: Embedded SurrealDB Storage & Durability Guarantees

[Index](../../../../../openwiki/index.md) | [Decisions](../decisions.md) | [System Architecture](../../../../../openwiki/system-architecture.md)

## Context
Mission control and closed-loop agentic governance require high-performance, embedded persistence without external database daemon dependencies (such as PostgreSQL or Docker). The storage must support complex operational items, commercial policy rule limits, product taxonomies, and audit ledgers while surviving unexpected process termination.

## Decision
We adopted **Embedded SurrealDB** backed by atomic file-based persistence:
1. `EmbeddedOperationalStore` manages in-memory graph queries and SurrealKV file engine persistence (`.exodus/fabric_store.json`).
2. Every state mutation (`Captured` $\to$ `Sandboxed` $\to$ `ContractVerified` $\to$ `HumanApproved` $\to$ `Promoted`) commits through thread-safe `Arc<Mutex<EmbeddedOperationalStore>>` with tamper-evident audit records.
3. Commercial discount evaluation, role limits, and taxonomy similarity indexes execute in-process with zero network overhead.

## Consequences
- **Positive**: Zero external dependencies; single executable binary distribution (`exodus-cli` and `exodus-desktop`).
- **Positive**: ACID transactional consistency with automatic file sync.
- **Negative**: Embedded storage requires explicit disk path management in multi-process test scenarios.
