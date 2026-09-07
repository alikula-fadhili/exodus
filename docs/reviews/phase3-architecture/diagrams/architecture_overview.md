---
okf_version: "0.2"
type: architecture
status: approved
sources:
  - Cargo.toml
  - AGENTS.md
  - docs/reviews/REVIEW_PLAN.md
tags:
  - architecture
  - diagrams
  - okf
  - phase-3
---

# Architecture Diagrams & Structural Blueprints

[Index](../../../../openwiki/index.md) | [System Architecture](../../../../openwiki/system-architecture.md) | [Decisions](../decisions.md)

This document collects architectural and system topology diagrams for Project Exodus, visualizing data flow, crate dependencies, execution boundaries, and human-in-the-loop control loops.

---

## 1. Workspace Crate Topology & Responsibility Matrix

```mermaid
graph TB
    subgraph CoreDomain ["1. Domain Foundation Layer"]
        exodus_core["exodus-core<br/>• Semantic types<br/>• MigrationOutcome<br/>• OperationalItem"]
        exodus_store["exodus-store<br/>• Embedded SurrealDB<br/>• ACID persistence<br/>• Audit ledger"]
        exodus_kernel["exodus-kernel<br/>• Plugin runtime<br/>• Maker policy engine<br/>• Theme providers"]
    end

    subgraph SyntaxGraph ["2. Syntax & Graph Analysis"]
        exodus_parser["exodus-parser<br/>• Tree-sitter grammars<br/>• AST symbol extraction"]
        exodus_graph["exodus-graph<br/>• ESG representation<br/>• Tarjan SCC & FAS<br/>• Topological sort"]
        exodus_planner["exodus-planner<br/>• Wave generator<br/>• Risk weighting<br/>• Human checkpoint"]
    end

    subgraph ExecutionLayer ["3. Agentic Execution & Repair"]
        exodus_worktree["exodus-worktree<br/>• Isolated sandboxes<br/>• Git worktree locks<br/>• Crash recovery"]
        exodus_agent["exodus-agent<br/>• Bounded repair (<=3)<br/>• Prompt orchestration"]
        exodus_transform["exodus-transform<br/>• Semantic translation<br/>• AST rewriting"]
        exodus_fallback["exodus-fallback<br/>• todo! stubbing<br/>• Migration debt"]
    end

    subgraph VerificationGate ["4. Grounded Verification & Knowledge"]
        exodus_verifier["exodus-verifier<br/>• rustc/cargo runners<br/>• Diagnostic parsing<br/>• Grounded unit gate"]
        exodus_case["exodus-case<br/>• Structural hashing<br/>• Promotion engine"]
        exodus_eval["exodus-eval<br/>• Baseline metrics<br/>• Scorecard generator"]
        exodus_cost["exodus-cost<br/>• Token usage advisory<br/>• Migration cost ledger"]
    end

    subgraph PresentationLayer ["5. Mission Control & Presentation"]
        exodus_cli["exodus-cli<br/>• Command dispatch<br/>• Axum web server"]
        exodus_desktop["exodus-desktop<br/>• Native Tauri v2 UI<br/>• Board control plane"]
    end

    %% Dependency Connections
    exodus_parser --> exodus_core
    exodus_graph --> exodus_core
    exodus_graph --> exodus_parser
    exodus_planner --> exodus_graph
    exodus_agent --> exodus_planner
    exodus_agent --> exodus_worktree
    exodus_agent --> exodus_transform
    exodus_transform --> exodus_fallback
    exodus_verifier --> exodus_core
    exodus_verifier --> exodus_store
    exodus_case --> exodus_core
    exodus_eval --> exodus_verifier
    exodus_kernel --> exodus_core
    exodus_cli --> exodus_kernel
    exodus_cli --> exodus_verifier
    exodus_cli --> exodus_case
    exodus_desktop --> exodus_kernel
    exodus_desktop --> exodus_verifier
    exodus_desktop --> exodus_agent
```

---

## 2. 6-Stage Human-in-the-Loop Operational Closed Loop

```mermaid
sequenceDiagram
    autonumber
    actor Maker as Maker / Engineer
    participant Board as Board Control Plane (Desktop / Web)
    participant Kernel as Exodus Kernel & Policy Engine
    participant Agent as Agent & Worktree Sandbox
    participant Verifier as Grounded Unit Gate Verifier
    participant Store as Embedded SurrealDB & Case Engine

    Maker->>Board: Ingest / Inline Create Task with Attached Prompt
    Board->>Kernel: Validate Maker Policy Directives
    alt Maker Policy Violated
        Kernel-->>Board: Reject or Emit Advisory Debt
    else Maker Policy Valid
        Kernel->>Store: Persist OperationalItem (State: Captured)
        Board->>Agent: Execute Harness Unit in Isolated Worktree
        Agent->>Agent: Bounded Repair (Iteration <= 3)
        Agent->>Verifier: Run Compiler & Test Behavioral Contracts
        alt Tests Fail / Debt Recorded
            Verifier-->>Board: UnitPromptDiagnostic (Root Cause, Error Snippet, Refinement)
            Board-->>Maker: Display Prompt Failure Breakdown & 1-Click Refine
        else Tests Pass
            Verifier->>Store: Update State: ContractVerified (Grounded Oracle)
            Store-->>Board: Ready for Human Approval
            Maker->>Board: Review Diff & 1-Click Approve
            Board->>Store: Promote to CASE-XXXX in Permanent Organizational Memory
        end
    end
```

---

## 3. Separation of Concerns Model

```text
┌───────────────────────────────────────────────────────────┐
│              PROJECT EXODUS ARCHITECTURE                  │
├─────────────────────────────┬─────────────────────────────┤
│   Exodus Semantic Graph     │  OpenWiki Knowledge Base    │
│   (Program Semantics Model) │   (Documentation Model)     │
├─────────────────────────────┼─────────────────────────────┤
│ • AST Nodes & Symbols       │ • OKF v0.2 Markdown Pages   │
│ • Cycles & FAS Resolution   │ • Repository Claims         │
│ • Unit Clusters & Waves     │ • Provenance & Traceability │
│ • Tarjan Algorithm          │ • System Architecture Guides│
│ • Code Transformation       │ • Architecture Decision Recs│
└─────────────────────────────┴─────────────────────────────┘
              ▲                             ▲
              │                             │
              └──────── STRICTLY DECOUPLED ─┘
```
