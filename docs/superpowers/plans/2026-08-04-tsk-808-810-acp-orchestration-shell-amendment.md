# Assembly ACP, Orchestration, and Workbench Product Amendment

**Date:** 2026-08-04  
**Status:** Approved planning amendment; implementation remains paused  
**Applies to:** TSK-808, TSK-809, TSK-810, and the current Assembly native-workbench product wave

> **Execution authority:** This document is a binding amendment to:
>
> - `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; and
> - `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`.
>
> The 2026-08-01 master plan remains authoritative for unaffected work packages. Where this
> amendment conflicts with the master plan's PTY-only conversation architecture, combined
> Sessions/Find layout, browser feedback payload, resource/usage scope, guarded-agent scope, or
> parallel schedule, **this amendment wins**. The 2026-08-02 plan remains historical evidence only.

## 1. Why this amendment exists

The latest product clarification changes several architectural assumptions that were still baked
into Work Packages 3, 8, 9, 10A, and 11:

1. Model, reasoning effort, permissions/runtime mode, collaboration mode, and fast/service tier are
   first-class composer controls. They are not presented as slash commands.
2. TUI-local commands are not copied into Assembly merely because the Codex terminal supports
   them. Assembly exposes only commands that have useful product semantics in Assembly.
3. The existing transcript-backed PTY mirror remains valuable, but it cannot be the permanent
   live protocol for tools, approvals, reasoning, plans, tasks, images, and subagents.
4. ACP is the preferred provider transport for structured sessions. It does not replace the PTY
   subsystem and does not, by itself, create a user-visible xterm terminal.
5. Agent orchestration is an Assembly product capability built **on top of** ACP. ACP standardizes
   sessions and events; it is not a durable workflow engine, scheduler, worktree allocator, or
   cross-provider coordinator.
6. Browser annotation must produce exact element facts and image context that can be attached to
   the current conversation, including screenshot drawing—not only copied Markdown text.
7. Resource and provider-usage surfaces need both compact overlays and full workspaces, driven by
   one shared data authority.
8. The left side is an active-work navigator, not a combined active-work and machine-wide session
   cupboard. Working and Done are Paneview sections. Resume/discovery is a separate Session
   Explorer surface.
9. Every section inside a left, right, or bottom tool tab uses Dockview Paneview. Full workspaces
   remain Dockview center panels. Plain flex/collapsible stacks are migration scaffolding, not the
   finished layout architecture.
10. AI assistance is available throughout the workbench through typed, contextual actions with
    preview, confirmation, deterministic validation, and audit receipts.

## 2. Research conclusions and concrete implementation choices

### 2.1 What ACP does—and does not do

An ACP client starts or connects to an **agent adapter process**, normally over stdio, and exchanges
structured session requests and notifications. In Assembly's target architecture:

```text
Assembly Rust ACP client
    -> pinned provider adapter process
        -> Codex App Server, Claude Agent SDK, or another ACP agent
    -> structured session events
    -> Assembly canonical timeline and workflow state
```

That adapter process is not an interactive terminal surface. Starting an ACP session therefore does
**not** automatically create:

- an Assembly `TerminalRegistry` entry;
- an xterm view;
- a login shell;
- a raw Codex or Claude TUI;
- a PTY-backed scrollback tombstone.

ACP can report command/tool terminal activity, and some adapters can expose interactive/background
terminal updates. Assembly renders that activity as typed tool items and may create an expandable
terminal viewer when the protocol supplies an interactive terminal handle. That is separate from
opening the provider's native CLI.

A raw CLI is an explicit ownership handoff:

```text
Structured ACP owner
    -> settle or interrupt active turn
    -> release structured writer lease
    -> launch `codex resume <id>` or `claude --resume <id>` in existing PTY service
    -> Terminal owner

Terminal owner
    -> stop/detach native CLI safely
    -> reconcile transcript/native history
    -> resume the same provider session through ACP
    -> Structured ACP owner
