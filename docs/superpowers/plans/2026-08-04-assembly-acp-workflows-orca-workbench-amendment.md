# Assembly ACP Runtime, Workflow Orchestration, and Workbench UX Amendment

**Date:** 2026-08-04  
**Repository baseline:** `main` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`  
**Status:** Planning authority amendment; this commit changes plans only and does not authorize product implementation.  
**Applies to:** TSK-808, TSK-809, TSK-810, the agent-orchestration portion of TSK-766, and the shell/browser/resources/usage/session UX described in the 2026-08-04 product clarification.

## 0. Authority and conflict rules

This is the newest planning authority for the areas listed below. Read it **after**
`2026-08-01-tsk-808-native-workbench-product-wave.md`. The large product-wave plan remains
binding everywhere this amendment does not explicitly replace it.

This amendment supersedes:

- the whole execution model in `2026-08-02-tsk-809-810-conversation-workbench.md`;
- Work Package 10A in the product-wave plan;
- the provider/runtime restrictions in Work Package 11 that permanently require the PTY to be the
  only writer;
- the session-column composition in Work Package 3 where `Find a session` shares the same left
  column as current work;
- the side-panel layout portions of Work Package 2 that do not make every stacked tool section a
  Paneview panel;
- the presentation details in Work Packages 8 and 9 where this amendment specifies the Orca-like
  browser, resource, space, and usage compact-to-expanded flows more precisely;
- the affected rows of the product-wave dispatch manifest and parallelization schedule.

Unchanged safety, Git, worktree, editor, Roslyn, hosted-GitHub, Markdown, extension, Settings,
confirmation, security, and test contracts in the product-wave plan remain active.

The older TSK-809/810 plan remains historical discovery evidence only. Do not dispatch from it.

---

## 1. Clarified product decisions

1. **New Claude Code and Codex conversations start as structured ACP sessions by default.** They do
   not start inside the user-visible xterm merely so the application can scrape or mirror them.
2. **The raw native CLI remains a first-class surface.** Moving between the structured conversation
   and the CLI is an explicit single-writer ownership handoff, never two live writers attached to
   one provider conversation.
3. **Existing/imported PTY sessions remain supported.** They enter as `terminal-owned` sessions and
   use the existing incremental transcript projection until the user deliberately transfers them
   to structured ownership.
4. **Model, reasoning effort, permission/execution mode, planning mode, fast/service tier, and other
   provider configuration are selectors in the composer or session controls.** They are not shown
   as slash commands.
5. **Only real command actions remain in the slash/command menu.** TUI-only commands such as theme,
   keymap, pets, terminal title, raw-scrollback, and TUI-local copy are omitted or replaced by
   Assembly-native actions. Provider-advertised commands are capability-gated.
6. **Pasting screenshots into the composer is P0.** The image must be previewed, removable, restored
   safely, and sent as an actual image content block in structured mode. A file-path-only prompt is
   a compatibility fallback for terminal-owned sessions, not the primary design.
7. **Tool calls, command output, file edits, diffs, plans, tasks, approvals, user questions,
   reasoning summaries, child agents, and their output are typed timeline items.** They are not
   flattened into assistant Markdown or inferred from prose.
8. **Assembly owns cross-provider orchestration.** A workflow may assign Claude Code, Codex, and
   later ACP providers to explicit roles such as orchestrator, implementer, code reviewer,
   security reviewer, test runner, and specification-compliance reviewer.
9. **Provider-native subagents and Assembly-managed delegated agents are different things.** Both
   appear in one agent tree, but only Assembly-managed runs receive Assembly workflow controls.
10. **All stacked panels inside left and right workbench regions use Dockview Paneview.** Working and
    Done are independent Paneview panels. The full session library is not embedded in the current
    work rail.
11. **The Browser has one live page that can move between docked, floating, and maximized
    presentations without reload.** The global bottom-right action island remains available.
12. **Resources and Usage each have a compact status-bar popover and a full center workbench.**
    Compact and expanded surfaces project the same stores; neither performs a second scan merely
    because it opened.
13. **AI is integrated through typed assist actions throughout the product.** It may draft,
    explain, validate, or prepare an action, but consequential effects still use the existing
    typed service and confirmation boundaries.

---

## 2. What ACP does—and does not do

### 2.1 ACP does not automatically create the user-visible terminal session

An ACP adapter is a provider process connected to Assembly over JSON-RPC, normally through stdio.
For Codex, the maintained adapter starts Codex App Server. For Claude, the maintained adapter uses
the Claude Agent SDK. The conversation can therefore run without an xterm or interactive TUI.

ACP agents may request client-side terminals for command tools and stream terminal output. Assembly
maps those requests onto its existing `TerminalRegistry` and xterm components, with an exact
`ownedId`, turn, tool-call ID, and process owner. These tool terminals are not the same as the
primary native Codex or Claude CLI UI.

The product exposes three related but distinct surfaces:

```text
Structured conversation
  ACP session; rich events; no user-visible TUI required

