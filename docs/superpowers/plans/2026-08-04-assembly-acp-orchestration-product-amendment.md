# Assembly ACP, Workflow Orchestration, and Workbench Surfaces — Authoritative Plan Amendment

**Date:** 2026-08-04  
**Status:** Approved product-direction amendment; implementation has not started  
**Branch:** `plan/assembly-acp-orchestration-shell`  
**Baseline:** `4e8192e0cdce791f53e93af39a6d2d2e2c322911`

> **Execution authority:** When this amendment conflicts with
> `2026-08-01-tsk-808-native-workbench-product-wave.md` or
> `2026-08-02-tsk-809-810-conversation-workbench.md`, this amendment wins. It supersedes the
> PTY-only conversation-runtime clauses in Work Package 10A, expands Work Package 11 into a real
> workflow/orchestration system, changes session discovery and side-region layout, and refines the
> Browser, Resources, Space, Usage, and AI-integration work packages. Unaffected safety,
> compatibility, Git, editor, Roslyn, worktree, and confirmation contracts remain in force.

## 1. Product decisions now locked

1. **Model, reasoning effort, permission mode, collaboration/plan mode, and other runtime options
   are first-class selectors.** They are not presented as slash commands. The visible values and
   choices come from the active runtime's advertised configuration capabilities.
2. **Slash commands are reserved for genuine commands.** Provider/project skills and useful
   provider actions may appear. TUI-presentation commands such as theme, terminal pets, raw
   scrollback, TUI keymap, status line, terminal title, and TUI-only session navigation are omitted
   from the structured composer.
3. **ACP is the primary structured runtime for new Claude Code and Codex conversations.** The
   application hosts an ACP client in Rust and launches version-pinned Claude and Codex ACP
   adapters. Future providers implement the same runtime contract rather than adding new frontend
   branches.
4. **ACP does not automatically create an Assembly native-terminal session.** An ACP agent may
   emit terminal/tool events and terminal output, which Assembly renders as typed timeline items.
   The user's interactive native CLI is a separate PTY-owned surface. Moving between structured
   conversation and native CLI is an explicit single-writer ownership handoff.
5. **Pasting screenshots into the composer is P0.** The image is previewed immediately, stored in
   the existing validated per-owned-session attachment vault, sent as a native ACP image/content
   block when supported, and never reduced to an unverified prose-only path in structured mode.
6. **Tools, commands, edits, reasoning, plans, TODO/task progress, approvals, structured questions,
   usage, and subagents are first-class timeline data.** They are not flattened into assistant
   Markdown.
7. **Provider-native subagents and Assembly-managed workflow agents are different concepts.** Both
   appear in a common agent hierarchy, but only Assembly-managed agents are guaranteed to support
   app-level start, retry, pause, cancel, follow-up, and role reassignment.
8. **Workflow orchestration is app-owned.** ACP is the provider transport; it is not the durable
   workflow engine. Assembly owns workflow definitions, node dependencies, attempts, budgets,
   approvals, retries, artifacts, and the run event ledger.
9. **Every visible side-region stack uses Dockview Paneview.** Working, Done, right-side tool
   sections, and other tab-contained vertical section stacks are Paneview groups, not bespoke
   flex/accordion implementations. Center destinations remain Dockview panels.
10. **Session discovery is separate from the working-session rail.** The left rail shows active
    work compactly. Scanned/resumable/history discovery lives in its own Dockview destination or
    optional right-side Paneview, never inside the same Working/Done column.
11. **Browser, Resources, Space, and Usage each have a compact entry surface and a full workspace.**
    The compact surface is for glance/action; the full workspace is a center Dockview destination
    with filtering, inspection, and history.
12. **AI assistance is available throughout the workbench but remains preview-first.** It may draft,
    explain, validate, compare, review, or prepare a typed action. It may not bypass the existing
    deterministic service, confirmation, stale-state, permission, worktree, Git, PR, or save
    validation boundaries.

## 2. Research conclusions and implementation references

The implementation should reuse patterns, not copy whole applications:

- **Official ACP Rust SDK:** use `agentclientprotocol/rust-sdk` for the Rust client connection,
  protocol types, request routing, capability negotiation, and test fixtures.
- **Codex ACP adapter:** `agentclientprotocol/codex-acp` already maps Codex app-server models,
  reasoning effort, fast mode, approval/sandbox modes, images, commands, edits, permission
  requests, terminal output, reasoning, plans, web search, review events, usage, and subagents.
- **Claude Agent ACP adapter:** `agentclientprotocol/claude-agent-acp` already maps the official
  Claude Agent SDK's images, tools/permissions, TODO lists, edit review, nested subagent
  transcripts, terminals, custom commands, and MCP servers.
