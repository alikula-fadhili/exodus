# Project Exodus: 5-Minute Solution Video Script & Storyboard

**Title**: Project Exodus: Graph-Guided, Agent-Assisted Legacy Code Modernization
**Target Length**: 4:30 - 5:00 minutes

---

### [0:00 - 0:45] Problem & The Intended User
* **Visual**: Show diagram of legacy Python codebase migrating to Rust with compiler errors.
* **Voiceover**: "Every enterprise engineering team wants to modernize legacy systems to Rust for safety and performance. But when developers use raw LLM prompts or naive agents, multi-file migrations fail over 70% of the time. Why? Direct prompting is blind to dependency cycles, hallucinates types, and worse—invents believable dummy code for dynamic reflection, creating silent production bugs."

---

### [0:45 - 1:30] Baseline Failure Demonstration
* **Visual**: Show baseline comparison failing on circular dependency and `eval()` reflection.
* **Voiceover**: "Here is the baseline: a direct single-agent prompt translating our 10 synthetic benchmark fixtures. It achieves only a 30% behavioral pass rate and 40% compilation rate. It fails completely on cross-file circular imports and dynamic constructs."

---

### [1:30 - 2:30] The Solution: Project Exodus Architecture
* **Visual**: Show the 10-crate modular architecture in terminal (`exodus-parser`, `exodus-graph`, `exodus-planner`, `exodus-verifier`).
* **Voiceover**: "Project Exodus solves this with a 10-stage graph-guided engine:
  1. Tree-sitter extracts AST symbols.
  2. We build the language-neutral Exodus Semantic Graph (ESG) with Tarjan SCC cycle detection.
  3. We generate wave-ordered migration plans with human approval gates.
  4. Deterministic transforms handle supported constructs.
  5. Unresolvable semantics are explicitly emitted as typed `todo!` fallbacks with Migration Debt tracking.
  6. A bounded in-loop verifier runs `cargo check` and `cargo test` with at most 3 repair iterations."

---

### [2:30 - 3:45] Live End-to-End CLI Demo
* **Visual**: Terminal screen running the CLI commands live.
* **Commands Shown**:
  1. `cargo run -p exodus-cli -- eval --fixtures fixtures`
  2. `cargo run -p exodus-cli -- analyze fixtures/01_typed_functions`
  3. `cargo run -p exodus-cli -- plan fixtures/01_typed_functions`
  4. `cargo run -p exodus-cli -- approve .exodus/plan.json`
  5. `cargo run -p exodus-cli -- migrate fixtures/01_typed_functions --output target/migrated_01`
  6. `cargo run -p exodus-cli -- verify target/migrated_01`
* **Voiceover**: "Watch the live execution. The evaluation harness tests all 10 fixtures in under a second. Notice the result: 100% compilation rate and 90% behavioral pass rate. For untranslatable reflection, Exodus safely emits explicit migration debt rather than breaking the build."

---

### [3:45 - 4:15] The Changelog & Experiment Removed
* **Visual**: Display the Improvement Changelog table.
* **Voiceover**: "Our Improvement Changelog shows how we progressed from 30% to 90% pass rate. One key experiment we removed: early on, we tried allowing the agent to guess dynamic types using raw `unsafe` Rust. We removed this immediately because safety is non-negotiable; instead, we adopted explicit typed fallback stubs."

---

### [4:15 - 5:00] Hot Take & Conclusion
* **Visual**: Display Hot Take slide and GitHub repository summary.
* **Voiceover**: "Our hot take: *An agent that refuses to lie and produces explicit, measurable migration debt is 10x more valuable in production than an agent that pretends to migrate 100% of code with silent runtime bugs.* Project Exodus provides reliable, verifiable code migration you can trust."