Tool terminal
  Assembly-owned PTY created for an ACP command/tool call; expandable from the timeline

Raw native CLI
  Interactive `codex` or `claude` TUI; entered through an explicit ownership handoff
```

### 2.2 Single-writer state machine

Extend `OwnedSession` with an explicit execution owner instead of deriving ownership from the
presence of `ptySessionId`:

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';

export interface AgentRuntimeBinding {
  providerProfileId: string;
  acpSessionId: string | null;
  nativeSessionId: string | null;
  providerGeneration: number;
  owner: AgentExecutionOwner;
  capabilities: AgentRuntimeCapabilities | null;
}
```

Rules:

- one `ownedId` may have one provider writer;
- structured-to-terminal waits for or interrupts the active turn, persists the last acknowledged
  event, releases the ACP session writer, then starts the provider's exact resume command;
- terminal-to-structured stops/detaches the native TUI, incrementally imports turns written by the
  CLI, resumes the ACP session, then grants structured ownership;
- transition failure returns to the last valid owner and keeps the transcript visible;
- merely switching the visible Session/Editor/Browser tab never changes execution ownership;
- a raw terminal for an unrelated shell/run configuration is unaffected;
- no handoff guesses a provider ID from a title or path.

### 2.3 Provider process topology

Create one supervised ACP adapter process per configured provider profile/account where the
adapter supports multiple sessions. Do not spawn one adapter per tab by default.

```text
AgentRuntimeManager
├── ProviderProcess[Codex profile A]
│   ├── ACP session owned-1
│   └── ACP session owned-8
├── ProviderProcess[Claude profile A]
│   ├── ACP session owned-2
│   └── ACP session owned-3
└── transcript projections for terminal-owned/imported sessions
```

Pin adapter versions. Prefer a packaged standalone `codex-acp` sidecar. The Claude ACP packaging
spike must choose and prove one of: bundled Node 22 runtime, reviewed packaged executable, or an
explicit supported local Node requirement. Do not invoke `npx ...@latest` per session.

### 2.4 Concrete Rust layout

Refactor the existing `src-tauri/src/agent_conversation` module rather than adding another agent
subsystem:

```text
src-tauri/src/agent_conversation/
├── mod.rs
├── manager.rs
├── ownership.rs
├── capabilities.rs
├── canonical_event.rs
├── event_buffer.rs
├── attachments.rs
├── handoff.rs
├── transcript/
│   ├── mod.rs
│   ├── watcher.rs
│   ├── claude.rs
│   └── codex.rs
└── acp/
    ├── mod.rs
    ├── client.rs
    ├── process.rs
    ├── session.rs
    ├── terminal_bridge.rs
    └── provider_profile.rs
```

Use the official ACP Rust SDK as the protocol client. The existing handwritten
`agent_conversation/provider.rs` remains a disposable spike until the ACP route passes acceptance;
it is then removed or retained only as a separately tested direct-Codex fallback. It must not be
silently wired alongside the ACP manager.

---

## 3. Canonical provider-neutral conversation contract

ACP is the wire boundary, not Assembly's permanent UI schema. Normalize every provider into a
small application-owned event model inspired by T3 Code's item lifecycle separation.

```ts
export type AgentItemKind =
  | 'user-message'
  | 'assistant-message'
  | 'reasoning'
  | 'plan'
  | 'command'
  | 'file-change'
  | 'mcp-tool'
  | 'dynamic-tool'
  | 'web-search'
  | 'image-view'
  | 'subagent'
  | 'task'
  | 'approval'
  | 'user-input'
  | 'context-compaction'
  | 'review'
  | 'warning'
  | 'error'
  | 'unknown';

export type AgentContentChannel =
  | 'assistant'
  | 'reasoning'
  | 'reasoning-summary'
  | 'plan'
  | 'command-output'
  | 'file-change-output';
```

Events include `ownedId`, provider profile, ACP/native session IDs, generation, monotonic sequence,
turn ID, item ID, request ID, timestamp, normalized payload, and optional bounded raw-provider
metadata for diagnostics.

Required lifecycle events:

- session started/configured/state changed/exited;
- turn started/completed/interrupted/failed;
- item started/updated/completed;
- content delta;
- approval opened/resolved;
- user input requested/resolved;
- plan/task/subagent progress;
- token/context/rate-limit updates;
- runtime warning/error.

The reducer preserves source order. Text before a command, command output, and text after the
command remain three ordered items rather than one message plus a detached tool array.

### 3.1 Dynamic controls, not slash commands

