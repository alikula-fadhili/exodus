---
okf_version: "0.2"
type: reference
status: planned
sources:
  - openwiki/INSTRUCTIONS.md
  - openwiki/project-overview.md
---

# Glossary

[Index](index.md) | [Project Overview](project-overview.md) | [Semantic Graph Specification](semantic-graph-specification.md)

* **ESG (Exodus Semantic Graph)**: The language-neutral graph representation of legacy code symbols, dependencies, and risk metrics.
* **OKF (Open Knowledge Format)**: Standardized schema (v0.2) for repository knowledge graphs and concept documentation.
* **Migration Debt**: Explicit, reviewable stubs (`todo!`) and annotations emitted when automated conversion cannot resolve a construct.
* **Verified Outcome**: Migration outcome where target code compiles and passes behavioral equivalence test contracts.
* **Degraded Outcome**: Migration outcome where target code compiles by utilizing explicit fallback stubs.
* **Blocked Outcome**: Migration outcome requiring human engineering intervention due to unsolvable semantics.
