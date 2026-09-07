---
okf_version: "0.2"
type: component
status: planned
sources:
  - crates/exodus-parser/src/lib.rs
  - crates/exodus-core/src/lib.rs
---

# Parsing Architecture

[Index](index.md) | [Semantic Graph Specification](semantic-graph-specification.md) | [Parser Crate](../crates/exodus-parser/Cargo.toml)

The parsing architecture in `exodus-parser` decouples language frontends from downstream graph construction using tree-sitter grammars.

## Strategy
1. **Grammar Ingestion**: Load tree-sitter grammars (e.g. `tree-sitter-python`).
2. **CST Extraction**: Traverse Concrete Syntax Tree to discover scopes, definitions, and calls.
3. **Symbol Table Generation**: Normalize symbols into canonical URI strings.
4. **AST Queries**: Execute tree-sitter query patterns to extract docstrings, type annotations, and control flow blocks.