Map ACP session configuration options into controls:

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'reasoning-effort'
    | 'permission-mode'
    | 'collaboration-mode'
    | 'service-tier'
    | 'boolean'
    | 'other';
  label: string;
  description: string | null;
  value: string | boolean | null;
  choices: AgentConfigChoice[];
  mutable: boolean;
}
```

Primary composer controls are Provider, Model, Effort, Permissions/Mode, and optional Fast/Service
Tier. Plan mode may be a mode control rather than `/plan` when advertised that way. Unknown
options remain available under More Settings.

The displayed value is always observed provider state. A user selection becomes pending until an
ACP update confirms it; rejection, timeout, disconnect, or a mismatched observed value rolls back
with a readable reason.

For terminal-owned sessions, unsupported live controls are either read-only or expose **Open native
picker**, which transfers focus to the raw CLI. They never pretend a PTY command succeeded before
the transcript confirms it.

### 3.2 Command catalog

The command menu merges:

1. commands advertised by the active ACP session;
2. project/provider skills;
3. Assembly-local commands such as Open Terminal, Fork Session, Open Diff, and Attach Context;
4. workflow commands exposed by an active workflow run.

Do not copy the whole Codex TUI command enum into the structured UI. Exclude TUI-only theme,
keymap, Vim, pets, title/statusline, raw scrollback, TUI copy, and app-exit commands. Model,
effort, permissions, plan/mode, and service tier belong to controls. Selecting a menu item never
auto-sends unless the descriptor explicitly represents an immediate safe local action.

---

## 4. P0 screenshot, image, and browser-feedback path

### 4.1 Composer paste

Retain the validated attachment vault in `agent_conversation/attachments.rs` and extend it with
exact deletion/pruning and restored-preview support.

Structured send constructs ACP prompt content:

```text
TextContent(draft)
ImageContent(image/png, bytes or protocol-approved resource)
ImageContent(image/png, ...)
EmbeddedContext(optional file/browser facts)
```

It does not merely append a filesystem path. Terminal-owned compatibility send keeps the existing
managed-path reference behavior because the native CLI owns input parsing.

Requirements:

- paste and file chooser support PNG, JPEG, WebP, and provider-supported formats;
- validate decoded signature, size, aggregate count/bytes, owner, canonical root, and symlink
  boundary in Rust;
- show a preview before upload/send completion;
- preserve draft and attachments on send failure;
- clear and revoke only after the exact provider acknowledges the prompt;
- key async work by `ownedId` plus generation;
- restored missing files appear as removable unavailable attachments;
- never serialize blob URLs or user-owned source-file deletion rights.

### 4.2 Browser annotations become first-class context

A browser annotation contains immutable page and selection facts plus an optional image:

```ts
export interface BrowserAgentContext {
  annotationId: string;
  workspaceId: string;
  tabId: string;
  browserGeneration: number;
  url: string;
  title: string;
  selector: string;
  accessibleName: string | null;
  textSnippet: string;
  rect: BrowserRect;
  intent: 'change' | 'question';
  note: string;
  screenshotAttachmentId: string | null;
  sourceHash: string;
}
```

Sending to a conversation stages a reviewable context chip and image in the existing composer. It
never auto-submits, overwrites a nonempty draft without merge preview, or targets a different
session after a switch.

---

## 5. Two kinds of subagents

### 5.1 Provider-native subagents

Codex and Claude may create their own helper agents. Render their live ACP events and nested output
when the adapter exposes identity/parent metadata. Otherwise import them from durable transcripts.

Provider-native child capabilities are honest:

- inspect transcript/output;
- see state, role/label, parent, elapsed time, tool activity, and result;
- open the associated worktree or terminal when one is deterministically known;
- cancel or send direct input only when the provider protocol explicitly supports it.

Do not imply Assembly can reassign or continue an opaque native child merely because it can display
its output.

### 5.2 Assembly-managed delegated agents

Assembly-managed agents are normal child ACP sessions created by the workflow broker. They have
stable `ownedId`, parent run/node links, explicit provider profile, model, effort, mode, working
directory/worktree, budget, and lifecycle controls. They can be paused, canceled, continued, or
reassigned according to workflow policy.

Both child types appear in one tree with a visible origin badge: **Provider subagent** or
**Workflow agent**.

---

## 6. App-owned workflow and orchestration engine

Provider-native subagents alone cannot satisfy cross-provider workflows. Add an Assembly-owned
durable workflow engine over ACP sessions, using the existing append-only orchestration event
ledger rather than a second unrelated database.

### 6.1 Workflow definitions

```ts
export interface WorkflowDefinitionV1 {
  version: 1;
  id: string;
  name: string;
  description: string;
  inputs: WorkflowInputDefinition[];
  roles: WorkflowRoleDefinition[];
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  maxParallelAgents: number;
  maxDepth: number;
  maxIterations: number;
  budget: WorkflowBudget;
  workspacePolicy: WorkflowWorkspacePolicy;
  approvalPolicy: WorkflowApprovalPolicy;
}

export interface WorkflowRoleDefinition {
  id: string;
  label: string;
  providerProfileId: string;
  modelOption: string | null;
  effortOption: string | null;
  modeOption: string | null;
  systemInstructions: string;
  allowedTools: string[];
  outputContract: WorkflowOutputContract;
}
```

Node kinds:

- prompt/agent task;
- deterministic command or test through a saved Run Configuration;
- human approval/decision;
- condition;
- parallel fan-out;
- join;
- review gate;
- repair loop;
- artifact/checkpoint;
- completion.

A common template is:

```text
Specification
  -> Implementer
  -> Build/Test
  -> Code Review + Specification Compliance + Security Review (parallel)
  -> Join
  -> Repair if findings remain (bounded loop)
  -> Final verification
  -> Human acceptance
