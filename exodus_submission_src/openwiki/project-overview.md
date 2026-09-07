---
okf_version: "0.2"
type: overview
status: planned
sources:
  - README.md
  - Cargo.toml
  - openwiki/INSTRUCTIONS.md
---

# Project Overview

[Index](index.md) | [Architecture](system-architecture.md) | [ADRs](architectural-decisions.md)

Project Exodus is a graph-guided, agent-assisted legacy code migration engine. It addresses the high failure rate of large-scale legacy modernization projects by combining deterministic semantic graph analysis with bounded, agentic translation.

## Core Principle

> Reliable migration means maximizing verified behavior while turning unresolved semantics into explicit, measurable and reviewable migration debt.

## Ten-Stage Pipeline

1. **Parse**: Parse legacy source files using tree-sitter.
2. **ESG Construction**: Construct the language-neutral Exodus Semantic Graph.
3. **Analysis**: Analyze dependencies, call graphs, cycles, and risk scores.
4. **Plan Generation**: Compute topological migration waves.
5. **Human Approval**: Require explicit review and signoff before risky actions.
6. **Transform**: Translate supported constructs into the target language.
7. **Explicit Fallbacks**: Emit reviewable stubs and migration debt records for unmapped semantics.
8. **Verification**: Format, compile, and execute behavioral test suites.
9. **Bounded Repair**: Run constrained, deterministic repair loops on compiler/test errors.
10. **Report**: Output separated classifications: `Verified`, `Compatible`, `Degraded`, and `Blocked`.
