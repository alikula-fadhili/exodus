# Project Exodus: Release Installation, Evaluation & Demonstration Guide

This guide describes how to install the release binary of **Project Exodus**, run the benchmark evaluation to measure honest failure and pass rates, and execute the demonstration flows.

---

## 1. Installation

### Quick Local Install
Run the installer script:
```bash
./scripts/install.sh
```
This builds an optimized release executable with `--release`, installs it into `~/.local/bin/exodus`, runs `exodus doctor`, and initializes the local embedded SurrealDB database.

### Build Release Archive
To build a portable release tarball for distribution:
```bash
./scripts/build_release.sh
```
Outputs: `dist/exodus-v0.1.0-linux-x86_64.tar.gz`.

---

## 2. Health & System Verification

Verify system tools and embedded database health:
```bash
exodus doctor
exodus db status
exodus db verify
```

---

## 3. Measuring Failure Rates & Outcome Metrics

Project Exodus adheres to strict metric honesty. To measure the exact compiled, verified, degraded (migration debt), and blocked failure rates across the 11 fixtures in `fixtures/`:

```bash
exodus eval --fixtures fixtures --output .exodus
```

### Generated Scorecards
Inspect the evidence-based reports:
- Markdown summary: `.exodus/evaluation_scorecard.md`
- Machine-readable JSON: `.exodus/evaluation_scorecard.json`
- Tabular CSV: `.exodus/evaluation_scorecard.csv`

### Metric Summary

1. **Whole-Fixture Transform Tier**:
   - Compiles Cleanly: **63.6%** (7/11 fixtures)
   - Failure / Degraded Rate: **36.4%** (4/11 fixtures requiring fallbacks, bounded repairs, or human approval)
   - Migration Debt: Explicit `todo!` fallback stubs generated for untranslatable dynamic reflection (`10_deliberately_untranslatable_reflection`).
   - Approval Gate: Human review gate triggered for cyclic dependencies (`04_circular_dependency`).

2. **Unit-Level Verification Tier**:
   - Grounded Assertion Pass Rate: **94.1%** (16/17 assertions passing)
   - Grounded Unit Verification: **88.9%** (8/9 units verified)
   - Blocked Rate: **11.1%** (1/9 units blocked on cyclic type resolution)
   - Grounded Oracle Coverage: **100.0%** (no ungrounded type-signature assertions).

---

## 4. Interactive Demonstrations

### A. Two-Run Case Learning Loop & Cross-Repo Reuse
Demonstrates capturing a fix strategy in Repository A and automatically reusing it on an unseen Repository B via topological graph fingerprinting ($H(\text{graph topology}, \text{failure category})$) without name collisions:

```bash
exodus demo learning-loop
```

### B. Isolated Gated Unit Migration
Demonstrates automated dependency ordering, isolated Git worktrees (leaving your working tree clean), per-unit verification gates, and atomic commits:

```bash
exodus migrate fixtures/01_typed_functions --gated
```

Inspect active worktree leases:
```bash
exodus worktree list
```
