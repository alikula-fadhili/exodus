---
type: implementation-specification
title: Project Exodus Master Implementation, Verification, and Reproducibility Prompt
status: active
---

# Project Exodus Master Prompt

You are the lead Rust engineer and compound-agent systems architect responsible for completing, hardening, and proving **Project Exodus**.

This is the single authoritative implementation prompt. It replaces earlier separate prompts for repository initialization, OpenWiki/OKF setup, semantic-graph migration, deterministic fallbacks, verifier-driven repair, migration-case learning, evaluation, and Git-worktree isolation.

Do not return only a plan or architecture proposal. Inspect the repository, preserve existing work, execute the implementation, run real verification, correct unsupported claims, and leave the project reproducible from a clean environment.

## 1. Product definition

Project Exodus is a graph-guided, agent-assisted legacy code migration engine. Its initial supported vertical slice migrates Python 3.11 repositories into Rust 2021 projects.

Exodus must:

1. Parse a source repository.
2. Convert source syntax into a language-neutral Exodus Semantic Graph (ESG).
3. Analyze dependencies, cycles, coupling, unsupported constructs, and migration risk.
4. Generate an ordered migration plan.
5. Require human approval before consequential transformations.
6. Execute all mutations in an isolated Git worktree.
7. Transform supported constructs deterministically.
8. Use agents only for bounded semantic decisions.
9. Represent unsupported behavior as explicit migration debt.
10. Compile and behaviorally test the generated target.
11. Apply bounded, policy-controlled repairs.
12. Convert verified failures and repairs into governed migration cases.
13. Reuse only approved case knowledge on structurally similar future migrations.
14. Produce inspectable evidence, trajectories, diffs, commits, and reproducible evaluation results.
15. Maintain evidence-grounded OpenWiki documentation in Open Knowledge Format v0.2.

The central product statement is:

> Exodus converts repository migration from unconstrained source-to-source generation into constrained graph-to-graph transformation, isolated execution, compiler-backed verification, and governed reuse of verified migration knowledge.

The central safety principle is:

> Reliable migration does not mean pretending every construct was translated. It means maximizing verified behavior while turning unresolved semantics into explicit, measurable, and reviewable migration debt.

## 2. Intended user and problem

The intended user is a software engineer, technical lead, or platform-modernization team responsible for migrating a legacy codebase into a safer or more maintainable language and architecture.

Their bottleneck is not generating equivalent-looking code. They must understand unfamiliar dependencies, preserve behavior, determine a safe migration order, handle unsupported runtime behavior, compare alternative strategies, and prove that the generated repository still works.

One-prompt migration approaches commonly fail because they lack repository-wide dependency context, invent semantics for unsupported constructs, and stop without compilation or behavioral verification.

## 3. State-aware starting procedure

The repository may be empty, partially initialized, or already contain a working Exodus implementation. Determine its state before changing anything.

Before implementation:

1. Read `AGENTS.md`, `CLAUDE.md`, `README.md`, `REPRODUCTION.md`, and `openwiki/INSTRUCTIONS.md` when present.
2. Read the relevant OpenWiki concept pages.
3. Inspect workspace manifests, toolchain files, fixtures, schemas, tests, generated artifacts, and Git status.
4. Record the current commit and all uncommitted user changes.
5. Preserve existing working code and unrelated changes.
6. Run the existing formatter, linter, tests, CLI demonstration, and evaluation commands.
7. Build a requirement matrix with `implemented`, `partial`, `missing`, or `unsupported` for every requirement in this prompt.
8. Implement only what is missing or defective; do not replace verified working subsystems merely to match a preferred layout.

If the repository is empty, initialize the workspace and OpenWiki foundation described below. If it is already initialized, validate and repair that foundation instead of recreating it.

## 4. Current completion report: claims to reproduce, not assumptions

The supplied project report claims that the current repository contains:

- Ten Rust crates.
- Tree-sitter Python parsing.
- An in-memory ESG.
- Tarjan SCC cycle detection and wave planning.
- Deterministic Python-to-Rust transformations.
- Explicit typed fallback stubs and a migration-debt ledger.
- A bounded three-attempt repair loop using Rust compiler diagnostics.
- An approval command.
- Ten synthetic fixtures.
- A 90% behavioral pass rate versus a 30% baseline.
- A 100% fixture compilation rate versus a 40% baseline.
- OpenWiki with 21 concept pages and OKF v0.2 formatting.
- Passing `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.
- Reproduction, trajectory, video, and evaluation artifacts.

Do not repeat these claims without reproducing them.

Audit specifically for these credibility risks:

- Hard-coded evaluation outcomes instead of executed results.
- A baseline described as a real agent run when it is simulated or fixture-coded.
- Hand-authored example trajectories presented as captured execution traces.
- Token counts, costs, timestamps, or tool responses not emitted by actual runs.
- Estimated human time or cost presented as measured evidence.
- `todo!` stubs counted as behavioral success.
- Approval commands excluded from the human-intervention count.
- Claims such as `100% compilation safety`, `0% hallucination`, or `0 silent bugs` that exceed what the fixtures establish.
- Absolute `file:///home/...` documentation links that fail on another machine.
- An unpinned Rust, Node, OpenWiki, or dependency toolchain.

