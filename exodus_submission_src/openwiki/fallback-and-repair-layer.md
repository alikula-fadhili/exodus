---
okf_version: "0.2"
type: component
status: planned
sources:
  - crates/exodus-fallback/src/lib.rs
  - crates/exodus-core/src/lib.rs
---

# Fallback and Repair Layer

[Index](index.md) | [Transformation Engine](transformation-engine.md) | [Verification Lifecycle](verification-lifecycle.md)

When automated or agentic transformation cannot resolve complex or unsupported semantics, `exodus-fallback` activates.

## Principles
1. **Explicit Debt**: Never silently omit unsupported code. Insert `todo!("Exodus Migration Debt: ...")` stubs.
2. **Reviewable Annotations**: Attach attributes/comments indicating origin file, symbol name, and unresolved dependency.
3. **Outcome Demotion**: Downgrade the symbol outcome classification to `Degraded` and log entry in `.exodus/debt.json`.
4. **Compile-Ready Interfaces**: Ensure surrounding signatures compile so dependent waves are unblocked.
