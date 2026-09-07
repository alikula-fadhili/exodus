---
okf_version: "0.2"
type: reference
status: implemented
sources:
  - crates/exodus-cli/src/main.rs
  - Cargo.toml
---

# CLI Reference

[Index](index.md) | [Reproduction Guide](reproduction-guide.md) | [Artifact Schemas](artifact-schemas.md) | [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md)

The `exodus` command-line binary (`cargo run -p exodus-cli --`). Every subcommand accepts a global
`--json` flag for machine-readable output; commands that touch a repository take positional
arguments (not `--path`-style flags) unless noted.

## Repository-level pipeline

* `exodus sense [source] [-o|--output <dir>]` — runs the repository-wide Sense phase across polyglot source code, Bash/Python automation, Dockerfiles, Compose specs, CI workflows, Terraform modules, documentation, and damaged fragments, emitting `intent_graph.json` and persisting claims in embedded SurrealDB.
* `exodus intent show [source]` — inspects reconstructed migration intent claims, category breakdowns, and evidence traces.
* `exodus intent conflicts [source]` — identifies contradictory or provisional intent claims requiring resolution.
* `exodus intent review [source] / exodus review [source]` — launches the agent-led sequential terminal review panel with redacted evidence cards, alternative choices, and immediate SurrealDB persistence.
* `exodus analyze [source] [-o|--output <dir>]` — parses the repository and writes `graph.json`/`architecture.json`.
* `exodus plan [source] [-o|--output <dir>]` — generates `plan.json` incorporating the `MigrationIntentContract`, `TargetPathPlan`, and `ConcurrencyMappingPlan`.
* `exodus approve [plan] [-a|--approver <name>]` — marks a plan approved.
* `exodus migrate [source] [-o|--output <dir>] [--force] [--gated]` — transforms and scaffolds a
  target crate. `--gated` routes through the real per-unit verification gate instead (dependency-
  ordered unit-by-unit generate/compile/contract-verify inside an Exodus-owned worktree, with a
  real atomic commit per verified unit and a merge proposal) — see
  [Unit-Level Migration and Behavioral-Contract Verification](unit-verification.md).
* `exodus verify [target]` — runs `rustfmt`/`cargo check`/`cargo test` on a scaffolded target.
* `exodus report [-d|--dir <dir>]` — prints the verification hierarchy (unit tier, aggregated from
  real `.exodus/contracts/*/verification.json` artifacts; module-integration tier, from
  `report.json`; and a note that repository/end-to-end tiers are not implied by either) plus the
  existing report/fallbacks/architecture summaries.
* `exodus eval [-f|--fixtures <dir>] [-o|--output <dir>]` — runs the full benchmark suite: the
  whole-fixture transform tier (Exodus vs. a real, executed naive baseline — never a hardcoded
  per-fixture guess) plus the unit-level tier for every fixture with a grounded `contracts.json`.
  See [Evaluation Methodology](evaluation-methodology.md).

## Units — ESG boundary and per-unit gate

* `exodus units list [source]` — lists dependency-ordered `VerificationBoundary`s (`Unit` or
  `Cluster`, with the cluster's grouping reason).
* `exodus units show <unit-id> [-s|--source <dir>]` — shows a boundary's member nodes and whether a
  grounded contract exists for it.
* `exodus units verify <unit-id> [-s|--source <dir>]` — runs the real per-unit gate for one boundary
  (in a scratch directory, not a full worktree — this is an ad hoc check, not a migration run) and
  writes `.exodus/contracts/<unit-id>/{behavioral-contract,verification}.json`.

## Contracts

* `exodus contracts show <unit-id> [-s|--source <dir>]` — prints the real generated contract if one
  exists; otherwise the grounded-but-not-yet-verified source contract if `<source>/contracts.json`
  has one; otherwise reports "not generated" honestly. Never fabricates a contract.
* `exodus contracts verify <unit-id> [-s|--source <dir>]` — runs the gate and reports real
  assertion-level pass/fail counts.

## Cases — governed Migration Case Engine

* `exodus cases list` / `exodus cases show <case-id>`
* `exodus cases search <query>` — free-text structural search across every *promoted* case's unit
  ID, failure description, and fingerprint (not filtered to one hardcoded failure category).
* `exodus cases review <case-id>` — prints the case's real evidence (diagnostic text, failed
  assertion, confidence statement).
* `exodus cases approve <case-id> [-a|--approver <name>]` / `exodus cases reject <case-id>`
* `exodus cases test --mode replay` — schema-validates every promoted case, recomputes its
  structural fingerprint from its own stored ESG subgraph and checks it against the stored value,
  and confirms it's retrievable via structural search — no network inference, no target generation.
* `exodus cases test --mode verify` — would regenerate the target and re-run the real gate against
  each promoted case's regression fixture; honestly reports that regression-fixture generation on
  promotion isn't implemented yet rather than fabricating a pass.

## Worktree — Git isolation and leasing

* `exodus worktree list` / `exodus worktree inspect <task-id>`
* `exodus worktree resume <task-id>` — shows a lease's real on-disk/status state.
* `exodus worktree preserve <task-id>` — marks a lease `DirtyReviewRequired` and persists it.
* `exodus worktree cleanup <task-id>` — removes the worktree only if it isn't dirty; a dirty
  worktree is preserved and reported, not force-deleted.
* `exodus worktree discard <task-id> --confirm <task-id>` — force-removes even a dirty worktree,
  gated on repeating the task ID.
