# Assembly Superpowers Plan Authority

This directory contains historical plans, active master plans, amendments, execution evidence,
and generated audits. A newer plan does not automatically erase every older decision, so
implementation agents must resolve authority before dispatching work.

## Current authority order

### 1. ACP, workflows, Orca-inspired shell, and affected product surfaces

**Current authority:**

`2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`

Use it for:

- structured ACP sessions and native-CLI handoff;
- the single app-owned `AgentRuntimeManager`;
- model, effort, permissions/mode, and other provider configuration selectors;
- first-class screenshot/image input;
- typed reasoning, tools, approvals, plans, task lists, file changes, and user input;
- provider-native subagents;
- Assembly-managed multi-agent workflows and Agent Control Center;
- compact active-work navigation, separate Session Library, and Paneview side panels;
- floating/maximized browser, element grab, annotation, and screenshot markup;
- Resource Manager, Workspace Space, Usage popup, and Usage analytics;
- reusable AI assistance recipes across PRs, Git, forms, save review, browser feedback,
  Problems, and run configurations;
- the revised parallel schedule and first implementation action for those lanes.

This amendment supersedes conflicting clauses in the two plans below while preserving their
unaffected safety and implementation detail.

### 2. Native workbench master plan

`2026-08-01-tsk-808-native-workbench-product-wave.md`

It remains authoritative for unaffected work, including:

- contrast and accessibility;
- Assembly product-identity compatibility migration;
- Settings restoration;
- editor, nested file Dockview, CodeLens/Peek, LSP, and Roslyn requirements;
- local Git, diffs, explorer, filesystem, and worktree safety;
- hosted GitHub prepare/confirm/execute boundaries;
- native child-webview URL/profile/capability isolation;
- Markdown security;
- extension/API investigation;
- controller-owned seams, heavy-runner limits, native acceptance, proof ledger, PR, and cleanup.

Where this master plan says the PTY must permanently own every conversation, forbids an agent
runtime, combines resumable history into the active left rail, uses hand-built primary side
accordions, limits the browser to its older presentation states, or provides the older parallel
schedule for conversation/orchestration, use the 2026-08-04 amendment instead.

### 3. Conversation discovery plan

`2026-08-02-tsk-809-810-conversation-workbench.md`

Historical discovery only. Do not dispatch its retired PTY-only checklist. Its preserved findings
and current task routing are summarized inside that file and implemented through the 2026-08-04
amendment.

## Conflict resolution

Apply this order:

1. the most recent explicit amendment for the exact capability;
2. the TSK-808 master plan for unaffected details;
3. older task-specific plans as historical evidence only;
4. current repository code and tests for what already exists;
5. direct user clarification over an older written assumption.

If two active documents still disagree about process ownership, persistence, security,
worktrees, provider capabilities, or a destructive/remote action, stop and obtain a controller
or user decision before editing.

## Dispatch requirements

Before dispatching an implementation packet:

- record the current repository SHA and re-anchor named symbols;
- name the exact controlling section and inherited master-plan rules;
- state the one runtime/store/service authority being extended;
- list exact editable and forbidden files;
- preserve `ownedId` as the stable session identity;
- use provider-advertised capabilities instead of guessed catalogs;
- keep consequential actions previewed, confirmed, revalidated, and audited;
- retain the global SQL, worktree, heavy-runner, native-proof, process-cleanup, and evidence rules
  from the master plan;
- stop on an unresolved architecture choice rather than silently reviving a superseded design.

## Implementation authorization

These plans define product and implementation intent. They do not by themselves authorize broad
implementation. The current first action for the affected ACP/workflow/browser lanes is Work
Package A0 in the 2026-08-04 amendment, after explicit authorization and creation of the
controller-owned integration worktree.
