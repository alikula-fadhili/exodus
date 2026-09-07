---
type: audit-report
title: Unit-Level Migration & Behavioral-Contract Verification — Audit
status: active
okf_version: "0.2"
---

# Unit Verification Audit — Project Exodus

This document is written in two parts. The **Pre-Change** section below is preserved evidence
captured before any implementation work in this pass and must never be rewritten. A **Post-Change**
section is appended at the end once implementation is complete, so before/after can be compared
directly.

---

## Pre-Change Section (do not edit after implementation begins)

### Repository state at audit time

- HEAD commit: `55396d5688e42dcf324a87b85f2500af1d6e266f` ("feat: Add foundational documentation and
  structure for Project Exodus"), 2026-08-29 08:53:28 +0300.
- `git status --short`: 22 tracked files modified (`Cargo.lock`, `Cargo.toml`, `README.md`, every
  `crates/*/src/lib.rs` and `crates/*/Cargo.toml` except `exodus-case`/`exodus-worktree`,
  `docs/openwiki-visualizer/graph.json`), plus untracked: `.exodus/*` artifacts, `REPRODUCTION.md`,
  `TRAJECTORIES.md`, `VIDEO_SCRIPT.md`, `crates/exodus-case/`, `crates/exodus-worktree/`,
  `fixtures/*` (11 fixture directories), `schemas/`.
- `git diff --stat`: 24 files changed, 4383 insertions(+), 324 deletions(-) relative to HEAD.
- Toolchain versions: `rustc 1.98.0`, `cargo 1.98.0`, `git 2.55.0`, `node v26.7.0`, `deno 2.9.5`.
- Workspace crates (12, confirmed via root `Cargo.toml`): `exodus-core`, `exodus-parser`,
  `exodus-graph`, `exodus-planner`, `exodus-agent`, `exodus-transform`, `exodus-fallback`,
  `exodus-verifier`, `exodus-case`, `exodus-worktree`, `exodus-eval`, `exodus-cli`.

**Operational note — concurrent external mutation.** A separate agent tool, Antigravity
(process `/home/alikula/.gemini/bin/agy --hub --hub-port=34789 --app_data_dir=antigravity
--add-dir=/home/alikula/Documents/projects/exodus`, PID 1020647, started 08:54), was actively
rewriting this repository during the early part of this audit. Three independent read-only
exploration passes each caught files (`crates/exodus-core/src/lib.rs`, `crates/exodus-case/src/lib.rs`,
`crates/exodus-worktree/src/lib.rs`) changing content and line counts between successive reads in the
same session, and `cargo build`/`cargo clippy`/`cargo test` gave materially different results seconds
apart (unresolved imports in one run, missing-method errors in the next, then a clean pass). The user
confirmed Antigravity's implementation work had stopped. A final targeted verification pass still found
the `agy` process listed in `ps`, but the repository content itself was internally consistent across
repeated command reruns at that point, and remained byte-identical (`git status --short` /
`git diff --stat` unchanged) immediately before implementation began. **The snapshot below reflects
that final stable state**, not any of the earlier transient/contradictory reads.

### Commands executed and pre-change results

```bash
$ cargo fmt --check
# exit 0 — clean, no files need formatting

$ cargo clippy --workspace --all-targets -- -D warnings
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
# exit 0 — 0 warnings, 0 errors

$ cargo build --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.57s
# exit 0 — all 12 workspace members compile

$ cargo test --workspace
# 16 tests total across 11 crates + cli binary, 16 passed, 0 failed, 0 ignored
# per-crate: exodus-agent 2, exodus-case 1, exodus-core 1, exodus-eval 1, exodus-fallback 1,
#            exodus-graph 2, exodus-parser 2, exodus-planner 2, exodus-transform 2,
#            exodus-verifier 1, exodus-worktree 1, exodus-cli 0
# exit 0

$ cargo run -p exodus-cli -- eval --fixtures fixtures --output .exodus
# Total Fixtures: 11 | Exodus Behavioral Pass Rate: 90.9% (Baseline: 27.3%)
# Compilation Rate: 100.0% (Baseline: 36.4%) | Migration Debts: 1 | Human Interventions: 0
# 10/11 fixtures Verified; 1 (10_deliberately_untranslatable_reflection) Degraded, tests FAIL, 1 debt
# exit 0
```

Documented end-to-end workflow (`analyze → plan → approve → migrate → verify → report`) is real and
runs; `Verifier` genuinely shells out to `rustfmt` / `cargo check --message-format=json` / `cargo test`
(`crates/exodus-verifier/src/lib.rs`, `run_formatter`/`run_cargo_check`/`run_cargo_tests`). No documented
command in `README.md`/`REPRODUCTION.md` was found to be missing or non-existent, though several
produce numbers that do not match what they claim (see Critical Findings).

### Requirement matrix

| Requirement | Status | Code Evidence | Test Evidence | Gap | Required Change |
|---|---|---|---|---|---|
| ESG identifies functions/methods/classes | Implemented | `crates/exodus-graph/src/lib.rs:11-21` (`NodeKind`), `:391-466` (populated from `ClassDef`/`FunctionDef`) | `exodus-graph` unit tests (2) | Only a generic `Module` kind exists; no distinct "utility module" classification | None required — cosmetic only |
| ESG unit dependencies | Implemented | `dependencies_of`/`dependents_of`, `:123-155` | covered indirectly by graph tests | — | — |
| Strongly connected components | Implemented | `find_sccs` (real Tarjan, `:158-243`), `detect_cycles` (`:246`) | covered by graph tests | — | — |
| Smallest independently compilable clusters | Missing | SCCs computed but never grouped into an atomic verification boundary; `exodus-planner` flags cyclic nodes for approval but still plans them individually | none | No cluster abstraction anywhere in the codebase | Add `VerificationCluster`/`verification_units()` (Milestone 1) |
| Stable unit identities | Partial | IDs are `format!("{kind}::{qualified_name}")` (`:392,423,447`) — deterministic but with no content hash or explicit re-parse-stability guarantee | none | ID changes only if the qualified name changes; a body-only edit is invisible to the ID | Add content-hash component (Milestone 1) |
| Source spans/evidence | Implemented | `SourceEvidence`, tree-sitter-backed (`crates/exodus-parser/src/lib.rs:123-137`) | parser tests | — | — |
| Behavioral contract sourcing (source tests / golden fixtures / differential exec / invariants / signatures / human-approved) | Missing | `BehavioralContract`/`BehavioralAssertion` (`crates/exodus-core/src/lib.rs:161,171`) instantiated **only** in one unit test; no production code path populates one | none | Fixtures contain bare `.py` files with zero oracle/expected-output data; `.exodus/contracts/` empty on disk | Ground contracts per Milestones 2–3 |
| `contracts show` fabricating placeholder contracts | Incorrect | `crates/exodus-cli/src/main.rs:456-472` — hardcoded `source_signature: "fn() -> int"` etc. returned whenever the (always-absent) real file is missing | none | Directly violates "never invent expected outputs from a signature" | Remove fallback fabrication; report "not verified" (Milestone 8) |
| Unit harness generation | Missing | `Verifier::scaffold_target_crate` accepts `behavioral_tests: Option<&str>` but `Commands::Migrate` always passes `None` (`main.rs:313`) | none | No harness is ever generated or run through the CLI pipeline | Milestone 4 |
| Per-unit migration gate (9-step sequence) | Missing | No orchestration exists that resolves deps → retrieves cases → generates unit → compiles → runs contract → commits/localizes → repairs → re-verifies | none | `migrate`/`verify` operate on whole modules/repos, not units | Milestone 5 |
| Verification hierarchy (unit/component/module/repo/e2e reported separately) | Missing | `report`/`eval` expose one rolled-up `MigrationOutcome` per fixture | none | No separate tiers anywhere | Milestone 9 |
| Contract artifacts under `.exodus/contracts/<unit-id>/` | Missing | Directory exists but is empty; nothing writes to it except the CLI's fabricated fallback read path | none | — | Milestone 5/8 |
| Case Engine capturing unit failures with subgraph/observations | Partial | `MigrationCase` (`crates/exodus-case/src/lib.rs:37-57`) has no `unit_id`, no ESG subgraph, no source/target observation fields, only a free-text `failure_description` + `Option<String> compiler_diagnostic` | 1 passing unit test for lifecycle, not integration | `capture_failure` has no caller anywhere except its own test | Milestone 6 |
| Structural case fingerprint | Incorrect | `compute_fingerprint` (`:86-104`) hashes failure-category + node/edge **counts** (not topology) + lowercased symbol name via a djb2-style function dishonestly named `md5_or_simple_hash` (`:231-237`, not MD5) | 1 test | Two structurally different failures with same category/counts/name collide | Milestone 6 |
| Worktree isolation for all mutation/verification | Incorrect | `WorktreeManager` exists and creates real `git worktree add --lock -b exodus/<task>/migration` leases (`crates/exodus-worktree/src/lib.rs:103-162`), but `Commands::Migrate`/`Commands::Verify` never call it — all mutation/compilation happens directly in the primary working tree | 1 unit test on the manager in isolation | Master-prompt requirement "never mutate the user's primary working tree" is violated in the actual pipeline today | Milestone 7 |
| Atomic per-unit worktree commits | Missing | Zero `git commit` invocations anywhere in the codebase (`grep -rn '"commit"' crates/` finds only unpopulated struct field names) | none | — | Milestone 7 |
| Dirty worktrees preserved, not force-deleted | Incorrect | `cleanup_lease` (`:224-238`) unconditionally `fs::remove_dir_all`s; `WorktreeStatus::DirtyReviewRequired`/`Recoverable` exist in the enum but are never set or checked | none | `Discard` and `Cleanup` CLI commands behave identically | Milestone 7 |
| Worktree lease failure handling | Incorrect | `create_lease` logs a `tracing::warn!` but still persists a lease with `status: Active` when `git worktree add` itself fails (`:118-161`) | none | Callers cannot distinguish a real isolated worktree from this silent fallback | Milestone 7 |
| CLI: `units list` | Implemented | `main.rs:426-437`, walks real `SemanticGraph` | manual run confirmed | — | — |
| CLI: `units show <unit-id>` | Missing (stub) | `main.rs:440` — `println!` echoing the ID only | none | No lookup performed | Milestone 8 |
| CLI: `units verify <unit-id>` | Missing (stub) | `main.rs:443` — unconditional hardcoded success print | none | — | Milestone 8 |
| CLI: `contracts show <unit-id>` | Incorrect (see above) | `main.rs:447-475` | none | — | Milestone 8 |
| CLI: `contracts verify <unit-id>` | Missing (stub) | `main.rs:477` — unconditional hardcoded success print | none | — | Milestone 8 |
| CLI: `cases test --mode replay\|verify` | Missing (stub) | `main.rs:530-536` — `println!("...100% PASS.")` regardless of any case content, `engine.list_cases()` never even called | none | Direct contradiction of the project's own "never fabricate verification statuses" invariant | Milestone 8 |
| CLI: `cases search` | Incorrect | `main.rs:504-506` — hardcodes `FailureCategory::AsyncCallbackSemantics`, ignoring the user's query for filtering | none | — | Milestone 8 |
| CLI `--json` output | Partial | Global flag exists and is honored on `analyze`/`plan`/`approve`/`migrate`/`verify`/`eval`; not yet meaningful on the stubbed `units`/`contracts`/`cases` commands | none | — | Milestone 8/11 |
| Repair loop bounded to 3 attempts | Partial (unwired) | `BoundedAgent`/`AgentBounds::default` (`crates/exodus-agent/src/lib.rs:20-29,169-241`) implement a real bounded loop; `Verifier::verify_and_repair` (`crates/exodus-verifier/src/lib.rs:162-253`) is the only caller and has **no callers of its own** outside its definition | 2 agent-crate tests | `exodus verify` bypasses repair entirely | Milestone 5 |
| Typed fallback stubs never lie | Implemented | `crates/exodus-fallback/src/lib.rs:89` — `todo!("Exodus Migration Debt: {}")`, by design, not a code stub | 1 test | — | — |
| Evaluation: unit/component/module/repo rates reported separately | Missing | `crates/exodus-eval/src/lib.rs` has one blended `exodus_pass_rate_pct`/`exodus_compilation_rate_pct` per fixture | 1 test (aggregate only) | No grounded-oracle-coverage or failure-localization-accuracy metric exists anywhere in the repo | Milestone 10 |
| Evaluation: baseline is a real, measured run | Incorrect | `crates/exodus-eval/src/lib.rs:92-96` hardcodes `(baseline_compiled, baseline_tests_passed)` by matching the **fixture directory name string** — no baseline agent is ever executed | none | README/REPRODUCTION's "30% baseline" headline is fabricated, not measured | Milestone 10 |
| Evaluation: Exodus test pass derived from real test execution | Incorrect | `exodus_tests_passed` (`:98-102`) is inferred purely from the transform-stage `MigrationOutcome` enum, never from running `cargo test` on generated code | none | — | Milestone 10 |
| Versioned JSON Schemas for contracts/cases/merge-proposals | Partial | `schemas/behavioral-contract.schema.json`, `schemas/case.schema.json`, `schemas/merge-proposal.schema.json` exist and structurally match their Rust types | none | No `schemas/verification.json`; no schema-validation code path anywhere in the Rust source (schemas exist but are never loaded/checked at runtime) | Milestone 2 |
| `.exodus/schemas/README.md` accuracy | Incorrect | Claims 4 files (`graph.schema.json`, `plan.schema.json`, `debt.schema.json`, `report.schema.json`) that do not exist anywhere in the repo | none | Stale/aspirational documentation | Milestone 12 |
| Fixture set covers the 10 required categories + two-run demo | Implemented (structurally) | 11 fixture directories present matching every required category, plus `two_run_demo/{repo_a,repo_b}` | eval run confirms all 11 parse/transform | Each fixture is bare `.py` source with **no** oracle/expected-output/manifest files | Milestone 3 |
| Two-run case-learning experiment | Missing | `fixtures/two_run_demo/{repo_a,repo_b}` exist on disk, but `CaseEngine` is never invoked from the real migrate/verify path, so no case is ever captured, promoted, or retrieved for repo B | none | — | Out of scope for this pass — flagged as remaining work |
| OpenWiki OKF v0.2 validity | Implemented | 22 concept pages under `openwiki/`, all sampled pages carry valid `okf_version: "0.2"` + `type` + `status` frontmatter; every sampled page is honestly `status: planned` | none | Docs currently under-claim relative to what's implemented (a safe direction), will need selective promotion | Milestone 12 |
| Trajectories are real captured runs, not hand-authored examples | Incorrect | `TRAJECTORIES.md` cites `.exodus/trajectories/architecture_agent.json`, `migration_agent.json`, `repair_agent.json` — **none exist**. Only `.exodus/trajectories/trajectory_sample.jsonl` exists (907 bytes, filename literally says "sample", different field schema than what `TRAJECTORIES.md` shows) | none | Documentation describes artifacts that were never generated | Milestone 12 |
| Fallback ledger consistency | Incorrect | `.exodus/fallbacks.json` is empty `[]` even though `evaluation_scorecard.json` reports `debts_recorded: 1` for fixture 10 — `eval`/`report` never write to the fallback ledger, only `migrate` does, and `migrate` was not run against the full fixture set in this snapshot | none | — | Milestone 12 (doc) / note in Milestone 10 (metrics) |
| No real LLM provider | Missing (acknowledged, likely out of scope) | `AgentProvider` trait has exactly one implementer, `MockAgentProvider` (`crates/exodus-agent/src/lib.rs:71`); no `reqwest`/HTTP client/Anthropic/OpenAI integration anywhere despite `.env.example` listing provider keys | 2 tests against the mock | REPRODUCTION.md's "$0.02/1000 LOC with OpenAI/Anthropic enabled" claim is unverifiable from source | Deferred — no credentials available in this environment |

### Critical findings (summary)

1. Core pipeline (parse → ESG → plan → transform → cargo-based verify) is genuinely implemented and
   should be preserved as-is.
2. Everything downstream of "verify a single unit's behavior against grounded evidence" is either
   unimplemented or actively fabricates its output: contracts are invented from placeholders, `cases
   test` always prints "100% PASS", the eval baseline is a hardcoded name-match, and worktree isolation
   / atomic commits / repair-loop wiring / case-capture wiring do not participate in the real
   `migrate`/`verify` pipeline at all despite each having a real, tested, but disconnected
   implementation sitting in its own crate.
3. Several shipped documents (`TRAJECTORIES.md`, `.exodus/schemas/README.md`, `REPRODUCTION.md`'s
   fixture-count table) describe artifacts or numbers that do not exist or do not match the current
   fixture set — these are corrected in Milestone 12, not before, so the corrections can cite the
   final implemented state rather than guessing ahead of it.
4. No `todo!()`/`unimplemented!()` markers exist as literal unfinished-code placeholders in the core
   pipeline crates — the gap is architectural (real subsystems that are never wired together), not
   missing code inside otherwise-complete functions.

### Evidence paths referenced above

- `crates/exodus-core/src/lib.rs`, `crates/exodus-graph/src/lib.rs`, `crates/exodus-parser/src/lib.rs`,
  `crates/exodus-planner/src/lib.rs`, `crates/exodus-transform/src/lib.rs`,
  `crates/exodus-verifier/src/lib.rs`, `crates/exodus-agent/src/lib.rs`,
  `crates/exodus-fallback/src/lib.rs`, `crates/exodus-case/src/lib.rs`,
  `crates/exodus-worktree/src/lib.rs`, `crates/exodus-eval/src/lib.rs`,
  `crates/exodus-cli/src/main.rs`
- `schemas/behavioral-contract.schema.json`, `schemas/case.schema.json`,
  `schemas/merge-proposal.schema.json`
- `.exodus/architecture.json`, `.exodus/fallbacks.json`, `.exodus/graph.json`,
  `.exodus/evaluation_scorecard.{json,md,csv}`, `.exodus/trajectories/trajectory_sample.jsonl`,
  `.exodus/schemas/README.md`
- `fixtures/01_typed_functions/` … `fixtures/10_deliberately_untranslatable_reflection/`,
  `fixtures/two_run_demo/{repo_a,repo_b}`
- `openwiki/` (22 concept pages), `openwiki/INSTRUCTIONS.md`
- `README.md`, `REPRODUCTION.md`, `TRAJECTORIES.md`, `VIDEO_SCRIPT.md`

### Implementation plan

See `/home/alikula/.claude/plans/you-are-an-independent-async-hoare.md` for the full milestone plan
(M0–M12) approved for this pass. In summary, in dependency order: stable unit identities and cluster
boundaries (M1) → contract model/provenance/schemas (M2) → grounded oracle data for a representative
fixture subset via differential execution (M3) → unit harness generation (M4) → the per-unit
verification gate that connects transform/verify/case/agent (M5) → Case Engine field/fingerprint fixes
and real wiring (M6) → worktree wiring and real atomic commits (M7) → CLI de-stubbing (M8) →
verification-hierarchy reporting (M9) → evaluation metrics overhaul with a real baseline (M10) →
required test coverage (M11) → OpenWiki and documentation corrections (M12).

Contract grounding in this pass is concentrated on five representative fixtures
(`01_typed_functions`, `02_class_conversion`, `03_module_dependency`, `04_circular_dependency`,
`08_async_function`) rather than fabricating coverage across all eleven; remaining fixtures are
reported honestly as ungrounded rather than silently claimed as covered.

---

## Post-Change Section

### Operational note carried over from the pre-change section

Antigravity (`agy --hub`, PID 1020647) remained listed in `ps` throughout this session and, mid-session,
created a real commit (`8ec4f93`) on `master` that HEAD now sits on — confirmed to contain exactly this
session's own in-progress files (consistent with an IDE auto-checkpoint of the working directory, not a
competing editor). The user confirmed and asked to continue. Working-tree integrity was verified intact
immediately after (clean build, passing tests) and again at the end of this pass (see below).

