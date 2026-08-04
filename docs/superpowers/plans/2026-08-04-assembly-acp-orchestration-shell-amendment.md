# Assembly ACP, Workflow Orchestration, and Workbench-Surface Amendment

**Date:** 2026-08-04  
**Status:** Product direction locked by the user; implementation has not started  
**Scope:** Conversation runtime, model and permission controls, screenshots, tools, plans, child agents, workflow orchestration, agent control, browser feedback, resources, usage, session navigation, Paneview layout, and app-wide AI assistance.

## 0. Plan authority

This document is the newest execution amendment for:

- `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`
- `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`

It does not discard the evidence, safety work, Roslyn contract, terminal lifecycle, Git/worktree safety, browser security requirements, or implementation-packet discipline in those files. It changes the product and runtime decisions named below.

This amendment supersedes these parts of the older plans:

1. The statement that the interactive PTY must remain the permanent sole conversation writer.
2. Work Package 10A's transcript-only conversation architecture.
3. Work Package 11's limited guarded-agent scope where it conflicts with the workflow engine and Agent Control Center specified here.
4. The existing left-column decision that mixes owned work with the full resumable-session browser.
5. The assumption that Working and Done remain ordinary CSS collapsibles instead of Paneview panes.
6. Resource and provider usage presentation where only one surface is described instead of compact and expanded surfaces.
7. Browser presentation where any requirement below is more precise than the prior text.

The single-writer safety rule remains mandatory, but the writer can now be the structured ACP runtime or the native CLI terminal. The active owner is explicit and transferable.

---

## 1. Locked product decisions

1. **New Claude Code and Codex sessions open as structured conversations by default.** They do not start a hidden interactive TUI merely so the app can read its transcript.
2. **Raw CLI is a separate ownership mode.** Opening raw Claude Code or Codex transfers the native conversation from the structured runtime to a PTY. Returning to Conversation transfers it back. Two writers never control one native conversation concurrently.
3. **Model, reasoning effort, permission/execution mode, collaboration/plan mode, fast/service tier, and similar runtime settings are selectors, not slash commands.** They come from the active provider's advertised configuration and are confirmed by the provider.
4. **TUI-local commands are not copied into Assembly's command menu.** Commands such as terminal theme, terminal pets, TUI keymap, raw-scrollback rendering, terminal title, and similar client-local behavior stay in the native TUI and appear only after opening the raw CLI.
5. **Screenshot paste and attachment is P0.** A pasted image must preview immediately, survive session switches, and reach the exact structured provider message as an image content block. Terminal-owned sessions retain a safe managed-file fallback.
6. **The conversation timeline is typed and rich.** It renders assistant text, reasoning summaries, commands and output, file changes and diffs, MCP/dynamic tools, plans and tasks, approvals, structured questions, context compaction, reviews, errors, and child-agent activity.
7. **Provider-native child agents are visible, but Assembly owns durable workflows.** Native Claude/Codex subagents are normalized and displayed. Cross-provider orchestration, scheduling, budgets, retries, gates, worktree allocation, and role contracts belong to Assembly's workflow engine.
8. **Workflows can use an orchestrator plus role-based children.** Built-in role templates include implementer, code reviewer, spec-compliance reviewer, tester, fixer, researcher, and integrator; users can create custom roles and DAGs.
9. **All major surfaces use the existing Dockview family.** Center destinations are Dockview panels. Working/Done/Settled and right-side tool sections use Dockview Paneview. Do not add another layout engine.
10. **The left side is a compact active-work rail, not the archive/session browser.** It shows owned workspaces and active sessions with compact rows, hover details, and explicit accordion details. Resumable/discovered sessions move to a separate Session Explorer surface.
11. **Browser, Resource Manager, Workspace Space, and Usage each have compact and expanded surfaces.** Compact surfaces are fast summaries. Expanded surfaces are real Dockview workspaces, not enlarged popovers.
12. **Browser annotations become structured conversation attachments.** Element identity, bounded page context, screenshot/crop or markup, note, intent, URL, tab, workspace, and generation travel together and remain reviewable before submission.
13. **AI assistance is available throughout the product through typed assist actions.** It proposes PR text/reviews, form patches, run configurations, browser feedback, summaries, and save checks. It never becomes an untyped arbitrary mutation path.

---

## 2. What ACP does and does not do

### 2.1 ACP does not create a user-visible terminal by itself

An ACP client starts or connects to an **agent adapter process** and exchanges structured protocol messages over stdio or another supported transport. That process may start Codex app-server, the Claude Agent SDK, OpenCode, or another runtime. It is not an xterm session and does not require a visible terminal pane.

ACP can carry structured command/tool activity and terminal output produced by the agent's tools. That output belongs in the conversation timeline. It is different from giving the user the native full-screen Claude Code or Codex TUI.

Assembly therefore keeps two distinct concepts:

```text
Structured provider process
  ACP adapter -> provider runtime -> typed conversation events

Native CLI terminal
  PTY -> Claude Code/Codex TUI -> terminal screen and provider transcript
```