```

### 6.2 Deterministic engine is the authority

The workflow engine owns scheduling, limits, state, retries, cancellation, and completion. An
optional orchestrator LLM may propose delegation and next actions, but it cannot bypass the graph,
role allow-list, worktree lock, budget, depth, concurrency, or confirmation gates.

Persist append-only events such as:

- workflow created/started/paused/completed/failed/canceled;
- node queued/started/waiting/blocked/completed/failed/skipped;
- agent spawn requested/started/turn completed/continued/canceled/exited;
- artifact produced/accepted/rejected;
- approval requested/resolved;
- retry/iteration advanced;
- budget updated/exhausted.

Projections derive the current run. Never infer a node's status from assistant prose.

### 6.3 Local workflow MCP companion

Expose a token-authenticated local MCP server to orchestrator sessions. Register it through the
ACP session's MCP configuration rather than injecting shell commands.

Initial tools:

```text
assembly_workflow_delegate
assembly_workflow_status
assembly_workflow_continue
assembly_workflow_cancel
assembly_workflow_submit_result
assembly_workflow_attach_artifact
assembly_workflow_request_decision
```

The delegation broker validates the caller's parent ACP session and workflow node, then creates a
fresh or resumable child ACP session through an injected `ConnectionSpawner` trait. This follows
the proven CodeG pattern—parent tool call, local companion, broker, child ACP session, linked
outcome—but Assembly keeps child sessions available for continued work instead of making every
child one-shot.

Each call has an opaque task ID, parent tool-call ID, child `ownedId`, status, timestamps, bounded
result, optional token usage, and stable error code. Cancellation races, parent teardown, depth
limits, turn limits, and child refusal/empty/max-token outcomes have deterministic tests.

### 6.4 Worktree and mutation locking

Workflow nodes declare one of:

- `read-only-shared`: reviewers may share a worktree concurrently;
- `exclusive-existing`: one mutating agent owns the selected worktree;
- `isolated-linked-worktree`: Assembly creates/assigns a dedicated worktree through the existing
  validated worktree service after confirmation/policy approval;
- `no-workspace`: research or planning only.

Two mutating agents never write the same worktree concurrently. Reviews may read the implementer's
worktree after the implementation checkpoint. A workflow cannot remove or force-clean a worktree
outside the existing safety and confirmation authority.

### 6.5 Agent Control workbench

Add a center Dockview destination named **Agents** and a right-tool summary Paneview. It includes:

- Workflow Library;
- New Run / Resume Run;
- active runs and history;
- graph/list toggle;
- current phase, loop iteration, queue, decisions, blockers, artifacts, and budgets;
- one row per agent with provider, model, effort, mode, role, node, state, elapsed time, tokens,
  worktree, last activity, and child count;
- Open Conversation, Open Worktree, Open Terminal, Pause, Continue, Cancel, Reassign, and Retry
  actions when capability/policy permits;
- full nested output and tool activity;
- explicit distinction between provider-native and Assembly-managed children;
- append-only audit/history and links to commits, PRs, files, test results, and screenshots.

The control surface is a projection over workflow/session stores. It does not create a third agent
runtime.

---

## 7. Conversation workbench UI replacement

Replace Work Package 10A with the following component boundary:

```text
ConversationSurface.svelte
├── ConversationHeader.svelte
├── ConversationTimeline.svelte
│   └── TimelineItem.svelte
│       ├── MessageItem.svelte
│       ├── ReasoningItem.svelte
│       ├── PlanItem.svelte
│       ├── CommandItem.svelte
│       ├── FileChangeItem.svelte
│       ├── ToolItem.svelte
│       ├── TaskItem.svelte
│       ├── AgentItem.svelte
│       ├── ApprovalItem.svelte
│       ├── UserInputItem.svelte
│       └── ErrorItem.svelte
├── ConversationAgentTree.svelte
├── ConversationComposer.svelte
├── AgentConfigControls.svelte
└── ConversationCommandMenu.svelte
```

### 7.1 Visual and interaction contract

- centered readable timeline;
- assistant narration remains visually dominant;
- compact expandable rows for tools, commands, tests, and file changes;
- collapsed reasoning summary after completion;
- proper sanitized Markdown, code highlighting, file links, copy actions, tables, and task lists;
- file-change items open the existing Monaco DiffEditor;
- command items reveal exact command, cwd, output, duration, and exit status;
- plan/task items show structured progress and actions;
- approval and structured-input requests render inline;
- composer is visually light and unboxed, while controls remain discrete;
- auto-follow only while already near the bottom; otherwise show Jump to latest;
- virtualize long timelines with stable item identity;
- parent and child scroll/draft/selection are isolated by `ownedId` and generation.

### 7.2 Stop, steer, and queued input

When supported by the active ACP adapter:

- Stop interrupts the active turn, not the whole workspace;
- steering adds user input to the running turn through the provider capability;
- queued prompts are visible, reorderable, and cancelable;
- unavailable features are absent or disabled with the protocol reason.

---

## 8. Left rail, Session Library, Dockview, and Paneview

### 8.1 Compact current-work rail

Refactor the left region around project/worktree identity, not wide independent cards.

Recommended composition:

```text
Left region Paneview
├── Workspaces pane
│   └── project/worktree rows with nested active agents
├── Working pane
│   └── compact current-session rows
└── Done pane
    └── compact completed rows
