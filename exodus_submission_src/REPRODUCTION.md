# Project Exodus: Clean Environment Reproduction Guide

This guide lets any reviewer clone, build, run, and reproduce the benchmark results and migration
workflows from a clean environment. Every command below was actually run to produce the numbers
cited in `docs/reviews/unit-verification-audit.md`'s post-change section — none are guessed.

---

## 1. Prerequisites & Toolchain

* **Rust**: `1.98+` stable (`cargo`, `rustfmt`, `clippy`) — this repo was built/tested against `rustc 1.98.0`.
* **Git**: any recent version (worktree isolation requires the real `git` CLI).
* **Python 3** (only needed to *regenerate* fixture contracts via `scripts/ground_fixture_contracts.py`, not to run Exodus itself).
* **Node.js** (optional, for the OpenWiki graph visualizer only): `Node 22`.

```bash
rustc --version
cargo --version
git --version
```

---

## 2. Quality gates

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All three currently pass (38 tests, 0 failed, across 12 workspace crates). `cargo test --workspace`
is **slow by design** — roughly 10–11 minutes measured on this run — because several tests really
compile and behaviorally test freshly scaffolded Rust crates via real `cargo check`/`cargo test`
subprocesses (the unit-verification gate integration tests in `crates/exodus-verifier/tests/`, and
`crates/exodus-eval`'s benchmark-suite tests) rather than asserting against mocked/simulated output.
This is a deliberate trade: real evidence over fast-but-fake tests.

---

## 3. Benchmark evaluation (Exodus vs. a real, executed baseline)

```bash
cargo run -p exodus-cli -- eval --fixtures fixtures --output .exodus
```

Reports two tiers, kept separate (never blended into one success rate — see
[Evaluation Methodology](openwiki/evaluation-methodology.md)):

- **Whole-fixture transform tier**: Exodus's real transform output vs. a real, executed, naive
  non-graph-guided baseline (one `todo!()`-stub per function, no type mapping, no class support) —
  both sides are actually compiled via `cargo check`, never guessed by fixture name.
- **Unit-level tier**: the real per-unit verification gate run against every fixture with a
  grounded `contracts.json` (currently 5 of 11 fixtures — `01`–`04`, `08`).

Generated artifacts: `.exodus/evaluation_scorecard.{md,csv,json}`.

---

## 4. Regenerating grounded contracts (optional — already committed)

```bash
python3 scripts/ground_fixture_contracts.py
```

Actually imports and runs the real Python source for `fixtures/{01_typed_functions,
02_class_conversion, 03_module_dependency, 04_circular_dependency, 08_async_function}` and writes
each fixture's `contracts.json` from the genuine captured output (differential execution — see
[Unit-Level Migration and Behavioral-Contract Verification](openwiki/unit-verification.md)).

---

## 5. Whole-repository migration pipeline

```bash
cargo run -p exodus-cli -- analyze fixtures/01_typed_functions --output .exodus
cargo run -p exodus-cli -- plan fixtures/01_typed_functions --output .exodus
cargo run -p exodus-cli -- approve .exodus/plan.json --approver "LeadArchitect"
cargo run -p exodus-cli -- migrate fixtures/01_typed_functions --output target/migrated_01
cargo run -p exodus-cli -- verify target/migrated_01
cargo run -p exodus-cli -- report --dir .exodus
```

---

## 6. Real per-unit gate: worktree isolation, atomic commits, merge proposal

```bash
cargo run -p exodus-cli -- migrate fixtures/01_typed_functions --gated
```

Creates a real Exodus-owned Git worktree (never touches your primary working tree), runs the
dependency-ordered per-unit gate, commits atomically after each verified unit, and prints a merge
proposal. Inspect it directly:

```bash
cargo run -p exodus-cli -- worktree list
cargo run -p exodus-cli -- worktree inspect <task-id>
```

For a fixture with a real, currently-open transform-engine defect, run the same command against
`fixtures/02_class_conversion` — it reports one genuinely `Blocked` unit and a captured Migration
Case with the real `rustc` diagnostic, rather than a forced pass.

---

## 7. Ad hoc single-unit checks, contracts, and the Case Engine

```bash
cargo run -p exodus-cli -- units list fixtures/01_typed_functions
cargo run -p exodus-cli -- units verify function::math_ops::add --source fixtures/01_typed_functions
cargo run -p exodus-cli -- contracts show function::math_ops::add --source fixtures/01_typed_functions
cargo run -p exodus-cli -- cases list
cargo run -p exodus-cli -- cases test --mode replay
cargo run -p exodus-cli -- cases test --mode verify
```

---

## 8. Cost and runtime

No real LLM provider is wired into this environment — `AgentProvider` has exactly one
implementation, `MockAgentProvider` (deterministic, offline). There is no inference cost to measure
here, and none is claimed. Runtime figures are measured wall-clock time on the environment this
guide was written in (see §2 above for the test-suite figure); expect variance on different
hardware.
