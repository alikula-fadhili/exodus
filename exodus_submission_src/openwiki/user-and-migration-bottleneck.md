---
okf_version: "0.2"
type: workflow
status: planned
sources:
  - openwiki/INSTRUCTIONS.md
  - openwiki/project-overview.md
---

# User and Migration Bottleneck

[Index](index.md) | [Human Approval Model](human-approval-model.md) | [Migration Planner](migration-planner.md)

## The Problem
Manual code modernization is slow, error-prone, and cost-prohibitive. Conversely, naive LLM migrations fail due to hallucination, hidden semantic drift, circular dependencies, and lack of behavioral verification.

## Bottleneck Mitigations
1. **Topological Ordering**: Migration proceeds from foundational leaves upward to minimize cascading compilation breaks.
2. **Deterministic Pre-checks**: Dependency cycles and unsupported dynamic patterns are flagged before transformation.
3. **Structured Review**: The user inspects risk scores and plan waves in `.exodus/plan.json` before applying code modifications.
4. **Exodus Debt Logging**: Instead of guessing dynamic runtime behavior, Exodus inserts explicit `todo!` stubs and creates measurable debt logs.