- **T3 Code:** use its provider-neutral event and item lifecycle as the taxonomy reference. Do not
  port its Effect server or React client. Assembly needs a smaller Rust-native contract.
- **Jockey:** use its Tauri/Rust ACP patterns for discovered models, modes, config options,
  commands, reconnect, prewarm, and per-session runtime state.
- **CodeG:** use its delegation broker as a reference for a parent-linked ACP child session,
  provider selection, depth limits, status, cancellation, terminal outcomes, and an MCP tool bridge.
  Assembly's workflow engine must be multi-turn and durable rather than CodeG's current one-shot
  default.
- **Orca:** use its interaction patterns for the browser toolbar and annotations, compact resource
  popover, full Space manager, compact usage roster, full usage analytics, compact worktree/session
  rows, hover identity/detail, and separation between compact and full workspaces.

No external reference becomes a second source of product state. Existing Assembly services remain
owners of sessions, terminals, Git, worktrees, browser state, resources, settings, editor models,
and confirmations.

## 3. Runtime architecture

### 3.1 One app identity, one provider writer

`ownedId` remains the stable Assembly identity. Extend `OwnedSession`; do not key the application
by ACP session ID, Codex thread ID, Claude session ID, or PTY ID.

```ts
export type AgentExecutionOwner =
  | 'structured-acp'
  | 'native-pty'
  | 'transitioning-to-acp'
  | 'transitioning-to-pty'
  | 'stopped';

export type AgentActivityState =
  | 'starting'
  | 'idle'
  | 'running'
  | 'waiting-for-approval'
  | 'waiting-for-input'
  | 'paused'
  | 'completed'
  | 'failed';

export interface OwnedAgentRuntimeRef {
  provider: string;
  adapterId: string;
  acpSessionId: string | null;
  nativeSessionId: string | null;
  generation: number;
  executionOwner: AgentExecutionOwner;
  activity: AgentActivityState;
}
```

A native provider conversation has exactly one writer lease. A structured ACP client and an
interactive TUI may never submit concurrently. Read-only transcript projection may continue while
the native PTY owns the session.

### 3.2 Rust `AgentRuntimeManager`

Refactor the existing `src-tauri/src/agent_conversation` module rather than adding a parallel
runtime:

```text
src-tauri/src/agent_conversation/
├── mod.rs
├── manager.rs
├── runtime.rs
├── capabilities.rs
├── events.rs
├── handoff.rs
├── attachments.rs
├── acp/
│   ├── client.rs
│   ├── sidecar.rs
│   ├── session.rs
│   └── normalization.rs
└── transcript/
    ├── mod.rs
    ├── watcher.rs
    ├── claude.rs
    └── codex.rs
```

The manager is keyed by `ownedId` and owns:

- the current execution-owner lease;
- the ACP sidecar process/connection when structured-owned;
- provider and client capabilities;
- provider/native session IDs;
- turn cancellation and approval/input request routing;
- generation and monotonically increasing event sequence;
- recent event replay for frontend resynchronization;
- ownership handoff and rollback;
- transcript projection while PTY-owned;
- process cleanup on explicit close, never on frontend unmount.

Use the official Rust ACP SDK rather than a handwritten JSON-RPC implementation. The existing
unwired `provider.rs` remains a spike only and is removed after ACP parity is proven.

### 3.3 Sidecar packaging

Do not run `npx ...@latest` at session start.

1. Pin exact adapter and protocol versions in a checked-in runtime manifest.
2. Build or package signed/hashed macOS arm64 and x64 sidecars where the upstream project supports
   standalone binaries.
3. Where the Claude adapter still requires Node, either bundle one reviewed Node runtime or ship a
   guided prerequisite check. The implementation packet must choose one after measuring bundle
   size and startup latency; it may not silently depend on a GUI app's shell PATH.
4. Verify sidecar hash, executable path, and version before launch.
5. Expose adapter availability and remediation in Settings and New Session.
6. Keep stderr in a bounded diagnostic ring; do not mix it into the conversation.

### 3.4 Capability-driven configuration