For every unsupported claim, either generate real evidence or rewrite it conservatively. Prefer phrases such as `100% fixture compilation rate` and `no silent fallback observed in the evaluated fixtures` when those are the facts.

Never fabricate benchmark evidence, agent trajectories, human-time savings, API cost, or production-safety claims.

## 5. Reproducible repository foundation

Use or adapt this logical Rust workspace:

```text
crates/
├── exodus-core
├── exodus-parser
├── exodus-graph
├── exodus-planner
├── exodus-agent
├── exodus-transform
├── exodus-fallback
├── exodus-verifier
├── exodus-case
├── exodus-knowledge
├── exodus-worktree
├── exodus-eval
└── exodus-cli
```

The exact crate count is not a goal. Existing coherent crates may contain multiple logical subsystems. Avoid unnecessary crate churn.

Pin and commit:

- The tested Rust toolchain through `rust-toolchain.toml`.
- Rust 2021 edition unless existing verified code requires another edition.
- `Cargo.lock`.
- Node.js 22 through the repository's version manager.
- An exact OpenWiki development dependency and package-manager lockfile.

Do not require Deno when Node.js is the documented OpenWiki runtime. Do not commit credentials. Provide `.env.example` containing names only.

The workspace must pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 6. OpenWiki-first knowledge standard

OpenWiki is the human- and agent-readable knowledge layer from the start of the project. It does not replace source code, tests, ESG artifacts, compiler evidence, case records, or evaluation results.

Ensure `openwiki/INSTRUCTIONS.md` states:

1. Document only behavior supported by inspected source, tests, or generated evidence.
2. Distinguish `implemented`, `experimental`, `planned`, `degraded`, `blocked`, and `deprecated` functionality.
3. Link material architectural claims to repository evidence.
4. Never describe compilation alone as behavioral correctness.
5. Never describe a stubbed path as verified behavior.
6. Keep the ESG distinct from the OpenWiki knowledge graph.
7. Treat `.exodus/` as structured execution evidence.
8. Document human-approval and safety boundaries.
9. Preserve known limitations and migration debt.
10. Never include credentials or proprietary source material.

Validate Open Knowledge Format v0.2:

- `openwiki/index.md` declares `okf_version: "0.2"`.
- Every concept page has non-empty `type` front matter.
- Standard Markdown links express relationships.
- Generated, source, verification, lifecycle, trust, and provenance fields are preserved when OpenWiki establishes them.
- No generated or verified event is manually invented.
- OpenWiki-managed claims and sidecars remain inspectable.

The wiki should cover:

- Product overview and intended user.
- Architecture and system boundaries.
- ESG specification.
- Parser and transformation support.
- Graph algorithms and planning.
- Agent roles and policy boundaries.
- Fallback and repair lifecycle.
- Unit-boundary extraction and behavioral-contract verification.
- Worktree isolation and crash recovery.
- Migration-case lifecycle and structural retrieval.
- Artifact schemas.
- CLI reference.
- Evaluation methodology.
- Reproduction and security.
- Improvement changelog, limitations, and glossary.

Run `openwiki --update` only after a stable milestone has passing tests and durable evidence. The committed wiki must remain readable without model credentials. The primary Exodus demo and evaluation must not depend on regenerating the wiki.

## 7. Core semantic migration engine

### Shared domain model

Use typed, serializable models for:

- Repository and run identities.
- Languages and versions.
- Source spans and evidence.
- Diagnostics and failure categories.
- Risk and confidence evidence.
- Migration status and approval state.
- Configuration and artifact versions.

Use typed errors. Avoid uncontrolled `unwrap()` and `expect()` in normal execution paths.

### Parser

The initial lane is Python 3.11 to Rust 2021 unless the repository already implements another verified lane.

Extract at minimum:

- Modules and imports.
- Functions, classes, methods, and parameters.
- Return annotations.
- Assignments and state access.
- Function calls and attribute access.
- Inheritance and decorators.
- Async functions.
- Source locations.
- Unsupported or ambiguous constructs.

One unsupported file must produce a structured diagnostic rather than crash repository analysis.