```

One provider conversation has one writer at a time. Merely switching the visible Session view never
starts a second writer.

### 2.2 Provider path

Use the official ACP Rust SDK as the client boundary and pin provider adapters:

- **Codex:** use the official Codex ACP adapter first. It already translates Codex App Server
  threads, turns, items, reasoning, plans, commands, file changes, approvals, user input, images,
  reviews, web search, token/rate information, and subagent activity. Package a pinned macOS
  standalone adapter binary rather than running `npx @latest` per session.
- **Claude Code:** use the official Claude Agent ACP adapter, which is built on the Claude Agent
  SDK and supports images, permissions, tools, TODO/plan state, edit review, terminals, commands,
  and nested subagent updates. The packaging packet must either bundle a known Node 22 runtime or
  produce a reviewed standalone sidecar. It must not silently depend on whatever GUI PATH happens
  to contain.
- **Future providers:** register an `AcpProviderDescriptor` instead of adding provider branches to
  Svelte components. A provider that lacks ACP remains available through the existing PTY plus
  transcript-projection adapter with an honest reduced capability set.

Do not copy either adapter's internal UI assumptions into Assembly. ACP updates are normalized at
one Rust boundary into Assembly-owned types.

### 2.3 Open-source implementation references

Use these as implementation references, with license/attribution review before copying substantial
code:

- `agentclientprotocol/rust-sdk`: Rust client lifecycle, connections, protocol types, and test
  patterns.
- `agentclientprotocol/codex-acp`: Codex App Server normalization, config options, image input,
  plans, terminal/tool events, and subagent metadata.
- `agentclientprotocol/claude-agent-acp`: Claude Agent SDK normalization, permissions, nested
  subagent transcripts, TODOs, images, and terminals.
- `recailai/jockey`: a Tauri ACP client with runtime/session state, discovered models, modes,
  config options, and advertised commands.
- `xintaofei/codeg`: ACP multi-agent delegation using an app/broker-owned delegation tool and
  child ACP sessions. Reuse the architectural pattern, not its entire application.
- `pingdotgg/t3code`: canonical provider event/item taxonomy and projection ideas. Do not port its
  entire Effect server into Rust.
- `stablyai/orca`: MIT-licensed product/reference implementation for browser annotation, compact
  and full resource/space/usage surfaces, worktree hover/detail behavior, and active-work density.
- Dockview Paneview: the installed `dockview-core` Paneview primitive is the collapsible/resizable
  section owner for VS Code-style sidebars. Keep the repository's pinned version until a separate
  dependency upgrade is approved.

## 3. Locked product decisions

### 3.1 Structured sessions are the default

- A new Claude or Codex session created from Assembly starts as a structured ACP-owned session.
- A plain terminal still starts through the existing PTY path.
- Existing scanned/resumable terminal sessions remain importable. They start terminal-owned or
  read-only until the user chooses **Return to Conversation** and the ownership handoff succeeds.
- The raw terminal remains one action away, but opening it is an ownership transition, not a CSS
  visibility toggle over two simultaneously active writers.
- `ownedId` remains the stable Assembly identity across provider sessions, PTYs, workflows,
  worktrees, editor/browser state, and persistence.

### 3.2 Composer controls are capabilities, not slash commands

The fixed primary composer control order is:

```text
[Provider] [Model] [Effort] [Permissions/Mode] [Plan/Collaboration] [Fast/Service Tier] [More]
```

Only supported controls render. Values come from current session configuration options and
provider confirmations. Assembly never invents model names or assumes one provider's effort list
applies to another.

Commands are split into three catalogs:

1. **Provider semantic commands:** commands advertised by the active ACP session that make sense
   outside the native TUI, such as compact, review, status, skills, or provider-specific workflows.
2. **Assembly commands:** open terminal, return to conversation, fork session, open file, open
   worktree, create PR, show resources, show Agent Control, and similar app actions.
3. **Skills:** discovered provider/project skills, shown in a distinct section and inserted or
   invoked using the provider-supported syntax.

Do not show native-TUI-only commands such as terminal theme, pets, title/statusline, raw-terminal
copy mode, Vim/keymap configuration, or TUI-local process lists. Do not duplicate Model,
Permissions, Plan, Effort, or Fast as slash rows when a first-class control exists.

### 3.3 Native and Assembly-managed child agents are distinct

- **Native provider subagents** are created by Codex or Claude. Assembly observes their lifecycle,
  output, tools, and parent relationship when the provider supplies it.
- **Assembly workflow agents** are created by the Assembly workflow engine through ACP. Assembly
  owns their role, provider, model, effort, permission profile, worktree, budget, retries, and
  lifecycle.

Both appear in Agent Control, but they have different provenance and controls. Assembly must never
pretend a provider-native subagent is an Assembly-managed workflow node merely because its title
looks similar.

## 4. Target runtime architecture

### 4.1 Rust module layout

Refactor the existing `src-tauri/src/agent_conversation` area into one provider-neutral runtime:

```text
src-tauri/src/agent_runtime/
├── mod.rs
├── manager.rs
├── types.rs
├── capabilities.rs
├── event_buffer.rs
├── persistence.rs
├── ownership.rs
├── handoff.rs
├── attachments.rs
├── transcript_projection/
│   ├── mod.rs
│   ├── watcher.rs
│   ├── codex.rs
│   └── claude.rs
└── acp/
    ├── mod.rs
    ├── client.rs
    ├── process.rs
    ├── provider_registry.rs
    ├── session.rs
    └── normalize.rs
```

The migration may initially keep the current module path and move files only after tests prove
behavior. Do not perform a rename-only wave that obscures functional changes.

Core state:

```rust
pub enum AgentExecutionOwner {
    Structured,
    Terminal,
    Transitioning,
    Stopped,
}

pub enum AgentSessionState {
    Starting,
    Idle,
    Working,
    WaitingForApproval,
    WaitingForInput,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider: String,
    pub native_session_id: Option<String>,
    pub pty_session_id: Option<String>,
    pub execution_owner: AgentExecutionOwner,
    pub state: AgentSessionState,
    pub generation: u64,
    pub capabilities: AgentCapabilities,
    pub configuration: Vec<AgentConfigOption>,
    pub last_sequence: u64,
}
```

### 4.2 ACP provider registry

```rust
pub struct AcpProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment_policy: ProviderEnvironmentPolicy,
    pub packaging: ProviderPackaging,
}

pub struct AgentCapabilities {
    pub load_session: bool,
    pub resume_session: bool,
    pub images: bool,
    pub embedded_context: bool,
    pub tools: bool,
    pub approvals: bool,
    pub user_input: bool,
    pub plans: bool,
    pub tasks: bool,
    pub reasoning: bool,
    pub terminals: bool,
    pub native_subagents: bool,
    pub advertised_commands: bool,
    pub configuration_options: bool,
}
```

The registry is data-driven. Svelte does not branch on `provider === "codex"` to decide which
controls or timeline items exist.

### 4.3 Canonical event and item model

Replace the current message-heavy minimum event set with ordered, typed items. Preserve raw
provider metadata under a namespaced field for diagnostics and future features.

```rust
pub enum AgentItemKind {
    UserMessage,
    AssistantMessage,
    Reasoning,
    Plan,
    TaskList,
    CommandExecution,
    FileChange,
    McpToolCall,
    DynamicToolCall,
    WebSearch,
    ImageView,
    ImageGeneration,
    NativeSubagent,
    ContextCompaction,
    Review,
    Error,
    Unknown,
}