### 2.2 One native conversation, one writer

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';
```

The manager refuses a second writer for the same `ownedId` and provider-native session ID. A handoff has prepare, release, acquire, reconcile, and rollback phases. A failed target acquisition restores the previous owner or leaves the session stopped with a clear recovery action.

### 2.3 Provider portability

ACP is the preferred provider boundary because it gives Assembly a common transport for Codex, Claude Code, and future ACP-compatible runtimes. It does not erase provider differences. Every capability is discovered and gated. Provider-specific metadata is retained in an extension bag so new features are not discarded while the canonical contract catches up.

---

## 3. Researched implementation path

### 3.1 Foundation

- Use the official `agentclientprotocol/rust-sdk` as the Rust client boundary.
- Use a pinned, packaged `agentclientprotocol/codex-acp` sidecar for Codex. It already maps Codex app-server models, effort, fast mode, modes, images, tools, file changes, plans, reasoning, reviews, token usage, and subagent metadata.
- Use a pinned `agentclientprotocol/claude-agent-acp` distribution for Claude Code. It uses the official Claude Agent SDK and supports images, permissions, tools, TODOs, edit review, terminals, commands, and nested subagent transcript metadata.
- The Claude adapter currently requires a compatible Node runtime. Packaging must choose one of: bundled signed Node runtime, a reviewed standalone bundle, or an explicit minimum local Node requirement. No implementation packet may leave this implicit.
- Keep a direct Codex app-server implementation only as a provider-specific escape hatch behind the same runtime trait. It is not the first implementation and must not fork the UI contract.

### 3.2 Useful external references

- **T3 Code:** canonical event taxonomy, provider adapter boundary, rich timeline, plans, approvals, and Markdown behavior.
- **Jockey:** Tauri/Rust ACP process lifecycle, discovered models, modes, config options, and commands.
- **CodeG:** ACP child-session delegation broker, parent/child linkage, depth limits, cancellation, per-agent config defaults, status cards, and MCP-mediated delegation.
- **Orca:** browser grab/annotation/markup interaction, compact-to-expanded resource and usage surfaces, workspace-space treemap, and compact workspace rows/hover details.

These projects are implementation references, not permission to copy source blindly. Before reusing code rather than behavior, record its license, dependency impact, architecture assumptions, and attribution requirements. Orca's Electron webview code is specifically not transplanted into Tauri; Assembly reproduces the interaction contract through managed WKWebViews and main-webview overlays.

---

## 4. Revised runtime architecture

```text
OwnedSession (stable ownedId)
        |
        v
AgentConversationManager -- one entry per ownedId
        |
        +-- Structured owner
        |     ACP client connection
        |       +-- codex-acp -> codex app-server
        |       +-- claude-agent-acp -> Claude Agent SDK
        |       +-- future ACP provider
        |
        +-- Terminal owner
        |     existing TerminalService/TerminalRegistry
        |     incremental transcript projection
        |
        +-- Handoff coordinator
        |     prepare/release/acquire/reconcile/rollback
        |
        +-- Canonical AgentEvent stream
                |
                +-- conversation reducer/store
                +-- workflow projection
                +-- agent-control projection
                +-- durable event journal/snapshots
```

### 4.1 Existing authorities retained

Retain and extend:

- `ownedId` and `OwnedSession` as the application identity.
- `TerminalService`, `TerminalRegistry`, scrollback, process ownership, and tombstones.
- per-session editor/browser/diff/Dockview workspace snapshots.
- the managed conversation attachment vault.
- Tauri IPC and typed events; do not add an internal WebSocket server for local desktop operation.
- the existing conversation reducer/store as the only frontend conversation state owner.

### 4.2 New Rust layout

```text
tauri-svelte-preview/src-tauri/src/agent_conversation/
  mod.rs
  manager.rs
  ownership.rs
  capabilities.rs
  events.rs
  event_buffer.rs
  persistence.rs
  handoff.rs
  attachments.rs
  transcript/
    mod.rs
    watcher.rs
    codex.rs
    claude.rs
  providers/
    mod.rs
    runtime.rs
    acp_client.rs
    acp_process.rs
    adapter_manifest.rs
    direct_codex.rs       # optional escape hatch, not first milestone