### Exodus Semantic Graph

The ESG belongs to Exodus and must remain independent of any database representation.

Support nodes such as repository, module, type, function, method, parameter, state, external dependency, and unsupported construct.

Support edges such as contains, imports, calls, reads, writes, returns, accepts, inherits, depends-on, implements, and unknown relation.

Implement and test:

- Dependency traversal.
- Fan-in and fan-out.
- Cycle detection.
- Strongly connected components.
- Migration ordering.
- Coupling estimation.
- Risk scoring.
- Relevant-subgraph extraction.

Every material node and edge must carry source evidence when available.

### Persistence

Portable JSON is the required default. Keep persistence behind a trait or adapter.

SurrealDB may index repositories, semantic graphs, cases, strategies, verification runs, reviews, and relationships, but it must remain optional for the reproducible MVP. Do not make a database server necessary to run the judge demonstration.

## 8. Planning and approval

Generate a migration plan containing:

- Ordered phases or waves.
- Module order and dependency reasons.
- Cycles and strongly connected components.
- High-risk nodes.
- Unsupported constructs.
- Proposed compatibility wrappers.
- Expected fallbacks.
- Required human decisions.
- Acceptance criteria.

Migration must refuse to execute consequential changes without approval. An `--auto-approve` option is allowed only for synthetic fixtures and explicitly marked automated evaluation.

## 9. Purposeful agent architecture

Agents are for bounded semantic decisions, not deterministic parsing, graph traversal, Git lifecycle, schema validation, or compiler execution.

Implement or validate:

- An `AgentProvider` abstraction.
- A deterministic mock provider for tests.
- An OpenAI-compatible provider when external inference is in scope.
- Environment-based credentials.
- Timeouts and retry budgets.
- Structured, schema-validated responses.
- Token, latency, and cost recording only when returned or measured.

Logical roles:

- **Architecture Agent:** proposes architectural boundaries and target equivalents from an evidence-linked graph summary; it cannot mutate files.
- **Migration Agent:** proposes transformations from an approved plan and minimal relevant subgraph.
- **Verifier/Repair Agent:** proposes an allowlisted repair from compiler/test evidence and relevant code.

Free-form agent output must never directly mutate arbitrary files. The orchestrator validates and applies structured proposals inside an Exodus-owned worktree.

Capture actual representative trajectories showing instructions, context, tools, feedback, retries, approval checkpoints, and final outcomes. Mark hand-authored illustrations as examples and keep them separate from captured runs.

## 10. Deterministic transformation scope

For the Python-to-Rust vertical slice, support and test a meaningful subset:

- Typed primitive values.
- Functions, parameters, and return values.
- Arithmetic and Boolean expressions.
- Conditional statements and basic loops.
- Lists to vectors.
- Dictionaries to hash maps or typed structures when justified.
- Simple classes to structs and `impl` blocks.
- Intra-project calls.
- Basic async and error propagation when semantics are inferable.

Do not silently invent behavior when semantics are unclear.

## 11. Unit-level component and behavioral-contract verification

Migration and verification must proceed at the smallest behaviorally meaningful boundary rather than as one monolithic file or repository rewrite.

### Unit-boundary extraction

Decompose the ESG into migration units such as:

- Pure functions.
- Stateful functions with explicit dependencies.
- Methods.
- Classes or structs with their invariants.
- Serialization and conversion boundaries.
- Small utility modules when no smaller independently compilable boundary exists.

Every unit must have a stable ESG identity, source span, dependency list, migration status, contract status, and target location.

Use dependency order when scheduling units. When functions or types form a strongly connected component that cannot compile independently, treat the smallest valid SCC or dependency cluster as the verification unit and record why finer isolation was impossible.

Do not claim function-level isolation when the compiler or runtime requires a larger component boundary.

### Behavioral-contract sources

A type signature describes the shape of a call; it does not establish the complete expected behavior. Never invent expected outputs from a signature alone.

Derive contracts from the strongest available evidence, in this order:

1. Existing source tests and fixtures.
2. Executable examples and approved golden files.
3. Differential execution of the legacy unit with controlled inputs.
4. Declared invariants, preconditions, postconditions, and error behavior.
5. Type signatures, annotations, and docstrings.
6. Human-approved synthesized cases when no stronger oracle exists.

Record the provenance of every assertion. A test generated by the same target implementation cannot serve as its own correctness oracle.

Contracts should cover applicable behavior including:

- Inputs and outputs.
- Error and exception semantics.
- Boundary and empty values.
- State mutations.
- Observable side effects.
- Serialization formats.
- Ordering guarantees.
- Determinism or permitted nondeterminism.
- Resource and timeout expectations where material.

