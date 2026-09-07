# Claude Development Guide — Project Exodus

Welcome to Project Exodus. This file contains instructions and commands for AI coding assistants working in this repository.

## Commands

```bash
# Build and check all workspace crates
cargo check --workspace

# Run all workspace unit tests
cargo test --workspace

# Run formatting check
cargo fmt --check

# Format code
cargo fmt

# Update / export OpenWiki documentation
deno install
npm run wiki:visualize
```

## Code Guidelines

* **Rust Edition**: 2021 edition with Rust stable.
* **Workspace Crates**: Place domain logic in appropriate `crates/exodus-*` crates.
* **Error Handling**: Use `thiserror` for library crates and `anyhow` for CLI/applications.
* **Serialization**: Use `serde` with `derive` for all AST, graph, and plan models.
* **Logging**: Use `tracing` for structured diagnostic events.

## Documentation (Open Knowledge Format v0.2)

* All concept documents in `openwiki/` must have YAML frontmatter with `okf_version: "0.2"` and a valid `type`.
* Update `openwiki/INSTRUCTIONS.md` when introducing new documentation rules.
* Planned features must be labeled `status: planned` or `status: draft`.
