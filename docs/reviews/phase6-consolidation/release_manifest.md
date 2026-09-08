---
okf_version: "0.2"
type: specification
status: approved
sources:
  - Cargo.toml
  - AGENTS.md
  - docs/reviews/REVIEW_PLAN.md
  - docs/reviews/phase6-consolidation/consolidated_report.md
tags:
  - release-manifest
  - deployment
  - binary-artifacts
  - roadmap
  - phase-6
  - okf
---

# Project Exodus: Release Manifest & Post-Review Deployment Guide

[OpenWiki Index](../../../openwiki/index.md) | [Review Plan](../REVIEW_PLAN.md) | [Consolidated Report](consolidated_report.md) | [Phase 5 Go/No-Go](../phase5-readiness/go_no_go.md)

---

## 1. Release Specification

- **Product**: Project Exodus
- **Release Version**: `0.1.0-rc1`
- **Edition**: Rust 2021
- **Target Architecture**: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- **License**: MIT / Apache-2.0 dual license

---

## 2. Binary Artifacts & Footprint

| Artifact | Type | Primary Path | Stripped Size | Purpose |
|---|---|---|---|---|
| **`exodus`** | Executable (CLI & Web Gateway) | `target/release/exodus` | ~64 MB | Core migration engine, planner, agent orchestrator, web HITL dashboard |
| **`exodus-desktop`** | Executable (Native GUI) | `target/release/exodus-desktop` | ~36 MB | Native graphical interface, Kanban board, interactive visual inspection |
| **`openwiki`** | Documentation Bundle | `openwiki/` | ~120 KB | Self-contained knowledge base adhering to OKF v0.2 |

### Build Command
```bash
cargo build --release --workspace
```

---

## 3. Environment & Runtime Prerequisites

| Dependency | Minimum Version | Recommended | Notes |
|---|---|---|---|
| **Operating System** | Linux (kernel 5.4+), macOS 12+, Windows 10+ | Ubuntu 22.04 LTS / macOS 14 | Linux requires `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev` for GUI |
| **Git** | 2.40.0+ | 2.45+ | Required for worktree isolation (`git worktree add/remove/prune`) |
| **Rust Toolchain** | 1.80.0+ | 1.98.0+ | Required by `exodus-verifier` for compilation gates |
| **Tree-sitter Grammars** | Bundled | Bundled via C bindings | Native tree-sitter parsers statically linked |

---

## 4. Post-Review Phase 2 Expansion Roadmap

With Phase 1 implementation formally verified and certified through the 6-phase review, the following milestones define the upcoming development cycle:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Project Exodus Phase 2 Expansion Roadmap                 │
└─────────────────────────────────────────────────────────────────────────────┘
  │
  ├── 1. Polyglot Source Expansion
  │   ├── TypeScript / ECMAScript AST parser & type inference adapter
  │   ├── Python 3.10+ AST parser & dynamic type stub generator
  │   └── Go 1.22+ package & interface translation adapter
  │
  ├── 2. Distributed Execution & Scalability
  │   ├── Distributed worktree allocation across remote runner nodes
  │   ├── Remote SurrealDB cluster synchronization for enterprise case reuse
  │   └── Distributed caching for verification artifacts
  │
  └── 3. Autonomous Feedback & Continuous Self-Tuning
      ├── Automated prompt tuning based on migration debt telemetry
      ├── Fine-tuned small language model (SLM) specialized for Rust idioms
      └── Live bidirectional IDE plugins (VS Code, JetBrains, Antigravity)
```

---

## 5. Deployment Verification Procedure

Prior to executing enterprise migrations in production environments:

1. **Self-Check Diagnostic**:
   ```bash
   ./target/release/exodus doctor
   ```
2. **Execute Benchmark Baseline**:
   ```bash
   ./target/release/exodus eval --benchmarks
   ```
3. **Launch Web HITL Gate (Local or Headless)**:
   ```bash
   ./target/release/exodus web --port 8080
   ```
4. **Inspect OpenWiki Integrity**:
   ```bash
   ./target/release/exodus wiki verify
   ```