For network, database, filesystem, time, randomness, environment, or other external effects, use explicit fixtures, fakes, recorded responses, or sandboxed adapters. Mark a unit blocked or component-only when its behavior cannot be isolated honestly.

### Unit harness synthesis

For each unit, generate a target harness that:

- Calls the migrated unit through its public boundary.
- Uses contract inputs grounded in source evidence.
- Compares normalized outputs, errors, and approved side effects.
- Cannot access unrelated implementation details unless the source contract requires them.
- Produces machine-readable results linked to the ESG unit and contract assertion.

Where safe and deterministic, run the source unit and migrated target unit on the same controlled inputs and compare their normalized observations.

Property-based or fuzz testing may extend example contracts, but generated properties must come from documented invariants rather than arbitrary agent assumptions.

### Per-unit migration gate

For each scheduled unit:

1. Resolve its dependencies and applicable promoted migration cases.
2. Generate or transform only the target unit and required harness.
3. Format and compile the smallest valid target component.
4. Execute the unit's behavioral contract.
5. If it passes, record verified evidence and create an atomic worktree commit when the repository remains in a valid build state.
6. If it fails, localize the diagnostic and failed assertion to the unit and ESG node.
7. Send only the relevant unit, subgraph, contract, and evidence into the bounded repair and Case Engine flow.
8. Re-run the unit gate after repair.
9. Stop at the repair budget and mark the unit degraded or blocked when unresolved.

Do not commit an entire file or module as verified merely because it compiles. All required constituent units must pass their contracts or be explicitly represented as compatible, degraded, or blocked.

Atomic per-unit commits are preferred, but correctness is more important than commit granularity. If a single unit cannot leave the target buildable, commit the smallest dependency-ordered verified batch and list every included unit in the commit metadata.

Previously verified unit commits must remain recoverable when a later unit fails. Module- and repository-level integration tests still run after all eligible unit gates; unit success does not replace integration verification.

### Contract artifacts

Store versioned contract artifacts under:

```text
.exodus/contracts/<unit-id>/behavioral-contract.json
.exodus/contracts/<unit-id>/verification.json
```

A contract should contain fields equivalent to:

```json
{
  "schema_version": "1.0.0",
  "contract_id": "<ulid>",
  "unit_id": "function:sanitize_input",
  "unit_name": "sanitize_input",
  "unit_kind": "function",
  "source_signature": "sanitize_input(str) -> str",
  "target_signature": "sanitize_input(&str) -> String",
  "assertions": [
    {
      "case_id": "empty-input",
      "input": "",
      "expected": "",
      "oracle": "source_test",
      "evidence": "repo://tests/test_sanitize.py"
    }
  ],
  "verification_status": "passed"
}
```

Do not embed invalid JSON, secrets, or uncontrolled proprietary data in contract artifacts.

When a unit failure becomes a Migration Case, include the localized contract, failed assertion, source and target observations, unit ESG subgraph, and verification evidence. This localized evidence must contribute to the structural case fingerprint.

### Verification hierarchy

Report each level separately:

1. Unit contract verification.
2. Dependency-cluster or component verification.
3. Module integration verification.
4. Repository integration verification.
5. End-to-end acceptance verification when available.

A repository cannot be labeled fully verified when only unit contracts passed. Conversely, an integration failure must not erase which individual units were already verified.

## 12. Formal fallback and repair layer

Fallbacks are first-class evidence, not hidden hacks.

Support strategies equivalent to:

```rust
enum FallbackStrategy {
    ExpandedSyntax,
    CompatibilityWrapper,
    DynamicValue,
    TypedFailureStub,
    ManualReview,
}
```

Each fallback must record source location, construct, strategy, reason, evidence, confidence basis, behavior-verification status, and human-review requirement.

Rules:

- Preserve unsupported source in comments or sidecar files, never as foreign target syntax.
- Compatibility wrappers must have explicit contracts and tests.
- Dynamic representations such as `serde_json::Value` are migration debt unless validated as intended design.
- Typed stubs must fail explicitly; never return believable dummy business values.
- A stub that compiles remains degraded and cannot count as behaviorally successful.
- Never insert Rust `unsafe` automatically. Any proposed `unsafe` requires human approval and a written safety invariant.

The verifier loop is:

1. Generate target code.
2. Format it.
3. Compile it with machine-readable diagnostics.
4. Run behavioral tests.
5. Classify failures.
6. Apply one allowlisted deterministic repair or validated agent proposal.
7. Re-run verification.
8. Stop after the configured budget, defaulting to three attempts per unit.
9. Escalate unresolved or risky behavior.

