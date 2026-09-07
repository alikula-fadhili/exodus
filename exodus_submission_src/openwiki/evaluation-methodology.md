---
okf_version: "0.2"
type: evaluation
status: implemented
sources:
  - crates/exodus-eval/src/lib.rs
  - crates/exodus-core/src/lib.rs
  - docs/reviews/unit-verification-audit.md
---

# Evaluation Methodology

[Index](index.md) | [Verification Lifecycle](verification-lifecycle.md) | [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md) | [Artifact Schemas](artifact-schemas.md)

`exodus eval` (`Evaluator::run_full_benchmark_suite`) reports two tiers **separately**, never
blended into one success-rate number:

## Whole-fixture transform tier (`EvaluationSummary`)

For every fixture directory, both Exodus's real transform output and a real baseline are
independently scaffolded and compiled via the same `Verifier::run_cargo_check` — nothing here is
guessed by fixture name.

* **Exodus side**: `exodus_compiled` (real `cargo check`), `exodus_outcome` (the transform stage's
  aggregate `MigrationOutcome`), `exodus_tests_passed` (compiled *and* outcome is
  `Verified`/`Compatible`).
* **Baseline side**: a real, executed, non-graph-guided "naive" translator
  (`Evaluator::naive_baseline_transform`) — one `todo!()`-stub function per source function, every
  parameter/return type-erased to `serde_json::Value`, no dependency ordering, and **no class
  support at all** (classes are silently dropped, a genuine capability gap, not simulated).
  `baseline_compiled` is measured from a real `cargo check` on that output.
  `baseline_tests_passed` is always `false` — not guessed, but a principled derivation: every
  baseline function body is an unconditional `todo!()`, which panics on any call, so it cannot pass
  a real behavioral test by construction (`test_naive_baseline_never_reports_a_fabricated_pass` in
  `crates/exodus-eval/src/lib.rs`).

Previously (before this pass), `(baseline_compiled, baseline_tests_passed)` were a hardcoded
`match` on the fixture directory *name string* — no baseline was ever executed, and
`exodus_tests_passed` was inferred purely from the transform-stage enum, never from actually
running `cargo test`. Both are now real, measured values.

## Unit-level tier (`UnitLevelSummary`, `Evaluator::run_unit_level_evaluation`)

Runs the real per-unit gate (see
[Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md)) against every
fixture that ships a grounded `contracts.json`, entirely separate from the transform tier above:

* `units_verified` / `units_compatible` / `units_degraded` / `units_blocked`
* `unit_contract_pass_rate_pct` — real assertions passed / real assertions total
* `grounded_oracle_coverage_pct` — the share of evaluated units that had *any* grounded contract to
  check against at all (an ungrounded unit is `Degraded` by the gate's own outcome rule)
* `repair_attempts`, `cases_captured`

As of this pass: 5 of 11 fixtures are grounded (8 verification boundaries), of which 6 units
genuinely verify and 3 are genuinely `Blocked` on real, currently-open transform-engine defects
(see [Known Limitations](known-limitations.md)) — not a 100% pass rate, reported honestly.

## Metrics this page previously claimed but that did not exist in code

"Verification Rate"/"Compatibility Rate"/"Degradation Ratio"/"Blocker Ratio"/"Bounded Repair
Efficiency" as named here were aspirational — no code computed a ratio against "total transformed
symbols" this way. The actual computed fields are the ones listed above; use those names when
reading `.exodus/evaluation_scorecard.json`.
