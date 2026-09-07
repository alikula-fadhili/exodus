---
okf_version: "0.2"
type: component
status: planned
sources:
  - crates/exodus-transform/src/lib.rs
  - crates/exodus-core/src/lib.rs
---

# Transformation Engine

[Index](index.md) | [Fallback and Repair Layer](fallback-and-repair-layer.md) | [Verification Lifecycle](verification-lifecycle.md)

`exodus-transform` translates verified AST nodes into idiomatically sound target language code.

## Transformation Pipeline
1. **Syntax Mapping**: Direct translation of arithmetic, control structures, and primitive data mappings.
2. **Type Idiomatization**: Mapping dynamically typed collections into strongly typed Rust structs, enums, `Option<T>`, and `Result<T, E>`.
3. **Ownership Modeling**: Identifying shared vs owned lifetimes and inserting appropriate ownership semantics (`Clone`, `Rc`, `Arc`, references).
4. **Agent-Guided Refinement**: For complex idiomatic translations, invoke bounded agent transforms with targeted context.