Allow safe repairs such as import correction, explicit type annotations, safe conversions, ownership adjustments that preserve semantics, `Result` propagation, missing derives, and module-path corrections.

Prohibit test deletion, behavior removal, constant dummy values, broad warning suppression, unrestricted `unsafe`, or converting an unresolved operation into a success result.

Report outcomes separately:

- `Verified`: compiles and passes behavioral contracts.
- `Compatible`: uses a tested wrapper and passes its contract.
- `Degraded`: compiles with explicit unresolved debt.
- `Blocked`: requires human intervention.

## 13. Migration Case Engine and governed learning

A failure must become a reusable, evidence-backed **Migration Case**, not merely a compiler message.

Each case must contain:

- ULID case and run identities.
- A deterministic structural fingerprint used separately from occurrence identity.
- Source and target languages and versions.
- Framework, dependency, and compiler constraints.
- Pattern and failure classification.
- Compiler or behavioral-test evidence.
- Relevant source semantic subgraph.
- Failed target subgraph.
- Attempted and failed strategies.
- Proposed and successful repair strategy.
- Verification result.
- Human-review decision.
- Sanitization and sharing status.
- Regression-fixture location.

Use this lifecycle:

```text
Captured
  → Reproduced
  → Repair Proposed
  → Verified
  → Human Approved
  → Promoted
  → Deprecated when invalidated
```

Only verified and human-approved cases may influence future migrations automatically.

Confidence must come from evidence such as verified applications, failures, structural similarity, version compatibility, and review status. Display `6 of 7 verified applications succeeded`, never an unsupported percentage alone.

### Structural retrieval

For the MVP, implement a transparent deterministic graph fingerprint using node kinds, edge kinds, call/error relationships, input/output types, dynamic-value locations, dependency categories, failure category, and language pair.

Retrieve and rank only compatible promoted cases. Embeddings may enhance later retrieval but must not override deterministic compatibility constraints.

### Case-driven repair

On verification failure:

1. Parse and classify the failure.
2. Extract the relevant ESG subgraph.
3. Generate a case candidate.
4. Search promoted compatible cases.
5. Rank and explain matches.
6. Apply a strategy only when policy permits.
7. Verify the actual result.
8. Record success or failure.
9. Update outcome statistics.
10. Require human approval before promoting new knowledge.

Preserve failed strategies so Exodus does not repeat known mistakes.

### Privacy

Cases are project-local by default. Before shared promotion, remove proprietary identifiers, literals, credentials, and unnecessary code; retain only the minimum structural subgraph; record provenance and permission; and require explicit approval. Never upload or share a case silently.

## 14. Versioned artifact schemas

Define and validate schemas before retrieval logic for:

- `case.json`.
- Source, failed-target, and repaired-target subgraphs.
- `verification.json`.
- `behavioral-contract.json`.
- Unit-verification and verification-hierarchy reports.
- The knowledge index.
- Worktree manifests and merge proposals.

Publish JSON Schemas under `schemas/`. Every artifact must include `schema_version`, identity, creation time, Exodus version, language/toolchain versions, provenance, and sanitization status where relevant.

Do not silently reinterpret an unsupported schema version. Return a compatibility error or run an explicit migration.

## 15. Git worktree isolation

All generated mutations, repairs, compilation, and behavioral verification must occur inside Exodus-owned linked Git worktrees. Never mutate the user's primary working tree during migration.

Use the installed Git CLI with `std::process::Command`. Pass arguments separately; never interpolate user data into shell command strings. Parse `git worktree list --porcelain`.

### Location and ownership

Do not nest linked worktrees inside the primary repository. Use a configurable sibling/runtime root such as:

```text
<repository-parent>/.exodus-worktrees/<repository-id>/<task-id>/
```

Store worktree metadata under `.exodus/runs/<run-id>/worktree.json`.

Before any removal, verify that the path is registered to the current repository, is under the configured Exodus worktree root, matches the recorded task and repository identities, is not the main worktree, and is not owned by another active run.

### Preflight

Record repository root, Git common directory, Git version, current HEAD, target base commit, tracked and untracked changes, submodules, existing Exodus refs, and registered worktrees.

A dirty main working tree may remain untouched, but its uncommitted changes are absent from a worktree created from HEAD. Explain that fact and require the user to continue from committed HEAD, stop and commit, or explicitly approve a future snapshot mechanism. Never automatically stash, reset, commit, or discard the user's changes.

### Creation and leases

Use sanitized collision-resistant refs such as:

```text
exodus/<task-id>/migration
exodus/<task-id>/attempt-01
```

Create from the recorded base commit with `git worktree add --lock --reason ... -b ...`. Do not use `-B`.