pub enum AgentContentChannel {
    AssistantText,
    ReasoningText,
    ReasoningSummary,
    PlanText,
    CommandOutput,
    FileChangeOutput,
    ToolOutput,
}

pub enum AgentEventKind {
    SessionStarted,
    SessionConfigured,
    SessionStateChanged,
    TurnStarted,
    TurnCompleted,
    TurnInterrupted,
    ItemStarted,
    ItemUpdated,
    ItemCompleted,
    ContentDelta,
    ApprovalRequested,
    ApprovalResolved,
    UserInputRequested,
    UserInputResolved,
    UsageUpdated,
    RateLimitsUpdated,
    ChildrenUpdated,
    Warning,
    Error,
}
```

Every event carries `ownedId`, provider, generation, sequence, timestamp, optional native session,
turn, item, request, parent-item, workflow-run, workflow-node, and workflow-agent IDs. Sequence
gaps trigger snapshot/reconciliation without clearing valid history.

### 4.4 Persistence and recovery

Use the existing append-only philosophy:

- canonical conversation events are appended under the Assembly app-data directory;
- provider raw frames are optional bounded diagnostics, not the UI database;
- periodic snapshots speed replay but never replace the event log;
- transcript projection can import native history after terminal ownership;
- provider/session IDs are deduplicated by native identity and canonical item ID;
- a crash preserves completed items and marks the active turn interrupted/unknown until the
  provider is reconciled;
- no provider transcript is copied wholesale into browser localStorage.

The current 500 ms full-tail transcript polling becomes an incremental Rust watcher with file
identity, byte offset, partial-line buffering, rotation/truncation detection, and occasional full
reconciliation. It remains the adapter for terminal-owned or externally created sessions, not the
primary structured protocol.

## 5. Conversation workbench revision

### 5.1 Timeline and visual hierarchy

Create typed timeline renderers instead of one generic message/card loop:

```text
ConversationTimeline.svelte
├── UserMessageItem.svelte
├── AssistantMessageItem.svelte
├── ReasoningItem.svelte
├── PlanItem.svelte
├── TaskListItem.svelte
├── CommandItem.svelte
├── FileChangeItem.svelte
├── ToolCallItem.svelte
├── ApprovalItem.svelte
├── UserInputItem.svelte
├── NativeSubagentItem.svelte
└── ErrorItem.svelte
```

Rules:

- assistant prose is the dominant, quiet reading surface;
- reasoning is first-class and collapses after completion;
- commands/tools are compact rows with expandable input/output, status, duration, cwd, and files;
- file changes open the existing native DiffEditor rather than rendering giant diff cards;
- plans and task lists show stable step IDs and status transitions;
- approvals and structured questions render inline at the exact item/request boundary;
- native subagents can be expanded into their nested transcript when capability metadata proves the
  relationship;
- normal text uses a real sanitized Markdown pipeline with syntax highlighting and file links;
- long output is bounded and virtualized; raw output remains available through disclosure/download;
- stable item IDs permit row virtualization without losing streamed content or focus.

### 5.2 Screenshot and image input—top-priority acceptance

This is the first conversation capability implemented after the ACP connection spike.

1. Paste or drop PNG, JPEG, GIF, or WebP into the composer.
2. Rust validates size, signature, aggregate count, canonical owner directory, and permissions and
   writes an opaque managed attachment.
3. The composer immediately shows a removable preview and preserves it per `ownedId` across session
   switches.
4. In structured ACP mode, send the image as an ACP image content block plus accompanying text. Do
   not reduce it to a path string when the provider advertises image input.
5. In terminal-projection mode, retain the current managed-path fallback and tell the provider the
   exact canonical file path once.
6. A failed send restores the exact draft and preview list. A successful send clears only the
   attachments included in that turn.
7. App restart recreates safe previews from managed paths; missing files become removable
   unavailable items.
8. Session removal and explicit attachment cleanup delete only the exact managed owner directory.

Acceptance uses both Claude and Codex and verifies the provider actually receives and reasons about
an image, not merely that a preview appeared.

### 5.3 First-class configuration controls

`AgentConfigOption` is provider-advertised and generic:

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'provider'
    | 'model'
    | 'reasoning'
    | 'permissions'
    | 'mode'
    | 'collaboration'
    | 'service-tier'
    | 'boolean'
    | 'unknown';
  label: string;
  currentValue: string | boolean | null;
  choices?: Array<{
    value: string;
    label: string;
    description?: string;
  }>;
  mutable: boolean;
  disabledReason?: string;
}
```

The UI maps recognized categories to primary controls and places unknown/provider-specific options
under **More**. It preserves provider order for models and effort levels. A displayed selection
changes only after the provider confirms it. Unsupported options are omitted or visibly read-only;
there is no decorative dropdown that lies about runtime state.

### 5.4 Commands and skills

Replace the hard-coded arrays in `ConversationSurface.svelte` with a session-owned resolved
catalog. Precedence is:

1. Assembly local commands;
2. provider-advertised semantic commands;
3. dynamic provider/project skills;
4. versioned fallback manifest only for a terminal-owned provider that cannot advertise commands.

The fallback manifest excludes TUI-only commands. Selecting a command either invokes a typed local
handler, invokes a typed provider command, or inserts text. Menu navigation never executes or sends.

### 5.5 Structured/terminal ownership handoff

Add explicit actions:

- **Open native CLI**
- **Return to Conversation**
- **Fork to native CLI**

The handoff state machine validates no active approval/input is orphaned, settles or interrupts the
turn, persists events, starts/stops the exact PTY or ACP session, and rolls back to the previous
owner on failure. It never leaves two writers hidden behind one rail row.

## 6. Assembly workflow orchestration

### 6.1 Product boundary