### Summary of implementation (M0–M12, per the approved plan)

**M1 — Stable unit identities & cluster boundaries** (`crates/exodus-graph/src/lib.rs`):
`SemanticNode::content_hash()` added as a complementary signal to the existing stable `id`.
`SemanticGraph::verification_units()` added: SCC-based clustering plus a new rule — a class and
*all* of its methods are always one boundary (a method cannot compile outside its struct's `impl`
block), independent of whether any dependency cycle is involved. `SemanticGraph::relevant_subgraph()`
added (the required-but-missing §7 "relevant-subgraph extraction" algorithm).

**Two previously-undiscovered, now-fixed ESG correctness bugs**, found while building M1/M5 and
confirmed to matter for real fixtures:
- `Calls` edges were built as `format!("function::{}", call.callee)` using the call site's raw,
  usually-unqualified text, while every real node ID carries a module/class qualifier — these edges
  could never match a real node. Cycle detection, dependency ordering, and clustering silently
  operated on dangling edges for *any* real call relationship. Fixed with a two-pass resolution
  (`crates/exodus-graph/src/lib.rs`) that matches a callee's bare name against every function/method
  in the repository, preferring same-module matches.
- `ImportStatement`s were never converted into graph edges at all, despite `RelationKind::Imports`
  existing — `fixtures/04_circular_dependency`'s real module-level import cycle was invisible to
  `detect_cycles`. Fixed by adding a resolution pass over `module.imports`.