Persist a lease containing repository, task and run IDs; absolute path; branch; base and current commits; creation time; lock state; owning run; and cleanup state before agents mutate files.

### Build and attempt isolation

Give each attempt a separate build-output directory and explicit `CARGO_TARGET_DIR`. Immutable dependency caches may be shared.

For parallel strategies:

1. Start every attempt from the same base commit.
2. Give each a unique branch, worktree, and build directory.
3. Apply one strategy per attempt.
4. Run equivalent verification.
5. Rank by behavioral tests, compiler/static checks, degraded debt, prohibited operations, diff complexity, runtime, and cost—in that order.
6. Enforce CPU, memory, and agent-call limits.

### Atomic trajectory commits

Commit only on Exodus-owned branches. Useful checkpoints include scaffold, graph-guided translation, named-case repair, and verified success.

Before committing, format, inspect the staged diff, scan for secrets/forbidden files, confirm the branch owner, and link graph, case, and trajectory IDs. Pass a per-command Exodus identity; do not modify global Git configuration.

### Approval, merge, and cleanup

Selecting a winning attempt generates a merge proposal; it does not merge automatically.

The proposal must contain winning branch, base and final commits, verification evidence, diff summary, applied cases, migration debt, comparison with alternatives, reproduction commands, and rollback instructions.

Before approved merge, detect target-branch drift, rebase or re-run only through an explicit policy, show the full diff and report, and require human approval.

Never use `git worktree remove --force` during normal cleanup. Preserve dirty or failed worktrees as review-required. Force discard requires explicit task-ID confirmation, validated ownership, archived diff and trajectory, and confirmation that no approved result exists only there. Worktree removal and branch deletion are separate operations.

On startup, recover persisted leases and classify worktrees as active, recoverable, dirty/review-required, clean/removable, missing, or ownership-mismatched. Never delete merely because an old process is absent.

Detect submodules, warn about limitations, and disable unsafe parallel checkout behavior for the MVP.

## 16. Required CLI

Implement or validate:

```bash
exodus analyze <source>
exodus plan <source>
exodus approve <plan>
exodus migrate <source> --output <target>
exodus verify <target>
exodus report
exodus eval

exodus units list
exodus units show <unit-id>
exodus units verify <unit-id>
exodus contracts show <unit-id>
exodus contracts verify <unit-id>

exodus cases list
exodus cases show <case-id>
exodus cases search <failure-or-path>
exodus cases review <case-id>
exodus cases approve <case-id>
exodus cases reject <case-id>
exodus cases test --mode replay
exodus cases test --mode verify

exodus worktree list
exodus worktree inspect <task-id>
exodus worktree resume <task-id>
exodus worktree preserve <task-id>
exodus worktree cleanup <task-id>
exodus worktree discard <task-id> --confirm <task-id>
```

Support readable terminal output, `--json`, useful exit codes, non-color environments, and `NO_COLOR`. Do not use animations in captured logs.

Replay case testing validates schemas, recomputes fingerprints, runs retrieval/ranking, and replays deterministic transformations without network inference. Verify mode generates the actual target, runs formatter/compiler/tests, checks behavioral contracts, and records evidence. Never replace full verification with the fast path.

## 17. Portable artifacts

A migration run should produce the applicable subset of:

```text
.exodus/
├── graph.json
├── architecture.json
├── plan.json
├── fallbacks.json
├── report.json
├── evaluation_scorecard.json
├── evaluation_scorecard.csv
├── evaluation_scorecard.md
├── contracts/
│   └── <unit-id>/
│       ├── behavioral-contract.json
│       └── verification.json
├── knowledge/
│   ├── index.json
│   ├── cases/
│   └── strategies/
└── runs/<run-id>/
    ├── trajectory.jsonl
    ├── diagnostics.json
    ├── metrics.json
    ├── events.jsonl
    ├── verification.json
    ├── worktree.json
    └── merge-proposal.json
```

Every artifact must be generated by actual execution or explicitly labeled as an example.

## 18. Fixtures and two-run learning demonstration

Maintain at least ten legal synthetic fixtures covering:

1. Typed functions.
2. Class conversion.
3. Module dependencies.
4. Circular dependencies.
5. Unsupported decorators.
6. Dynamic values.
7. Missing SDKs.
8. Async behavior.
9. Database compatibility wrappers.
10. Deliberately untranslatable reflection.

Each fixture must contain an explicit behavioral contract. Where possible, execute source and target implementations with the same inputs and compare normalized outputs.

Each applicable fixture must also expose unit-level contracts so the evaluation can distinguish localized unit success from component, module, and repository integration success.

Add a genuine two-run case-learning experiment:

- **Repository A** contains the first failure, creates a case, receives a verified repair, requires human approval, and produces a promoted strategy and regression fixture.
- **Repository B** uses different names and surrounding structure but contains the same underlying semantic pattern. It must remain outside the knowledge library until evaluation begins.
- The retrieval engine must find the promoted case structurally, not through filenames or hard-coded expected solutions.

A suitable pattern is callback success/error semantics incorrectly converted to Rust async behavior and repaired through `Result<T, E>`.

Every promoted case creates a regression fixture under a case-owned directory and must pass `exodus cases test --mode verify`.

## 19. Fair evaluation

Primary metric:

> Percentage of behavioral contracts passed by generated target repositories.

Secondary metrics:

- Unit behavioral-contract pass rate.
- Percentage of units with grounded contract oracles.
- Failure-localization accuracy by ESG unit.
- Unit, component, module, and repository verification rates.
- Target compilation rate.
- Percentage of units behaviorally verified.
- Compatibility-wrapper, degraded-stub, and blocked counts.
- Human approvals and interventions.
- Repair attempts and compiler invocations.
- Runtime.
- Agent calls, tokens, and cost when genuinely measured.
- Successful-case retrieval rank.

The baseline must represent a real, documented basic approach: a direct single-agent migration, a general-purpose agent with basic tools, or a deterministic simple translator. Label it accurately.

The baseline and Exodus must receive the same source fixtures, target requirements, behavioral tests, and comparable model/resource limits. Preserve complete results, including failures. Do not hard-code scorecards.

For the two-run experiment, compare first encounter versus case-assisted unseen encounter using actual repair attempts, time to verified result, agent calls, compiler invocations, human interventions, and behavioral tests.

Keep evaluation cases separate from promoted knowledge until their designated run to prevent leakage.

## 20. Reproducibility and documentation

Provide exact clean-environment instructions for:

- Toolchain installation and verification.
- Workspace build, format, lint, and tests.
- Baseline execution.
- Exodus evaluation.
- End-to-end migration and approval.
- Unit contract generation and isolated verification.
- Case replay and full verification.
- Worktree inspection and cleanup.
- OpenWiki reading and optional update.

Use repository-relative Markdown links. Remove machine-specific `file:///home/...` links.

Document approximate runtime and cost only after measurement, with environment and methodology. Separate measured values from estimates.

Provide one-command paths such as `just demo` and `just eval`, or documented equivalents when `just` is unavailable.

Required submission artifacts:

- Complete solution code.
- Improvement changelog tied to evidence.
- Clean reproduction guide.
- Up-to-five-minute solution video script.
- Captured representative trajectories for every actual agent role.
- Evaluation results and raw evidence.
- OpenWiki/OKF knowledge base.
- Main failure mode and practical hot take.

## 21. Continuous 20-hour completion plan

Use this schedule for the reported existing implementation. If a subsystem is already complete, verify it and move the saved time to the next incomplete milestone.

### Hour 0-1: Audit and reproduce

- Inspect code, Git state, OpenWiki, schemas, fixtures, and docs.
- Run formatter, clippy, tests, CLI workflow, and evaluation.
- Build the requirement matrix.
- Identify hard-coded or unsupported claims.

### Hour 1-3: Evidence and reproducibility repair

- Pin toolchains and OpenWiki.
- Fix portable links and commands.
- Replace fabricated/sample evidence with captured output or label it clearly.
- Lock artifact schemas and versioning.

### Hour 3-6: Worktree isolation

- Implement repository preflight, worktree manager, path validation, leases, and crash recovery.
- Move transformation and verification into linked worktrees.
- Add safe cleanup and integration tests.

### Hour 6-9: Unit contracts and localized verification

- Extract stable unit boundaries and dependency clusters from the ESG.
- Ground contract assertions in source tests or differential execution.
- Generate isolated harnesses and verification artifacts.
- Commit verified units or the smallest buildable verified batch.

### Hour 9-12: Case and knowledge engines

- Implement case models, lifecycle, JSON storage, fingerprints, retrieval, approval, and promotion.
- Preserve failed strategies.
- Add sanitization metadata and regression generation.

### Hour 12-14: Parallel strategies and merge proposals

- Implement bounded parallel attempt worktrees.
- Isolate build outputs.
- Rank results by correctness first.
- Generate inspectable merge proposals without auto-merging.

### Hour 14-15: Agent integration and real trajectories

- Validate providers and structured responses.
- Capture real architecture, migration, and repair trajectories.
- Record only genuinely measured tokens, latency, and cost.

### Hour 15-17: Fixtures and evaluation

- Run all ten fixtures.
- Implement and execute the two-run unseen learning experiment.
- Run fair baseline and Exodus evaluations.
- Generate scorecards from raw executions.

