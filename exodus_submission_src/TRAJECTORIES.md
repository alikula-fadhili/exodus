# Project Exodus: Agent Trajectories & Trace Logs

**Labeling correction (see `docs/reviews/unit-verification-audit.md`):** the JSON blocks below are
**hand-authored illustrative examples**, not captured execution traces. The file paths named in
each heading (`.exodus/trajectories/architecture_agent.json`, etc.) do **not exist on disk** —
nothing in the codebase currently persists a structured per-role trajectory file at those paths.
The only trajectory-shaped artifact that is real is `.exodus/trajectories/trajectory_sample.jsonl`
(explicitly named "sample"), and `BoundedAgent::trajectory()`
(`crates/exodus-agent/src/lib.rs`) genuinely records real `RepairAgent` steps in memory during a
run, but nothing writes that to disk today. Only `MockAgentProvider` exists in this environment —
no real LLM is wired in — so a *captured* trajectory from a real model is not currently obtainable
here regardless. Treat every JSON block below as "what this would look like," not as evidence of
an actual run.

This document contains representative trajectories illustrating multi-agent interactions across the Exodus migration lifecycle.

---

## 1. Architecture Agent Trajectory (illustrative example — not a real file)

```json
{
  "agent": "ArchitectureAgent",
  "task": "Construct Exodus Semantic Graph (ESG) & Dependency Cycle Analysis",
  "input": {
    "target_directory": "fixtures/04_circular_dependency",
    "parser": "tree-sitter-python"
  },
  "tool_calls": [
    {
      "tool": "parse_ast",
      "args": { "files": ["user.py", "order.py"] },
      "result": { "modules": 2, "functions": 2, "imports": 2 }
    },
    {
      "tool": "tarjan_scc",
      "args": { "nodes": ["function::user::get_user_summary", "function::order::get_user_orders"] },
      "result": { "sccs": [["function::user::get_user_summary", "function::order::get_user_orders"]], "cycle_detected": true }
    }
  ],
  "outcome": {
    "status": "APPROVAL_REQUIRED",
    "risk_score": 75,
    "blocker_reason": "Cycle detected between user.py and order.py. Human approval required in .exodus/plan.json."
  }
}
```

---

## 2. Migration Agent Trajectory (illustrative example — not a real file)

```json
{
  "agent": "MigrationAgent",
  "task": "Transform unsupported dynamic construct (eval)",
  "input": {
    "symbol": "dynamic_runtime::evaluate_runtime_code",
    "construct": "eval(expr_str)"
  },
  "decision": "FALLBACK_DEGRADED",
  "tool_calls": [
    {
      "tool": "emit_fallback_stub",
      "args": {
        "strategy": "TypedFailureStub",
        "symbol": "evaluate_runtime_code",
        "reason": "Dynamic reflection unsupported in static Rust"
      },
      "result": {
        "generated_code": "pub fn evaluate_runtime_code(expr_str: String) -> i64 {\n    todo!(\"Exodus Migration Debt: Dynamic reflection unsupported in static Rust\");\n}",
        "debt_recorded": true
      }
    }
  ]
}
```

---

## 3. Bounded Repair Agent Trajectory (illustrative example — not a real file)

```json
{
  "agent": "RepairAgent",
  "task": "Fix compilation diagnostic E0425 in generated target",
  "iteration": 1,
  "max_iterations": 3,
  "diagnostic": "error[E0425]: cannot find value `result` in this scope",
  "action": "Injected `let mut result = Vec::new();` into block header",
  "verifier_feedback": "cargo check passed (0 errors)",
  "outcome": "VERIFIED_COMPATIBLE"
}
```
