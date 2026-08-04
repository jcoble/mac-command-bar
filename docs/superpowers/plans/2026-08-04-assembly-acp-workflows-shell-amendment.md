# Assembly ACP Runtime, Workflows, and Workbench Shell Amendment

**Date:** 2026-08-04  
**Status:** Product direction locked by the user; implementation has not started  
**Repository baseline:** `jcoble/mac-command-bar` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`  
**Applies to:** TSK-808, TSK-809, TSK-810, the integrated-agent parts of TSK-766, and the browser/resource/session-shell direction clarified on 2026-08-04.

> For implementation agents: retain every execution, worktree, model-routing, focused-test,
> native-proof, security, SQL, compatibility, and cleanup rule from
> `2026-08-01-tsk-808-native-workbench-product-wave.md` unless this amendment explicitly
> replaces a product or architecture decision. This amendment does not authorize implementation
> before the controller refreshes the repository and writes exact task packets.

---

## 1. Plan authority and superseded decisions

This document amends both:

1. `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; and
2. `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`.

The older conversation file remains historical discovery evidence. The product-wave plan remains
authoritative for unaffected work packages. For conflicts in the following areas, this amendment
wins:

- shell composition, side panes, session navigation, and resumable-session discovery;
- browser presentation, capture, annotation, and conversation delivery;
- resource, disk-space, provider-usage, and compact/full presentation;
- conversation runtime, controls, commands, attachments, tools, plans, tasks, and child agents;
- integrated AI and multi-agent workflow orchestration;
- the lane dependency graph and parallel dispatch manifest for those areas.

The most important replacement is this:

> The PTY is no longer the permanent sole writer for every future Assembly-created Claude or
> Codex conversation. Assembly enforces **one writer at a time**, and that writer may be either
> the structured ACP runtime or the existing native CLI/PT​​Y runtime. New Assembly-created agent
> conversations default to structured ownership. Existing, imported, externally started, or
> explicitly terminal-owned sessions continue through the current PTY plus durable-transcript
> projection.

The app must never run the structured writer and native CLI writer against the same provider-native
conversation simultaneously.

---

## 2. Locked product decisions

1. **New Claude Code and Codex conversations are structured-first.** Assembly starts one ACP-backed provider session and renders typed events. It does not start a hidden full-screen TUI merely so the app can read a transcript.
2. **ACP is a protocol connection, not an xterm session.** An ACP adapter is a child process with structured RPC over stdio. It can report shell commands and terminal output as tool activity, but it is not the user-visible native CLI terminal.
3. **One conversation has one writer.** The active owner is either `structured` or `terminal`; switching performs an explicit release/acquire/reconcile transaction.
4. **The existing PTY and transcript scanners remain valuable.** They become the terminal-owned projection, import, recovery, and read-only history path rather than the universal live conversation engine.
5. **Model, reasoning effort, permissions/execution mode, collaboration/plan mode, fast/service tier, and similar settings are native selectors, not slash commands.** Values come from the active provider's advertised configuration.
6. **TUI-only commands stay out of Assembly's structured command menu.** Theme, terminal pets, Vim mode, keymap, title/statusline, raw scrollback, and similar client-local behavior remain inside the native TUI when raw CLI is open.
7. **Screenshot paste is priority zero.** Pasted images preview immediately, preserve order and session ownership, survive switching/restart, and reach the exact structured provider message as real image input.
8. **Tools, reasoning, plans, tasks/TODOs, approvals, structured questions, command output, file changes, reviews, web searches, and subagents are first-class timeline items.** They are never flattened into one assistant Markdown string.
9. **Subagents have two origins and one presentation model.** Provider-native Claude/Codex children and Assembly-spawned workflow children appear in one parent-scoped tree, but ownership and control capability remain explicit.
10. **ACP is the common provider transport, not the workflow scheduler.** Assembly owns workflows, roles, DAG/loop state, delegation, budgets, persistence, worktree policy, and UI.
11. **The browser has four presentation modes:** docked, floating, expanded over the workbench, and collapsed to the global bottom-right control. The same live page moves between states without reload.
12. **Browser feedback is a structured conversation attachment.** Grab element, annotate element, and draw on screenshot produce immutable bounded context that the user reviews before adding to a draft.
13. **Resources and provider usage each have compact and full surfaces.** Compact answers “what is happening now?”; full workspaces provide process trees, disk analysis, history, and safe actions.
14. **The left side is My Work, not the machine-wide session archive.** It shows current workspaces/worktrees and owned sessions as compact rows. Historical/resumable sessions move to a separate Session Library/Explorer.
15. **Every stack of side-region panels uses Dockview Paneview.** Center destinations use Dockview. Do not add another layout engine or custom flex-column pseudo-panes.
16. **AI is integrated through one typed contribution/action framework.** PR drafting, review assistance, check repair, browser feedback, form completion, save preflight, run-configuration generation, and workflow assistance all use deterministic facts -> AI proposal -> preview/revalidation -> typed execution.
17. **Future providers are capability-gated.** A provider is supported through ACP or an equivalent adapter implementing the same runtime interface. The UI claims only capabilities observed from that runtime.

---

## 3. What an ACP session is—and is not

### 3.1 Structured session