```

### 4.3 Provider runtime trait

```rust
#[async_trait]
pub trait StructuredAgentRuntime: Send + Sync {
    async fn initialize(&mut self) -> Result<AgentCapabilities, AgentRuntimeError>;
    async fn start(&mut self, request: StartAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn resume(&mut self, request: ResumeAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn send(&mut self, request: SendAgentInput) -> Result<(), AgentRuntimeError>;
    async fn interrupt(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError>;
    async fn set_option(
        &mut self,
        option_id: &str,
        value: AgentOptionValue,
    ) -> Result<(), AgentRuntimeError>;
    async fn respond_to_request(
        &mut self,
        request_id: &str,
        response: AgentRequestResponse,
    ) -> Result<(), AgentRuntimeError>;
    async fn close(&mut self) -> Result<(), AgentRuntimeError>;
}
```

The manager receives provider updates, assigns Assembly sequence/generation identity, stores a bounded replay buffer, persists durable events, and emits one Tauri event stream.

### 4.4 Canonical events and items

```ts
export type AgentContentChannel =
  | 'assistant'
  | 'reasoning'
  | 'reasoning-summary'
  | 'plan'
  | 'command-output'
  | 'file-change-output';

export type AgentItemKind =
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
  | 'subagent'
  | 'review'
  | 'context-compaction'
  | 'error'
  | 'unknown';

export type AgentEventKind =
  | 'session.started'
  | 'session.configured'
  | 'session.state.changed'
  | 'session.exited'
  | 'turn.started'
  | 'turn.completed'
  | 'turn.interrupted'
  | 'item.started'
  | 'item.updated'
  | 'item.completed'
  | 'content.delta'
  | 'request.opened'
  | 'request.resolved'
  | 'user-input.requested'
  | 'user-input.resolved'
  | 'usage.updated'
  | 'children.updated'
  | 'warning'
  | 'error';
```

Every event carries `ownedId`, provider, provider session ID, generation, sequence, timestamp, optional turn/item/request IDs, canonical payload, and optional bounded provider metadata.

The transcript reader becomes a `TerminalTranscriptProjection`. It emits the same canonical events from appended JSONL when possible, imports history, and reports limited capabilities. It is not the primary live structured protocol.

### 4.5 Incremental transcript projection

Replace the frontend 500 ms full-tail polling loop with a Rust-owned incremental watcher:

```rust
pub struct TranscriptCursor {
    path: PathBuf,
    file_identity: TranscriptFileIdentity,
    offset: u64,
    partial_line: Vec<u8>,
    generation: u64,
}
```

It reads only appended bytes, detects truncation/rotation, emits typed additions, and performs an occasional bounded reconciliation snapshot. A hidden session keeps its watcher without repeating directory scans or reparsing four megabytes twice per second.

---

## 5. Configuration controls and commands

### 5.1 Controls are provider configuration

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'reasoning-effort'
    | 'permission-mode'
    | 'collaboration-mode'
    | 'service-tier'
    | 'fast-mode'
    | 'boolean'
    | 'unknown';
  label: string;
  description: string | null;
  currentValue: string | boolean | null;
  choices: AgentConfigChoice[];
  mutable: boolean;
  disabledReason: string | null;
}
```

The composer footer renders recognized categories as first-class selectors. Unknown options go in a More menu. The UI never invents a model list or effort order. It preserves the provider-advertised order and changes the displayed value only after a provider confirmation/update.

### 5.2 Command catalog

Assembly merges:

1. provider-advertised commands that make sense in a structured client;
2. dynamic skills/prompts;
3. Assembly-local commands;
4. workflow commands when a workflow is active.

The default structured Codex list includes actions such as review, compact, status, MCP, skills, and provider-supported goal/plan actions. It does not include TUI-only theme, raw, title, statusline, keymap, Vim, pets, terminal-process listing, or debug commands.

Model, effort, permissions, mode, and fast/service tier never appear as ordinary slash rows when a selector is available.

Each command descriptor states whether it:

- inserts text;
- opens an Assembly surface;
- calls a typed provider operation;
- invokes an app-local action;
- requires raw CLI;
- is unsupported in the current ownership mode.

No command auto-submits merely because it was selected from completion.

---

## 6. P0 screenshot and rich-context attachment path

### 6.1 Structured attachment model

```ts
export type ConversationAttachment =
  | ImageAttachment
  | BrowserElementAttachment
  | BrowserMarkupAttachment
  | FileContextAttachment
  | DiffContextAttachment;
```

The existing Rust vault remains the one file authority. Add aggregate draft limits, owner-only file permissions, exact delete/prune commands, missing-file recovery, and persisted attachment IDs. Never persist blob URLs.

### 6.2 ACP send behavior

If the active provider advertises image input:

```text
user text block
image content block(s)
structured browser/file context block(s)
```

Images are sent as ACP image content rather than as a sentence containing a local file path. The adapter/provider performs its supported encoding.

If the provider does not advertise image input, Send is disabled for that image with a clear explanation or offers the explicit managed-file reference fallback when the agent can safely read the file. Assembly does not silently drop the image.

### 6.3 Terminal-owned fallback

For terminal-owned sessions, the current managed-file path strategy remains available. The app pastes one composed prompt and submits one Return. It keeps the draft and attachments until the terminal write succeeds.

### 6.4 Acceptance priority

Before timeline polish, native acceptance must prove:

1. paste PNG, JPEG, WebP, and a macOS screenshot;
2. immediate preview and removal;
3. two-session isolation;
4. app restart restoration;
5. one structured send containing the actual image;
6. provider response demonstrating it received the image;
7. exact cleanup with no user-owned file deletion;
8. unsupported-provider behavior that keeps the draft intact.

---

## 7. Workflow orchestration architecture

ACP normalizes agent sessions. It does not provide Assembly's durable workflow scheduler, worktree allocator, role policy, retry policy, or cross-provider DAG. Those are app-owned.

### 7.1 Workflow definitions

```ts
export interface AgentWorkflowDefinitionV1 {
  version: 1;
  id: string;
  name: string;
  description: string;
  orchestrator: WorkflowAgentTemplate;
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  maxConcurrency: number;
  maxDepth: number;
  maxTotalTurns: number | null;
  maxTotalTokens: number | null;
  timeoutSeconds: number | null;
  workspaceStrategy: 'shared-readonly' | 'dedicated-worktree' | 'mixed';
  completionPolicy: 'all-required' | 'orchestrator-decides' | 'manual-gate';
}

export interface WorkflowAgentTemplate {
  roleId: string;
  title: string;
  provider: string;
  modelOption: string | null;
  effortOption: string | null;
  permissionMode: string | null;
  collaborationMode: string | null;
  instructions: string;
  outputContract: WorkflowOutputContract;
  mayDelegate: boolean;
  maxChildren: number;
}
```

Built-in role templates:

- orchestrator/coordinator;
- implementer;
- code reviewer;
- spec-compliance reviewer;
- test runner/verification agent;
- defect fixer;
- researcher;
- integration/release agent.

Templates are editable copies. Provider model/effort/mode choices are validated against discovered capabilities at workflow start. A missing option blocks the node instead of silently downgrading it.

### 7.2 Durable run model

```ts
export type WorkflowRunState =
  | 'planned'
  | 'queued'
  | 'running'
  | 'waiting-for-agent'
  | 'waiting-for-user'
  | 'blocked'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type WorkflowNodeState =
  | 'pending'
  | 'ready'
  | 'launching'
  | 'running'
  | 'waiting-for-tool'
  | 'waiting-for-review'
  | 'waiting-for-approval'
  | 'retrying'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'skipped';
```

Persist append-only workflow events and periodic snapshots. Events include run/node/attempt IDs, parent/child edge, provider/native session IDs, ownedId, worktree, phase, progress, gates, artifacts, deterministic receipts, budget use, retry cause, and timestamps. Never infer node state from assistant prose.

### 7.3 Scheduling and worktrees

- A mutating child receives its own dedicated worktree unless the workflow explicitly serializes writes in one worktree.
- Read-only reviewers may share a stable snapshot or review a committed/checkpointed worktree.
- Two mutating nodes never write concurrently to the same worktree.
- Integration begins only after required implement/review/test gates pass.
- Worktree creation/removal uses the existing validated worktree service and confirmation rules.
- Workflow cleanup never bypasses dirty, unmerged, locked, primary-checkout, or ownership checks.

### 7.4 Two delegation paths

**Deterministic DAG path:** the engine starts ready nodes from the workflow graph. This is the default for repeatable workflows.

**Orchestrator-requested path:** expose a narrow MCP tool such as `assembly.delegate` to the orchestrator. Its broker validates:

- parent run/node/session;
- allowed role template;
- max depth and child count;
- provider/model/effort/mode availability;
- working directory and worktree policy;
- prompt/output bounds;
- token/turn/time budgets;
- cancellation and parent teardown.

This follows the proven CodeG broker pattern while preserving Assembly-owned policy. The orchestrator cannot pass arbitrary launch arguments or escape the workflow's provider/role/worktree allow-list.

Additional tools:

- `assembly.delegation_status`
- `assembly.cancel_delegation`
- later, after separate proof: `assembly.continue_delegation`

V1 child tasks may be one-shot. Persistent child sessions require explicit lifecycle and budget semantics before implementation.

### 7.5 Native provider subagents

Codex and Claude may spawn their own provider-native children. Assembly displays them and ingests their events. A native child becomes a durable workflow node only when a trusted correlation links it to a workflow node/tool request. Similar titles or directories are never enough.

---

## 8. Agent Control Center

Add a permanent center Dockview destination named **Agents** and a compact status projection available to the bottom action island/status bar.

### 8.1 Main views

- **Runs:** active and recent workflow runs.
- **Graph:** nodes, dependencies, gates, and critical path.
- **Agents:** parent/child tree grouped by role, provider, worktree, and state.
- **Timeline:** deterministic workflow events and provider activity.
- **Artifacts:** commits, diffs, test receipts, PRs, reports, and logs.
- **Templates:** workflow and role editor.

### 8.2 Node/agent details

Show:

- role and exact instructions/version;
- provider, model, effort, mode, permission policy;
- current loop phase and task/plan step;
- current tool and elapsed time;
- worktree, branch, ownedId, native session ID;
- retry, budget, depth, parent, and children;
- last deterministic receipt;
- live structured transcript/tool output;
- actions: Open conversation, Open worktree, Open diff, Open terminal when terminal-owned, Pause run, Cancel node, Retry failed node, Approve gate.

The UI does not claim a percentage unless the workflow has explicit weighted steps. Otherwise it shows completed/total steps and the named current phase.

### 8.3 Control safety

Pause/cancel/retry target exact run/node/attempt generations. A stale action is rejected. Cancelling the parent cancels or detaches children according to the workflow policy and records each outcome. A process that cannot be proven owned exposes no kill control.

---

## 9. Browser: Orca-inspired contract, Tauri implementation

The prior native-child-webview security and isolation rules remain. This amendment clarifies the required product states and attachment output.

### 9.1 Presentation states

```ts
export type BrowserPresentationMode =
  | 'collapsed-island'
  | 'docked'
  | 'expanded-overlay';
```

- **Collapsed island:** bottom-right floating control with active-page/annotation badge.
- **Docked:** normal Browser Dockview destination.
- **Expanded overlay:** same live tab fills the app content below macOS title-bar chrome and overlays rails/panes. It is not a second webview and does not reload.

The existing global action fan remains context-aware and collision-safe.

### 9.2 Toolbar actions

Required actions, capability-gated:

1. Import/context intake.
2. Grab page element.
3. Annotate page element.
4. Draw on screenshot.
5. Open browser devtools, development-only until release policy approves it.
6. Open current HTTP/S page in the default browser.
7. Profile menu: Default, New Profile, Import Cookies disabled until a security design exists, Viewport Size, Browser Settings.
8. Expand/Restore and Collapse.
9. Overflow for secondary safe actions.

### 9.3 Three distinct capture modes

**Grab element:** captures bounded element/page metadata and optional screenshot crop for immediate copy or composer attachment.

**Annotate element:** captures the same immutable target plus a user note and Change/Question intent. It queues feedback without sending.

**Draw on screenshot:** captures the visible browser viewport, opens main-webview markup tools, stores the marked image in the existing attachment vault, and creates a `BrowserMarkupAttachment`. The remote page never receives Tauri capabilities.

```ts
export interface BrowserContextAttachment {
  id: string;
  kind: 'browser-element' | 'browser-annotation' | 'browser-markup';
  workspaceId: string;
  tabId: string;
  generation: number;
  url: string;
  pageTitle: string;
  selector: string | null;
  role: string | null;
  accessibleName: string | null;
  textSnippet: string | null;
  rect: { x: number; y: number; width: number; height: number } | null;
  note: string | null;
  intent: 'change' | 'question' | null;
  imageAttachmentId: string | null;
  sourceHash: string;
  createdAt: string;
}
```

The composer renders this as a rich attachment chip/card. Structured send includes the image and a bounded, clearly delimited context block. It is never flattened into untrusted instruction text without labels.

### 9.4 Inspector security

Use a narrowly injected inspector script and a fixed message protocol. It may return target metadata and capture coordinates. It may not expose arbitrary eval, cookies, local storage, credentials, whole HTML, or cross-origin DOM access. Rust validates size and identity before emitting an event.

### 9.5 Browser implementation reference

Orca's `BrowserPane.tsx`, grab mode, annotation card, markup mode, and toolbar are interaction references. Assembly keeps the existing Tauri plan: managed WKWebViews, main-webview overlays, strict workspace profile isolation, generation guards, and no Tauri command capabilities on remote child views.

---

## 10. Resource Manager and Workspace Space

### 10.1 Compact Resource Manager

Add a status-bar/floating-island popover with:

- total CPU and RSS;
- grouped project -> worktree -> session/process tree;
- agent, terminal, browser, LSP, and run-configuration rows;
- small bounded CPU/RSS history sparklines;
- ports;
- refresh;
- exact owned-process stop with confirmation;
- link to the full Resource Manager.

Poll at two seconds only while the compact or full resource surface is visible. Hidden surfaces retain the last snapshot and refresh on activation/events instead of running a permanent UI timer.

External/unattributed processes show metrics but no stop action.

### 10.2 Compact Space section

The Resource popover includes a separate **Space** subsection:

- scanned bytes;
- reclaimable bytes;
- workspace count;
- updated time;
- Scan/Cancel/Refresh;
- Review action opening the full Workspace Space panel.

A running scan preserves and labels the previous completed result.

### 10.3 Full Resource Manager

Create a center Dockview panel with:

- grouped process tree and filters;
- CPU/RSS sort;
- ownership and process ancestry;
- language-server roots and logs;
- ports;
- safe stop/restart actions;
- memory-pressure policy and current evictions;
- links to exact session/worktree/browser/editor owners.

### 10.4 Full Workspace Space

Create a separate center panel or subview with:

- summary metrics;
- treemap by worktree and top-level directory;
- selected-workspace breakdown;
- searchable/sortable table;
- active-agent, open-editor, dirty-buffer, browser-tab, Git, PR, and issue facts;
- safe selection and batch cleanup preview;
- existing worktree archive/delete safety boundary.

Disk analysis is deterministic and bounded. Reclaimable does not mean deletable. The UI shows the exact safety reason per workspace.

---

## 11. Usage and analytics

### 11.1 Compact Usage popover

Add a provider roster popover with Detailed/Compact modes:

- one row per enabled provider;
- plan/account state;
- reset countdown;
- each available rate-limit window;
- used or remaining percentage preference;
- refresh;
- provider/account drill-in;
- Usage details & history;
- Manage Accounts.

Provider rate limits and local transcript/token analytics are different sources and are labelled separately.

### 11.2 Full Usage workspace

Create a center Dockview panel with:

- agents spawned, time agents worked, PRs created, tracking-since;
- total sessions and turns/events;
- daily activity heatmap;
- token mix: new input, cached input, output, reasoning;
- provider cards and filters;
- per-provider detail tabs;
- recent expensive/large sessions;
- context-window and rate-limit history when available;
- export of bounded nonsecret analytics.

Cost is labelled **estimated** and shown only when the data source and pricing model support a meaningful estimate. ChatGPT/Claude subscription quotas are not misrepresented as per-token invoices. Flat-rate subscription usage and API-priced usage remain separate.

The analytics store never persists prompts, assistant text, credentials, cookies, or environment values.

---

## 12. Sessions, workspaces, and Paneview layout

### 12.1 Left rail responsibility

Replace the current all-in-one `SessionsColumn.svelte` composition with a compact **Work** rail:

- repository/workspace headings;
- compact active owned-session rows;
- status dot + provider icon + title;
- selected state;
- optional small branch/task/PR badges only when space permits;
- hover card with provider, model, effort, branch, path, worktree, runtime state, PR/check hint, elapsed/last activity, and latest preview;
- chevron/keyboard expansion for full details and actions.

Target normal row height: 38–44 CSS pixels. The row does not expand its entire card on hover.

### 12.2 Working, Done, and Settled are Paneview panes

Use Dockview Paneview inside the left region:

```text
Work region Paneview
  Working
  Done
  Settled (when enabled by lifecycle migration)
```

Each pane has independent size, collapse state, count, and scroll. Pane sizes persist per shell layout. Minimize/restore parks content without destroying session state.

### 12.3 Session Explorer is separate

Move discovered/resumable sessions out of the left Work rail. Add **Session Explorer** as a center destination or right-side tool view chosen during the shell contract spike. It includes:

- provider/project/worktree filters;
- text search;
- active, resumable, stale-log-only, missing-worktree, and nonresumable states;
- exact metadata and hover/detail preview;
- Open/Resume/Fork/Inspect actions;
- grouping and pagination/virtualization for hundreds of sessions.

Opening Session Explorer never starts a provider. Resume is a separate explicit action.

### 12.4 All stacked side sections use Paneview

- Left Work sections: Paneview.
- Right tool sections: Paneview.
- Bottom Problems/terminal/output groups: existing Dockview/Paneview family.
- Center destinations: Dockview.
- Overlays/popovers/dialogs remain overlays, not panes.

Do not nest Paneview where a simple menu/popover is sufficient, and do not add another general layout library.

---

## 13. App-wide AI Assist Registry

Create one typed registry rather than adding bespoke prompt buttons to every component.

```ts
export interface AiAssistDefinition<Input, Output> {
  id: string;
  title: string;
  surface: AiAssistSurface;
  requiredFacts: AgentFactKind[];
  inputSchema: JsonSchema;
  outputSchema: JsonSchema;
  providerPolicy: AiProviderPolicy;
  mutationPolicy: 'advisory' | 'preview-required' | 'confirmation-required';
  buildInput(context: AiAssistContext): Input;
  validateOutput(output: unknown): Output;
  buildProposal(output: Output, context: AiAssistContext): AiAssistProposal;
}
```

Initial assist actions:

- draft PR title/body;
- review PR or local diff;
- address review comments;
- diagnose/fix failed checks;
- summarize session/worktree/project/PR;
- generate run configuration;
- convert browser annotations to an implementation request;
- suggest form values;
- validate a form before save;
- generate or review workflow definitions;
- review implementation against spec/compliance checklist.

### 13.1 Form assistance

An AI form action returns a schema-validated field patch. The UI shows old/new values and reasons per field. The user applies selected fields. The model never calls a generic save endpoint.

### 13.2 Save validation

Save guards combine deterministic validation with optional advisory AI review. AI review has a bounded timeout, never silently blocks save, and clearly distinguishes errors from suggestions. Consequential saves still pass the normal backend validation after any accepted patch.

### 13.3 PR and repository assistance

All hosted mutations use the existing prepare/confirm/execute GitHub boundary. The model proposes text or typed actions; it never publishes, merges, resolves, reruns, or pushes directly.

---

## 14. Revised work packages

### WP-A — ACP packaging and capability spike

**Sequential gate.** Prove on macOS arm64 and x64 as applicable:

- Rust ACP client can start, initialize, stop, and reap each adapter.
- pinned Codex sidecar strategy;
- pinned Claude adapter + Node/bundle strategy;
- login/auth behavior;
- image capability;
- config options/model/mode discovery;
- permissions and structured user-input requests;
- tools/plans/subagent events;
- restart/session load;
- process cleanup and logs;
- packaged-path resolution.

Output a capability matrix and stop for a decision if Claude packaging is not distributable or reliable.

### WP-B — Conversation manager, canonical events, and ownership

Implement `AgentConversationManager`, execution owner state, event identity, snapshots, durable journal, bounded replay, stale guards, and one-writer tests. Migrate the current registry; do not create a parallel manager.

### WP-C — Screenshot/image attachments P0

Implement structured image sends, restored previews, aggregate bounds, exact cleanup, browser attachment base types, and native proof before visual timeline work.

### WP-D — Provider configuration and command registry

Implement dynamic model/effort/permissions/mode/service-tier selectors, provider confirmation, capability gating, and the reduced structured command catalog. Remove hard-coded command arrays.

### WP-E — Rich conversation timeline

Implement typed item renderers, safe Markdown, reasoning, tools, outputs, diffs, plans/tasks, approvals, user questions, reviews, children, virtualization, and scroll-follow behavior.

### WP-F — Structured/raw ownership handoff

Implement structured -> terminal, terminal -> structured, fork-to-terminal, rollback, history reconciliation, and exact native proof. No hidden duplicate writer.

### WP-G — Workflow schema, journal, and scheduler

Implement versioned definitions, role templates, DAG validation, event journal, projections, budgets, retries, cancellation, worktree allocation, and gates.

### WP-H — Delegation broker and Assembly MCP companion

Implement `assembly.delegate`, status, cancel, depth/budget/worktree/provider validation, parent/child linking, and one-shot V1 child results. Keep the broker testable through a spawner trait.

### WP-I — Agent Control Center

Implement Runs/Graph/Agents/Timeline/Artifacts/Templates, exact control actions, transcript linking, and deterministic progress.

### WP-J — Browser interaction completion

Implement native browser spike, collapsed/docked/expanded states, grab, annotate, screenshot markup, profiles, viewport, external open, devtools policy, and rich conversation attachments.

### WP-K — Resource Manager and Space

Implement bounded ownership snapshot, compact popover, full resource panel, compact Space section, full treemap/table, safe stop, scan cancellation, and worktree cleanup delegation.

### WP-L — Usage roster and analytics

Implement compact usage popover, provider/account actions, full analytics workspace, local event aggregation, source labels, and honest cost semantics.

### WP-M — Work rail, Session Explorer, and Paneview conversion

Implement compact rows/hover cards, left Paneview groups, separate Session Explorer, right Paneview stack, persistence migration, keyboard/focus behavior, and no service destruction on collapse.

### WP-N — AI Assist Registry and surface integrations

Implement typed assists, schema validation, proposal previews, form patch UI, save advisory, PR actions, browser feedback action, run-configuration generation, and audit receipts.

### WP-O — Integrated native acceptance and cleanup

Run the complete multi-session, multi-worktree, multi-provider, browser, workflow, resource, usage, layout, restart, and process-cleanup matrix. Close tasks only from recorded native evidence.

---

## 15. Parallel execution map

### Wave 0 — sequential decisions

1. Re-anchor current `main`, task inventory, and file ownership.
2. WP-A ACP packaging/capability spike.
3. Freeze canonical event, config option, ownership, attachment, workflow, and Paneview contracts.
4. Milestone SOL-medium architecture review.

### Wave 1 — parallel foundation after contracts freeze

| Lane | Work | Shared seam rule |
| --- | --- | --- |
| Conversation runtime | WP-B Rust manager/events/persistence | controller registers Tauri state/commands |
| Codex adapter | WP-A/D Codex sidecar integration | consumes frozen runtime trait |
| Claude adapter | WP-A/D Claude sidecar integration | consumes frozen runtime trait |
| Attachments | WP-C vault/image/browser attachment support | no composer edits until integration |
| Timeline UI | WP-E components/fixtures | consumes frozen item/event types |
| Browser spike | WP-J child-view/overlay proof | no conversation write; emits attachment DTOs |
| Resource readers | WP-K Rust snapshot/space scanner | no shell mounting |
| Usage readers | WP-L provider/local analytics services | no shell mounting |
| Paneview shell | WP-M layout helpers/migration tests | controller owns root shell integration |

At most two heavy native/Rust runners; default one.

### Wave 2 — structured product integration

- config pickers + command registry;
- rich timeline + image composer;
- browser attachment composer cards;
- compact resource/usage surfaces;
- Work rail + Session Explorer;
- raw/structured handoff.

Run milestone review before workflows start.

### Wave 3 — workflow engine

Sequential core first:

1. WP-G schema/journal/projector/scheduler.
2. worktree allocation and gate semantics.
3. WP-H delegation broker.

Then parallel:

- built-in role templates and workflow editor;
- Agent Control Center projections/components;
- MCP companion and provider adapter tests;
- workflow fixture/native harness;
- AI Assist registry definitions that consume stable workflow actions.

### Wave 4 — surface intelligence and final integration

- PR/browser/form/save assists;
- full resource/space/usage workspaces;
- native process and restart acceptance;
- accessibility, narrow layout, reduced motion, and performance certification;
- final SOL-medium review.

---

## 16. Required tests and acceptance matrix

### 16.1 ACP/runtime

- one writer per owned/native session;
- model/effort/mode discovery and confirmed mutation;
- unsupported option and silent-downgrade refusal;
- image content reaches Claude and Codex;
- tool, plan, approval, user-input, and child events normalize;
- cancellation and adapter death;
- session resume/load;
- stale generation drop;
- sidecar and descendant cleanup;
- raw-terminal handoff with rollback.

### 16.2 Conversation

- rich Markdown safety;
- typed item ordering around tools;
- streamed deltas and final reconciliation;
- command output bounds;
- file diff linkage;
- task/plan updates;
- child nesting and unrelated-session exclusion;
- two-session draft/attachment/control/scroll isolation;
- long-session virtualization and no forced scroll when reading history.

### 16.3 Workflow

- DAG cycle/invalid-edge rejection;
- max concurrency/depth/turn/token/time enforcement;
- provider option validation;
- worktree collision prevention;
- deterministic and orchestrator-requested delegation;
- cancellation in spawn and send windows;
- parent teardown;
- retry and idempotent receipts;
- manual approval gates;
- provider-native child correlation;
- restart/replay without duplicate launches;
- artifact and final-output contracts.

### 16.4 Browser

- child-view clipping/z-order/input isolation;
- same live tab across docked/expanded/collapsed;
- grab, annotate, and draw modes;
- annotation generation/hash validation;
- image/structured context reaches exact conversation;
- no arbitrary eval/cookies/storage leakage;
- profile A/B isolation;
- viewport presets;
- Settings/dialogs above child view;
- keyboard, VoiceOver, reduced motion, trackpad, and narrow window.

### 16.5 Resources and usage

- bounded process scan and registry ownership;
- external process no-kill;
- stale PID/PGID refusal;
- compact/full refresh policy;
- scan cancellation and last-result retention;
- reclaimable versus deletable distinction;
- provider reset windows and unavailable states;
- rate limits versus local analytics separation;
- no prompt/secret persistence;
- estimated-cost labeling.

### 16.6 Navigation/layout

- Working/Done/Settled Paneview sizing/collapse/restore;
- compact rows and hover details;
- Session Explorer does not start agents on open;
- left/right/center snapshot migration;
- hidden panes preserve live terminal/provider/editor/browser state;
- keyboard focus and accessible names;
- settings and all root overlays remain reachable.

---

## 17. Stop conditions

Stop before implementation or return a decision packet when:

- the Claude ACP adapter cannot be packaged, authenticated, licensed, or updated safely;
- a provider does not advertise the required image/config/session capability;
- handoff cannot prove release of the prior writer;
- a workflow node would require concurrent writes to one worktree without serialization;
- provider-native child identity cannot be associated authoritatively;
- WKWebView z-order prevents main-webview annotation/confirmation controls;
- a process cannot be proven owned;
- Paneview conversion requires replacing the current shell or Dockview persistence model;
- a proposed AI assist needs arbitrary shell/API access instead of a typed action;
- a shared-seam file contains unmerged overlapping work.

Do not hide these outcomes behind a fallback that claims the requested capability exists.

---

## 18. Completion definition

Assembly is complete for this amendment when:

1. Claude Code and Codex start as structured ACP conversations without a hidden TUI.
2. The user can select provider-advertised model, effort, permissions/mode, and service options from native selectors.
3. Screenshots paste, preview, restore, and reach both providers as real image inputs.
4. Commands, reasoning, tools, diffs, plans/tasks, approvals, questions, token/context state, and nested child output render in one provider-neutral timeline.
5. Raw CLI ownership can be entered and exited without two writers or lost history.
6. A durable workflow can run an orchestrator plus implementation, review, spec, test, fix, and integration roles across validated providers/worktrees.
7. Agent Control shows exact state, loop/phase, children, tools, artifacts, budgets, and controls.
8. Browser grab, annotate, and screenshot markup become reviewable conversation attachments from docked or expanded mode.
9. Resource and Usage each have compact and full, accurate, safe surfaces.
10. The left Work rail is compact; Session Explorer is separate; Working/Done/Settled and side tool groups are Paneview panes.
11. PR, form, save, browser, run-configuration, and workflow AI assistance all use typed proposal and confirmation boundaries.
12. Restart restores sessions, workflows, layouts, attachments, browser state, and durable events without duplicate providers, PTYs, Roslyn processes, or workflow nodes.