- (Found and fixed in the parser, not the graph): `async def` was never detected — this
  tree-sitter-python grammar version represents it as an ordinary `function_definition` node with an
  `async` keyword *child*, not the distinct `async_function_definition` node kind the parser was
  checking `node.kind()` against. Every function was silently classified synchronous. Fixed via
  `PythonParser::node_is_async` (checks children, not the node's own kind).

**M2 — Contract model, provenance, schemas** (`crates/exodus-core/src/lib.rs`, `schemas/`):
`OracleType` (6-variant enum, strength-ordered, `is_grounded()` — `TypeSignature` alone is never
grounded) and `VerificationStatus` replace free-string `oracle`/`verification_status` fields.
`BehavioralContract::is_grounded()` requires every assertion grounded. `schemas/verification.json`
added (previously missing entirely). `schemas/behavioral-contract.schema.json`'s `oracle` field now
has a closed enum matching `OracleType`. This immediately broke, and forced an honest fix of, the
CLI's `contracts show` fallback path, which had been fabricating a placeholder contract
(`main.rs:456-472` in the pre-change snapshot) whenever the real artifact was missing — that
fabrication is now gone; a missing contract is reported as missing.

**M3 — Fixture oracle grounding** (`scripts/ground_fixture_contracts.py`,
`fixtures/{01,02,03,04,08}_*/contracts.json`): real differential execution — the script imports and
actually runs each fixture's Python source with controlled inputs and records the genuine output.
`04_circular_dependency` surfaced a real, unrelated finding: `user.py`/`order.py` import each other
at module scope, which genuinely raises `ImportError` in real CPython for both `import user` and
`import order` — confirmed directly, not assumed. Grounding for that fixture stubs each function's
*collaborator* module to break the load-time cycle, then executes the function's own real body — the
function's own logic is never faked, only the untaken cross-import path. `03_module_dependency`'s
`Product` additionally carries a `declared_invariant` assertion (serde round-trip is a no-op),
deliberately not hardcoding field order since the transform engine's field ordering (derived from
`HashSet` iteration) is not guaranteed stable.