ACP supplies provider-neutral sessions, prompts, tools, approvals, configuration, and events.
Assembly supplies:

- workflow definitions and versions;
- role definitions;
- scheduling and concurrency;
- parent/child workflow identity;
- cross-provider routing;
- worktree allocation and write leases;
- loops, joins, barriers, retries, and budgets;
- approval policy;
- durable progress and recovery;
- Agent Control UI;
- audit receipts.

Do not encode a workflow only as one giant orchestrator prompt. A model may propose the next action,
but the Rust workflow engine validates and records every transition.

### 6.2 Durable workflow model

Extend the existing append-only orchestration module rather than creating another run ledger.

```rust
pub struct WorkflowDefinition {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub roles: Vec<WorkflowRoleDefinition>,
    pub nodes: Vec<WorkflowNodeDefinition>,
    pub edges: Vec<WorkflowEdgeDefinition>,
    pub policies: WorkflowPolicies,
}

pub struct WorkflowRoleDefinition {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub permission_profile: String,
    pub workspace_access: WorkflowWorkspaceAccess,
    pub tools: Vec<String>,
    pub instructions: String,
}

pub enum WorkflowNodeKind {
    Orchestrator,
    AgentTask,
    DeterministicCheck,
    ApprovalGate,
    Join,
    Loop,
    Publish,
}

pub struct WorkflowPolicies {
    pub max_parallel_agents: u16,
    pub max_depth: u16,
    pub max_retries_per_node: u16,
    pub wall_clock_budget_seconds: u64,
    pub turn_budget: Option<u64>,
    pub token_budget: Option<u64>,
    pub require_review_before_publish: bool,
}
```

Runtime state includes `WorkflowRun`, `WorkflowNodeRun`, `WorkflowAgentRun`, `WorkflowArtifact`,
`WorkflowDecision`, `WorkflowLease`, and append-only `WorkflowEvent` records.

States are explicit:

```text
queued -> starting -> working -> waiting -> completed
                         |          |
                         |          -> awaiting-approval / awaiting-input
                         -> blocked / failed / cancelled
```

### 6.3 Provider-neutral delegation through MCP

The concrete cross-provider path follows the proven ACP-delegation pattern:

1. Assembly starts the orchestrator as an ordinary ACP session with role-specific provider/model/
   effort/mode.
2. Assembly supplies a narrowly scoped `assembly-workflow-mcp` server to that session through the
   provider adapter's supported MCP configuration.
3. The MCP server exposes typed tools only:

```text
assembly.workflow.get_state
assembly.workflow.list_roles
assembly.workflow.delegate
assembly.workflow.get_agent
assembly.workflow.wait
assembly.workflow.submit_artifact
assembly.workflow.request_review
assembly.workflow.request_approval
assembly.workflow.cancel_agent
assembly.workflow.complete_node
```

4. `delegate` names a workflow role and task facts. The orchestrator cannot pass a raw executable,
   unrestricted permission profile, arbitrary provider, hidden environment, or unapproved path.
5. Rust validates depth, concurrency, budget, role, workspace lease, and current node state.
6. The runtime starts a child ACP session through the selected provider adapter and links its
   `ownedId`, native session, workflow run/node, role, worktree, and parent tool request.
7. Child updates flow into the same canonical event bus and workflow projection.
8. The tool result returns the durable child identity and eventual structured result, not an
   unbounded transcript dump.

Package the MCP server as a reviewed Assembly sidecar or hidden executable subcommand. It connects
back to the main app through a local authenticated channel with one run-scoped token. It exposes no
general shell, Git, filesystem, process, credential, or Tauri command surface.

### 6.4 Workspace and worktree policy

- One writing workflow agent has one exclusive worktree write lease.
- Two writers never mutate the same worktree concurrently unless a workflow definition explicitly
  serializes them behind the same lease.
- Review/spec-compliance roles default to read-only access to the implementation worktree.
- Parallel implementation nodes receive separate worktrees and produce explicit integration
  artifacts.
- The orchestrator does not merge arbitrary branches. It proposes an integration action through
  the existing typed Git/worktree safety service.
- Dirty, conflicted, missing, or unmerged worktrees block the relevant transition and surface a
  human-readable decision.
- Cancelling an agent releases the lease only after its provider session and process tree are
  settled and the worktree state is recorded.

### 6.5 Role and workflow templates

Ship an editable role catalog with safe defaults:

- Orchestrator
- Implementer
- Unit/Integration Test Runner
- Code Reviewer
- Spec Compliance Reviewer
- Security Reviewer
- UI/Browser Reviewer
- PR/Checks Fixer
- Documentation Writer

Initial workflow templates:

#### Implement, review, and compliance loop

```text
Plan
 -> Implement
 -> Deterministic focused tests
 -> Code Review
 -> Spec Compliance Review
 -> if findings: Fix -> tests -> reviews (bounded loop)
 -> Final deterministic gate
 -> Prepare PR
 -> Human approval
```

#### Fix pull-request checks

```text
Read exact PR/check facts
 -> Fixer in existing or isolated worktree
 -> deterministic tests
 -> reviewer
 -> prepare commit/push/PR update
 -> human confirmation
```

#### Browser feedback implementation

```text
Collect selected elements + screenshots + notes
 -> UI implementer
 -> run fixture/browser verification
 -> visual reviewer
 -> spec/review barrier
 -> stage result and evidence
```

Workflow definitions are versioned JSON or TOML under the Assembly app-data/config directory and
may be exported into a project. Import validates schema, role/provider availability, permissions,
and tool allow-lists before activation.

### 6.6 Agent Control module

Add one state authority with two projections:

- **Compact Agent Control Paneview** in the right tool area: active workflow, phase, progress,
  active/waiting/failed counts, decisions, and quick controls.