```

Working and Done are separate Paneview panels with independent collapse, size, scroll, and
session-workspace restoration. They are not plain Svelte accordions sharing one flex column.

Compact row behavior:

- 28–34px primary row where practical;
- icon, title, state, provider/model, and relative activity;
- selected treatment without a full-width oversized card surface;
- hover card shows branch/worktree, provider/model/effort, task/PR, latest activity, ports, and
  resumability;
- chevron expands richer inline details and explicit actions;
- keyboard and touch receive the same information without requiring hover;
- project/worktree rows may nest their active sessions/agents like Orca, but `ownedId` remains the
  navigation identity.

### 8.2 Session Library is separate

Remove `Find a session` from `SessionsColumn.svelte`. Add a **Session Library** center Dockview
panel and optional right-tool entry. It contains all scanned/resumable sessions with provider,
project, worktree, branch, model/effort, date, title, status, and full-text filters. Opening the
library does not adopt or resume anything. Explicit Open/Resume/Fork actions do.

The left rail contains work the user has already adopted. The library is the searchable archive
and discovery surface.

### 8.3 All left/right stacked content uses Paneview

The center remains Dockview. Any left/right tab whose content contains multiple collapsible or
resizable sections uses the existing Paneview integration rather than custom height arithmetic.
This includes:

- Working and Done;
- Source Control subsections;
- Worktrees filters/list/detail where stacked;
- Problems groups;
- Context cards;
- Agent run summary sections;
- Resource summary sections.

Panels are parked and reused on hide/show; no service, terminal, browser, or editor instance is
recreated merely because a Paneview section closes.

---

## 9. Browser: docked, floating, maximized, annotate, and draw

The product-wave browser security and child-webview spike remain mandatory. Refine its
presentation state to match the clarified Orca behavior:

```ts
export type BrowserPresentationMode =
  | 'collapsed'
  | 'docked'
  | 'floating'
  | 'maximized';
```

- **Docked:** normal Browser center destination.
- **Floating:** resizable overlay above the workbench, initially large enough for useful review but
  leaving the editor/session context visible behind it.
- **Maximized:** fills the Assembly shell below native macOS chrome.
- **Collapsed:** browser chrome is hidden and restored from the global bottom-right action island.

One native browser page is teleported between measured hosts. URL, history, forms, cookies,
scroll, profile, viewport, tabs, and annotations survive every presentation transition. Hide the
child webview before moving bounds and show it only after the destination is measured.

### 9.1 Toolbar contract

The floating and maximized toolbar provides:

- Back, Forward, Reload, address bar;
- Import;
- Grab page element;
- Annotate page element;
- Draw on screenshot;
- development-only Browser DevTools;
- Open in default browser;
- overflow/profile menu;
- maximize/restore and minimize/collapse.

Overflow/profile menu includes Default profile, New Profile, Import Cookies, Viewport Size, and
Browser Settings. Cookie import remains disabled until an explicit security design proves source,
format, secret handling, profile scope, and deletion.

### 9.2 Grab and annotation

`Grab page element` captures a bounded element payload and optional screenshot/crop for immediate
copy or composer attachment. `Annotate page element` opens the Change/Question note card anchored
near the selected element and queues the result. Both use injected, fixed inspector code; there is
no arbitrary eval API.

### 9.3 Draw on screenshot

`Draw on screenshot`:

1. captures the visible page through the native browser snapshot API;
2. parks/temporarily hides the native webview;
3. opens an Assembly-owned HTML canvas/markup overlay;
4. supports pen, arrow, rectangle, text, undo/redo, clear, crop, cancel, and save;
5. saves the result into the same validated conversation attachment vault;
6. restores the live browser without reload;
7. stages the marked image in the selected conversation or browser feedback queue without
   auto-send.

Do not draw directly into an untrusted page DOM or serialize page cookies/storage.

### 9.4 Global action island

Keep one bottom-right `WorkbenchActionFab` under `ShellOverlays`. It offers context actions for
Session, Editor, Browser, Agents, Resources, Git/PR, and History. Its fan/sheet is collision-aware,
keyboard-operable, labelled, and never clipped inside a Dockview panel.

---

## 10. Resource Manager, Space, and Usage

### 10.1 Resource Manager compact-to-full flow

Add one status-bar resource segment. Opening it shows a compact popover with:

- total CPU and RSS;
- process tree grouped by repository, worktree, owned session, terminal, browser, and language
  server;
- ports;
- active/warm language-server roots;
- refresh;
- safe owned-process stop actions;
- Space summary and Review action.

Opening **Review** or the full-page action activates the **Resources** center Dockview destination.
The full workspace has tabs or Paneview groups for Processes, Language Servers, Ports, and Space.

Use a bounded OS snapshot joined with existing terminal/LSP/browser/session registries. Only
registry-proven owned processes are stoppable. Visible compact/full resource surfaces may refresh
on a shared, focus-aware interval; when neither is visible, stop polling and retain the latest
snapshot. A second surface never starts a second collector.

### 10.2 Space manager

The full Space surface includes:

- scanned, reclaimable, workspace count, and updated metrics;
- treemap with zoom;
- selected workspace top-level size breakdown;
- filter/sort/state controls;
- exact worktree/repository/path/branch/status facts;
- active agent, terminal, browser, editor, dirty-buffer, Git, PR, and issue facts where already
  available;
- multi-select cleanup through the existing worktree safety/confirmation authority;
- protected/active reasons and no destructive action for unknown ownership.

### 10.3 Usage compact-to-full flow

Add one status-bar Usage segment and popover showing provider rate-limit windows, reset countdowns,
plan/account state, refresh, compact/detailed mode, Usage details & history, and Manage Accounts.

The full **Usage & Analytics** center panel separates two data classes:

1. **Provider quota:** authoritative current provider windows and reset times, or an unavailable
   reason. Never infer quota from local token counts.
2. **Local analytics:** Assembly-observed sessions, turns, tool events, token usage, cache usage,
   durations, workflow runs, agents spawned, PRs prepared/created, and provider mix.

Estimated cost is visibly labelled **Estimated**, uses a versioned pricing table and model mapping,
and says Unknown for an unmapped model. Do not present a large dollar figure as provider billing.

Provider adapters and transcript scanners implement a `ProviderUsageSource` interface so Claude,
Codex, OpenCode, and later providers can contribute independently without UI conditionals.

---

## 11. AI integrated throughout Assembly

Create one typed `AgentAssistRegistry`; do not add ad hoc prompt buttons that write directly to a
PTY.

```ts
export type AgentAssistMode =
  | 'explain'
  | 'draft'
  | 'validate'
  | 'prepare-action';

