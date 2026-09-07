---
okf_version: "0.2"
type: reference
status: implemented
sources:
  - openwiki/INSTRUCTIONS.md
  - crates/exodus-fallback/src/lib.rs
  - docs/reviews/unit-verification-audit.md
---

# Known Limitations

[Index](index.md) | [Fallback and Repair Layer](fallback-and-repair-layer.md) | [Evaluation Methodology](evaluation-methodology.md) | [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md)

Documented limits and unsupported dynamic constructs:

## Unsupported Dynamic Semantics
* **Dynamic Code Execution**: `eval()`, `exec()`, and dynamic module loading require manual conversion or fallback stubs.
* **Dynamic Metaclasses / Monkey-Patching**: Dynamic runtime modification of classes is mapped to explicit trait objects or classified as `Blocked`.
* **Complex Cyclic Macros**: Multi-crate circular macro dependencies require human intervention.

## Transform-engine capabilities and fixed defects

The following 4 previously-open transform defects have now been resolved and verified with dedicated test coverage:

* **Receiver mutability inference (`&self` vs `&mut self`)**: Fixed via `TransformationEngine::infer_receiver_mutability`. Read-only methods (e.g. `get_price(&self)`, `get_balance(&self)`) generate immutable references, allowing clean caller binding. Mutating methods (e.g. `deposit(&mut self)`, `withdraw(&mut self)`) correctly receive mutable references.
* **Function body snippet indentation & dead code returns**: Fixed indentation normalization in multi-line function and method snippets. Nested blocks (such as `if ...: ... return X` followed by outer `return Y`) correctly close the `if` block, preventing unreachable code flattening.
* **Loop-variable shadowing**: Fixed variable re-declaration inside loops (`for x in items`) to avoid shadowing accumulators or outer bindings.
* **Call argument non-copy ownership cloning**: Added automatic `.clone()` injection for identifier arguments in function calls, avoiding use-after-move compiler errors.

With these fixes, the measured unit contract pass rate reached **94.1%** (16/17 assertions passing across grounded fixtures) and the whole-fixture compilation rate reached **63.6%** (7/11 fixtures compiling cleanly).

### Remaining open transform limitations
* Complex match statements or advanced pattern matching require manual verification or fallback stubs.
* Cross-module type imports outside `crate::*` hierarchy require human review or explicit re-exports.

## ESG / dependency-resolution gaps found and fixed in this pass

* **`Calls` edges never matched any real node.** The callee side of every call edge was built as
  `format!("function::{}", call.callee)` using the call site's raw (often unqualified) text, while
  every real function/method node ID always carries a module/class qualifier — so cycle detection,
  dependency ordering, and unit clustering silently operated on dangling edges for any actual call
  relationship. Fixed via a two-pass resolution that matches the callee's bare name against every
  function/method in the repository (`crates/exodus-graph/src/lib.rs`).
* **`ImportStatement`s were never converted into graph edges at all**, so an import-level cycle
  (`fixtures/04_circular_dependency`'s mutually-importing `user.py`/`order.py`) was invisible to
  `detect_cycles`/risk scoring despite the ESG's own `RelationKind::Imports` variant existing.
  Fixed by adding a resolution pass over `module.imports`.
* **`async def` was never detected.** This tree-sitter-python grammar version represents `async def`
  as an ordinary `function_definition` node with an `async` keyword *child*, not a distinct
  `async_function_definition` node kind the parser was checking `node.kind()` against — every
  function was silently classified synchronous. Fixed via `PythonParser::node_is_async`.

## Unit-verification scope not yet covered

* Contract grounding covers 5 of 11 fixtures (`01`–`04`, `08`); `05`–`07`, `09`–`10`, and
  `two_run_demo` are reported as ungrounded at the unit tier, not silently skipped.
* No real LLM provider exists (`MockAgentProvider` only) — repair attempts are real, invoked, and
  bounded, but cannot resolve genuine semantic defects.
* Regression-fixture generation on case promotion is not implemented; `exodus cases test --mode
  verify` reports this honestly rather than fabricating a re-verification pass.
* The two-run case-learning experiment (`fixtures/two_run_demo`) is not wired into the unit gate.

## Sense & Intent Engine Invariants & Limitations

* **Provisional Claim Grounding**: Human acceptance of a provisional claim allows it to guide planning and downstream synthesis, but does *not* falsely elevate its grounding tier to `Deterministic` or `Verified`. Behavioral contracts (`BehavioralContract`) remain the only grounded execution oracle.
* **Evidence Scrubbing**: `SecretScrubber` aggressively redacts secrets, keys, and tokens from snippets before hash calculation, preventing accidental credential storage or terminal emission.
* **Bounded Repair**: Automated repair cycles for unresolved claims or failed verifications are strictly bounded to $\le 3$ iterations per symbol before emitting fallback stubs and migration debt.

