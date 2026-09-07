---
okf_version: "0.2"
type: invariant
status: planned
sources:
  - openwiki/INSTRUCTIONS.md
  - crates/exodus-planner/src/lib.rs
---

# Human Approval Model & Interactive Review Gate

[Index](index.md) | [Migration Planner](migration-planner.md) | [Agent Boundaries](agent-boundaries.md) | [Security Boundaries](security-boundaries.md)

Human oversight is a non-negotiable invariant in the Exodus migration engine. Consequential actions—such as breaking circular dependency cycles, deleting files, applying irreversible AST transformations, or running LLM synthesized code mutations—are governed by explicit human approval gates.

## Core Invariant: Qualified Human Reviewer

> **Rule**: Never perform destructive changes, circular dependency breaking, or irreversible transformations without generating an approval request in `.exodus/plan.json` or prompting the interactive human reviewer in the active harness session.

## Checkpoint Levels & Interactive Review Flow

1. **Interactive Thought & Idea Review (`claude-code` & `agy` style)**:
   * During interactive migration sessions, before executing high-impact actions or complex transform waves, the agent pauses with a structured proposal:
     ```text
     🤔 Agent Thought Process / Proposed Action:
        Topic:    Circular Dependency Decoupling (user.py <-> order.py)
        Details:  Extract shared OrderSummary struct into common interface layer to break cyclic import.

     Approve this action? [Y/n/e(dit)]:
     ```
   * **`Y` (Approve)**: Proceeds with transformation and bounded verification.
   * **`N` (Reject/Defer)**: Aborts or halts the operation, preserving untouched state.
   * **`E` (Edit)**: Allows the user to inject manual constraints, guidelines, or prompt adjustments.

2. **Migration Intent & Architectural Conflict Review (`exodus intent review` / `exodus review`)**:
   * When critical claims are provisional, contradictory, or high-risk, the Exodus agent initiates a sequential review prompt presenting one claim at a time:
     ```text
     ================================================================================
     🔍 EXODUS INTENT APPROVAL REQUEST (1 of 3)
     Claim ID:    claim-01918374-88aa-742f-89bc-993412089123
     Category:    ConcurrencyRuntimeMapping
     Subject:     services/stream_processor.ts
     Grounding:   Provisional (Confidence: 0.65)
     Summary:     Map Node.js EventEmitter stream processing to Go errgroup and channel workers
     Evidence:    [docker/Dockerfile:L12-18] EXPOSE 8080 (hash: 4a2f8b...)
                  Redacted excerpt: "ENV WORKER_CONCURRENCY=16"

     Select an option:
       [a] Accept claim and record decision
       [r] Reject claim
       [1] Choose alternative: Map to Go sync.WaitGroup with buffered channels
       [2] Choose alternative: Map to Go context-driven worker pool with bounded semaphores
       [d] Defer claim (written to plan checkpoint as Blocked)
       [q] Quit review
     ================================================================================
     ```
   * **Strict TTY Check**: Requires an active, attached interactive terminal (`std::io::stdin().is_terminal()`).
   * **Explicit Consent**: Pressing `<Enter>` without input is explicitly rejected and will never be treated as affirmative consent.
   * **Alternative Branches**: Reviewers can choose synthesized alternatives with full traceability.
   * **Non-Interactive Batch Mode**: If executed in CI or non-TTY environments, pending critical claims are recorded in `.exodus/plan.json`, all dependent compilation/migration units are classified as `Blocked`, and execution halts without fabricating approval.
   * **Immediate Persistence**: All human review decisions (`Accepted`, `Rejected`, `Modified`, `Deferred`) are durably stored in embedded SurrealDB (`.exodus/data/surreal/`).

3. **Plan Approval Gate (`exodus approve`)**:
   * Topologically generated multi-wave plans containing risk flags or cycle breaks are saved to `.exodus/plan.json` in an `ApprovalPending` status (`is_approved: false`).
   * Execution is strictly blocked until an authorized reviewer signs off:
     ```bash
     exodus approve .exodus/plan.json --approver "LeadArchitect"
     ```

4. **Fallback & Debt Acceptance**:
   * For unresolvable dynamic reflection (e.g. `eval()`, runtime metaprogramming), the engine emits an explicit typed fallback stub (`todo!("Exodus Migration Debt: ...")`) and logs the debt to `.exodus/fallbacks.json`.

5. **Git Worktree Isolation & Merge Proposals**:
   * All mutations occur inside isolated Git worktrees (`.exodus/worktrees/`).
   * Upon successful per-unit verification, Exodus generates a non-destructive `MergeProposal` that requires final human sign-off before merging into the main branch.

## Session Control Flow & Keyboard Shortcuts

The interactive harness uses standard control-flow shortcuts for seamless operation:

* **Exit / Quit**: `exit`, `quit`, `/exit`, `/quit`, `:q`, `:wq`, `q`, `/bye`, `bye`
* **Interrupt**: `Ctrl+C` cleanly aborts the running command and returns to the prompt without corrupting state.
* **Directory Navigation**: `cd <path>` or `/cd <path>` changes the active working context.
* **Target Output**: `/out [path]`, `/output [path]`, `/target [path]` inspects or sets custom isolated target directories.

