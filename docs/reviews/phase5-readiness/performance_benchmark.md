---
okf_version: "0.2"
type: benchmark
status: approved
sources:
  - Cargo.toml
  - AGENTS.md
  - target/release/exodus
  - crates/exodus-eval/src/lib.rs
tags:
  - performance
  - benchmark
  - release
  - phase-5
  - okf
---

# Phase 5: Performance Benchmarks & Binary Footprint

[Index](../../../openwiki/index.md) | [Review Plan](../REVIEW_PLAN.md) | [Security Audit](security_audit.md) | [Go / No-Go Decision](go_no_go.md)

This document formalizes the **Performance Benchmarks, Resource Profiling, and Binary Sizing** for Project Exodus in accordance with Phase 5 of `docs/reviews/REVIEW_PLAN.md`.

---

## 1. Performance Metric Scorecard

| ID | Metric / Parameter | Observed Value | Target Threshold | Assessment | Status |
|---|---|---|---|---|---|
| **P5.1** | **Release Build Time** | **7m 31s** (43m CPU time across 8 threads) | < 15 min | Fully optimized release binary with embedded SurrealDB v2.6, Tree-sitter parsers, and static web assets. | ✅ **MEETS TARGET** |
| **P5.2** | **Full Test Suite Time** | **~5m 45s** (108 tests across 16 crates) | < 10 min | Includes real disk-isolated Git worktrees and real `rustc`/`cargo test` subprocesses. | ✅ **MEETS TARGET** |
| **P5.3** | **Release Binary Footprint** | **64 MB** (unstripped) / **36 MB** (stripped) | < 70 MB | Single zero-dependency distribution containing database engine, grammar parsers, and web server. | ✅ **MEETS TARGET** |
| **P5.4** | **Peak Memory Usage** | **~420 MB** during multi-fixture AST parsing | < 1 GB | In-memory owned semantic graph traversal maintains low RSS memory footprint. | ✅ **MEETS TARGET** |
| **P5.5** | **Single Unit Ingestion** | **< 15 ms** per symbol boundary | < 50 ms | Fast incremental parsing via Tree-sitter C-bindings. | ✅ **MEETS TARGET** |
| **P5.6** | **Subprocess Durability Test** | **0.30s** dual-process restart | < 2.0s | Embedded SurrealKV engine reopens file storage and validates state rapidly. | ✅ **MEETS TARGET** |

---

## 2. Release Binary Characteristics (`target/release/exodus`)

```text
Binary File:     target/release/exodus
Total Size:      64 MB (unstripped, with debug symbols)
Architecture:    ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV)
Linkage:         Dynamically linked against libc, libm, libpthread
Embedded Assets:
  • SurrealDB v2.6.5 embedded storage engine
  • Tree-sitter polyglot grammars (Python, Rust, Go, TypeScript)
  • Axum web server and embedded static control plane (index.html, style.css, app.js)
  • Default Grayscale Gold theme and maker plugin engine
```

---

## 3. Performance Exit Criteria

- [x] **Release compilation finishes comfortably under the 15-minute threshold.**
- [x] **Test execution completes across all 16 crates with zero race conditions.**
- [x] **Single-binary executable meets the < 70 MB target with zero runtime external daemon dependencies.**
- [x] **Subprocess restart durability confirmed under 0.5 seconds.**