ACP session config options and modes are normalized into app-owned controls:

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
    | 'select'
    | 'unknown';
  label: string;
  description: string | null;
  currentValue: string | boolean | null;
  values: Array<{
    value: string;
    label: string;
    description: string | null;
  }>;
  mutable: boolean;
  unavailableReason: string | null;
}
```

The composer places Model, Effort, Permissions, and current collaboration/plan mode in the primary
control row. Fast/service tier and unknown provider options live under More. A selector updates
only after the runtime confirms the value. A rejected, unsupported, stale-generation, or mismatched
update returns to the observed value and shows the reason.

### 3.5 Commands are not configuration

Create a merged command registry with these sources:

- provider-advertised commands supported by the active ACP adapter;
- provider/project skills;
- Assembly-local actions that make sense from the composer;
- no TUI-only presentation commands.

The command menu may include actions such as review, compact, goal, MCP status, or a project skill
when the runtime advertises them. Model, effort, permissions, plan/collaboration mode, service tier,
and fast mode are selectors. Theme, pets, raw scrollback, TUI keymap, terminal title/statusline,
TUI resume/archive/delete/app, and debug commands are not copied into the structured menu.

Every command descriptor says whether it inserts text, invokes an ACP command, invokes an
Assembly-local action, requires arguments, or is unavailable. Selecting a row never auto-submits.

## 4. Canonical conversation and task model

### 4.1 Typed event stream

Expand the existing generation/sequence protocol. Provider-specific details stay in optional raw
metadata; Svelte never branches on raw Codex/Claude wire messages.

```ts
export type CanonicalItemType =
  | 'user-message'
  | 'assistant-message'
  | 'reasoning'
  | 'plan'
  | 'task-list'
  | 'command-execution'
  | 'file-change'
  | 'mcp-tool-call'
  | 'dynamic-tool-call'
  | 'web-search'
  | 'image-view'
  | 'image-generation'
  | 'subagent-call'
  | 'context-compaction'
  | 'review'
  | 'error'
  | 'unknown';

export type AgentEventKind =
  | 'session.started'
  | 'session.capabilities.updated'
  | 'session.config.updated'
  | 'session.state.changed'
  | 'turn.started'
  | 'turn.completed'
  | 'turn.interrupted'
  | 'item.started'
  | 'item.updated'
  | 'item.completed'
  | 'content.delta'
  | 'approval.requested'
  | 'approval.resolved'
  | 'user-input.requested'
  | 'user-input.resolved'
  | 'plan.updated'
  | 'tasks.updated'
  | 'usage.updated'
  | 'children.updated'
  | 'runtime.warning'
  | 'runtime.error';
```

Keep ordered items. Do not attach all tools to one assistant message or flatten them into prose;
text before a tool, the tool, its output, and text after it must remain in order.

### 4.2 Conversation UI decomposition

Refactor the current monolithic surface:

```text
ConversationSurface.svelte
├── ConversationHeader.svelte
├── ConversationAgentTree.svelte
├── ConversationTimeline.svelte
│   └── TimelineItem.svelte
├── ConversationComposer.svelte
├── AgentConfigControls.svelte
├── ConversationCommandMenu.svelte
├── ConversationTaskSummary.svelte
└── ConversationHandoffControls.svelte
```

Use a real sanitized Markdown pipeline shared with the Markdown work package. Tool rows are compact
by default and expand to structured input/output, location, duration, status, and error details.
File-change rows open the existing Diff destination. Plans and tasks are first-class progress
surfaces. Approval and structured-input requests appear inline and target the exact request ID and
generation.

### 4.3 Screenshot and image attachment contract — P0

Preserve the existing validated Rust attachment vault and add ACP-native content blocks.

1. Paste or file selection captures PNG, JPEG, GIF, or WebP only after signature validation.
2. Enforce per-file, per-message, and per-session pending-byte/count limits.
3. Show immediate removable previews in the exact owning draft.
4. Structured ACP send passes an image content block when the provider advertises image support.
5. If the provider does not support images, disable image send with a plain reason; do not claim a
   path reference is equivalent.
6. Native-PTY send may use the provider's supported file-reference syntax as a compatibility path,
   clearly identified as terminal mode.
7. Clear and delete managed files only after successful send or explicit removal; a failed send
   restores the exact draft and previews.
8. Browser grabs and drawings become the same attachment type plus bounded source metadata.
9. Never persist blob URLs, cookies, complete page HTML, clipboard history, or user-owned source
   files.

### 4.4 Terminal ownership and handoff

`Open in native CLI` and `Return to Conversation` are backend state transitions, not CSS toggles.

Structured to terminal:

1. Finish or explicitly interrupt the active turn.
2. Persist/reconcile the latest native session ID and canonical events.
3. Release the ACP writer lease.
4. Start the existing PTY service with the provider's exact resume command.
5. Mark `native-pty` only after the PTY exists.
6. Start incremental read-only transcript projection.
7. On failure, close the partial PTY and restore the ACP lease.

Terminal to structured:

1. Require the native CLI to exit or receive explicit confirmation to stop it.
2. Reconcile appended transcript/history.
3. Resume the same provider-native session through ACP.
4. Verify native identity and latest turn boundary.
5. Acquire the ACP lease and restore structured controls.
6. On failure, leave the session terminal-owned and readable.

Also offer **Fork to native CLI**, which creates a new owned session and does not surrender the
current writer.

## 5. Agent hierarchy and workflow orchestration

### 5.1 Two child classes

```ts
export type AgentChildOwnership = 'provider-native' | 'assembly-managed';
```

- **Provider-native:** spawned internally by Codex/Claude. Assembly observes their identity,
  relationship, state, tool linkage, and transcript/output when exposed. Controls are capability
  gated; no generic kill/retry promise is made.
- **Assembly-managed:** created by the workflow engine as a separate ACP session with a known role,
  provider, model, effort, permissions, worktree, parent node, and lifecycle. Assembly can start,
  pause where supported, cancel, retry, send follow-up, and inspect complete output.

The Agent Tree visually distinguishes the two and never labels an observed provider child as an
Assembly-controlled worker.

### 5.2 Durable `WorkflowEngine`

Create one Rust workflow module rather than expanding frontend stores into an orchestrator:

```text
src-tauri/src/workflows/
├── mod.rs
├── engine.rs
├── commands.rs
├── decider.rs
├── projector.rs
├── store.rs
├── scheduler.rs
├── delegation.rs
├── artifacts.rs
├── approvals.rs
└── types.rs
```

The runtime database is local SQLite under the existing application-support compatibility root,
with WAL, schema migrations, bounded strings, and one Rust writer. It stores workflow control
state, not provider credentials, cookies, full environments, or arbitrary repository contents.
The existing orchestration JSONL remains untouched as a compatibility/audit export during this
wave; a later migration may import stable references after explicit proof.

Minimum tables:

```text
workflow_definitions
workflow_definition_versions
workflow_runs
workflow_nodes
workflow_attempts
workflow_dependencies
workflow_events          -- append only
workflow_artifacts
workflow_approvals
workflow_agent_links
agent_event_receipts
usage_samples
usage_daily_rollups
```

Event append and projection updates occur in one transaction. Filtering, grouping, sorting,
joins, and paging execute in SQLite, never after materializing the entire ledger in TypeScript.

### 5.3 Workflow definition

```ts
export interface WorkflowDefinition {
  id: string;
  name: string;
  version: number;
  description: string;
  inputs: WorkflowInputDefinition[];
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowDependency[];
  concurrencyLimit: number;
  stopPolicy: WorkflowStopPolicy;
  budget: WorkflowBudget;
}