```text
Assembly Rust AgentRuntimeManager
        |
        +-- ACP client connection over stdio
        |       +-- pinned Codex adapter -> codex app-server -> Codex thread
        |       +-- pinned Claude adapter -> Claude Agent SDK -> Claude session
        |       +-- future ACP adapter
        |
        +-- normalized AgentEvent stream
        +-- provider-native session/thread ID
        +-- advertised capabilities and configuration
        +-- no Assembly xterm/PT​​Y unless the user explicitly opens raw CLI
```

Starting a structured session launches an adapter/provider process and creates or resumes a
provider-native conversation. It does **not** create a terminal tab.

An agent may run shell commands. ACP reports those as structured tool/command events and may expose
interactive/background tool terminals. Those are runtime tools, not the provider's native TUI.

### 3.2 Terminal-owned session

```text
Assembly TerminalRegistry
        +-- login-shell PTY
                +-- interactive `claude`, `codex`, or another CLI
                        +-- durable transcript projected into AgentTimeline
```

The PTY is authoritative while terminal-owned. Assembly may project the transcript, but controls
requiring structured provider RPC are disabled or handed to the native picker honestly.

### 3.3 Ownership handoff

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';
```

Rules:

- **Open in native CLI:** finish or interrupt the active structured turn, persist/reconcile events, release the structured writer, then launch the native CLI with the exact provider-native session ID.
- **Return to Conversation:** stop/detach the native CLI, import appended transcript records, resume/load the same provider-native conversation through ACP, reconcile by provider item identity, then acquire structured ownership.
- **Fork to terminal:** use a provider-native fork when available, otherwise create a clearly related new continuation. The source structured session remains untouched.
- A failed handoff restores the prior owner and usable UI. `transitioning-*` may not survive restart without a recovery receipt.
- A future shared Codex app-server/remote-TUI optimization is allowed only after a separate native proof shows one thread, no duplicate writers, and exact history/settings reconciliation.

---

## 4. Researched provider path

### 4.1 Foundation

Use the official `agentclientprotocol/rust-sdk` as the Rust client boundary.

- **Codex:** package a pinned `agentclientprotocol/codex-acp` macOS sidecar. It already maps Codex app-server models, effort, fast mode, approval/sandbox, images, command execution, file changes, permission requests, MCP tools, terminal output, reasoning, plans, web search, reviews, token usage, and subagents. Prefer its standalone macOS builds over `npx ...@latest` at session start.
- **Claude Code:** package or launch a pinned `agentclientprotocol/claude-agent-acp` build backed by the official Claude Agent SDK. It supports images, permissions, tools, TODOs, edit review, interactive/background terminals, custom commands, and nested subagent transcript metadata.
- **Claude packaging gate:** the current adapter requires a compatible Node runtime. Before product implementation, prove one exact strategy: bundled signed Node runtime, reviewed standalone adapter bundle, or explicit minimum local Node requirement. Do not leave this implicit.
- **Future providers:** register an adapter descriptor containing executable, version probe, auth methods, capabilities, supported migration, and packaging policy. No provider-specific logic enters Svelte components.
- **Direct Codex app-server:** allowed only as an optional provider-specific escape hatch behind the same runtime trait, not as a second UI contract.

### 4.2 Reference implementations

Study behavior and architecture from:

- T3 Code: canonical provider events, typed item lifecycle, plans, approvals, Markdown, and provider-neutral orchestration boundary.
- Jockey: Tauri/Rust ACP process lifecycle and discovery of models, modes, config options, and commands.
- CodeG: ACP child-session delegation broker, parent/child linkage, per-agent defaults, depth limits, cancellation, status reports, and MCP-mediated delegation.
- Orca: browser grab/annotation/markup UX, compact-to-expanded resource and usage surfaces, workspace-space treemap, compact worktree rows, and hover details.

These are references, not permission to copy code blindly. Every reuse packet records license,
dependency impact, architecture assumptions, and attribution. Orca's Electron webview implementation
is not transplanted into Tauri; Assembly reproduces the interaction contract using managed native
webviews and main-webview overlays.

---

## 5. Runtime architecture

### 5.1 Existing authorities retained

Retain and extend:

- `ownedId` and `OwnedSession` as stable app identity;
- `TerminalService`, `TerminalRegistry`, scrollback, process ownership, and tombstones;
- per-session Editor/Browser/Diff/Dockview workspace snapshots;
- the managed conversation attachment vault;
- Tauri IPC/events; do not add a local WebSocket server for normal desktop operation;
- the existing conversation store/reducer as the only frontend conversation state owner;
- the existing append-only orchestration ledger, extended rather than replaced.

### 5.2 Rust module layout

Refactor the existing `src-tauri/src/agent_conversation` module rather than adding a parallel
conversation subsystem:

```text
tauri-svelte-preview/src-tauri/src/agent_conversation/
  mod.rs
  manager.rs
  runtime.rs
  capabilities.rs
  events.rs
  event_buffer.rs
  ownership.rs
  handoff.rs
  persistence.rs
  attachments.rs
  providers/
    mod.rs
    acp_client.rs
    process_supervisor.rs
    adapter_manifest.rs
    codex.rs
    claude.rs
    direct_codex.rs       # optional, deferred escape hatch
  transcript/
    mod.rs
    watcher.rs
    codex.rs
    claude.rs
