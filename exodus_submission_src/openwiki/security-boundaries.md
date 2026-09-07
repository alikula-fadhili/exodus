---
okf_version: "0.2"
type: security
status: planned
sources:
  - openwiki/INSTRUCTIONS.md
  - .env.example
---

# Security Boundaries

[Index](index.md) | [Agent Boundaries](agent-boundaries.md) | [Reproduction Guide](reproduction-guide.md)

Exodus enforces strict isolation and privacy rules:

## Invariants
1. **Secrets Isolation**: API tokens and private credentials must reside exclusively in local `.env` files and never be committed or written to documentation.
2. **Execution Sandboxing**: Verification test runs and compiler executions must be executed in sandboxed container environments where untrusted legacy code is executed.
3. **No Dynamic Telemetry Leaks**: Prompt orchestrators must redact private proprietary literals before invoking remote model providers.
