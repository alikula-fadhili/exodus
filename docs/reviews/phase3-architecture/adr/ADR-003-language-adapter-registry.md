---
okf_version: "0.2"
type: decision
status: approved
sources:
  - crates/exodus-core/src/lib.rs
  - crates/exodus-parser/src/lib.rs
tags:
  - adr
  - adapter-registry
  - tree-sitter
  - okf
---

# ADR-003: Language Adapter Registry Extensibility

[Index](../../../../../openwiki/index.md) | [Decisions](../decisions.md) | [System Architecture](../../../../../openwiki/system-architecture.md)

## Context
Project Exodus must ingest varied programming languages and frameworks without coupling parsing mechanics to pipeline orchestrators. Language detection and grammar dispatch must be dynamic, extensible, and thread-safe.

## Decision
We implemented the `AdapterRegistry` and `WorkspaceScanner` in `exodus-core` and `exodus-toolchain`:
1. Languages are identified by canonical `LanguageId` (`Python`, `Rust`, `TypeScript`, `Go`).
2. `SourceLanguageAdapter` defines AST symbol extraction, function boundary identification, and import dependency discovery via Tree-sitter.
3. Adapters register dynamically at kernel initialization and reject duplicate or ambiguous registrations.
4. Auto-detection scans repository manifests (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`) and binds the appropriate adapter chain automatically.

## Consequences
- **Positive**: Strict decoupling of language grammar parsers from core graph and verification crates.
- **Positive**: Clean error reporting when encountering unsupported syntax constructs rather than panics.
- **Negative**: Dynamic registration requires trait object dispatch (`Box<dyn SourceLanguageAdapter>`) for runtime polymorphism.