export interface WorkflowNodeDefinition {
  id: string;
  title: string;
  role:
    | 'orchestrator'
    | 'implementer'
    | 'code-reviewer'
    | 'spec-reviewer'
    | 'test-runner'
    | 'security-reviewer'
    | 'custom';
  providerPreference: string[];
  modelSelector: string | null;
  effortSelector: string | null;
  permissionMode: string | null;
  promptTemplate: string;
  worktreeStrategy: 'shared-readonly' | 'dedicated-worktree' | 'current-worktree';
  dependsOn: string[];
  maxAttempts: number;
  timeoutSeconds: number | null;
  completionContract: WorkflowCompletionContract;
  approvalBeforeStart: boolean;
  approvalBeforePublish: boolean;
}
```

Definitions are versioned and immutable once a run starts. Editing creates a new version. Each
attempt records the resolved provider/model/effort/mode and never silently changes because a
provider catalog changed later.

### 5.4 Delegation bridge

Expose app-owned tools through a local, token-authenticated MCP bridge available only to approved
orchestrator sessions:

```text
delegate_agent
get_agent_task_status
send_agent_follow_up
cancel_agent_task
list_workflow_artifacts
submit_workflow_result
request_workflow_approval
```

The bridge sends typed requests to `WorkflowEngine`; it never spawns a shell directly. Depth,
child count, concurrency, provider allow-list, cwd/worktree, model/effort, token/time budget, and
parent identity are validated in Rust. The parent receives a task ID immediately and can poll or
await according to workflow policy. Child output is stored as bounded result/artifact references;
the complete provider transcript remains in its native session.

The UI may also start nodes directly through the same engine. There is one delegation path,
whether initiated by a user, workflow scheduler, or orchestrator agent.

### 5.5 Run and node states

```ts
export type WorkflowRunState =
  | 'draft'
  | 'queued'
  | 'running'
  | 'waiting-for-approval'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type WorkflowNodeState =
  | 'blocked'
  | 'ready'
  | 'queued'
  | 'starting'
  | 'running'
  | 'waiting-for-input'
  | 'waiting-for-approval'
  | 'retrying'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'skipped';
