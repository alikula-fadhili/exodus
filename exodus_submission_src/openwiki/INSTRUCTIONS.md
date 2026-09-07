# OpenWiki Documentation Contract: Project Exodus

This document defines the strict documentation and knowledge contract for Project Exodus that all maintainers, human contributors, and agentic workflows must obey.

## Fundamental Rules

1. **Project Mission**: Project Exodus is a graph-guided, agent-assisted legacy code migration engine designed to reliably transform legacy source codebases into modern target languages while preserving behavioral correctness.
2. **Evidence-Based Documentation**: Document only behavior supported by source code, tests, or generated evidence. Speculative or assumed features must never be presented as implemented.
3. **Status Distinction**: Clearly distinguish `implemented`, `experimental`, `planned`, `degraded`, and `blocked` functionality across all documents and concept nodes.
4. **Supporting Evidence**: Every important architectural claim must identify supporting repository evidence (e.g. source paths, test suites, or execution artifacts).
5. **Compilation vs Correctness**: Compilation alone is not behavioral correctness. Code must satisfy behavioral test contracts and equivalence criteria to be deemed correct.
6. **No False Verification**: A stubbed migration must never be documented as verified. Unimplemented or fallback constructs represent explicit migration debt.
7. **Semantic Graph Role**: The Exodus Semantic Graph (ESG) is the program-semantics model representing parsed symbols, call graphs, type dependencies, and migration risks.
8. **Knowledge Graph Role**: OpenWiki is the project knowledge and documentation model adhering to the Open Knowledge Format v0.2 (OKF v0.2).
9. **Model Separation**: The Exodus Semantic Graph and the OpenWiki documentation graph must not be presented as interchangeable; each serves a distinct domain.
10. **Execution Evidence Directory**: `.exodus/` contains structured execution evidence, runs, schemas, metrics, and machine-readable migration logs.
11. **Behavioral Grounding**: Tests and behavioral contracts establish correctness. Transformations without behavioral verification are incomplete.
12. **Human Approval Gate**: Human approval is mandatory for unsafe, ambiguous, or consequential transformations before code execution or application.
13. **Data Protection**: Credentials, private source material, proprietary keys, and secret tokens must never enter documentation or committed wiki files.
14. **Exact Reproduction**: Reproduction commands must be exact, clean-environment executable, and version-pinned.
15. **Visible Migration Debt**: Known limitations, unsupported semantics, and accumulated migration debt must remain prominent, measurable, and reviewable.
