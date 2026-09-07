# Phase 1: Pre-Review Preparation Checklist

## Overview

This checklist tracks completion of all preparation activities for the Project Exodus review. Each item must be verified before proceeding to Phase 2.

**Phase Owner**: Review Team
**Target Completion**: 2026-08-31 to 2026-09-01
**Status**: In Progress

---

## 1. Environment Setup

| ID | Task | Command | Status | Notes |
|---|---|---|---|---|
| E1.1 | Verify Rust toolchain | `rustc --version`, `cargo --version` | ⬜ | Requires Rust 1.98+ |
| E1.2 | Verify Git version | `git --version` | ⬜ | Requires Git 2.55+ |
| E1.3 | Install clippy | `rustup component add clippy` | ⬜ | |
| E1.4 | Install rustfmt | `rustup component add rustfmt` | ⬜ | |
| E1.5 | Install cargo-audit | `cargo install cargo-audit` | ⬜ | |
| E1.6 | Install cargo-tarpaulin | `cargo install cargo-tarpaulin` | ⬜ | For coverage |
| E1.7 | Install cargo-deps | `cargo install cargo-deps` | ⬜ | For dependency analysis |
| E1.8 | Install cargo-udeps | `cargo install cargo-udeps` | ⬜ | For unused deps |
| E1.9 | Install cargo-count | `cargo install cargo-count` | ⬜ | For code metrics |
| E1.10 | Clean workspace | `cargo clean` | ⬜ | Start fresh |

---

## 2. Build Verification

| ID | Task | Command | Expected | Status | Notes |
|---|---|---|---|---|---|
| B2.1 | Debug build | `cargo build --workspace` | Success | ⬜ | |
| B2.2 | Debug build time | `time cargo build --workspace` | < 10 min | ⬜ | Document actual |
| B2.3 | Release build | `cargo build --release --bin exodus` | Success | ⬜ | |
| B2.4 | Release build time | `time cargo build --release --bin exodus` | < 15 min | ⬜ | Document actual |
| B2.5 | Release binary size | `du -sh target/release/exodus` | < 50 MB | ⬜ | Document actual |
| B2.6 | All targets build | `cargo build --workspace --all-targets` | Success | ⬜ | Includes examples, tests |

---

## 3. Quality Gates

| ID | Task | Command | Expected | Status | Notes |
|---|---|---|---|---|---|
| Q3.1 | Format check | `cargo fmt --check` | 0 changes | ⬜ | |
| Q3.2 | Clippy check | `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings | ⬜ | |
| Q3.3 | Test suite | `cargo test --workspace` | 67 passed | ⬜ | |
| Q3.4 | Test suite time | `time cargo test --workspace` | < 15 min | ⬜ | Document actual |
| Q3.5 | Test with features | `cargo test --workspace --all-features` | Success | ⬜ | |
| Q3.6 | Doc tests | `cargo test --workspace --doc` | Success | ⬜ | |

---

## 4. Metrics Collection

| ID | Task | Command | Output File | Status | Notes |
|---|---|---|---|---|---|
| M4.1 | Line count | `cloc .` or `tokei` | `baseline_metrics.json` | ⬜ | |
| M4.2 | Dependency tree | `cargo tree --all` | `dependency_tree.txt` | ⬜ | |
| M4.3 | Dependency graph | `cargo deps --all` | `dependency_graph.json` | ⬜ | |
| M4.4 | Code complexity | `cargo count --all` | `code_metrics.json` | ⬜ | |
| M4.5 | Test coverage | `cargo tarpaulin --workspace` | `coverage_report.html` | ⬜ | May be slow |

---

## 5. Inventory

| ID | Task | Method | Output File | Status | Notes |
|---|---|---|---|---|---|
| I5.1 | List all crates | `ls crates/` | `inventory.csv` | ⬜ | |
| I5.2 | Crate versions | Parse `Cargo.toml` files | `inventory.csv` | ⬜ | |
| I5.3 | Crate dependencies | Parse `Cargo.toml` files | `inventory.csv` | ⬜ | |
| I5.4 | Test counts | Count `#[test]` annotations | `inventory.csv` | ⬜ | |
| I5.5 | Binary targets | List all binaries | `inventory.csv` | ⬜ | |
| I5.6 | Feature flags | List all features | `inventory.csv` | ⬜ | |

---

## 6. Evaluation Suite