- **Full Agent Control center workspace:** graph/list, workflow plan, task queue, agents, nested
  transcripts, tool activity, worktrees, artifacts, budgets, decisions, and audit timeline.

The full workspace supports:

- pause/resume workflow;
- cancel workflow or exact child;
- retry failed node;
- approve/decline gate;
- steer one agent with an appended user instruction;
- open child transcript;
- open worktree/editor/diff;
- promote a workflow child into a standalone active-work session;
- copy handoff/context;
- inspect raw canonical event and provider metadata in development mode.

The UI never derives progress from prose. It projects durable workflow events and provider states.

### 6.7 Native subagents in Agent Control

Provider-native subagents appear beneath their owning ACP session with provenance `native`. Their
output, tools, reasoning, status, and relationship are shown when available. Controls are limited to
what the provider protocol actually supports. An Assembly-managed child has provenance `workflow`
and the full workflow controls above.

### 6.8 Recovery

On restart:

1. replay workflow events and snapshots;
2. inspect provider sessions and task-owned PTYs/processes;
3. mark unverifiable active nodes `reconciling`, never silently completed;
4. resume/load ACP sessions when supported;
5. recover terminal-projection history where required;
6. revalidate worktree leases and deterministic checks;
7. surface a decision for any ambiguous mutation or outcome-unknown remote action.

## 7. Browser and visual-feedback revision

### 7.1 One live browser, three presentation modes

The existing Work Package 8 child-webview spike remains mandatory. The finished browser has one
workspace/tab state and one native child webview that moves between:

- collapsed global floating control;
- docked Browser center panel;
- expanded root overlay covering the Assembly workbench below the macOS title bar.

It never reloads merely because presentation changes.

### 7.2 Toolbar contract

Match the capability density shown in the supplied Orca references while keeping Assembly styling:

- Back, Forward, Reload, URL
- Import/context intake
- Grab page element
- Annotate page element
- Draw on screenshot
- Development-only browser devtools
- Open in default browser
- Profiles and cookie/data policy
- Viewport size
- Expand/Restore
- Collapse
- Overflow and Browser Settings

`Import Cookies` remains disabled until a separate secret-source, format, isolation, deletion, and
consent plan is approved.

### 7.3 Three feedback capture modes

#### Grab page element

Captures a fixed, sanitized payload:

```ts
interface BrowserElementContext {
  workspaceId: string;
  tabId: string;
  generation: number;
  pageUrl: string;
  pageTitle: string;
  selector: string;
  tagName: string;
  role: string | null;
  accessibleName: string | null;
  textSnippet: string;
  rect: { x: number; y: number; width: number; height: number };
  screenshotAttachmentId: string | null;
}
```

The inspector is a fixed injected script, not an arbitrary eval API. Rust revalidates all bounds,
lengths, IDs, and owner/generation values.

#### Annotate page element

Freezes the selected element outline and opens a note card with **Change** and **Question** intent.
Saving creates a queued `BrowserAnnotation` with immutable page facts and a screenshot attachment.
It does not send immediately.

#### Draw on screenshot

1. Capture the visible native child-webview viewport using the platform adapter. On macOS, the
   implementation spike targets `WKWebView.takeSnapshot`; future platform adapters use their native
   capture API.
2. Store the source image in the same managed attachment vault as composer images.
3. Open an Assembly-owned canvas overlay with pen, arrow, rectangle, highlight, text, undo/redo,
   clear, crop, and cancel.
4. Save a flattened PNG plus bounded vector-operation JSON for later edit/replay.
5. Queue it as browser feedback with URL, viewport, timestamp, and optional element context.

For an element crop, capture the viewport and crop in Rust using the selected CSS rect, child-view
bounds, and device scale. Fail honestly if coordinate reconciliation cannot be proven.

### 7.4 Conversation integration

**Send to Conversation** creates one structured context attachment:

- bounded Markdown summary of each annotation;
- immutable element facts;
- one image content block per screenshot/drawing in structured ACP mode;
- exact managed paths only in terminal-owned fallback mode.

It targets the captured `ownedId` and generation, opens the existing Session destination, and stages
the content for review. It never overwrites a nonempty draft without a merge preview and never
presses Send automatically.

**Ask Agent** creates the same immutable context for the current conversation or selected workflow
agent. Browser page text remains delimited untrusted data.

## 8. Resource Manager, Space, and Usage revision

### 8.1 Shared compact/full surface rule

Each capability has one store/service and two projections:

```text
status-bar or floating compact control
    -> compact popover
    -> Open full workspace
        -> center Dockview panel
```

The compact and full views do not fetch or calculate independently.

### 8.2 Resource Manager

Keep the Work Package 9 bounded native snapshot and ownership rules. Add:

- compact total CPU/RSS line in the status area;
- project -> worktree -> owned session/process hierarchy;
- sparklines from a bounded in-memory sample ring;
- refresh and cleanup actions;
- exact owned-process stop only after typed confirmation;
- external processes visible but never stoppable;
- full Resource Manager center panel with sorting, filters, process tree, ports, LSP roots, logs,
  ownership facts, and pressure history.

### 8.3 Workspace Space manager

Separate disk inventory from live process resources while sharing canonical project/worktree facts:

- compact **Space** card shows scanned and safely reclaimable bytes;
- full center workspace includes summary metrics, treemap, selected workspace top-level items,
  searchable/sortable table, protection reason, and batch selection;
- scans are explicit/manual or idle-budgeted, never on cold launch;
- active, dirty, unmerged, protected, user-data, and unknown entries are not reclaimable;
- deletion delegates to existing worktree/archive/cleanup confirmations;
- no broad cache, Docker, or repository pruning command.

### 8.4 Provider quota and local usage analytics

Keep two truth classes separate:

