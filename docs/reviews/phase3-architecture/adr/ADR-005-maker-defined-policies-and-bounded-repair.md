---
okf_version: "0.2"
type: decision
status: approved
sources:
  - crates/exodus-kernel/src/lib.rs
  - crates/exodus-agent/src/lib.rs
  - crates/exodus-verifier/src/domain_verifiers.rs
tags:
  - adr
  - policy
  - maker-defined
  - bounded-repair
  - okf
---

# ADR-005: Maker-Defined Policy Ownership & Bounded Agent Repair

[Index](../../../../../openwiki/index.md) | [Decisions](../decisions.md) | [System Architecture](../../../../../openwiki/system-architecture.md)

## Context
A major anti-pattern in automated migration and agentic engineering tools is hardcoding rigid architectural policies, linter rules, and workflow restrictions into the core engine. This destroys developer autonomy and prevents engineering teams ("makers") from defining organization-specific rules, security constraints, and custom AI behavior. Furthermore, unrestrained LLM repair loops can spiral into unbounded iterations and hallucinations.

## Decision
We established two complementary invariants:
1. **Makers Define Policies (`CustomMakerPlugin`)**:
   - The engine provides plugin hooks (`PolicyGuard`, `AiStep`, `Theme`, `AppBehavior`), but does **not** hardcode inflexible business or architectural policies.
   - Makers define their own policies, rule directives, and guard checks in `.exodus/maker_plugins.json`.
   - The UI and kernel dynamically inspect, evaluate, and enforce these maker-defined rules before and during verification.
2. **Strictly Bounded Repair ($\le 3$ iterations)**:
   - Automated repair loops within `exodus-agent` must not exceed 3 attempts on a single symbol.
   - If unresolved after 3 attempts, the agent emits an explicit `todo!` fallback stub, records structured `MigrationDebt`, and generates a `UnitPromptDiagnostic` explaining the exact root cause and suggested prompt refinement to the human operator.

## Consequences
- **Positive**: Complete developer and organizational autonomy; teams craft guardrails tailored to their domain.
- **Positive**: Prevents infinite agent token burns and ensures bounded execution latency.
- **Negative**: Requires makers to maintain their plugin configuration for complex organizational rules.