```

Every transition is an append-only event with actor, parent, attempt, reason, and deterministic
receipt. Late events from an old attempt/generation are retained as diagnostics but cannot mutate
the current projection.

### 5.6 Initial workflow templates

Ship editable templates, not hard-coded behavior:

1. **Implement → Tests → Code Review → Spec Compliance**
2. **Investigate → Reproduce → Fix → Regression Review**
3. **Draft PR → Review Diff → Address Findings → Final PR Check**
4. **Parallel implementation lanes → Integrator → Cross-lane review**
5. **Browser feedback queue → UI implementer → visual verifier**

Each template defines where parallelism is legal. The scheduler launches nodes only after all
required dependency receipts are complete and the concurrency/worktree policy allows it.

### 5.7 Agent Control workspace

Add a center Dockview destination named **Agents** with nested Paneviews, not a modal dashboard:

- Workflow runs/history list.
- Run outline or DAG/plan view.
- Agent tree grouped by run, node, and attempt.
- Current node/task, provider/model/effort/mode, elapsed time, and worktree.
- Live typed event timeline.
- Approvals and requested input queue.
- Artifacts, commits, diffs, tests, PR links, and completion receipts.
- Pause run, cancel run, retry node, skip optional node, send follow-up, open conversation,
  open terminal, open worktree, and inspect logs—capability and state gated.

The compact Agent Dashboard indicator may show running/attention counts. Full control belongs in
the Agents destination.

## 6. Browser workbench refinement

The existing Browser Work Package remains largely correct. Refine it to match the evidenced Orca
interaction and the shared attachment/conversation contracts.

### 6.1 Three presentation sizes, one live page

```ts
export type BrowserPresentationMode = 'docked' | 'overlay' | 'expanded' | 'collapsed';
```

- **Docked:** normal Browser Dockview destination.
- **Overlay:** resizable floating browser above the workbench, matching the smaller screenshot.
- **Expanded:** fills the application client area beneath native macOS chrome, matching the larger
  screenshot.
- **Collapsed:** browser webview is parked and only the global action/FAB affordance remains.

All modes move the same native child webview and preserve URL, history, cookies/profile, scroll,
forms, tabs, viewport, annotations, and pending feedback. No second browser instance and no reload
for a presentation change.

### 6.2 Toolbar contract

The toolbar exposes, in order appropriate to available width:

- profile/current profile;
- address, back, forward, reload;
- Import;
- Grab page element;
- Annotate page element;
- Draw on screenshot;
- development-only browser devtools;
- Open in default browser;
- overflow with Default/New Profile, Import Cookies policy, Viewport Size, Browser Settings;
- Overlay/Expand/Restore and Collapse controls.

Every icon has a text tooltip and accessible label. Devtools are omitted from release unless the
release policy explicitly enables them.

### 6.3 Grab, annotate, and draw artifacts

A browser context attachment contains:

```ts
export interface BrowserContextArtifact {
  id: string;
  workspaceId: string;
  tabId: string;
  generation: number;
  kind: 'element-grab' | 'element-annotation' | 'markup-screenshot';
  url: string;
  title: string;
  selector: string | null;
  accessibleName: string | null;
  textSnippet: string | null;
  rect: { x: number; y: number; width: number; height: number } | null;
  note: string | null;
  intent: 'change' | 'question' | null;
  screenshotAttachmentId: string | null;
  createdAt: string;
}
```

- **Grab** captures bounded element semantics plus a clipped screenshot and adds a reviewable
  composer attachment.
- **Annotate** highlights the selected element and opens the Change/Question note card. Add queues
  the immutable artifact; it does not submit.
- **Draw** captures the current viewport, opens the markup surface, and stores the result through
  the same attachment vault.
- Copy All emits bounded Markdown plus attachment references.
- Send stages artifacts into the exact matching owned draft with conflict preview; it does not
  press Enter.
- Ask Agent opens an agent recipe using the same immutable artifact IDs.

### 6.4 Browser automation boundary

The browser may expose an approved automation service to an agent workflow: navigate, inspect
bounded accessibility/element facts, screenshot, click/type through explicit typed actions, and
wait for conditions. It never exposes arbitrary eval, cookies, storage, tokens, full DOM/HTML, or
unbounded network logs. Every mutation is scoped to workspace/tab/generation and visible in the
agent/tool timeline.

## 7. Resources, Space, and Usage

### 7.1 Compact Resource Manager and full Resources destination

The status-bar resource segment opens a compact popover like the supplied evidence:

- total CPU/RSS;
- project → worktree → session/process tree;
- live state dots;
- known ports;
- language servers;
- refresh;
- confirmation-gated stop only for proven Assembly-owned processes;
- Space summary/action at the bottom.

**Review resources** opens the full center **Resources** Dockview destination with Paneviews for
Processes, Sessions, Language Servers, Ports, and Logs. External or unproven processes never show
a stop action.

### 7.2 Compact Space summary and full Space workspace

Space is separate from live process resources. Its compact footer shows scanned bytes,
reclaimable bytes, workspaces, last update, Scan/Cancel, and Review.

The full Space destination includes:

- worktree treemap;
- selected worktree directory breakdown;
- searchable/sortable table;
- protected/main/active/dirty/unmerged/locked reasons;
- exact selected reclaimable total;
- archive as recoverable alternative;
- typed confirmation for destructive deletion;
- existing worktree safety service as the sole mutation authority.

### 7.3 Compact Usage roster and full Usage analytics

The status bar shows one quiet summary per enabled provider. Its popover provides Detailed/Compact,
per-provider windows and reset times, refresh, Usage details & history, and Manage Accounts.

The full center **Usage** destination provides:

- agents spawned, agent working time, PRs created, tracking start;
- provider selector and overview;
- total input/output/cache/reasoning tokens where source data distinguishes them;
- daily intensity/history;
- provider/session/model/workflow breakdown;
- cache share and context usage;
- rate-limit windows and reset history;
- estimated cost only when an exact versioned pricing source exists, prominently labelled
  estimated; otherwise omit cost rather than inventing it;
- pagination and aggregation in the Rust/SQLite query, not in Svelte.

ACP usage updates feed live session state. Provider transcript/account scanners may supply history
with source and confidence labels. Raw prompt/response content is not copied into usage storage.

## 8. Navigation, sessions, Dockview, and Paneview

### 8.1 Left navigation responsibilities

The left side is a compact work navigator, closer to the supplied Orca/ChatGPT evidence:

- top-level app destinations;
- project/repository headings;
- compact worktree/session rows beneath them;
- active/running/attention state;
- short title and provider/model where known;
- no permanently wide card body;
- hover/focus card with basic identity, branch/worktree, latest activity, model/effort, PR/check
  hint, and primary actions;
- explicit expansion for the full detail/actions, never hover-only functionality.

Do not duplicate the complete session transcript or resource state in the hover card.

### 8.2 Working and Done are Paneviews

Replace the custom independent flex/collapsible stacks with one left-region Paneview instance:

```text
Left Work Paneview
├── Working
├── Done
└── optional Settled
```

Each section has persisted size/collapse state, count, minimum, accessible separator, and its own
virtualized/scrollable rows. Expanding a row cannot squeeze other rows into slivers. Moving a
session between lifecycle states moves one row between Paneview sections without rebuilding the
session workspace.

### 8.3 Session discovery is a separate destination

Create a center **Sessions** destination, with an optional command to move it into a right-side
Paneview later. It owns:

- all provider-native resumable sessions;
- search;
- provider/project/worktree/date/model filters;
- grouped or flat view;
- exact resumability and missing-worktree reasons;
- preview/history details;
- Resume, Fork, Inspect, Copy ID/path, Open folder;
- no mixing with the active Working/Done rail.

The scanner and `OwnedSession` remain the data authority. Opening the discovery workspace starts no
provider or PTY until the user chooses a concrete action.

### 8.4 All tab-contained side stacks are Paneviews

- Center destinations: Dockview.
- Left/right/bottom vertical section stacks inside a destination or shell region: Paneview.
- Editor files: the existing planned nested Dockview file panels.
- Conversation timeline items, lists, forms, menus, and dialogs are not Paneviews.

Create one Paneview wrapper/service with versioned layout capture, parking/reuse, per-panel
permanence, visibility, and session/global scope. Do not instantiate a new layout engine in every
feature.

## 9. AI throughout the workbench

Create an app-owned `AssistanceRecipeRegistry`. A recipe declares inputs, context sources, provider
capability requirements, output schema, preview surface, validation, and optional typed action.

Initial recipes:

- draft PR title/body;
- review PR/diff;
- address review comments;
- explain failing checks;
- propose fixes for Problems;
- generate/edit run configuration;
- summarize session/worktree/project;
- review browser annotations;
- fill a form from deterministic facts;
- save-time validation/checklist;
- explain resource or cleanup risk;
- propose workflow definition from a goal.

Rules:

1. Read deterministic facts from existing services and include source references.
2. Treat repository, browser, issue, review, and remote text as delimited untrusted data.
3. Generate a typed proposal; never mutate from free-form model text.
4. Show preview, changed fields, confidence/unknowns, consequences, and source facts.
5. On accept, re-read expected state and invoke the existing typed service.
6. Remote/destructive actions retain their normal confirmation.
7. Save-time AI checks are advisory unless a deterministic validator independently blocks save.
8. A recipe may run in the current session or spawn an Assembly-managed workflow node, based on
   visible user choice.
9. Record bounded audit receipts and never credentials, cookies, environment secrets, or entire
   files unless the user explicitly attached them.

## 10. Changes to the existing master work packages

### Work Package 2 — shell and layout

- Add the shared Paneview host/layout contract.
- Convert left Working/Done and right tool sections to Paneview.
- Keep center Dockview and nested editor-file Dockview.
- Add Sessions, Agents, Resources, Space, and Usage center destinations.
- Preserve existing panel parking, activation gates, and per-owned-session center layout.

### Work Package 3 — sessions

- Compact active-work rows and hover/focus detail.
- Remove Find/Resume discovery from `SessionsColumn`.
- Add the separate Sessions destination.
- Preserve lifecycle state, exact actions, notifications, and `ownedId` authority.

### Work Package 8 — browser

- Add intermediate overlay mode.
- Lock the evidenced toolbar order and profile overflow.
- Unify Grab, Annotate, and Draw with the conversation attachment vault.
- Keep the same native webview across docked/overlay/expanded.
- Add bounded agent browser automation through typed actions.

### Work Package 9 — resources and usage

Split into independently implementable lanes:

- 9A native ownership/resource snapshot;
- 9B compact Resource Manager + full Resources destination;
- 9C Space scanner/compact/full workspace;
- 9D usage collectors/rate limits;
- 9E compact Usage roster + full Usage analytics.

### Work Package 10A — conversation

Replace the PTY-only execution authority with the `AgentRuntimeManager` and ACP structured runtime
in this amendment. Keep transcript projection for PTY-owned and external sessions. Replace slash
configuration commands with config selectors. Make screenshot image content P0. Render typed
items, plans, tasks, approvals, questions, and both child classes.

### Work Package 11 — integrated agent

Split into:

- deterministic context and recipe registry;
- durable WorkflowEngine;
- ACP delegation bridge;
- Agent Control destination;
- guarded typed actions and audit receipts.

The previous “guarded integrated agent” context/action layer remains useful but is not sufficient
for user-defined orchestrated workflows.

## 11. Parallel implementation plan

Shared seams are frozen in Wave 0. After that, independent lanes may share the integration
worktree only under exact file ownership and at most two heavy runner slots.

### Wave 0 — sequential architecture contracts

1. Re-anchor repository symbols at current `main`.
2. Create `AgentRuntimeManager`, event/config/capability type contracts, and ownership state machine
   with deterministic mock tests only.
3. Create workflow schema/event contract and migration plan, without launching providers.
4. Create shared Paneview registration/layout contract.
5. Add center destination IDs and controller-owned integration stubs.
6. Record sidecar packaging decision after macOS arm64/x64 startup and bundle-size spike.

### Wave 1 — parallel foundation lanes

- **A: ACP runtime:** Rust SDK client, pinned Codex adapter, pinned Claude adapter, capabilities,
  lifecycle, reconnect, stop, config changes, mock/recorded fixtures.
- **B: Canonical conversation:** rich event reducer, typed timeline items, safe Markdown, task/plan
  projections, long-list behavior.
- **C: Attachments:** image vault extensions, ACP image blocks, preview restore/cleanup, aggregate
  limits.
- **D: Paneview shell:** shared host, Working/Done conversion, right section conversion, layout
  persistence/migration.
- **E: Workflow store:** SQLite migrations, append/project transaction, query API, scheduler unit
  tests.
- **F: Navigation:** compact rows, hover/focus details, separate Sessions destination.

### Wave 2 — parallel product lanes

- **G: Composer/config controls:** pickers, commands/skills, approvals/questions, Stop, handoff UI.
- **H: Delegation:** MCP bridge, child spawn/link/status/cancel/follow-up, depth/concurrency/budget.
- **I: Agent Control:** run list, DAG/outline, agent tree, attempts, approvals, artifacts, controls.
- **J: Browser:** native webview spike, overlay/expanded modes, toolbar, profiles, Grab/Annotate/Draw.
- **K: Resources:** native process ownership, compact popover, full Resources.
- **L: Space:** scan, treemap, selection, worktree-safe delete/archive.
- **M: Usage:** live usage, history collectors, SQLite rollups, compact/full UI.
- **N: Assistance recipes:** deterministic context, typed proposals, PR/forms/save checks.

### Wave 3 — integration and migration

1. New structured sessions start ACP-owned and receive native identity before appearing live.
2. Existing/adopted PTY sessions remain PTY-owned with incremental transcript projection.
3. Add terminal/ACP handoff and fork flows.
4. Join Assembly-managed workflow agents and provider-native children in one hierarchy.
5. Connect Browser artifacts, PR recipes, Problems, run configurations, Git, resources, and usage
   to exact owned/workflow IDs.
6. Migrate layouts to Paneview/Dockview versions without losing current session workspaces.
7. Run provider/session/workflow/browser/resource/usage native acceptance serially.
8. Perform SOL-medium milestone review after each integrated wave.

## 12. Required tests and native acceptance

### Runtime and conversation

- New/resume/reconnect/cancel for Claude and Codex.
- Model/effort/permissions/mode choices discovered and confirmed.
- No configuration disguised as a slash command.
- Screenshot paste/preview/remove/restart/send for both providers.
- Text + images sent once to the exact owned session.
- Commands, edits, reasoning, plans, tasks, approvals, user questions, usage, and errors render in
  correct order.
- Long conversation scrolling does not pull a reader away from older text.
- Structured/terminal handoff proves one provider writer and rolls back on every failure point.

### Workflows

- DAG validation, cycle rejection, concurrency, dependencies, optional nodes, retries, timeout,
  pause/cancel, stale attempt events, and restart replay.
- Cross-provider parent/child delegation with exact model/effort/mode defaults.
- Depth/child/budget/worktree bounds.
- Implement/review/spec workflow runs in parallel where declared and serially where dependent.
- Approval and requested input resume the exact run/node/attempt.
- Full child output, artifacts, commit/test/PR links, and cancellation are inspectable.
- Provider-native children remain visible but are not falsely controllable.

### Shell and navigation

- Working/Done Paneview resize/collapse/restore and lifecycle moves.
- Right-side Paneview parking/reuse and no startup IO.
- Compact rows, keyboard hover-equivalent details, accessible actions.
- Sessions discovery is separate and starts nothing on open/filter.
- Center and per-session layout migration preserves editor/browser/diff/conversation state.

### Browser

- One live page across docked/overlay/expanded/collapsed.
- Toolbar actions, profiles, viewport, external open, and release devtools policy.
- Element Grab, annotation card, screenshot markup, Copy All, Send staging, and Ask Agent.
- Owner/generation mismatch and draft conflict retain artifacts.
- Child webview never covers dialogs, tooltips, FAB, annotations, Settings, or confirmation UI.

### Resources, Space, and Usage

- One bounded OS snapshot and proven ownership before stop.
- Compact Resource Manager and full Resources navigate to exact session/process.
- Space scan, treemap, protected reasons, selected total, archive/delete safety.
- Usage reset windows, live ACP usage, history rollups, provider/model/session/workflow filters,
  source labels, and no invented cost.

### AI integration

- Hostile remote/browser/review text remains data.
- Typed proposal schema validation.
- Expected-state mismatch blocks execution.
- PR draft/review, form fill, save check, browser feedback, and workflow proposal previews.
- Cancel produces zero mutation; accept invokes exactly one existing typed service and audit receipt.

## 13. Stop conditions

Stop before implementation and return a decision packet when:

- an adapter cannot provide a required feature and no honest capability-degraded UX is specified;
- Claude adapter packaging requires an unapproved runtime/distribution change;
- a provider cannot resume the same native session during terminal handoff;
- a child webview cannot stay below main-webview HTML overlays;
- an ACP/protocol update requires provider-specific logic in Svelte;
- workflow storage would copy secrets, arbitrary repository content, or full provider transcripts;
- a lane needs a second session, terminal, Git, worktree, browser, settings, editor, or confirmation
  authority;
- a Paneview migration would discard an existing layout instead of versioning it;
- provider-native child association cannot be proven;
- a requested AI action lacks a deterministic typed service and revalidation path.

## 14. Completion definition

Assembly supports Claude Code and Codex as structured ACP conversations with native image
attachments, model/effort/permission selectors, typed tools/edits/reasoning/plans/tasks,
approvals/questions, usage, and visible subagents. The native CLI remains available through a safe
single-writer handoff. Users can define and run durable cross-provider workflows with orchestrator,
implementer, reviewer, test, and compliance roles; inspect every node and agent; and control
Assembly-managed attempts from the Agents workspace.

The workbench uses compact project/work rows, separate session discovery, Dockview destinations,
and Paneview side sections. Browser, Resources, Space, and Usage each offer a compact entry and a
full workspace. Browser grabs, annotations, and drawings become exact reviewable conversation
context. AI assistance is available throughout PRs, forms, saves, browser feedback, Git, resources,
and workflows without bypassing deterministic services, previews, confirmations, or stale-state
checks.
