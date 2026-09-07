---
okf_version: "0.2"
type: invariant
status: planned
sources:
  - crates/exodus-agent/src/lib.rs
  - openwiki/human-approval-model.md
---

# Agent Boundaries

[Index](index.md) | [Transformation Engine](transformation-engine.md) | [Fallback and Repair Layer](fallback-and-repair-layer.md)

Agentic assistants operate within strict, bounded parameters to prevent runaway token expenditure or uncontrolled modifications:

## Invariants
1. **Bounded Iterations**: Max 3 repair attempts per failing symbol before falling back to `Degraded` status.
2. **Context Sandboxing**: Prompts receive strictly bounded AST slices, symbol definitions, and compiler error snippets.
3. **Structured JSON I/O**: Agents communicate via typed schemas, not unstructured free-form text.
4. **Deterministic Gatekeepers**: LLM proposals must pass through rustfmt and rustc validation before acceptance.