1. **Provider quota:** provider-reported current limits and reset windows from ACP/account APIs when
   available. Unsupported providers show a reason.
2. **Local analytics:** facts indexed from local provider transcripts/events—sessions, turns,
   provider, model, input/output/cache/reasoning tokens, duration, and dates.

For scalable historical analytics, introduce an optional local SQLite analytics index in the Rust
resource/usage package. It contains no transcript text, prompts, credentials, or files—only bounded
numeric/session metadata and source cursors. All filtering, grouping, aggregation, sorting, and
paging runs in SQL. A migration/version table and generated-SQL tests are mandatory. If the user
disables analytics, do not index and delete only this derived database on confirmation.

Compact Usage popover:

- Detailed/Compact toggle;
- Claude, Codex, and enabled providers;
- current quota percentage/reset when trustworthy;
- unavailable/error reason;
- link to usage details and account management.

Full Usage workspace:

- active days, sessions, agents/workflow children, turns, PRs created;
- daily intensity heatmap;
- token mix;
- provider/model/session breakdown;
- cache share and reasoning share;
- time range and provider/model/workflow filters;
- optional estimated cost from a versioned price catalog, visibly labelled **Estimate** and never
  presented as a provider bill.

## 9. Navigation, sessions, and Paneview revision

### 9.1 Layout rule

- Full workspaces are Dockview center panels.
- Left, right, and bottom tool tabs contain Dockview **Paneview** sections.
- Temporary popovers/dialogs remain overlay primitives, not Paneview panels.
- A plain CSS collapsible may remain only while migrating a section to Paneview.

Create one Svelte-owned Paneview host over the installed `dockview-core` `createPaneview` API:

```text
shell/layout/paneviewHost.ts
shell/layout/paneviewTypes.ts
components/layout/ShellPaneviewHost.svelte
```

It supports stable panel IDs, Svelte-owned body/header hosts, expanded state, min/max body size,
reordering, keyboard separators, `toJSON/fromJSON`, versioned migration, and no component recreation
on collapse/restore.

### 9.2 Left active-work navigator

The left side shows only Assembly-owned active work:

- Working Paneview
- Done Paneview
- optional Settled Paneview when nonempty

Each is a real Paneview panel with independent collapse and resize. Default width is compact and
IDE-like; rows are dense rather than large cards.

Row contract:

- one-line title and provider/status glyph;
- optional worktree/branch/task badge;
- current activity label;
- hover/focus detail popover with model/effort, full path, latest turn, elapsed time, PR/checks,
  ports, and workflow membership;
- selected row may expand an inline accordion for actions and deeper facts;
- action controls appear on focus-within as well as hover;
- hover is never the only way to discover a required fact/action.

The active-work navigator does not list every transcript on the machine.

### 9.3 Separate Session Explorer

Move Find/Resume/Adopt out of `SessionsColumn.svelte` into one `SessionExplorerPane.svelte` registered
as a right-side tool Paneview. It owns:

- project/provider/date/state filters;
- search;
- grouped resumable/native sessions;
- model/worktree/resumability metadata;
- read-only transcript inspection;
- Resume, Continue in New, Fork, and Adopt actions;
- no duplicate scanner/store.

A command and left-nav action open Session Explorer. A future **Open in center** action may move the
same component host into a center panel; it must not create a second explorer instance.

### 9.4 All side panels use Paneview

Migrate the right tool roster sections—Source Control, Worktrees, Run Configurations, Problems,
Context, Resources, Agent Control, Sessions, and future tools—to Paneview-owned sections within the
selected tool tab. Persist size/order/expanded state per workspace where appropriate. A tool tab can
contain multiple sections, but sections never implement their own competing resize/collapse model.

## 10. AI assistance throughout Assembly

### 10.1 One assist-action registry

Extend the guarded action work into a reusable registry rather than sprinkling prompt strings in
components:

```ts
export interface AiAssistAction<Input, Proposal> {
  id: string;
  title: string;
  contexts: AssistContextKind[];
  risk: 'read' | 'draft' | 'local-mutation' | 'remote-mutation';
  buildFacts(input: Input): Promise<AssistFactPacket>;
  requestProposal(packet: AssistFactPacket): Promise<Proposal>;
  validateProposal(proposal: Proposal, packet: AssistFactPacket): AssistValidation;
  preview(proposal: Proposal): AssistPreview;
  execute?(proposal: Proposal, confirmation: AssistConfirmation): Promise<AssistReceipt>;
}
```

Every action uses the selected existing conversation or a short-lived structured ACP utility
session. It shows provider/model/effort before execution and never silently launches an unrelated
agent.

### 10.2 Initial integrated actions

- Draft PR title/body from deterministic Git/PR facts.
- Review a PR or selected diff and stage findings.
- Address review comments or failed checks through a workflow template.
- Suggest/fill form fields in PRs, run configurations, workflow definitions, annotations, and
  settings descriptions.
- Generate a run configuration proposal from bounded repository facts.
- Explain errors, diagnostics, resources, worktree state, or provider usage.
- Review browser annotations and prepare implementation tasks.
- Summarize old sessions/worktrees/projects with source-linked, invalidating receipts.
- Prepare Git/worktree cleanup or repair through existing confirmation boundaries.

### 10.3 Save-time checks

Create a `PreflightCheckPipeline`:

1. deterministic synchronous validators run first;
2. hard deterministic errors block save with exact fields;
3. optional agent review runs asynchronously against a bounded fact packet;
4. agent findings are suggestions/warnings unless a separately configured policy requires review;
5. the user can apply individual fixes, save anyway, or open the full conversation;
6. no provider text directly mutates the form;
7. every accepted consequential fix is validated and audited.

This provides the requested “make sure I am not screwing something up” behavior without turning an
LLM into an invisible save hook.

