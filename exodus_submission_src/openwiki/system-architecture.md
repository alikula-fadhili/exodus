---
okf_version: "0.2"
type: architecture
status: planned
sources:
  - Cargo.toml
  - crates/exodus-core/src/lib.rs
  - crates/exodus-graph/src/lib.rs
---

# System Architecture

[Index](index.md) | [Semantic Graph](semantic-graph-specification.md) | [Crates](../Cargo.toml)

Exodus is architected as a modular Rust workspace divided into decoupled pipeline crates:

```text
crates/
├── exodus-core/       # Domain primitives, outcome types, errors
├── exodus-parser/     # Tree-sitter frontend abstractions
├── exodus-graph/      # Exodus Semantic Graph (ESG) structures and algorithms
├── exodus-planner/    # Topological wave planner and risk evaluation
├── exodus-agent/      # Bounded agent orchestration and prompt harnesses
├── exodus-transform/  # Semantic code generation and AST transformation
├── exodus-fallback/   # Fallback stub generation and debt emission
├── exodus-verifier/   # Compiler, linter, and test verification runners
├── exodus-eval/       # Outcome metrics and benchmark aggregation
└── exodus-cli/        # Main CLI entry point
```

## Data Flow & Migration Lifecycle

```text
Evidence Sources (Code, Bash, Containers, CI, Terraform, Docs)
   │
   ▼ [Sense Phase: SenseOrchestrator & Multi-Evidence Adapters]
MigrationIntentGraph (Sensed Claims, Contradictions, Traceability)
   │
   ▼ [Human Approval Gate: Terminal Interactive Review / Plan Checkpoints]
MigrationIntentContract (Approved Architectural Decisions & Rules)
   │
   ▼ [exodus-parser & exodus-graph]
Exodus Semantic Graph (ESG)
   │
   ▼ [exodus-planner (with Intent & TargetPathPlan & ConcurrencyPlan)]
Migration Plan (Waves, Risk, Approval Checkpoints, Traceability Manifest)
   │
   ▼ [exodus-transform & exodus-fallback & Dynamic Target Language Registry]
Target Source Code + Fallback Stubs (TypeScript -> Go, Python -> Rust, etc.)
   │
   ▼ [exodus-verifier (UniversalTargetVerifier) <-> exodus-agent (Bounded Repair <= 3)]
Verified Target Workspace & Metric Scorecards
   │
   ▼ [exodus-eval & exodus-store (Embedded SurrealKV v5)]
Outcome Classification & Durable State Persistence (.exodus/data/surreal/)
```

## Core Architectural Subsystems

1. **Universal Polyglot Foundation (`exodus-core`)**:
   - Universal string-backed `LanguageId` with normalization and transparent string comparison traits.
   - Dynamic `TargetLanguageSpecRecord` storing signature transformation rules, manifest templates, and toolchain configurations in embedded SurrealKV.
   - Time-ordered UUIDv7 identifiers across entities.

2. **Sense & Repository-Wide Migration Intent (`exodus-parser::sense`)**:
   - Multi-evidence extraction: `TextFirstSegmenter`, `BashEvidenceAdapter`, `ContainerEvidenceAdapter`, `WorkflowEvidenceAdapter`, `TerraformEvidenceAdapter`, and `DocumentationEvidenceAdapter`.
   - `SenseOrchestrator` reconciles cross-source claims and surfaces architectural contradictions.
   - `SecretScrubber` redacts tokens, credentials, and private keys prior to hashing or prompt generation.

3. **Agent-Led Interactive Review Gate (`exodus-agent`, `exodus-cli`)**:
   - Terminal interactive review cards for critical and contradictory claims.
   - Strict TTY gating (`std::io::stdin().is_terminal()`), alternative branching, explicit refusal to treat empty `<Enter>` as consent.
   - Non-interactive batch execution writes pending decisions to `.exodus/plan.json` and marks affected units as `Blocked`.

4. **Planning & Verifier Pipeline (`exodus-planner`, `exodus-verifier`)**:
   - `TargetPathPlan` enforcing output root preservation and duplicate/directory collision detection.
   - `ConcurrencyMappingPlan` mapping runtime primitives (e.g., `Promise.all` -> `golang.org/x/sync/errgroup`).
   - `UniversalTargetVerifier` supporting multi-target toolchains (Rust `cargo`, Go `gofmt`/`go test`, etc.) with strict metric honesty.