```

Evolve the existing `AgentConversationRegistry` into one `AgentRuntimeManager`, keyed by
`ownedId`. Do not register a second Tauri state object for conversations.

### 5.3 Runtime interface

```rust
#[async_trait]
pub trait StructuredAgentRuntime: Send + Sync {
    async fn initialize(&mut self) -> Result<AgentCapabilities, AgentRuntimeError>;
    async fn start(&mut self, request: StartAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn resume(&mut self, request: ResumeAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn send(&mut self, request: SendAgentInput)
        -> Result<TurnHandle, AgentRuntimeError>;
    async fn steer(&mut self, request: SteerAgentTurn)
        -> Result<(), AgentRuntimeError>;
    async fn interrupt(&mut self, turn_id: Option<&str>)
        -> Result<(), AgentRuntimeError>;
    async fn set_option(&mut self, option_id: &str, value: AgentOptionValue)
        -> Result<(), AgentRuntimeError>;
    async fn respond_to_request(
        &mut self,
        request_id: &str,
        response: AgentRequestResponse,
    ) -> Result<(), AgentRuntimeError>;
    async fn close(&mut self) -> Result<(), AgentRuntimeError>;
}
```

The manager assigns Assembly generation/sequence identity, keeps a bounded replay buffer, persists
durable canonical events, and emits one Tauri stream. Raw provider frames are bounded diagnostics,
not the frontend schema.

### 5.4 Capability model

```ts
export interface AgentCapabilities {
  provider: string;
  adapterVersion: string;
  loadSession: boolean;
  resumeSession: boolean;
  forkSession: boolean;
  images: boolean;
  embeddedContext: boolean;
  steering: boolean;
  reasoning: boolean;
  plans: boolean;
  tasks: boolean;
  toolCalls: boolean;
  terminalTools: boolean;
  fileChanges: boolean;
  permissionRequests: boolean;
  structuredUserInput: boolean;
  nativeSubagents: boolean;
  nestedSubagentTranscripts: boolean;
  availableCommands: boolean;
  sessionConfigOptions: boolean;
}
```

Capabilities are observed runtime facts. Missing/unknown means unavailable, not “probably supported.”

---

## 6. Native configuration controls, not slash commands

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'reasoning'
    | 'permission'
    | 'mode'
    | 'service-tier'
    | 'fast-mode'
    | 'boolean'
    | 'unknown';
  label: string;
  description?: string;
  currentValue: string | boolean | null;
  values?: Array<{
    value: string;
    label: string;
    description?: string;
  }>;
  mutable: boolean;
  unavailableReason?: string;
  providerData?: unknown;
}
```

The composer renders recognized categories in this order:

```text
[Provider] [Model ▾] [Effort ▾] [Mode/Permissions ▾] [Fast/Service tier] [More ▾]
```

Rules:

1. Values come from ACP session configuration updates/provider discovery, never a hard-coded model list.
2. Preserve provider-advertised order, especially effort progression.
3. Selecting a value calls the provider option operation and displays `Applying…` until confirmation.
4. If a new model cannot use the prior effort, adopt the provider-advertised default/fallback and explain the change.
5. Unknown config options appear under More without provider-specific component code.
6. Terminal-owned sessions expose observed read-only metadata plus **Open native picker** when safe; they do not fake direct mutation.

### 6.1 Command catalog

Merge only:

- provider-advertised commands meaningful through the structured runtime;
- discovered skills/prompts;
- Assembly-local navigation/workflow actions.

Typical structured rows when supported:

```text
/review
/compact
/status
/mcp
/skills
/goal
/new-workflow
/open-terminal
```

Omit TUI-only rows such as:

```text
/theme /pets /vim /keymap /title /statusline /raw
```

`/model`, `/permissions`, `/agent`, effort, and service tier are not primary slash rows. Keyboard
aliases may focus the native control but do not send text to the provider.

No command selection auto-submits.

---

## 7. Canonical conversation, tools, plans, tasks, and children

### 7.1 Ordered item model

```ts
export type AgentTimelineItem =
  | UserMessageItem
  | AssistantMessageItem
  | ReasoningItem
  | PlanItem
  | TaskListItem
  | CommandExecutionItem
  | ToolCallItem
  | FileChangeItem
  | WebSearchItem
  | ImageItem
  | ApprovalRequestItem
  | UserInputRequestItem
  | SubagentItem
  | ReviewItem
  | ContextCompactionItem
  | WarningItem
  | ErrorItem;
```

Every item includes stable app ID, provider item ID, turn ID, parent item ID when nested, lifecycle
status, timestamps, and optional bounded provider metadata. Assistant text, reasoning, command
output, plan output, and file-change output stream as separate channels.

### 7.2 Canonical events

```ts
export type AgentEvent =
  | SessionStartedEvent
  | SessionConfiguredEvent
  | SessionStateChangedEvent
  | TurnStartedEvent
  | TurnCompletedEvent
  | TurnInterruptedEvent
  | ItemStartedEvent
  | ItemUpdatedEvent
  | ItemCompletedEvent
  | ContentDeltaEvent
  | ApprovalRequestedEvent
  | ApprovalResolvedEvent
  | UserInputRequestedEvent
  | UserInputResolvedEvent
  | UsageUpdatedEvent
  | ChildrenUpdatedEvent
  | RuntimeWarningEvent
  | RuntimeErrorEvent;
```

Keep `ownedId`, generation, monotonic sequence, stale-generation rejection, and snapshot
resynchronization. Workflow and UI projections consume canonical events only.

### 7.3 Presentation

- Assistant prose is the dominant reading surface.
- Reasoning is collapsible, open while streaming and collapsed after completion unless pinned open.
- Commands/tools are compact rows with expandable input, output, duration, cwd, exit code, and locations.
- File-change rows open the existing Diff workspace.
- Plans render provider steps and only expose Implement/Revise/Dismiss when the runtime advertises those actions.
- Provider TODO/task updates are first-class task lists, not Markdown inference.
- Approval and structured-input requests show only provider-supplied choices.
- Requests expire on generation change, turn completion, ownership handoff, or provider expiry.
- Long conversations use stable item identity, row virtualization, and near-bottom-only scroll following.

### 7.4 Unified agent tree

```ts
export interface AgentNode {
  id: string;
  parentId: string | null;
  origin: 'provider-native' | 'assembly-workflow';
  provider: string;
  nativeSessionId: string | null;
  workflowRunId: string | null;
  workflowStepId: string | null;
  roleId: string | null;
  label: string;
  state: 'queued' | 'starting' | 'working' | 'waiting' | 'completed' | 'failed' | 'cancelled' | 'unknown';
  depth: number;
  startedAt: string | null;
  updatedAt: string | null;
  transcriptAvailable: boolean;
  controllable: boolean;
}
```

Provider-native children are controlled only through supported provider actions. Imported transcript
children remain read-only. Assembly-spawned workflow children are controllable because Assembly
owns those ACP sessions.

---

## 8. Screenshot and rich-context attachments—priority zero

### 8.1 Composer behavior

1. `Cmd+V` with image files consumes only the image payload and creates ordered previews immediately.
2. Draft, image order, validation state, and managed attachment IDs are scoped by `ownedId` and generation.
3. A failed image write leaves text and previously saved images intact.
4. Send is disabled while any image is still validating/writing.
5. Images survive session switches and restart when their managed file exists.
6. Removing a preview deletes only its owned managed file after canonical containment validation.
7. A successful send clears previews only after the provider accepts the prompt request.

### 8.2 Structured send

When `capabilities.images` is true, send image content blocks through ACP in the same prompt as the
text and embedded browser/file context, preserving order.

A managed-file resource fallback is allowed only after explicit capability negotiation. Otherwise
the UI explains that the provider does not accept images; it never silently turns the screenshot
into a local path-shaped prompt.

### 8.3 Terminal fallback

Terminal-owned sessions retain the current managed-file reference strategy. Assembly performs one
bracketed paste and one Return, preserving the draft until the PTY accepts both writes.

### 8.4 Vault safety

Retain and extend the current Rust vault:

- PNG, JPEG, WebP, and provider-supported GIF;
- byte-signature verification;
- per-file, per-draft aggregate, and retained-session limits;
- opaque names and owner-only permissions;
- no symlink traversal;
- canonical owned-session root containment;
- no user-owned file deletion;
- no blob URL persistence;
- exact pruning receipts.

Native acceptance for screenshot send lands before timeline visual polish.

---

## 9. Terminal transcript projection

The current JSONL parser becomes `TerminalTranscriptProjection`, implementing the same canonical
event output with explicitly limited capabilities.

Replace the frontend 500 ms full-tail poll with a Rust incremental watcher:

```rust
pub struct TranscriptCursor {
    path: PathBuf,
    file_identity: TranscriptFileIdentity,
    offset: u64,
    partial_line: Vec<u8>,
    generation: u64,
}
```

It caches the path, reads only appended bytes, detects rotation/truncation, emits normalized events,
and occasionally reconciles with a bounded snapshot. It never scrapes terminal pixels.

This projection supports imported/external/native-CLI sessions and handoff reconciliation. It is
not the primary runtime for newly created structured sessions.

---

## 10. Multi-agent workflows and orchestration

ACP supplies sessions and events. Assembly supplies orchestration.

### 10.1 Two modes

**Deterministic workflow mode:** Assembly executes a versioned DAG/loop. The model does not choose
scheduling unless the definition contains an explicit AI decision node.

**Agent-directed delegation mode:** an orchestrator can call an Assembly MCP delegation service.
The broker validates role, task, provider settings, worktree, depth, budgets, and ownership before
spawning a child ACP session.

### 10.2 Workflow definition

```ts
export interface WorkflowDefinitionV1 {
  version: 1;
  id: string;
  name: string;
  description: string;
  entryNodeId: string;
  roles: WorkflowRoleDefinition[];
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  budgets: WorkflowBudget;
  failurePolicy: WorkflowFailurePolicy;
  workspaceStrategy: 'shared-readonly' | 'dedicated-worktree' | 'mixed';
}

export interface WorkflowRoleDefinition {
  id: string;
  label: string;
  providerPreference: string[];
  modelOption: string | null;
  effortOption: string | null;
  modeOption: string | null;
  configOptions: Record<string, string | boolean>;
  promptTemplateId: string;
  allowedContext: WorkflowContextKind[];
  allowedActions: string[];
  completionContract: WorkflowCompletionContract;
  mayDelegate: boolean;
  maxChildren: number;
}

export type WorkflowNodeDefinition =
  | AgentTaskNode
  | ParallelGroupNode
  | BarrierNode
  | HumanApprovalNode
  | ConditionNode
  | RetryLoopNode
  | DeterministicCommandNode
  | SummaryNode;

export interface WorkflowBudget {
  maxAgents: number;
  maxParallelAgents: number;
  maxDepth: number;
  maxTurnsPerAgent: number;
  maxRetryIterations: number;
  timeoutMs: number;
  tokenBudget?: number;
}
```

Built-in role templates, delivered only after fixtures:

- orchestrator/coordinator;
- implementer;
- code reviewer;
- spec-compliance reviewer;
- tester/verification agent;
- defect fixer;
- researcher;
- integrator/release agent.

Templates are editable copies. Model/effort/mode choices are validated against discovered runtime
capabilities at start. Missing or downgraded options block the node instead of silently changing it.

### 10.3 Durable run state

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

Extend the existing append-only orchestration ledger rather than creating a second run log. Record
run/node/attempt IDs, parent/child edge, provider/native session IDs, ownedId, worktree, phase,
gates, artifacts, budgets, retries, and timestamps. Reconstruct state with a pure reducer. Never
infer workflow state from assistant prose.

### 10.4 Scheduling and worktrees

- Mutating children receive dedicated worktrees unless writes are explicitly serialized.
- Read-only reviewers may share a stable snapshot/checkpoint.
- Two mutating nodes never write concurrently to the same worktree.
- Integration starts only after required implement/review/test gates pass.
- Worktree creation/removal uses the existing validated service and safety confirmation.
- Cleanup never bypasses dirty, unmerged, locked, primary-checkout, or ownership rules.

### 10.5 Delegation broker

Expose a narrow Assembly MCP companion following the proven CodeG broker shape:

```text
assembly.delegate
assembly.delegation_status
assembly.cancel_delegation
```

`assembly.delegate` accepts an allow-listed role ID and bounded task/context, not arbitrary launch
arguments. It enforces parent run/node/session, max depth/children, provider/model/effort/mode,
worktree policy, time/turn/token budgets, cancellation, and parent teardown.

V1 may be one-shot: child first-turn result returns to the parent and the full child transcript
remains inspectable. Persistent continuation tools are deferred until lifecycle/budget semantics are
proven.

Provider-native subagents are ingested and displayed. They become durable workflow nodes only when
an authoritative correlation links them to a workflow node/tool request.

### 10.6 Example workflows

1. Implement -> Test -> Code review -> Spec compliance -> Fix loop -> Human approval.
2. Investigate bug -> Reproduce -> Root-cause review -> Implement -> Regression test.
3. Draft PR -> Review diff -> Run checks -> Prepare PR text -> Human publish.
4. UI audit -> Browser annotations -> Implement -> Visual review.

No template silently publishes, merges, deletes worktrees, or bypasses approvals.

---

## 11. Agent Control Center

Register a permanent center Dockview destination named **Agents**. Internal composition uses
Paneview and tabs as appropriate:

- Runs and workflow definitions;
- workflow graph/active loop;
- unified agent tree;
- selected agent transcript/tool activity;
- artifacts and receipts;
- decisions/approval queue;
- role/template editor.

Show per agent/node:

- role and exact instruction/template version;
- provider, model, effort, permission/mode, service tier;
- current workflow phase, node, loop iteration, and task/plan step;
- current tool and elapsed time;
- worktree, branch, ownedId, native session ID;
- parent/children/depth;
- retries and budgets;
- last deterministic receipt;
- live transcript/tool output;
- actions: Open conversation, Open worktree, Open diff, Open terminal when terminal-owned, Pause run, Cancel node, Retry, Approve gate.

Do not show a percentage unless the workflow has explicit weighted steps. Otherwise show named phase
and completed/total nodes.

Pause/cancel/retry target exact run/node/attempt generations. Stale actions are rejected. Kill is
never offered for an unproven process owner.

---

## 12. Browser—Orca-inspired interaction, Tauri architecture

The master plan's native child-webview security/profile isolation remains in force.

### 12.1 Presentation modes

```ts
export type BrowserPresentationMode =
  | 'docked'
  | 'floating'
  | 'expanded'
  | 'collapsed';
```

- **Docked:** normal Browser center destination.
- **Floating:** resizable/movable overlay larger than the docked pane, preserving workbench behind it.
- **Expanded:** fills Assembly content below macOS title-bar chrome and overlays session/editor/side panes.
- **Collapsed:** hides browser chrome/webview while retaining tabs/profile/history/annotations; the bottom-right island remains.

Move the same managed page between measured bounds. Hide before moving, measure final bounds, then
reveal. No transition creates a second page or reload.

### 12.2 Toolbar

Capability-gated actions:

```text
Profile / workspace browser identity
Back / Forward / Reload
Address bar
Import context
Grab page element
Annotate page element
Draw on screenshot
Viewport size
Development-only Devtools
Open in default browser
Overflow / Browser Settings
Float / Restore / Expand
Collapse
```

Import Cookies remains disabled until a separate security design defines source, format, scope,
secret handling, and deletion.

### 12.3 Three capture modes

**Grab element:** bounded DOM/accessibility metadata plus optional exact element screenshot for quick
context attachment.

**Annotate element:** same immutable target plus user note and Change/Question intent. Add queues the
item; it does not send.

**Draw on screenshot:** capture visible browser viewport, open main-webview markup tools
(pen/shape/arrow/text/undo/clear), store the marked image through the attachment vault, and retain
page URL/viewport/timestamp.

```ts
export interface BrowserContextAttachment {
  id: string;
  workspaceId: string;
  tabId: string;
  generation: number;
  kind: 'element' | 'annotation' | 'marked-screenshot';
  pageTitle: string;
  pageUrl: string;
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

The feedback panel supports edit, remove, Copy All, Add to Conversation, and Ask Workflow. Adding to
Conversation targets the exact captured `ownedId`/generation, shows a merge preview for a nonempty
draft, and never auto-submits.

The inspector uses a narrow injected script/message protocol. It may return bounded target metadata
and capture coordinates; it may not expose arbitrary eval, cookies, storage, credentials, whole HTML,
or cross-origin DOM access.

---

## 13. Resource Manager, Workspace Space, and Usage

### 13.1 Compact Resource Manager

A status/floating-island popover contains:

- total CPU and RSS;
- expandable project -> worktree -> session/process tree;
- owned terminal, agent-adapter, browser, LSP, and run-configuration rows;
- listening ports;
- bounded CPU/RSS sparklines;
- exact owned-process stop with confirmation;
- inactive workspace count;
- compact Space subsection with scanned/reclaimable bytes and Scan/Review.

Poll every two seconds only while the compact or full resource surface is visible. Hidden surfaces
retain the last snapshot and refresh on activation/events.

External/unattributed processes show metrics but no stop action.

### 13.2 Full Resources workspace

Register a center Dockview destination using Paneview:

- grouped process/resource tree and filters;
- selected process/session details;
- CPU/RSS sort and history;
- ownership/process ancestry;
- language-server roots, policy, and logs;
- ports;
- links to exact session/worktree/browser/editor owners;
- safe stop/restart;
- memory-pressure policy and current evictions.

### 13.3 Workspace Space

Compact Space shows scanned, reclaimable, workspace count, updated time, and Scan/Cancel/Review. A
running scan keeps and labels the prior result.

The full Space workspace contains:

- summary metrics;
- treemap by worktree/top-level directory;
- selected workspace breakdown;
- searchable/sortable table;
- active-agent, terminal, open-editor, dirty-buffer, browser, Git, PR, issue, and checkpoint facts;
- safe selection/batch cleanup preview;
- existing archive/delete safety boundary.

Reclaimable is not equivalent to deletable. Show the exact safety reason per workspace.

### 13.4 Compact Usage popover

Provider roster with Detailed/Compact modes:

- provider icon/name;
- plan/account state;
- reset countdown;
- all observed quota windows;
- used/remaining display preference;
- refresh;
- provider/account drill-in;
- Usage details & history;
- Manage Accounts.

Provider quota/rate-limit data and local transcript/token analytics are separate labelled sources.

### 13.5 Full Usage workspace

Register a center Dockview destination with:

- agents spawned, time worked, PRs created, tracking since;
- total sessions and turns/events;
- daily activity heatmap;
- token mix: new input, cached input, output, reasoning;
- provider cards/tabs and project/session breakdown;
- recent large sessions;
- context-window and quota history when available;
- export of bounded nonsecret analytics.

Cost is labelled **estimated** and shown only when provider/model identity and a versioned pricing
model make it meaningful. Subscription quotas are not represented as per-token invoices. Analytics
never persist prompts, responses, credentials, cookies, or environment values.

---

## 14. Shell composition, Paneview, and session navigation

### 14.1 Layout contract

- Center destinations: existing Dockview.
- Left/right/bottom stacked regions: Paneview from the existing Dockview ecosystem.
- Internal independently resizable/collapsible workspaces: Paneview.
- Dialogs, menus, hover cards, browser overlays, and global action island: existing overlay/portal layer.

Create one adapter instead of direct Paneview calls everywhere:

```text
shell/layout/workbenchPaneview.ts
components/layout/WorkbenchPaneviewHost.svelte
components/layout/WorkbenchPaneHeader.svelte
```

The adapter owns pane IDs, persistence version, min/max size, collapse, move/reorder, focus,
accessibility, parking, and restore. Feature data remains in feature stores.

### 14.2 Left rail: My Work

The left region contains only current work:

```text
Project
  Worktree/workspace
    owned agent session
    owned agent session
```

Rows target 38–44 CSS pixels and show provider icon, concise title, state, optional model,
branch/task/PR hint, relative activity, and approval/attention indicator.

Hover/focus opens a bounded card with full title, folder/worktree, provider/model/effort, state, last
activity, latest preview, PR/check hint, and primary actions. Hover does not resize the list row.

Click selects. Chevron/details expands an accordion row with full metadata/lifecycle actions.

Working, Done, and later Settled are independent Paneview panes with their own size, collapse, count,
and scroll. Collapsing parks content; it does not destroy provider/PTY/editor/browser state.

### 14.3 Session Library/Explorer

Move machine-wide discovered/resumable sessions out of My Work. Add a separate center or right-tool
surface selected during the shell spike. It includes:

- provider/project/worktree filters;
- search;
- active, resumable, stale-log-only, missing-worktree, and nonresumable states;
- metadata, hover/detail preview;
- Open/Resume/Fork/Inspect actions;
- grouping and virtualization/paging for hundreds of sessions.

Opening the library never starts a provider. Resume is a distinct action.

### 14.4 Side regions

All left and right stacked feature sections become Paneview panes. Bottom Problems/output and center
surfaces stay in the same Dockview family. Do not nest Paneview for a simple menu or popover.

---

## 15. App-wide AI contribution and action framework

Do not add bespoke prompt buttons with ad hoc execution to every component.

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

Initial contributions:

- draft PR title/body;
- review local diff or PR;
- address review comments;
- diagnose/fix failed checks;
- summarize session/worktree/project/PR;
- generate run configuration;
- convert browser annotations to implementation request;
- suggest form values;
- validate form before save;
- generate/review workflow definitions;
- review implementation against spec/compliance checklist.

Form assistance returns a schema-validated field patch. The UI shows old/new/reason per field and the
user applies selected fields. The model never calls a generic save endpoint.

Save preflight combines deterministic validation with optional bounded advisory AI review. AI never
silently blocks save and normal backend validation always runs after an accepted patch.

Hosted mutations use the existing prepare/confirm/execute GitHub boundary. AI proposes typed actions;
it never directly publishes, merges, resolves, reruns, pushes, deletes, or approves.

---

## 16. Revised work packages

### WP-A — ACP packaging and capability spike

Sequential gate. Prove on native macOS:

- Rust ACP client start/init/stop/reap;
- pinned Codex sidecar;
- pinned Claude adapter/runtime strategy;
- auth/login;
- images;
- config/model/mode discovery;
- permissions and structured questions;
- tools/plans/tasks/subagents;
- session load/resume;
- packaged paths and cleanup.

Output a capability matrix. Stop for a decision if Claude packaging is not distributable/reliable.

### WP-B — Agent runtime manager, canonical events, ownership

Implement the one manager, owner state, event identity, bounded replay, persistence, stale guards,
and one-writer tests. Migrate the existing registry; do not create a parallel store/runtime.

### WP-C — Screenshot/image attachments P0

Implement structured image sends, restored previews, aggregate limits, exact cleanup, browser
attachment base types, and native Claude/Codex proof before timeline polish.

### WP-D — Provider configuration and command registry

Implement model/effort/permission/mode/service-tier selectors, provider confirmation, capability
gating, reduced command catalog, and removal of hard-coded command arrays.

### WP-E — Rich timeline and child presentation

Implement safe Markdown, typed item renderers, reasoning, commands, tools, outputs, diffs, plans,
tasks, approvals, questions, reviews, children, virtualization, and scroll behavior.

### WP-F — Structured/raw ownership handoff

Implement structured -> terminal, terminal -> structured, fork-to-terminal, rollback, transcript
reconciliation, and exact native proof.

### WP-G — Workflow schema, ledger, projector, scheduler

Implement definitions, role templates, DAG validation, event extension, projections, budgets, retries,
cancellation, worktree allocation, and gates.

### WP-H — Delegation broker and Assembly MCP companion

Implement delegate/status/cancel, depth/budget/worktree/provider validation, parent/child linking,
one-shot V1 results, and a testable spawner trait.

### WP-I — Agent Control Center

Implement Runs/Graph/Agents/Timeline/Artifacts/Templates, exact controls, transcript linkage, and
deterministic progress.

### WP-J — Browser interaction completion

Implement native webview spike, four presentation states, toolbar, grab, annotate, screenshot markup,
profiles, viewport, external open, devtools policy, and conversation/workflow attachments.

### WP-K — Resource Manager and Space

Implement bounded ownership snapshot, compact popover, full Resources panel, compact Space, treemap,
table, safe process controls, scan cancellation, and worktree cleanup delegation.

### WP-L — Usage roster and analytics

Implement compact usage popover, provider/account actions, full analytics workspace, local event
aggregation, source labels, and honest cost semantics.

### WP-M — My Work, Session Library, Paneview conversion

Implement compact rows/hover cards, left Paneview groups, separate session browser, right Paneview
stack, persistence migration, keyboard/focus, and no service destruction on collapse.

### WP-N — AI contribution/action framework

Implement typed assists, schema validation, proposal previews, form patches, save advisory, PR
integrations, browser feedback action, run-config generation, and workflow assistance.

### WP-O — Integrated native acceptance

Run the multi-session, multi-worktree, multi-provider, browser, workflow, resource, usage, layout,
restart, and process-cleanup matrix. Close tasks only from recorded native evidence.

---

## 17. Parallel execution map

### Wave 0 — sequential decisions

1. Re-anchor current main/task inventory/file ownership.
2. WP-A packaging/capability spike.
3. Freeze event, option, ownership, attachment, workflow, and Paneview contracts.
4. SOL-medium architecture review.

### Wave 1 — parallel foundation after contracts freeze

| Lane | Work | Shared seam rule |
| --- | --- | --- |
| Runtime manager | WP-B Rust manager/events/persistence | controller registers Tauri state/commands |
| Codex adapter | WP-A/D Codex integration | consumes frozen runtime trait |
| Claude adapter | WP-A/D Claude integration | consumes frozen runtime trait |
| Attachments | WP-C vault/image/browser DTOs | no composer edits until integration |
| Timeline UI | WP-E components/fixtures | consumes frozen item/event types |
| Browser spike | WP-J native view/overlay proof | emits attachment DTOs only |
| Resource readers | WP-K Rust snapshot/space scan | no shell mounting |
| Usage readers | WP-L provider/local analytics | no shell mounting |
| Paneview shell | WP-M adapter/migration tests | controller owns root-shell integration |

At most two heavy native/Rust runners; default one.

### Wave 2 — structured product integration

- config pickers and command registry;
- rich timeline and image composer;
- browser context cards;
- compact resource/usage surfaces;
- My Work and Session Library;
- raw/structured handoff.

Milestone review before workflow implementation.

### Wave 3 — workflow engine

Sequential core first:

1. schema/ledger/projector/scheduler;
2. worktree allocation/gates;
3. delegation broker.

Then parallel:

- built-in role templates/workflow editor;
- Agent Control Center;
- MCP companion/provider tests;
- workflow fixture/native harness;
- AI contributions consuming stable workflow actions.

### Wave 4 — intelligence and final integration

- PR/browser/form/save assists;
- full resource/space/usage workspaces;
- native restart/process acceptance;
- accessibility, narrow layout, reduced motion, and performance certification;
- final SOL-medium review.

---

## 18. Required acceptance matrix

### ACP/runtime

- one writer per owned/native conversation;
- model/effort/mode discovery and confirmed mutation;
- unsupported option and silent-downgrade refusal;
- image input reaches both Claude and Codex;
- tools/plans/tasks/approvals/questions/children normalize;
- cancellation, adapter death, resume/load, stale generation;
- sidecar/descendant cleanup;
- raw-terminal handoff and rollback.

### Conversation

- safe rich Markdown;
- item order around tools;
- streamed deltas/final reconciliation;
- command output bounds;
- diff linkage;
- plan/task updates;
- child nesting and unrelated-session exclusion;
- two-session draft/attachment/control/scroll isolation;
- long-session virtualization/no forced scroll while reading history.

### Workflow

- cycle/edge rejection;
- concurrency/depth/turn/token/time limits;
- provider option validation;
- worktree collision prevention;
- deterministic and orchestrator-requested delegation;
- cancellation during spawn/send;
- parent teardown;
- retry/idempotent receipts;
- manual gates;
- provider-native child correlation;
- restart/replay without duplicate launches;
- artifact/output contracts.

### Browser

- child-view clipping/z-order/input isolation;
- same live page across docked/floating/expanded/collapsed;
- grab/annotate/draw;
- generation/hash validation;
- exact conversation/workflow attachment delivery;
- no arbitrary eval/cookies/storage leakage;
- profile isolation and viewport presets;
- Settings/dialogs over child view;
- keyboard, VoiceOver, reduced motion, trackpad, narrow window.

### Resources/usage

- bounded scan and ownership;
- external process no-kill;
- stale PID/PGID refusal;
- compact/full refresh policy;
- scan cancellation/last-result retention;
- reclaimable versus deletable distinction;
- provider windows/unavailable states;
- quota versus local analytics separation;
- no prompt/secret persistence;
- estimated-cost labels.

### Navigation/layout

- Working/Done/Settled Paneview sizing/collapse/restore;
- compact rows and hover details;
- Session Library does not start agents on open;
- layout snapshot migration;
- hidden panes preserve live terminal/provider/editor/browser state;
- keyboard focus/accessibility;
- Settings and root overlays remain reachable.

---

## 19. Stop conditions

Stop before editing or return a decision packet when:

- the Claude adapter cannot be packaged, authenticated, licensed, or updated safely;
- a provider lacks required image/config/session capability;
- handoff cannot prove release of the prior writer;
- a workflow would require concurrent writes to one worktree without serialization;
- provider-native child identity cannot be correlated authoritatively;
- native webview z-order prevents main-webview annotation/confirmation controls;
- a process cannot be proven owned;
- Paneview requires replacing the current shell/Dockview persistence model;
- an AI assist needs arbitrary shell/API mutation rather than a typed action;
- a shared-seam file contains overlapping unmerged work.

Do not hide these outcomes behind a fallback that claims the requested feature exists.

---

## 20. Completion definition

This amendment is complete when:

1. Claude Code and Codex start as structured ACP conversations without a hidden TUI.
2. Provider-advertised model, effort, permissions/mode, and service options are native selectors.
3. Screenshots paste, preview, restore, and reach both providers as real image input.
4. Commands, reasoning, tools, diffs, plans/tasks, approvals, questions, context, and nested children render in one provider-neutral timeline.
5. Raw CLI ownership can be entered/exited without duplicate writers or lost history.
6. A durable workflow can run orchestrator, implementation, review, spec, test, fix, and integration roles across validated providers/worktrees.
7. Agent Control shows exact state, phase, children, tools, artifacts, budgets, and safe controls.
8. Browser grab, annotation, and screenshot markup become reviewable conversation/workflow attachments from docked, floating, or expanded mode.
9. Resource and Usage each have compact and full accurate/safe surfaces.
10. My Work is compact; Session Library is separate; side stacks use Paneview.
11. PR, form, save, browser, run-config, and workflow AI assistance use typed proposal/confirmation boundaries.
12. Restart restores sessions, workflows, layouts, attachments, browser state, and durable events without duplicate providers, PTYs, Roslyn processes, or workflow nodes.