| ID | Task | Command | Expected | Status | Notes |
|---|---|---|---|---|---|
| E6.1 | Run evaluation | `cargo run -p exodus-cli -- eval --fixtures fixtures --output .exodus` | Success | ⬜ | |
| E6.2 | Verify scorecard | Check `.exodus/evaluation_scorecard.md` | Generated | ⬜ | |
| E6.3 | Verify JSON output | Check `.exodus/evaluation_scorecard.json` | Generated | ⬜ | |
| E6.4 | Verify CSV output | Check `.exodus/evaluation_scorecard.csv` | Generated | ⬜ | |
| E6.5 | Evaluation runtime | `time cargo run -p exodus-cli -- eval ...` | < 60 sec | ⬜ | Document actual |

---

## 7. End-to-End Pipeline

| ID | Task | Commands | Expected | Status | Notes |
|---|---|---|---|---|---|
| P7.1 | Analyze fixture | `cargo run -p exodus-cli -- analyze fixtures/01_typed_functions --output .exodus` | Success | ⬜ | |
| P7.2 | Plan fixture | `cargo run -p exodus-cli -- plan fixtures/01_typed_functions --output .exodus` | Success | ⬜ | |
| P7.3 | Approve plan | `cargo run -p exodus-cli -- approve .exodus/plan.json --approver "Reviewer"` | Success | ⬜ | |
| P7.4 | Migrate fixture | `cargo run -p exodus-cli -- migrate fixtures/01_typed_functions --output target/test_mig` | Success | ⬜ | |
| P7.5 | Verify migration | `cargo run -p exodus-cli -- verify target/test_mig` | Success | ⬜ | |
| P7.6 | Generate report | `cargo run -p exodus-cli -- report --dir .exodus` | Success | ⬜ | |

---

## 8. Gated Migration

| ID | Task | Command | Expected | Status | Notes |
|---|---|---|---|---|---|
| G8.1 | Gated migration | `cargo run -p exodus-cli -- migrate fixtures/03_module_dependency --gated` | Success | ⬜ | |
| G8.2 | List worktrees | `cargo run -p exodus-cli -- worktree list` | Shows worktrees | ⬜ | |
| G8.3 | Inspect worktree | `cargo run -p exodus-cli -- worktree inspect <task-id>` | Shows details | ⬜ | Use actual task ID |

---

## 9. Case Engine

| ID | Task | Command | Expected | Status | Notes |
|---|---|---|---|---|---|
| C9.1 | List cases | `cargo run -p exodus-cli -- cases list` | Shows cases | ⬜ | |
| C9.2 | Test replay | `cargo run -p exodus-cli -- cases test --mode replay` | Success | ⬜ | |
| C9.3 | Test verify | `cargo run -p exodus-cli -- cases test --mode verify` | Success | ⬜ | |

---

## 10. Documentation Verification

| ID | Task | Document | Status | Notes |
|---|---|---|---|---|
| D10.1 | Verify README | `README.md` | ⬜ | Check all commands work |
| D10.2 | Verify REPRODUCTION | `REPRODUCTION.md` | ⬜ | Check all commands work |
| D10.3 | Verify ROADMAP | `ROADMAP.md` | ⬜ | Review for accuracy |
| D10.4 | Verify AGENTS.md | `AGENTS.md` | ⬜ | Review constraints |

---

## 11. Final Checklist

Before declaring Phase 1 complete, verify:

- [ ] All environment requirements met
- [ ] All builds succeed (debug and release)
- [ ] All quality gates pass (fmt, clippy, test)
- [ ] All metrics collected and documented
- [ ] All inventory items catalogued
- [ ] Evaluation suite runs successfully
- [ ] End-to-end pipeline works
- [ ] Gated migration works
- [ ] Case engine works
- [ ] All documentation verified
- [ ] Review environment is reproducible

---

## Completion Summary

| Category | Total | Completed | Pending | Failed |
|---|---|---|---|---|
| Environment | 10 | 0 | 10 | 0 |
| Build | 6 | 0 | 6 | 0 |
| Quality Gates | 6 | 0 | 6 | 0 |
| Metrics | 5 | 0 | 5 | 0 |
| Inventory | 6 | 0 | 6 | 0 |
| Evaluation | 5 | 0 | 5 | 0 |
| E2E Pipeline | 6 | 0 | 6 | 0 |
| Gated Migration | 3 | 0 | 3 | 0 |
| Case Engine | 3 | 0 | 3 | 0 |
| Documentation | 4 | 0 | 4 | 0 |
| **Total** | **54** | **0** | **54** | **0** |

**Completion**: 0% (0/54 tasks complete)

---

## Next Actions

- [ ] Assign Phase 1 owner
- [ ] Schedule Phase 1 start
- [ ] Execute tasks in order
- [ ] Update status as tasks complete
- [ ] Escalate any failed items

---

## Notes

Add any observations, issues, or context here:

```

```

---

**Last Updated**: 2026-08-30
**Updated By**: Mistral Vibe
