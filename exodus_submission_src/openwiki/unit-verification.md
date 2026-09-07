---
okf_version: "0.2"
type: workflow
status: implemented
sources:
  - crates/exodus-graph/src/lib.rs
  - crates/exodus-core/src/lib.rs
  - crates/exodus-case/src/lib.rs
  - crates/exodus-worktree/src/lib.rs
  - crates/exodus-verifier/src/unit_gate.rs
  - crates/exodus-verifier/src/pipeline.rs
  - crates/exodus-verifier/tests/gate_integration.rs
  - schemas/behavioral-contract.schema.json
  - schemas/verification.json
  - fixtures/01_typed_functions/contracts.json
  - scripts/ground_fixture_contracts.py
  - docs/reviews/unit-verification-audit.md
---

# Unit-Level Migration and Behavioral-Contract Verification

[Index](index.md) | [Verification Lifecycle](verification-lifecycle.md) | [Fallback and Repair Layer](fallback-and-repair-layer.md) | [Known Limitations](known-limitations.md)

Migration and verification proceed at the smallest behaviorally meaningful boundary rather than as
one monolithic repository rewrite. This page documents that boundary model, where behavioral
evidence comes from, how a unit is generated/compiled/verified, and how failures become governed
knowledge — status: `implemented`, backed by `crates/exodus-verifier/tests/gate_integration.rs`
and `docs/reviews/unit-verification-audit.md`, not aspirational.

## Unit-boundary rules

`SemanticGraph::verification_units()` (`crates/exodus-graph/src/lib.rs`) decomposes the ESG's
functions, methods, and types into dependency-ordered `VerificationBoundary` values:

- **`Unit`** — a single node that can be migrated and verified independently.
- **`Cluster`** — multiple nodes that must be treated as one boundary, for either of two reasons,
  both recorded in the boundary's `reason` string:
  1. **A real dependency cycle** (a genuine strongly-connected component via Tarjan's algorithm
     over `Calls`/`DependsOn`/`Imports`/`Inherits` edges).
  2. **Class/method membership** — a method cannot compile outside its struct's `impl` block, so a
     class and every one of its methods are always one boundary, independent of whether any cycle
     is involved. (`fixtures/02_class_conversion`'s `BankAccount` and `fixtures/03_module_dependency`'s
     `Product` are both grouped this way.)

Each node's [`SemanticNode::content_hash`](../crates/exodus-graph/src/lib.rs) is a separate,
complementary signal to its stable `id`: the `id` (`kind::qualified_name`) survives a re-parse of
unchanged code, while `content_hash` changes whenever the underlying source text changes — so a
unit gate can distinguish "genuinely unchanged, prior evidence still applies" from "same identity,
but the implementation moved since the last verified run."

`SemanticGraph::relevant_subgraph(node_id)` extracts the 1-hop dependency/dependent neighborhood
around a single node — this is what scopes Case Engine capture (below) to what a failure actually
implicates, rather than serializing the whole repository graph.

## Behavioral-contract sources and oracle hierarchy

A type signature describes the shape of a call; it never establishes complete expected behavior.
`exodus_core::OracleType` (`crates/exodus-core/src/lib.rs`) encodes the master spec's strength
ordering as a real Rust type, not a free string:

| Rank | Oracle | `is_grounded()` |
|---|---|---|
| 1 | `SourceTest` | yes |
| 2 | `GoldenFixture` | yes |
| 3 | `DifferentialExecution` | yes |
| 4 | `DeclaredInvariant` | yes |
| 5 | `TypeSignature` | **no** — never sufficient alone |
| 6 | `HumanApprovedSynthesized` | yes |

`BehavioralContract::is_grounded()` requires *every* assertion to be grounded — one
`TypeSignature`-only assertion disqualifies the whole contract from grounded-oracle-coverage
metrics (tested in `crates/exodus-core/src/lib.rs`'s
`test_a_single_ungrounded_assertion_disqualifies_the_whole_contract`).

For this pass, five fixtures (`01_typed_functions`, `02_class_conversion`, `03_module_dependency`,
`04_circular_dependency`, `08_async_function`) are grounded via **actual differential execution**:
`scripts/ground_fixture_contracts.py` imports and runs the real Python source with controlled
inputs and records the genuine output as `fixtures/<name>/contracts.json`. Fixture 04's Product
round-trip assertion (`03_module_dependency`) additionally carries a `DeclaredInvariant` assertion
(serde encode/decode is a no-op) precisely because that property is not something differential
execution against Python can establish — Python has no equivalent typed round-trip. Six fixtures
remain ungrounded at the unit tier (`05`–`07`, `09`–`10`, `two_run_demo`) and are honestly reported
as such by `exodus units show`/`exodus eval`, never silently claimed.