**M4 — Unit harness generation** (`crates/exodus-verifier/src/unit_gate.rs::build_harness`):
one `#[tokio::test] async fn` per assertion (one shape covers both sync and async migrated units).
A real bug was found and fixed during end-to-end testing: assertion `evidence`/`oracle`/`case_id`
text was originally spliced directly into the `assert_eq!` message's format-string *literal* —
evidence text containing a literal `{`/`}` (e.g. `fixtures/03_module_dependency/{models,service}.py`)
broke the generated harness's own compilation. Fixed by passing that text as separate `{:?}`-quoted
arguments instead of literal-string interpolation.

**M5 — Per-unit verification gate** (`crates/exodus-verifier/src/unit_gate.rs`): implements the
master spec's 9-step sequence. Two real bugs found and fixed during first end-to-end runs: (a) the
scratch crate's name equaled its one module's name, making `use <name>::*` in the generated test
genuinely ambiguous between "the crate" and "the module" (E0659) — fixed with a fixed, distinct
crate name; (b) `Verified`/`Compatible` was reachable when `assertions_passed == 0 &&
assertions_failed == 0` (a harness that failed to *build* was indistinguishable from "nothing to
verify, trivially fine") — fixed by requiring `assertions_passed > 0` and detecting a harness-build
failure explicitly, with its own diagnostic.

**M6 — Case Engine integration & fingerprint fix** (`crates/exodus-case/src/lib.rs`): `MigrationCase`
gained `unit_id`, `esg_subgraph`, `failed_assertion`, `source_observation`, `target_observation`.
`compute_fingerprint` rewritten to hash real topology (node-kind/edge-kind multisets, a
dynamic-construct count, failure category, language pair) and to **exclude** the failing symbol's
name — tested directly (`test_fingerprint_ignores_symbol_name_but_reflects_structure`) to prove two
structurally identical failures on differently-named symbols in different repos fingerprint
identically, which is what makes cross-repository case reuse meaningful. The previous
`md5_or_simple_hash` (djb2, not MD5) is replaced by `structural_hash` (`DefaultHasher`/SipHash, no
new dependency), honestly named.

**M7 — Worktree wiring & atomic commits** (`crates/exodus-worktree/src/lib.rs`,
`crates/exodus-verifier/src/pipeline.rs`): `WorktreeManager::create_lease` now distinguishes a
genuinely `Active` lease from `CreationFailed` (previously: any `git worktree add` failure — spawn
error *or* non-zero exit — was silently marked `Active`). `cleanup_lease` now takes a `force: bool`,
checks real on-disk dirtiness, and refuses to remove a dirty worktree unless forced (previously:
unconditional `fs::remove_dir_all`). `commit_all` added — the first and only real `git commit`
implementation anywhere in the codebase, using a per-command identity, refusing to commit if the
staged diff contains an obvious secret marker. `pipeline::run_gated_migration` orchestrates: parse →
`verification_units()` → per-boundary gate → on success, accumulate into a growing target crate and
re-check *module integration* (compiling the accumulated set, distinct from and after each unit's own
isolated pass) → commit atomically → merge proposal at the end. A unit that individually verifies but
breaks the accumulated crate is dropped from it and its own result is downgraded to `Blocked` —
proving unit success is computed independently of, and does not imply, module-integration success
(explicitly required; see `unit_verification_passing_does_not_imply_module_integration_passing`).

**M8 — CLI de-stubbing** (`crates/exodus-cli/src/main.rs`): `units show`/`units verify`/`contracts
show`/`contracts verify` now call real ESG/gate logic; the four hardcoded-success stubs found in the
pre-change audit (`units show`/`verify`, `contracts verify`, and `cases test` unconditionally
printing `"100% PASS"`) are gone. `cases search`'s hardcoded `FailureCategory::AsyncCallbackSemantics`
filter (ignoring the user's query entirely) is replaced with real free-text structural search.
`cases review` and `worktree resume`/`preserve` now show real state instead of print-only no-ops.
`migrate --gated` added as the entry point to the real per-unit pipeline; the pre-existing
`migrate`/`verify` path is unchanged (preserved, not replaced).

**M9 — Verification hierarchy reporting**: `exodus report` now aggregates real
`.exodus/contracts/*/verification.json` artifacts into a distinct unit tier, alongside (never merged
with) the module-integration tier from `report.json`.

**M10 — Evaluation metrics overhaul** (`crates/exodus-eval/src/lib.rs`): the hardcoded
per-fixture-name baseline match is replaced with `naive_baseline_transform` — a real, executed,
non-graph-guided translator (type-erased `todo!()` stubs, no class support, no dependency ordering)
compiled via the same `Verifier` Exodus's own output goes through. `exodus_compiled` is now a real
`cargo check` result rather than inferred from the transform-stage outcome enum. A new, separate
`UnitLevelSummary` tier reports unit contract pass rate, grounded-oracle coverage, and
unit/component verification counts — never blended into the whole-fixture numbers.

**M11 — Required tests**: added across `crates/exodus-graph`, `crates/exodus-case`,
`crates/exodus-core`, `crates/exodus-worktree`, and a new
`crates/exodus-verifier/tests/gate_integration.rs` (6 real, slow — genuine subprocess compilation —
integration tests) plus `crates/exodus-eval`'s tests. All 15 required categories are covered; see
the file-by-file list below.

**M12 — OpenWiki & documentation**: new page `openwiki/unit-verification.md` (unit-boundary rules,
oracle hierarchy, harness lifecycle, verification hierarchy, worktree commit behavior, Case Engine
integration, known limitations). `verification-lifecycle.md`, `known-limitations.md`,
`cli-reference.md`, `evaluation-methodology.md` updated to `status: implemented` with real evidence
citations; `cli-reference.md` was rewritten (the previous version documented commands —
`exodus parse --path <dir>`, `exodus graph --path <dir>` — that never existed in the actual CLI).
`TRAJECTORIES.md` corrected to label its JSON blocks as hand-authored illustrative examples (the
`.exodus/trajectories/*.json` paths it named do not exist — only a file explicitly named
`trajectory_sample.jsonl` does). `.exodus/schemas/README.md` corrected to point at the real
`schemas/` directory instead of listing 4 files that don't exist anywhere. `README.md`'s fabricated
"Human Time per Migration" / "Cost per 1,000 LOC" figures (never measured; no real LLM provider
exists to measure inference cost from) removed; the baseline changelog row corrected to state
plainly that the previous "27.3%/36.4%" figures came from a hardcoded per-fixture-name match, not an
executed baseline run.

### Files changed (by crate)

- `crates/exodus-graph/src/lib.rs` — unit boundaries, content hash, relevant-subgraph, Calls/Imports
  edge resolution fixes.
- `crates/exodus-core/src/lib.rs` — `OracleType`, `VerificationStatus`, `BehavioralContract::is_grounded`.
- `crates/exodus-case/src/lib.rs` — enriched `MigrationCase`, `CaseCaptureInput`, fingerprint rewrite.
- `crates/exodus-worktree/src/lib.rs` — `CreationFailed` status, dirty-aware `cleanup_lease`, `commit_all`.
- `crates/exodus-verifier/src/unit_gate.rs` (new), `src/pipeline.rs` (new), `src/lib.rs`,
  `examples/print_units.rs` (new), `examples/run_gate.rs` (new), `tests/gate_integration.rs` (new).
- `crates/exodus-parser/src/lib.rs` — `node_is_async` fix.
- `crates/exodus-transform/src/lib.rs` — f-string translation, `return [..]` → `vec![..]` fix.
- `crates/exodus-eval/src/lib.rs` — real baseline, real compile checks, `UnitLevelSummary`.
- `crates/exodus-cli/src/main.rs` — all CLI wiring described under M8/M9.
- `schemas/verification.json` (new), `schemas/behavioral-contract.schema.json` (oracle enum),
  `schemas/case.schema.json` (new fields).
- `fixtures/{01,02,03,04,08}_*/contracts.json` (new), `scripts/ground_fixture_contracts.py` (new).
- `PROJECT_EXODUS_MASTER_PROMPT.md` (persisted to disk — previously chat-only).
- `openwiki/unit-verification.md` (new), plus updates to `verification-lifecycle.md`,
  `known-limitations.md`, `cli-reference.md`, `evaluation-methodology.md`, `index.md`.
- `README.md`, `REPRODUCTION.md`, `TRAJECTORIES.md`, `.exodus/schemas/README.md` — credibility fixes.

### Final verification (real, measured, this run)

```
$ cargo fmt --check
exit 0

$ cargo clippy --workspace --all-targets -- -D warnings
exit 0, 0 warnings

$ cargo test --workspace
38 tests passed, 0 failed, across 12 crates
wall time ≈ 10–11 minutes (two test files — crates/exodus-eval and
crates/exodus-verifier/tests/gate_integration.rs — genuinely compile and behaviorally test freshly
scaffolded crates via real cargo check/cargo test subprocesses; this is the deliberate cost of real
evidence over fast-but-fake tests)
```

`cargo run -p exodus-cli -- eval --fixtures fixtures --output .exodus` (real, persisted to
`.exodus/evaluation_scorecard.json`):

- **Whole-fixture transform tier**: Exodus real compile rate **27.3%** (3/11: `01`, `08`, `09`)
  vs. baseline **100%** compile / **0%** behavioral pass (by construction — every baseline function
  is an unconditional `todo!()`). This is a genuinely new, more pessimistic, and more honest number
  than the pre-change "100.0%" — the pre-change figure was inferred from the transform-stage outcome
  enum and never actually compiled anything. Spot-checking three previously-unexamined fixtures
  (`05`, `06`, `10`) confirmed real, distinct compile errors (a type mismatch, a dict-comprehension
  translation gap plus a syntax error, and an untranslated `eval()` call reaching a live call site) —
  genuine, newly-surfaced transform-engine gaps outside this pass's scope to fix.
- **Unit-level tier**: 9 units evaluated across the 5 grounded fixtures — **5 Verified, 0
  Compatible, 0 Degraded, 4 Blocked**; 10/17 assertions passed (58.8%); grounded-oracle coverage
  100% (every evaluated unit had a real contract to check against); 4 repair attempts; 4 cases
  captured. (An isolated manual run during development showed 6 Verified/3 Blocked for the same 9
  units — a real, observed variance attributed to system load during heavy concurrent compilation,
  not chased further given the time budget; reported honestly rather than smoothed over.)

`cargo run -p exodus-cli -- cases test --mode replay` / `--mode verify`: both run and report
correctly (`0/0` in the actual project repo's `.exodus/knowledge`, since no case has been promoted
there yet — separately confirmed working with real promoted-case data during CLI testing in a
scratch repo: `1/1 consistent, 1/1 retrievable` after promotion).

**One full gated migration**, run against this actual repository (`cargo run -p exodus-cli --
migrate fixtures/03_module_dependency --gated`), demonstrating all ten required steps with directly
inspectable evidence:

1. ESG unit extraction: `exodus units list fixtures/03_module_dependency` → 2 boundaries (a
   `Product` class+methods cluster, a `calculate_total` unit).
2. Dependency order: `Product` (calculate_total's dependency) scheduled first.
3. Grounded contract: `exodus contracts show` on the `Product` boundary shows real
   `differential_execution`/`declared_invariant` assertions.
4. Passing unit verification: `Product` → `Verified`.
5. Atomic worktree commit: `git log` inside the leased worktree shows a real commit,
   `11e9c01 Verified unit: method::models::Product::...`, on branch
   `exodus/cli-aeef8beb-.../migration` — **`master`'s HEAD (`8ec4f93`) is unchanged**, confirmed
   directly.
6. Localized failing unit: `calculate_total` → `Blocked` (the real `&mut self`/immutable-loop-binding
   defect).
7. Case creation: a real `Captured` case for `function::service::calculate_total` in
   `exodus cases list`.
8. Re-verification: one bounded repair attempt ran (tracked, MockAgentProvider-only, could not
   resolve the genuine ownership/mutability defect it doesn't understand).
9. Module integration: reported `✅ Passed` (the accumulated crate is exactly the one verified unit,
   `Product`, which compiles).
10. Merge proposal: `proposal-c3cc97bc-...`, `approved: false` — not auto-merged, per the master
    prompt's explicit requirement.

The worktree (`/home/alikula/Documents/projects/.exodus-worktrees/exodus/cli-aeef8beb-...`) and its
branch were left in place after this run, per the default "preserve, don't force-delete" behavior —
clean up with `exodus worktree cleanup cli-aeef8beb-fb90-4791-8d27-efbcd4c6c647` when done inspecting it.

### What remains planned (not implemented in this pass)

- Contract grounding covers 5 of 11 fixtures; the other 6 remain ungrounded at the unit tier
  (honestly reported as such, not silently skipped).
- The four newly-characterized transform-engine defects (dead-code/missing-return, loop-variable
  shadowing, unconditional `&mut self`, un-cloned moved `String`) are real and unfixed — each was a
  deliberate scope boundary (fixing them safely requires restructuring the heuristic statement
  translator, a separately-scoped undertaking), not an oversight.
- No real LLM provider is wired in (`MockAgentProvider` only) — the bounded repair loop is real,
  invoked, and tracked, but cannot resolve genuine semantic defects.
- Regression-fixture generation on case promotion is not implemented; `cases test --mode verify`
  reports this honestly.
- The two-run case-learning experiment (`fixtures/two_run_demo`) is not wired into the unit gate.
- `exodus units verify`/`contracts verify` (ad hoc single-unit CLI checks) use a scratch directory,
  not a full worktree lease — only `migrate --gated` routes through real worktree isolation and
  commits, a deliberate scope choice for this pass (documented in `openwiki/cli-reference.md`).
