# Project Exodus: Architecture Roadmap

This document outlines the post-hackathon vision and future engineering milestones for **Project Exodus**.

---

## Post-Hackathon Capabilities & Enhancements

### 1. Dynamic Model Routing & Live AI Cost Advisory (`exodus-cost` Pro)
* **Live Provider Integration**: Direct streaming support for Gemini 1.5 Pro/Flash, Claude 3.7 Sonnet (Thinking), and OpenAI o3-mini.
* **Semantic AST Cost Advisor**: LLM-driven architectural complexity assessment to automatically downgrade trivial leaf modules to fast/cheap models and reserve high-reasoning frontier models for cyclic clusters.
* **Dynamic Token-Bucket Rate Limiter**: Proactive backoff avoiding HTTP 429 rate limit errors when dispatching 100+ concurrent subagent translation tasks.

### 2. Multi-Language Target Ecosystem (`exodus-kernel` Plugins)
* **Python to Go**: Automatic struct pointer vs value semantics, goroutine channel concurrency mapping.
* **JavaScript/TypeScript to Rust**: Translating npm ecosystem packages and async promises to Tokio/Axum primitives.
* **Java to Modern C++ / Rust**: Memory management transition from JVM GC to RAII and affine type ownership.

### 3. Native Slint Desktop & Web UI
* Real-time 3D ESG dependency graph visualizer.
* Interactive wave scheduler with drag-and-drop checkpoint approvals.
* Live diff inspector with side-by-side AST highlight and compiler error overlay.

### 4. Continuous Repository Migration & GitHub Bot
* GitHub App / Action for continuous PR modernizations.
* Automated triage bot converting unsupported legacy constructs directly into triage issues with reproducible test bundles.