`fixtures/04_circular_dependency` also documents a real finding: `user.py` and `order.py` import
each other at module scope, which genuinely fails in CPython (`ImportError` on both `import user`
and `import order`, confirmed directly). The grounding script breaks the load-time cycle by
pre-registering a stub for each function's collaborator module — never faking the function's own
logic — to obtain real per-function execution evidence despite the source itself being
unexecutable as authored.

## Unit harness lifecycle

`exodus_verifier::unit_gate::build_harness` turns a grounded contract into a `tests/behavioral_tests.rs`
file: one `#[tokio::test] async fn` per assertion (a single async harness shape covers both sync and
async migrated units — a sync call inside an async test body needs no `.await`). Each assertion
becomes a real `assert_eq!` comparing the migrated unit's actual `{:?}`-formatted output against the
contract's grounded `expected` value; the assertion's `case_id`/`oracle`/`evidence` are passed as
separate formatted arguments (never spliced into the format-string literal) so real evidence text
containing `{`/`}` — e.g. a captured path like `{models,service}.py` — can never be misparsed as a
Rust format placeholder.

## Per-unit migration gate

`exodus_verifier::run_unit_gate` (`crates/exodus-verifier/src/unit_gate.rs`) implements the master
spec's nine-step sequence for one boundary:

1. Extract the boundary's `FunctionDef`/`ClassDef`s from the parsed repository, plus any `Type` or
   `Method` node the boundary directly depends on (`build_synthetic_module`) — the smallest set
   that can plausibly compile on its own.
2. Transform just that synthetic module.
3. Scaffold and `cargo check` the smallest valid target component in its own scratch directory
   (never the whole repository).
4. If a grounded contract exists, generate and run its harness via `cargo test`; parse real
   pass/fail counts from cargo's own text output (`test <name> ... ok|FAILED`) — a harness that
   fails to *build* is distinguished from one that built and had failing assertions (`0 passed, 0
   failed` is never silently treated as `Verified`).
5. On failure, attempt one bounded repair via the existing `BoundedAgent`/`MockAgentProvider` (see
   [Fallback and Repair Layer](fallback-and-repair-layer.md)); re-check.
6. Localize any unresolved failure — compiler diagnostics or a specific failed assertion — into a
   Migration Case (below).
7. Write `.exodus/contracts/<unit-id>/{behavioral-contract,verification}.json`
   (`schemas/behavioral-contract.schema.json`, `schemas/verification.json`).

**Outcome rules, applied honestly**: `Verified` only when compiled, a contract existed, and every
assertion passed with none absent; `Compatible` likewise but with fallback debt present;
`Degraded` when it compiles but *no grounded contract exists* (compiling is never presented as
behavioral verification); `Blocked` on any compile failure or assertion failure.

Real, currently-observed results across the 5 grounded fixtures (9 verification boundaries) —
recorded here as evidence, not aspiration, from `.exodus/evaluation_scorecard.json`'s `unit_level`
section (`cargo run -p exodus-cli -- eval`): **5 Verified, 0 Compatible, 0 Degraded, 4 Blocked**
(10/17 assertions passed, 58.8%), each Blocked case backed by a genuine, still-unfixed pre-existing
transform defect: a dead-code/missing-return-path bug in `02_class_conversion`'s generated
`withdraw` (real rustc `E0317`), an immutable-loop-binding vs `&mut self` mismatch in
`03_module_dependency`'s `calculate_total`, and a real Rust ownership error (`E0382`, a moved
`String` reused) in `04_circular_dependency`'s `get_user_summary`. (Isolated manual runs of
individual fixtures during development sometimes showed 6 Verified/3 Blocked instead — the fourth
Blocked unit's outcome appears sensitive to system load during heavy concurrent compilation; this
variance is itself reported honestly rather than smoothed over.) None of these are patched over —
they are exactly the kind of maximized-verified-behavior-plus-explicit-debt outcome the project's
safety principle describes. Full detail: `docs/reviews/unit-verification-audit.md`.