## 11. Work-package amendments

### 11.1 Work Package 2—Paneview foundation

Add the Svelte Paneview host and migrate left/right/bottom section ownership before redesigning
session cards. Keep the existing Dockview center roster work.

### 11.2 Work Package 3—active work and Session Explorer

Replace the combined My Work/Find design with the narrow active-work navigator and separate Session
Explorer above. Working, Done, and Settled are Paneview sections.

### 11.3 Work Package 8—browser

Retain the child-webview isolation and global floating action surface. Add screenshot capture,
drawing, image attachment payloads, three explicit feedback modes, and exact ACP conversation
integration.

### 11.4 Work Package 9—resources, Space, and Usage

Split one data authority into compact/full Resource Manager, Space manager, provider quota, and
local usage analytics. Add the derived local analytics index only under the privacy/SQL contract.

### 11.5 Work Package 10A—structured agent runtime and conversation

The PTY-only architecture in the master plan is superseded. Work Package 10A now includes:

1. ACP connection/packaging spike for Codex and Claude;
2. canonical event/item contract;
3. structured session default and ownership state machine;
4. image input first;
5. model/effort/permissions/mode/service-tier controls;
6. tool, approval, user-input, plan/task, reasoning, diff, and terminal item rendering;
7. native subagent observation;
8. incremental transcript projection fallback;
9. raw CLI ownership handoff;
10. migration and recovery.

### 11.6 New Work Package 10B—workflow orchestration and Agent Control

Implement the durable workflow engine, role catalog, MCP delegation boundary, child ACP sessions,
worktree leases, loops/barriers/budgets, templates, Agent Control compact/full UI, audit, and
recovery specified in section 6.

### 11.7 Work Package 11—AI assist actions

Work Package 11 consumes the certified structured runtime and workflow engine. It adds contextual
assist/preflight actions and proposals; it does not create a second conversation or orchestration
runtime.

## 12. File ownership and implementation packets

### Packet A—shared contracts, sequential

Controller-owned:

- `ownedSessions.ts`
- `sessionWorkspaces.ts`
- `tauriSource.ts`
- Rust `main.rs`, Cargo manifests, package manifests
- capability registration
- ShellFrame/center roster
- shared Paneview registration types
- canonical event/workflow ID types

Freeze these before parallel lanes.

### Packet B—ACP runtime and packaging

Agent-owned after freeze:

- Rust `agent_runtime/acp/**`
- provider descriptors and sidecar resolver
- ACP fixtures/tests
- adapter packaging scripts/config

### Packet C—conversation projection and UI

Agent-owned:

- `shell/conversation/**`
- conversation components
- canonical reducer/projector tests
- attachment frontend and Markdown render integration

### Packet D—workflow engine

Agent-owned:

- Rust workflow/orchestration engine modules
- Assembly workflow MCP sidecar/server
- workflow definition parser/validator
- workflow fixtures and replay tests

### Packet E—Agent Control UI

Agent-owned:

- `shell/agents/**`
- Agent Control compact/full components
- workflow graph/list/task/decision projections

### Packet F—browser feedback

Agent-owned:

- browser Rust module and fixed inspector script
- browser store/model/presentation
- overlay, annotation, drawing, and feedback components
- browser fixtures/tests

### Packet G—resources, Space, and Usage

Agent-owned:

- resource/usage Rust modules
- derived analytics migrations/queries when approved
- resource, space, quota, analytics stores/services/components

### Packet H—navigation and Paneview

Agent-owned:

- Paneview host
- left active-work navigator
- Session Explorer
- right/bottom Paneview migration
- layout migration tests

### Packet I—AI assist and preflight

Agent-owned:

- assist-action registry
- fact packet builders
- preflight pipeline
- proposal/preview components
- PR/form/save-time integrations outside shared seams

## 13. Parallel execution schedule

### Phase 0—sequential research spikes

1. Re-anchor current `main` and update proof ledger.
2. Prove Codex ACP new/resume/image/config/tool/subagent flow in a Rust harness.
3. Prove Claude ACP new/resume/image/config/tool/nested-subagent flow and decide packaging.
4. Prove child-webview clipping plus macOS snapshot capture and coordinate reconciliation.
5. Prove installed Dockview Paneview can preserve Svelte-owned hosts and serialize sizes/order.
6. Freeze canonical agent events, config options, workflow IDs, ownership state, Paneview
   registration, and attachment payloads.

No broad UI lane starts before these contracts are written back into the master plan/proof ledger.

### Phase 1—parallel foundation

Run in parallel after Phase 0, respecting the two-heavy-runner maximum:

| Lane | Work | Heavy runner |
| --- | --- | --- |
| 1 | ACP manager, provider registry, sidecar lifecycle | Rust slot 1 |
| 2 | canonical conversation reducer/projectors and typed item UI fixtures | Node-light |
| 3 | workflow engine/event replay/definition validation | Rust slot 2 |
| 4 | Paneview host, active-work navigator, Session Explorer | Node-light |
| 5 | browser HTML state/annotation/drawing UI against mock native backend | Node-light |
| 6 | resource/Space/Usage models and fixture UI | Node-light |
| 7 | AI assist/preflight pure contracts | Node-light |

### Phase 2—rolling provider/native integration

1. Conversation ACP integration uses Rust slot released by lane 1.
2. Browser native child-view/snapshot work uses the next Rust slot.
3. Workflow MCP sidecar and child ACP sessions use the next slot.
4. Resource process/LSP/usage backend and optional analytics DB use the next slot.
5. Each lane releases the heavy slot immediately after its focused command.

### Phase 3—dependent product integration

