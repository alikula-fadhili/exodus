# Agent Operational Guidelines & Contract

This repository governs the implementation and execution of **Project Exodus**. All autonomous agents, LLM assistants, and automated contributors must adhere to this contract.

## Non-Negotiable Invariants

1. **Evidence-Based Operation & Strict Metric Honesty**: Never fabricate implementations, benchmark baselines, or verification statuses. All claims must be derived from actual subprocess execution (`cargo check`, `cargo test`, diff executions). A 27.3% measured compile rate with explicit debt/failure reasons is strictly preferred over a fabricated 100% claim.
2. **Outcome Classification**: Maintain strict separation between outcome tiers:
   * `Verified`: Passes compiler checks AND behavioral test contracts.
   * `Compatible`: Compiles cleanly without stubs, awaiting test suite validation.
   * `Degraded`: Relies on explicit `todo!` fallback stubs; recorded as `MigrationDebt`.
   * `Blocked`: Fails compilation or transformation after bounded repair attempts.
3. **Atomic Boundary & Grounded Oracles**: Classes and their methods must always group into a single verification unit. A type signature alone is never a grounded behavioral oracle (`is_grounded() == false`). Unit contract verification success does not imply module or repository integration success and must be reported on distinct tiers.
4. **Symbol-Agnostic Structural Hashing**: Case engine fingerprints must hash graph topology and failure categories while strictly excluding symbol names to enable genuine cross-repository pattern reuse.
5. **Bounded Repair**: Agents working on automated code repair must not exceed **3 iterations** on a single symbol. If unresolved, emit an explicit fallback stub and record migration debt.
6. **Human Approval Gate**: Never perform destructive changes, circular dependency breaking, or irreversible transformations without generating an approval request in `.exodus/plan.json`.
7. **Separation of Concerns**: Keep the **Exodus Semantic Graph (ESG)** (program semantics model) strictly decoupled from the **OpenWiki Knowledge Base** (documentation model).
8. **Secrets & Privacy**: Never commit or emit credentials, private keys, or confidential source snippets into documentation or public artifacts.

## Crates Responsibility Matrix

| Crate | Primary Focus |
|---|---|
| `exodus-core` | Shared domain models, `MigrationOutcome`, `MigrationDebt`, `BehavioralContract`, error types |
| `exodus-parser` | Tree-sitter grammars, AST traversal, symbol extraction |
| `exodus-graph` | ESG representation, cycle detection (Tarjan/FAS), topological sorting, unit clusters |
| `exodus-planner` | Wave generation, risk weighting, approval checkpoints |
| `exodus-agent` | Bounded repair controller, LLM prompt orchestration |
| `exodus-transform` | Semantic construct translation to idiomatic Rust |
| `exodus-fallback` | `todo!` stub generation, migration debt emission |
| `exodus-verifier` | Formatting (`rustfmt`), compilation (`rustc`), test runners (`cargo test`), unit gate |
| `exodus-case` | Governed Migration Case Engine, structural graph fingerprinting, promotion |
| `exodus-worktree` | Non-destructive Git worktree isolation, leases, crash recovery, atomic commits |
| `exodus-eval` | Evaluation metrics, benchmark scorecards, real executed baseline |
| `exodus-cli` | Command-line dispatch and execution flows |
