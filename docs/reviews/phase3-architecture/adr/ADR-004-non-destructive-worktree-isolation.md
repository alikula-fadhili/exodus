---
okf_version: "0.2"
type: decision
status: approved
sources:
  - crates/exodus-worktree/src/lib.rs
tags:
  - adr
  - worktree
  - git
  - sandboxing
  - okf
---

# ADR-004: Non-Destructive Git Worktree Sandboxing

[Index](../../../../../openwiki/index.md) | [Decisions](../decisions.md) | [System Architecture](../../../../../openwiki/system-architecture.md)

## Context
Automated code transformations, compiler checks, and agentic repair loops can fail, produce broken syntax, or leave dirty working directory states. Operating directly in the developer's working tree risks data loss, interrupted commits, and unrecoverable work-in-progress state.

## Decision
We enforce strict **Non-Destructive Worktree Isolation** via `exodus-worktree`:
1. All agentic repairs, transformations, and verification runs execute exclusively inside isolated Git worktrees located under `.exodus/worktrees/<task-id>`.
2. Worktrees use dedicated ephemeral branches (`exodus/<task-id>/migration`).
3. Lease locks prevent concurrent processes from modifying the same worktree.
4. Crash recovery automatically discovers and reclaims abandoned worktree leases.
5. Only upon successful contract verification and explicit human approval is the worktree branch promoted or merged to the parent branch.

## Consequences
- **Positive**: Zero risk of dirtying or destroying user uncommitted code.
- **Positive**: Enables parallel agent execution across independent waves without git conflicts.
- **Negative**: Disk usage increases proportional to active concurrent worktrees (mitigated by automated lease cleanup).