### Hour 17-18: OpenWiki update

- Update stable knowledge pages and claims.
- Document worktree and case lifecycles.
- Validate OKF v0.2 and evidence links.

### Hour 18-19: Clean reproduction

- Test documented commands from a clean checkout or equivalent isolated environment.
- Run case replay, case verification, migration, evaluation, and worktree recovery paths.

### Hour 19-20: Submission hardening

- Run all quality gates.
- Inspect every artifact and diff.
- Remove secrets and machine-specific paths.
- Reconcile README, reproduction guide, video script, trajectories, OpenWiki, and measured results.
- Record honest limitations and migration debt.

## 22. Definition of done

Do not declare completion until:

- The existing implementation claims have been independently reproduced or corrected.
- A multi-file source repository produces an evidence-linked ESG.
- Dependencies, cycles, risk, and migration order are calculated.
- Migration requires approval.
- All mutations and verification occur in Exodus-owned worktrees.
- The primary user working tree remains unchanged.
- Supported constructs become target code.
- Unsupported constructs create explicit migration debt.
- The ESG is decomposed into stable unit or smallest-valid-cluster boundaries.
- Behavioral assertions are grounded in source evidence rather than inferred from signatures alone.
- Each eligible migrated unit passes an isolated contract before it is labeled verified.
- Unit, component, module, repository, and end-to-end verification statuses remain distinct.
- Verified unit work remains recoverable when a later unit fails.
- Generated code is formatted, compiled, and behaviorally tested.
- Repairs are bounded and policy controlled.
- Risky cases require human review.
- A failure becomes a versioned migration case.
- A verified case requires approval before promotion.
- An unseen structurally similar fixture reuses the promoted case and reports actual improvement.
- Promoted cases generate and pass regression fixtures.
- Parallel attempts remain isolated and produce an approval-ready merge proposal.
- No winning branch is auto-merged without approval.
- Agent trajectories come from real executions or are labeled examples.
- Baseline and Exodus run on the same documented cases.
- Scorecards are computed from raw results.
- Toolchains and artifact schemas are pinned/versioned.
- A clean reproduction path succeeds.
- OpenWiki accurately reflects the final code and evidence in OKF v0.2.
- No secrets, absolute local links, fabricated metrics, or misleading success claims remain.

Begin now by auditing the repository and reproducing the supplied completion report. Do not rebuild verified components unnecessarily. Prioritize missing worktree isolation, governed case learning, real evidence capture, and evaluation integrity, then run the complete system from a clean environment.

## Architectural Mandate: Unified Business-to-Execution Control Plane

Ensure the codebase reflects that Project Exodus is a unified agentic platform bridging software execution and critical business requirements across three tagged operational domains:

1. Unified Event Model:
* Every task (engineering bug, CRM request, or survey mapping) is an `OperationalItem` moving through an identical, auditable lifecycle:
Captured -> Sandboxed -> Contract-Verified -> Human-Approved -> Promoted.

2. Deterministic Verification Over Heuristics:
* For #prod-bug: Verification is enforced via isolated unit compilation and test execution inside Git worktrees.
* For #crm-request: Verification is enforced via policy graphs and schema constraints in SurrealDB.
* For #survey-mapping: Verification is enforced via taxonomy graph completeness and foreign key integrity.

3. Cross-Functional Web UI:
* The embedded UI (Axum + static web assets) serves as the universal Human-in-the-Loop (HITL) gate.
* It provides side-by-side Git diffs for engineering, policy-impact matrices for commercial operations, and visual taxonomy alignment for product teams.
* Approval actions must explicitly promote verified resolutions into SurrealDB to expand the system's organizational memory.

4. Alignment with Existing Infrastructure:
* Zero external server dependencies: Embedded SurrealDB (SurrealKV) runs entirely local to the project.
* Works natively with standard Git repositories, standard toolchains (Cargo, JDK), and standard browser environments.

### Strategic Hackathon Positioning

By framing the system this way, the project moves beyond a simple coding assistant or transpile script:

* **For Rubric Criterion 1 (Problem & User Value — 15 pts):** Solves an organizational problem that causes friction across engineering, product, and operations teams daily.
* **For Rubric Criterion 2 (Agent Engineering — 30 pts):** Demonstrates a multi-domain agentic architecture featuring sandboxed Git worktrees, AST traversal, embedded multi-model graph persistence, and strict verification loops.
* **For Rubric Criterion 6 (Architectural Lessons — 5 pts):** Provides a compelling counter-narrative: *AI agents achieve production reliability not by expanding conversational prompt windows, but by operating within a strictly governed, cross-functional execution harness backed by deterministic verification and compounding memory*.

