---
okf_version: "0.2"
type: reference
status: planned
sources:
  - package.json
  - rust-toolchain.toml
  - .nvmrc
---

# Reproduction Guide

[Index](index.md) | [CLI Reference](cli-reference.md) | [Artifact Schemas](artifact-schemas.md)

Reproduce and verify the Exodus development environment:

## Prerequisites
* **Rust**: Pinned via `rust-toolchain.toml` (stable, 2021 edition).
* **Node.js**: Pinned to 22 via `.nvmrc` (or Deno / Bun runtime for JS toolchains).

## Reproduction Commands
```bash
# Install pinned JavaScript / OpenWiki dependencies
deno install

# Check formatting, compile, and run test suites across all crates
cargo fmt --check
cargo check --workspace
cargo test --workspace

# Update and visualize OpenWiki knowledge graph
npm run wiki:update
npm run wiki:visualize
```