- Agent Control integrates after workflow replay and child-session contracts pass.
- Browser Send/Ask Agent integrates after image attachments and owner-generation guards pass.
- AI PR/save/form actions integrate after deterministic facts and structured sessions pass.
- Full Usage analytics integrates after privacy setting, index migration, and SQL query tests pass.
- Shared ShellFrame, capabilities, routes, package scripts, and settings are applied once by the
  controller from returned receipts.

### Phase 4—serialized certification

Run focused tests, full Node checks/build, Rust tests/build, native multi-provider workflow proof,
browser proof, resource/process cleanup, SOL review, security review, and only then PR/task closure.

## 14. Required focused tests

Add or revise focused tests for:

- ACP initialization, capability negotiation, new/resume/load/close, image prompts, config options,
  tool lifecycle, approvals, user input, plans/tasks, native subagents, disconnect/recovery;
- one-writer ownership and handoff rollback;
- canonical event ordering, sequence gaps, replay, raw metadata bounds;
- per-session draft/control/attachment/scroll isolation;
- command catalog without TUI-only/config duplicates;
- screenshot paste/drop, managed vault, exact provider image receipt, cleanup;
- workflow schema, graph validation, role/provider availability, budgets, loops, joins, retries,
  cancellation, leases, restart reconciliation;
- MCP delegation allow-list, run token, depth/concurrency limits, prompt/path/permission injection;
- native versus workflow child provenance;
- Paneview host identity, collapse/resize/reorder, serialization/migration, no component recreation;
- compact left rows, hover/focus detail, Session Explorer separation;
- browser element selection, snapshot crop, drawing serialization, owner/generation guards, draft
  conflict, image attachment payload;
- resource/Space shared stores, protected deletion, provider unsupported states;
- analytics migrations and SQL-side filters/grouping/aggregation/paging;
- AI fact-packet bounds, hostile input separation, preview/confirmation, preflight apply/save-anyway;
- app restart with active ACP sessions, terminal sessions, workflow runs, browser tabs, and
  Paneview/Dockview layouts.

## 15. Native acceptance matrix

A single rebuilt-Tauri campaign must prove:

1. Start one Codex and one Claude structured session without creating PTYs.
2. Paste screenshots into both composers and prove both providers receive image input.
3. Change model, effort, and permissions through first-class controls and receive authoritative
   confirmation.
4. Render reasoning, commands, output, file changes, approvals, structured questions, plans/tasks,
   terminals, and native subagents in one consistent timeline.
5. Open native CLI for one session, prove the ACP writer is released, work in the CLI, and return to
   the structured conversation with reconciled history and no duplicate writer.
6. Run the Implement -> Test -> Review -> Spec Compliance -> Fix loop with at least two providers,
   exact worktree leases, parallel read-only reviewers, bounded retry, and human approval before a
   consequential Git/PR action.
7. Pause, steer, retry, cancel, and restart/recover a workflow from Agent Control.
8. Show native and workflow child agents with correct provenance and nested output.
9. Use the bottom-right browser control to expand the same live page, grab/annotate an element,
   draw on a screenshot, and attach the images/facts to the exact conversation without auto-send.
10. Restore the same browser to docked/collapsed modes without reload or profile leakage.
11. Open compact and full Resource Manager, Space, and Usage surfaces and prove they share data and
    truthful ownership/quota/source labels.
12. Use the narrow left active-work navigator, independently resize/collapse Working and Done
    Paneviews, inspect hover details, and open machine-wide sessions only in Session Explorer.
13. Restart and restore Dockview, Paneview, session, workflow, conversation, browser, attachments,
    resource settings, and active work without duplicate processes.
14. Prove AI draft/review/preflight actions never mutate or publish without validation and required
    confirmation.
15. Quit and verify every task-owned ACP adapter, MCP sidecar, PTY, native child webview, Roslyn/
    BuildHost, workflow child, browser fixture, and test process exits or is intentionally persisted
    by an explicit supported policy.

## 16. Stop conditions

Stop and request a decision when:

- a provider adapter cannot prove image input, model/effort/mode configuration, or session resume;
- Claude packaging would depend on an unbounded/unverified system Node installation;
- ACP and the native CLI cannot safely hand off one session without two writers;
- the proposed canonical model would require provider branches throughout Svelte;
- a workflow transition cannot be represented as a typed, durable event;
- the orchestrator would require unrestricted shell/filesystem/process tools;
- two writers need the same worktree without serialization;
- native child webview z-order prevents Assembly controls from receiving input;
- macOS screenshot capture/crop coordinates cannot be proven;
- provider usage has no stable source but the UI would imply a real quota;
- historical analytics would require in-memory grouping/filtering/paging instead of SQL;
- Paneview would recreate a terminal/editor/browser/resource component on collapse/restore;
- AI save-time review would silently block or mutate user data;
- more than two heavy runners overlap;
- native proof or cleanup cannot be completed.

## 17. Completion definition

This amendment is implemented when:

- Claude and Codex new sessions are ACP-owned by default, with explicit safe terminal handoff;
- screenshot attachment works end-to-end for both providers;
- model, effort, permissions/mode, plan/collaboration, and service tier are truthful first-class
  controls;
- one typed timeline renders the required tools, reasoning, plans/tasks, approvals, terminals,
  diffs, and native subagents;
- Assembly can run durable cross-provider role workflows with worktree safety, loops, reviews,
  Agent Control, recovery, and audit;
- browser element/screenshot feedback attaches exact facts and images to the matching session;
- Resource Manager, Space, and Usage have compact and full views over shared authorities;
- the left side is a compact active-work navigator, session discovery is separate, and side/bottom
  sections are Paneviews;
- contextual AI actions are available throughout the workbench without bypassing deterministic
  validation or confirmation;
- all required native evidence, security review, cleanup receipts, and individually tracked task
  closure states are complete.