export interface AgentAssistAction<I, O> {
  id: string;
  label: string;
  mode: AgentAssistMode;
  buildFacts(input: I): AgentFactSnapshot;
  buildPrompt(facts: AgentFactSnapshot): AgentPromptRecipe;
  parseResult(text: string): O;
  applyResult?: (result: O) => AgentActionProposal;
}
```

Initial integrations:

- draft PR title/body from deterministic Git/issue facts;
- review a PR or local diff;
- address review findings;
- diagnose/fix checks;
- resolve conflicts through a prepared workflow;
- propose form values in New Session, Run Configuration, PR, workflow, and Settings-adjacent
  dialogs;
- pre-save advisory review for configured files/projects;
- validate a pending save, run configuration, workflow definition, PR action, or cleanup plan;
- explain resource, Git, worktree, session, or provider state;
- attach Browser feedback and screenshots;
- prepare test/review/spec-compliance workflows.

Rules:

- deterministic validators run first and remain authoritative;
- AI never silently blocks ordinary Save by default; an opt-in project policy may require a named
  advisory/check with timeout and bypass reason;
- draft/fill actions populate fields and never submit;
- remote/page/repository prose is delimited untrusted data;
- every mutation delegates to an existing typed service and confirmation boundary;
- facts are generation/hash checked immediately before apply;
- no arbitrary shell tool is added; user-defined Run Configurations remain the explicit command
  path;
- every prepared/executed action receives an append-only audit receipt.

---

## 12. Migration of the current repository implementation

### 12.1 Preserve

- stable `ownedId` identity and rail persistence;
- `TerminalRegistry`, `terminalService.ts`, xterm views, scrollback, reattach, and tombstones;
- session workspace snapshots, Editor/Browser/Diff state, and Dockview layout;
- attachment vault validation;
- provider transcript scanners and child discovery as import/projection paths;
- Git, worktree, LSP/Roslyn, Settings, browser-security, and confirmation authorities.

### 12.2 Replace or refactor

- replace 500ms full-tail transcript polling with a Rust incremental watcher for terminal-owned
  sessions;
- replace the intentionally unwired conversation registry with `AgentRuntimeManager`;
- replace the small timeline union with the canonical typed event/item model;
- split `ConversationSurface.svelte` into the typed component boundary in section 7;
- replace hard-coded command arrays with ACP/runtime command and config discovery;
- remove `Find a session` from `SessionsColumn.svelte`;
- migrate Working/Done and right stacked panels to Paneview;
- supersede PTY-command-based model/effort/permission controls with ACP config controls for
  structured-owned sessions;
- retain PTY picker fallback only for terminal-owned sessions.

### 12.3 Session compatibility

- existing saved sessions hydrate as terminal-owned when a live PTY survives;
- scanned historical sessions remain inspectable and resumable;
- new agent sessions default structured;
- **Open in Native CLI** and **Return to Conversation** perform ownership handoff;
- app restart resumes the recorded owner and never starts both paths;
- old workspace snapshots migrate additively; missing ACP fields become unknown/null;
- no transcript, attachment, terminal scrollback, editor tab, or browser state is deleted by the
  migration.

---

## 13. Implementation waves and parallel ownership

### Wave A — sequential contracts and spikes

These run before broad parallel implementation because every later lane depends on them.

1. **A0 Re-anchor and plan receipt**
   - refresh `origin/main`, record SHA, active WIP, package versions, and exact current files;
   - map old plan sections to this amendment;
   - no implementation.
2. **A1 ACP/ownership contract**
   - freeze `AgentRuntimeBinding`, execution-owner state machine, provider profile, capability,
     canonical event, config option, prompt-content, and terminal-bridge interfaces;
   - prove one minimal Codex and Claude ACP session from Rust without UI;
   - prove adapters expose image input, models/config/modes, tools, approvals, and session resume;
   - decide Claude sidecar packaging from measured evidence.
3. **A2 Shell layout contract**
   - freeze center Dockview roster, left/right Paneview owners, Session Library destination, parked
     element lifecycle, and snapshot versions.
4. **A3 Workflow contract**
   - freeze workflow definition/run/node/agent/event schemas, role/config mapping, MCP tool schema,
     worktree lock policy, limits, and audit events.
5. **A4 Native browser presentation spike**
   - prove one child webview moves docked -> floating -> maximized -> docked with no reload and HTML
     chrome/annotation overlays remain above it.

A missing or failed spike returns a decision packet. Do not compensate with transcript scraping,
DOM fakery, or a second provider process.

### Wave B — parallel foundation lanes

After A1–A4 contracts are committed, these may run in parallel under exclusive path ownership:

| Lane | Owned area | Output |
| --- | --- | --- |
| B1 | Rust ACP manager/provider supervisor | sessions, resume, capabilities, events, terminal bridge |
| B2 | conversation TypeScript model/reducer/store | canonical projection, stale guards, snapshots |
| B3 | conversation timeline/Markdown components | typed read-only fixture UI |
| B4 | attachments and prompt-content path | screenshot paste, image ACP send, cleanup |
| B5 | left rail/Paneview/Session Library | compact rows, hover/detail, separated library |
| B6 | browser native registry/presentation | one live page across docked/floating/maximized |
| B7 | resource and Space collectors | bounded joined snapshots, treemap data, safe ownership |
| B8 | provider usage sources/local analytics | quota and analytics contracts with provenance |
| B9 | workflow event store/broker/MCP companion | durable runs, delegation, status, cancel, linked children |

Controller-owned shared seams during integration:

- `src-tauri/src/main.rs` and capabilities;
- `src/routes/next/+page.svelte`;
- `ShellFrame.svelte`, `ShellOverlays.svelte`, center roster, and package manifests;
- top-level `OwnedSession` migration and workspace snapshot version;
- command registry and global action island registration.

### Wave C — parallel product lanes

After Wave B integration:

- C1 config selectors, approvals, user input, stop/steer, and commands;
- C2 live tool/plan/task/subagent timeline items and inline tool terminals;
- C3 Agent Control center and workflow graph/list;
- C4 browser grab, annotation queue, screenshot markup, and composer integration;
- C5 Resource compact/full UI and Space manager;
- C6 Usage popover/full analytics UI;
- C7 AI Assist registry and PR/form/save/browser/resource integrations;
- C8 terminal/structured handoff and migration UI.

Each lane uses focused fixture tests. At most two heavy native/build runners execute concurrently;
the controller normally uses one.

### Wave D — integration and native certification

1. integrate all controller-owned seams serially;
2. run focused Node/Rust tests by lane;
3. run one full TypeScript/Svelte gate;
4. run Rust tests serially where process/global state requires it;
5. run rebuilt-Tauri scenario matrix;
6. perform read-only SOL-medium milestone review;
7. fix only evidenced defects and repeat the failed proof;
8. update task ledger only after individual acceptance evidence exists.

---

## 14. Required focused test suites

Add or supersede focused suites for:

- `agentRuntimeOwnership.test.mjs`;
- Rust ACP manager/provider fixtures;
- `agentCanonicalEvents.test.mjs`;
- `agentConfigControls.test.mjs`;
- `conversationImageAttachments.test.mjs` plus Rust attachment tests;
- `conversationTimelineItems.test.mjs`;
- `conversationSessionHandoff.test.mjs`;
- `workflowDefinition.test.mjs`;
- `workflowProjection.test.mjs`;
- Rust workflow broker/delegation/cancellation/depth/budget tests;
- `agentControlViewModel.test.mjs`;
- `sessionRailPaneview.test.mjs`;
- `sessionLibrary.test.mjs`;
- `browserPresentation.test.mjs`;
- `browserAnnotations.test.mjs`;
- `browserScreenshotMarkup.test.mjs`;
- Rust browser bounds/profile/security tests;
- `resourceViewModel.test.mjs` and Rust resource ownership tests;
- `workspaceSpace.test.mjs`;
- `providerUsageSources.test.mjs`;
- `usageAnalytics.test.mjs`;
- `agentAssistRegistry.test.mjs`;
- existing Git/worktree/LSP/Settings/session-workspace regression suites.

Recorded provider fixtures must be redacted and version-labelled. Tests compare semantic events,
not fragile raw field order.

---

## 15. Native end-to-end acceptance

One rebuilt macOS application run must prove all of the following without a duplicate provider
process:

1. Start a new Codex structured session, choose model, effort, and permissions from selectors,
   paste two screenshots, remove one, send the other as image content, and inspect assistant,
   reasoning, command, output, file-change, plan/task, approval, and token events.
2. Repeat the equivalent Claude path, including tool permission and nested subagent output.
3. Open the structured session in the native CLI, add a turn, return to Conversation, and prove
   exact history reconciliation and single-writer ownership.
4. Run a cross-provider workflow with an orchestrator, implementer, code reviewer, and
   specification-compliance reviewer; show parallel review, bounded repair loop, agent statuses,
   artifacts, cancellation, and final approval.
5. Switch sessions/workspaces repeatedly while agents continue. Drafts, controls, images, child
   selection, editor tabs, browser, diff, Paneview sizes, and Dockview layouts remain isolated.
6. Use the compact left rail, hover details, expanded row, independent Working/Done panes, and
   separate Session Library to resume the exact same-title session in the correct worktree.
7. Open Browser docked, float it over the editor, maximize it, grab an element, add a Change
   annotation, draw on a screenshot, stage both in the exact conversation, restore docked, and
   collapse to the action island without page reload.
8. Open the Resource popover, navigate an owned session, safely stop a disposable owned process,
   open full Resources, inspect language servers/ports, then open Space and review a disposable
   worktree cleanup through the existing confirmation flow.
9. Open Usage compact and full views, distinguish quota from local analytics, show unavailable
   provider data honestly, and verify estimated cost is labelled and unmapped models remain
   unknown.
10. Use AI assistance to draft but not submit a PR, review a diff, propose form values, and run an
    opt-in save advisory. Cancel one consequential action and prove no mutation; confirm one
    disposable action and prove one append-only receipt.
11. Verify keyboard-only, VoiceOver, reduced motion, minimum/narrow window, 200% display scaling,
    settings/dialog z-order over the browser, and no hidden webview intercepting clicks.
12. Record process tree, PTY count, adapter count, Roslyn count, CPU/RSS, cleanup, and surviving
    owned sessions before and after.

---

## 16. Stop conditions

Stop and return a decision packet rather than weakening the product when:

- the selected ACP adapter cannot prove required image, config, approval, tool, or resume behavior;
- Claude sidecar packaging cannot meet a reproducible supported runtime contract;
- ownership handoff creates two writers or cannot reconcile the native session deterministically;
- the browser child view cannot remain below Assembly chrome/dialogs or move without state loss;
- provider-native child metadata cannot prove parent association;
- workflow delegation cannot authenticate the parent or enforce depth/concurrency/worktree/budget
  limits;
- a shared shell/session/LSP/Git/worktree seam contains overlapping uncommitted work;
- a feature would require arbitrary eval, arbitrary shell execution, cookie serialization, hidden
  credentials, or bypassing an existing confirmation authority;
- a panel migration would recreate terminals, browsers, editors, or provider sessions when a
  Paneview section closes;
- observed provider state cannot confirm a control change;
- a source cannot distinguish provider quota from local usage analytics.

---

## 17. Research references adopted by this amendment

- **Official ACP Rust SDK:** client/agent/proxy/conductor foundation for the Rust runtime.
- **Codex ACP adapter:** Codex App Server mapping for models, effort, modes, images, tools,
  approvals, plans, terminal output, reviews, usage, and subagents.
- **Claude Agent ACP adapter:** Claude Agent SDK mapping for images, permissions, TODOs, edit
  review, terminals, commands, MCP, and nested subagents.
- **T3 Code:** provider-neutral event/item lifecycle and adapter separation; use the taxonomy, not
  its entire Effect/TypeScript server architecture.
- **Jockey:** practical Tauri/Rust ACP discovery of models, modes, config options, commands, and
  permission responses.
- **CodeG:** concrete ACP delegation broker, local MCP companion, typed parent/child links,
  cancellation/depth/budget outcomes, and agent-specific config defaults.
- **Orca (MIT):** behavior and component references for persistent browser views, grab/annotation,
  screenshot markup, compact/full Resource Manager and Space, compact/full Usage, and dense
  worktree/session hover/detail presentation. Reuse only reviewed compatible code and preserve
  required attribution for copied substantial portions.

These references reduce invention, but Assembly retains its own `ownedId`, PTY, Dockview,
Paneview, worktree, LSP, Git, browser-security, confirmation, and append-only orchestration
contracts.
