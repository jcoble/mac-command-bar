# TSK-809 and TSK-810 Conversation Workbench — Historical Discovery Plan

**Status:** Superseded for implementation on 2026-08-04.  
**Current execution authority:**
`docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md`  
**Master plan retained for unaffected work:**
`docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`

Do not dispatch implementation agents from the old PTY-only checklist that previously lived in
this file. Its useful findings were folded into the current amendment, and its detailed history
remains available in Git.

## Why this plan was superseded

The earlier plan correctly protected the running native CLI from a duplicate provider process,
but it made the existing PTY the permanent writer for every future conversation. The clarified
product requires a richer and more general contract:

- new Codex and Claude sessions start as structured ACP sessions by default;
- the native CLI is an explicit single-writer handoff/fallback, not the hidden transport behind
  every rendered conversation;
- model, effort/thought level, permissions/mode, Fast/service tier, and related options are
  capability-driven selectors rather than slash commands;
- pasted screenshots are sent as first-class image content where the provider supports them;
- tool calls, approvals, reasoning, plans, task lists, file changes, command output, and
  structured user input remain typed timeline items;
- provider-native child agents have live nested output where the adapter exposes it;
- Assembly also owns a separate deterministic workflow engine for configurable orchestrator,
  implementer, tester, reviewer, spec-compliance, security, fixer, and verifier roles;
- future ACP providers use the same app-owned runtime and canonical event model.

The old architecture remains useful only for terminal-owned sessions, imported/external sessions,
read-only history, and recovery.

## Preserved repository decisions

The current amendment continues to preserve these findings from this plan:

1. `ownedId` is the stable app identity. Provider session IDs, PTY IDs, workflow IDs, and child
   IDs remain metadata.
2. Conversation, draft, attachment, control, child-selection, and scroll state are isolated by
   `ownedId` and generation.
3. The existing TerminalService/TerminalRegistry remains the native CLI and user-terminal
   authority.
4. The existing attachment vault remains the validated per-session image-storage boundary.
5. The JSONL transcript parser remains the terminal projection/import/recovery adapter, but moves
   from repeated full-tail frontend polling to incremental Rust-side updates.
6. The raw native terminal remains one explicit action away and its scrollback/process lifecycle
   are preserved.
7. Child association must be proven from provider-native parent metadata and canonical transcript
   roots. Similar names are not parentage.
8. Provider-native child sessions are read-only unless the active provider explicitly advertises
   direct input. Assembly-managed workflow agents are controlled only by WorkflowEngine.
9. Missing metadata remains unknown; the UI never invents a model, effort, context percentage,
   parent relation, or runtime state.
10. Browser feedback and other product surfaces target the exact current `ownedId` and generation
    and never auto-submit a draft.

## Current routing for TSK-809

Use the 2026-08-04 amendment:

- **A0:** prove pinned Codex/Claude ACP adapters, image input, configuration options, typed events,
  resume/load, subagents, packaging, resource cost, and absence of a user-visible PTY.
- **A1–A3:** freeze and implement the one `AgentRuntimeManager`, canonical capabilities/events,
  provider adapters, and incremental terminal projection.
- **A4:** implement screenshot paste first, then model/effort/mode controls, typed timeline,
  approvals, plans/tasks, safe Markdown, live children, long-history behavior, and the reduced
  provider/Assembly command menu.
- **A5:** implement structured/native-CLI single-writer handoff and fork behavior.
- **A12:** certify the real Tauri app and close TSK-809 only from current evidence.

## Current routing for TSK-810

Use the 2026-08-04 amendment:

- provider-native subagents are normalized and rendered live through A3/A4;
- terminal-owned historical children remain available through incremental transcript projection;
- Assembly workflow agents are created and controlled by A6/A7;
- the Agent Control Center shows parent/child hierarchy, provenance, provider/model/config,
  workflow node, plan/tasks, tools, worktree, output, timing, state, retry/cancel/gate controls,
  and exact artifacts;
- child association, cycle/depth/count limits, stale generations, and cross-owner/path escape are
  fixture-tested;
- TSK-810 closes only after both provider-native and Assembly-managed children are proven in the
  rebuilt desktop app.

## Non-negotiable runtime rule

One native provider conversation has one writer at a time:

```text
Structured ACP owner
    ⇅ explicit handoff with reconciliation and rollback
Native Claude Code or Codex CLI owner
```

Tool terminals created for ACP command/tool calls are separate app-owned processes. They do not
become the native CLI session and do not occupy `OwnedSession.ptySessionId`.

## Implementation stop

Stop rather than returning to this historical checklist if the current amendment cannot prove:

- first-class image input for Codex and Claude;
- authoritative model/effort/mode discovery and updates;
- one-writer handoff;
- provider-native child correlation;
- pinned/packaged adapter lifecycle and complete cleanup;
- deterministic workflow scheduling outside agent prose.

All unaffected accessibility, security, worktree, Git, Roslyn, browser-isolation, cleanup,
parallel-runner, and native-evidence rules remain inherited from the TSK-808 master plan.