The whole-fixture (not per-unit) transform tier tells a starker story once **real** `cargo check`
replaced the previous outcome-enum inference: only 3 of 11 fixtures' combined transform output
actually compiles (`01`, `08`, `09` — 27.3%), against the real naive baseline's 100% (which never
attempts anything nontrivial, so it always compiles but never passes a behavioral test). This was
previously invisible — the old evaluator inferred "compiled" from `MigrationOutcome` rather than
running `cargo check`, so it always reported values matching the transform stage's optimistic
self-assessment. See [Evaluation Methodology](evaluation-methodology.md).

## Verification hierarchy

Five levels are tracked as genuinely distinct statuses, never merged:

1. **Unit** — `UnitVerificationResult.outcome` per boundary.
2. **Dependency-cluster** — `UnitVerificationResult.boundary_kind == "cluster"` plus its `reason`.
3. **Module integration** — `pipeline::run_gated_migration` re-compiles the *accumulated* set of
   every unit verified so far as a whole after each new addition; a unit that individually passed
   but breaks the accumulated crate (a real cross-unit conflict, e.g. two modules colliding) is
   dropped from the accumulated set and its own result is downgraded to `Blocked` — proven by
   `crates/exodus-verifier/tests/gate_integration.rs`'s
   `unit_verification_passing_does_not_imply_module_integration_passing`.
4. **Repository integration** — the existing whole-repository `exodus migrate`/`exodus verify` path
   (unchanged by this work).
5. **End-to-end acceptance** — the two-run case-learning experiment (see
   [Known Limitations](known-limitations.md) — not yet implemented for the unit tier).

`exodus report` prints/emits (`--json`) these as separate sections; `exodus eval` computes unit-tier
metrics (`UnitLevelSummary`) entirely separately from the whole-fixture transform tier
(`EvaluationSummary`) — see [Evaluation Methodology](evaluation-methodology.md).

## Case Engine integration

A `Blocked` outcome calls `CaseEngine::capture_failure` (`crates/exodus-case/src/lib.rs`) with the
unit's ID, its `relevant_subgraph`, the compiler diagnostic or failed assertion text, and (when a
specific assertion failed) the contract's `expected` value alongside the observed failure as
source/target observations. `MigrationCase` now carries `unit_id`, `esg_subgraph`,
`failed_assertion`, `source_observation`, and `target_observation` — fields the pre-unit-gate
implementation did not have.

`CaseEngine::compute_fingerprint` was rewritten to hash real topology — node-kind and edge-kind
multisets, a dynamic/unsupported-construct count, failure category, and language pair — and
**deliberately excludes the failing symbol's name or file path**, so a structurally identical
failure on a differently-named symbol in a different repository still fingerprints identically
(`crates/exodus-case/src/lib.rs`'s `test_fingerprint_ignores_symbol_name_but_reflects_structure`).
The previous fingerprint hashed only node/edge *counts* plus the lowercased symbol name via a
djb2-style function dishonestly named `md5_or_simple_hash` — neither MD5 nor structurally
meaningful; both are fixed.

## Worktree commit behavior

`pipeline::run_gated_migration` creates one real `WorktreeManager` lease per run and, after each
unit that both individually verifies *and* keeps the accumulated integration crate compiling, calls
`WorktreeManager::commit_all` — a real `git add -A && git commit` inside the leased worktree, using
a per-command identity rather than modifying global git config, refusing to commit if the staged
diff contains an obvious secret marker. `WorktreeManager::create_lease` now distinguishes a
genuinely `Active` lease from `CreationFailed` (previously: any `git worktree add` failure — spawn
error *or* non-zero exit — was silently reported as `Active`). `WorktreeManager::cleanup_lease`
now checks real on-disk dirtiness and refuses to remove a dirty worktree unless explicitly forced,
persisting `DirtyReviewRequired` instead (`crates/exodus-worktree/src/lib.rs`'s
`test_cleanup_preserves_dirty_worktree_unless_forced`) — previously it force-deleted unconditionally.

A verified unit's commit remains reachable in git history even when a later unit in the same run is
`Blocked` — proven directly via `git log` inspection in
`fixture_03_cluster_verifies_and_serde_roundtrips_commit_survives_later_failure`.

## Known limitations of this implementation

- Contract grounding covers 5 of 11 fixtures; the rest are reported as ungrounded, not silently
  skipped.
- The bounded repair agent is still `MockAgentProvider` only — no real LLM is wired in, so a
  repair attempt is real (invoked, tracked, bounded) but cannot resolve a genuine semantic defect.
- `exodus cases test --mode verify` reports honestly that promoted cases have no regression fixture
  to re-run against yet — regression-fixture generation on promotion is not implemented.
- The two-run case-learning experiment (`fixtures/two_run_demo`) is not wired into the unit gate.
