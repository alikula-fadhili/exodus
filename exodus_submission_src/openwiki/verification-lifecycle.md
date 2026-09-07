---
okf_version: "0.2"
type: workflow
status: implemented
sources:
  - crates/exodus-verifier/src/lib.rs
  - crates/exodus-verifier/src/unit_gate.rs
  - crates/exodus-core/src/lib.rs
---

# Verification Lifecycle

[Index](index.md) | [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md) | [Human Approval Model](human-approval-model.md) | [Evaluation Methodology](evaluation-methodology.md)

Verification ensures generated code matches behavioral specifications through a multi-tier gate.
This page covers the whole-module/repository verification path (`exodus verify`, genuinely wired to
`rustfmt`/`cargo check`/`cargo test`); see
[Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md) for the smaller,
per-unit boundary this now also runs at (`exodus units verify`, `exodus migrate --gated`) — the two
are distinct tiers, not the same check at different granularity, and a repository is never labeled
verified merely because unit contracts passed.

## Stages
1. **Format Check**: `cargo fmt --check` ensures valid, readable syntax.
2. **Compilation Check**: `cargo check --workspace` verifies type safety, borrow checker compliance, and linkage.
3. **Unit Tests**: Synthesized and migrated unit tests executed via `cargo test`.
4. **Behavioral Equivalence**: Compare input/output behavior against recorded legacy test fixtures.

## Outcome State Machine
* **Passes All Stages** -> `Verified`
* **Compiles Cleanly, Tests Pending** -> `Compatible`
* **Contains Fallback Stubs, or Compiles With No Grounded Contract To Check** -> `Degraded`
* **Fails Compilation, or a Grounded Assertion Fails, After the Repair Loop** -> `Blocked`

## Bounded repair loop wiring

`exodus_verifier::Verifier::verify_and_repair` wires `BoundedAgent`/`MockAgentProvider` into the
whole-crate path; `exodus_verifier::unit_gate::run_unit_gate` wires the same bounded-repair pattern
into the per-unit path. Both are real, invoked, and bounded (default 3 attempts —
`AgentBounds::default`) — but since only a deterministic `MockAgentProvider` exists (no real LLM is
wired in this environment), a repair attempt is tracked and counted honestly, not silently assumed
to fix a genuine semantic defect it cannot actually understand.
