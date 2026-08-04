# Assembly ACP Runtime, Orchestration, and Workbench Amendment

**Date:** 2026-08-04  
**Status:** Execution-authority amendment; implementation remains paused until the user authorizes it.  
**Baseline:** `jcoble/mac-command-bar` `main` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`.  
**Supersedes:** The provider/runtime, conversation controls, command catalog, session-navigation, browser presentation, resource/usage presentation, and integrated-agent portions of:

- `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`
- `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`

The 2026-08-01 product-wave plan remains authoritative for unaffected packages such as product identity, contrast, editor/LSP, Git, GitHub safety, filesystem safety, Markdown, settings compatibility, and the existing execution discipline. The 2026-08-02 conversation plan remains historical discovery evidence only.

---

## 1. Why this amendment exists

The current master plan overcorrected after discovering that a live PTY session and a second structured provider process could both write to one native Claude/Codex conversation. It locked the PTY in as the permanent sole writer and treated the structured view as a transcript projection.

That is safe, but it cannot deliver the requested product:

- native model, effort, permission, and mode pickers;
- rich live tool calls, command output, file edits, plans, tasks, approvals, and structured questions;
- first-class screenshot attachments in the composer;
- live sub-agent output and control of app-managed agents;
- provider-neutral orchestration across Claude, Codex, and future ACP providers;
- a workflow engine with parallel roles, bounded review loops, gates, retries, and visible progress;
- the Codex/T3-style conversation experience without parsing terminal pixels or guessing state from prose.

The corrected rule is not “the PTY is always the writer.” It is:

> **One native conversation has exactly one active writer at a time. The writer may be Assembly's structured ACP runtime or the existing native CLI PTY. Switching surfaces performs an explicit ownership handoff.**

The existing `ownedId`, terminal registry, transcript scanners, worktree/session state, and per-session workspace snapshots remain the product foundation. They are extended rather than replaced.

---

## 2. Research conclusions and implementation references

The implementation should reuse proven ideas, not copy whole applications.

### 2.1 ACP is the structured runtime boundary

Use the official `agentclientprotocol/rust-sdk` as Assembly's Rust ACP client. ACP is a JSON-RPC protocol over a transport such as stdio. Starting an ACP adapter starts a headless adapter/provider process; it does **not** create an xterm or a visible native CLI session.

An ACP-backed agent can still execute commands and expose terminal/tool output. Assembly renders those as typed command/tool items. When a provider requests a genuinely interactive terminal, Assembly may allocate a separately classified tool terminal through the existing terminal backend. That tool terminal is not the native Claude/Codex TUI and is not the session's writer.

Provider adapters for the first wave:

- `agentclientprotocol/codex-acp`: starts Codex app-server and exposes models, reasoning effort, fast mode, permission/sandbox modes, images, plans, reasoning, commands, file changes, MCP calls, reviews, web search, token usage, and sub-agent activity.
- `agentclientprotocol/claude-agent-acp`: uses the official Claude Agent SDK and exposes images, tool calls and permissions, edit review, TODO/task updates, nested sub-agent transcripts, interactive/background terminals, custom commands, and MCP.

Packaging is pinned and reproducible. Do not run `npx ...@latest` for each session. Build or vendor versioned sidecars with hashes. Codex ACP can be packaged as a standalone macOS sidecar. Claude Agent ACP currently requires Node 22 or newer; the packaging spike must prove either a bundled Node 22 runtime plus pinned adapter bundle or an approved standalone packaging path. A user's arbitrary global Node installation is not the packaged-product contract.

### 2.2 Jockey proves dynamic ACP configuration belongs in UI controls

`recailai/jockey` is a useful Tauri/Rust reference for remembering ACP-discovered models, modes, config-option definitions, and session-scoped available commands. Its product explicitly manages role, model, and MCP choices through UI controls instead of pretending they are all slash commands.

Reuse the pattern, not its database or complete app architecture:

- cache config-option definitions by runtime/provider version;
- cache current values by owned session and generation;
- populate controls from advertised options;
- keep available commands separate from model/mode configuration;
- never claim a value changed until the provider confirms it.

### 2.3 T3 Code supplies the canonical event taxonomy

Use T3 Code's provider-neutral separation of sessions, turns, items, content deltas, requests, tasks, plans, tools, usage, and raw provider data as a taxonomy reference. Do not port its full Effect server or React client.

### 2.4 CodeG proves cross-provider delegation needs an app-owned broker

`xintaofei/codeg` demonstrates the concrete pattern required for Assembly workflows:

1. an orchestrator agent calls an app-owned MCP delegation tool;
2. an authenticated local companion/broker receives the call;
3. the broker starts a separate ACP child session with provider/model/mode defaults;
4. the child runs independently and reports structured lifecycle events;
5. the broker correlates the child result to the parent tool call;
6. cancellation, depth, time, token, and turn limits remain app-owned.

Assembly should implement this as an extension of its existing append-only orchestration authority. ACP supplies provider sessions and event transport. ACP alone is not the workflow engine.

### 2.5 Orca supplies concrete interaction references

The supplied screenshots are from `stablyai/orca`, whose public source confirms the relevant patterns:

- `BrowserPane.tsx`: persistent browser views, element grab, annotations, screenshot markup, profile controls, devtools/external open, and agent delivery;
- `ResourceUsageStatusSegment.tsx`: compact process/worktree/session resource popover;
- `WorkspaceSpaceCompactPanel.tsx` and `WorkspaceSpaceManagerPanel.tsx`: compact disk summary plus full treemap/table review and safe cleanup;
- `UsageRosterPanel.tsx` and `StatsPane.tsx`: compact multi-provider usage popover plus full usage analytics;
- `WorktreeCard*`: compact worktree/session rows, hover identity, metadata, expandable details, and separate detail modules.

Reuse the interaction model and safety lessons. Keep Assembly's Svelte/Tauri/Dockview/Rust architecture.

---

## 3. Locked product decisions

1. New Claude and Codex sessions start in **Structured** mode by default through ACP.
2. The native CLI remains one click away as an explicit **Open in Terminal** handoff, not as a permanently duplicated runtime.
3. Existing/scanned native CLI sessions remain terminal-owned until the user explicitly returns them to Structured mode.
4. `ownedId` remains Assembly's permanent workspace/session identity. ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, workflow run IDs, and child IDs are metadata.
5. Model, effort/thinking, permissions/mode, fast/service tier, and other advertised configuration are real selectors. They are not slash commands.
6. The slash/command menu contains only provider commands, skills, and Assembly actions that are meaningful in the current execution mode. TUI-only commands such as terminal theme, pets, keymap, raw scrollback, terminal title, and TUI status line are omitted.
7. Screenshot paste is the first end-to-end structured-runtime acceptance slice, not late polish.
8. Provider-internal sub-agents and Assembly-managed workflow agents are different things:
   - provider-internal children are observed and inspected through provider events/metadata;
   - Assembly-managed agents are created, configured, paused, cancelled, retried, and routed by Assembly's workflow engine.
9. Cross-provider orchestration uses Assembly-managed ACP sessions and an app-owned MCP broker. It does not depend on Claude or Codex implementing identical native sub-agent behavior.
10. Center destinations use Dockview. Left, right, and bottom stacked regions use Dockview Paneview. Dialogs, menus, popovers, hover cards, and full-shell overlays are not Paneviews.
11. The compact left work column contains active work only. The large resumable-session catalog moves to a dedicated **Session Library** destination and is not mixed into Working/Done.
12. Browser state has four presentations over the same live tab: Docked, Floating, Full Screen, and Collapsed.
13. Resources and Usage each have a compact status-bar/popover view and a separate full center workspace.
14. AI assistance is available throughout the workbench through a shared proposal/validation registry. It never becomes an unvalidated mutation path.
15. Every asynchronous response carries stable owner identity plus generation and is discarded when stale.

---

## 4. Runtime architecture

### 4.1 Execution ownership state machine

Extend `OwnedSession` additively:

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';

export interface OwnedAgentRuntimeState {
  owner: AgentExecutionOwner;
  runtimeProfileId: string | null;
  providerSessionId: string | null;
  activeTurnId: string | null;
  generation: number;
  lastError: string | null;
}
```

Rules:

- `structured`: one ACP connection is allowed to send turns, control settings, answer approvals, and interrupt.
- `terminal`: one existing PTY/native CLI is allowed to accept user input. Structured state is a read-only transcript projection/importer.
- `transitioning-*`: both writers are disabled. The manager reconciles history and acquires/relinquishes ownership.
- `stopped`: neither writer exists. History and workspace remain inspectable.

A runtime transition is a backend transaction with rollback semantics. The frontend cannot flip this state optimistically.

### 4.2 Rust owner: `AgentRuntimeManager`

Refactor the existing `src-tauri/src/agent_conversation` module instead of creating a parallel agent subsystem.

Create:

```text
tauri-svelte-preview/src-tauri/src/agent_conversation/runtime/
├── mod.rs
├── manager.rs
├── acp_client.rs
├── sidecar.rs
├── provider_catalog.rs
├── config_options.rs
├── tool_terminals.rs
└── handoff.rs
```

Extend existing:

```text
agent_conversation/mod.rs
agent_conversation/protocol.rs
agent_conversation/transcript.rs
agent_conversation/attachments.rs
```

`AgentRuntimeManager`, keyed by `ownedId`, owns:

- the execution-owner state machine;
- ACP sidecar process lifecycle and stderr ring;
- ACP initialize/auth/session start/load/resume/close;
- provider/native IDs;
- active turn and cancellation;
- advertised capabilities, config options, commands, and skills;
- permission/user-input requests;
- normalized event sequencing and snapshot recovery;
- transcript import/reconciliation;
- structured/native-terminal handoff;
- process ownership metadata for Resource Manager.

The current unwired handwritten `provider.rs` remains disabled during migration. After ACP acceptance it is removed, or retained only as an explicitly selected direct-Codex optimization behind the same manager interface. It must never silently become a second runtime.

### 4.3 Provider catalog

```rust
pub struct AgentRuntimeProfile {
    pub id: String,
    pub provider: AgentProvider,
    pub display_name: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub environment_policy: RuntimeEnvironmentPolicy,
    pub packaging_version: String,
    pub sha256: String,
}
```

Initial profiles:

- `codex-acp-pinned`
- `claude-agent-acp-pinned`

Future providers register through the same catalog. The UI never branches on a new provider merely to render standard messages, tools, config options, approvals, tasks, or child runs.

### 4.4 Sidecar isolation and resource policy

Default first release: one isolated ACP sidecar per structured owned session. Pooling/multiplexing is permitted only after a measured provider-specific capability proves that sessions can be independently cancelled, authenticated, closed, and resource-accounted without cross-session leakage.

Every sidecar:

- starts in the exact canonical worktree/cwd;
- receives `COMMANDBAR_SESSION_ID`/future Assembly compatibility identity;
- has bounded stderr capture;
- is assigned a process group and resource owner;
- is killed only through the runtime manager;
- never inherits arbitrary frontend-provided environment values;
- is surfaced in Resource Manager under its owned session.

### 4.5 Native CLI handoff

#### Structured to Terminal

1. Reject while an approval, structured question, or unsaved provider request is unresolved unless the user explicitly cancels it.
2. Interrupt or wait for the active turn.
3. Persist/reconcile the last canonical sequence and provider session ID.
4. Close/detach the ACP writer.
5. Launch the existing PTY in the same cwd with the provider's exact resume command.
6. Confirm the native CLI accepted the target session.
7. Commit owner `terminal`; otherwise stop the PTY and restore structured ownership.

#### Terminal to Structured

1. Require the native CLI process to be stopped or explicitly close it through existing terminal safety.
2. Incrementally import transcript changes produced by the CLI.
3. Start the pinned ACP adapter and load/resume the exact provider session.
4. Reconcile native item IDs with the canonical timeline.
5. Confirm provider session/config state.
6. Commit owner `structured`; otherwise leave the session terminal/stopped and preserve history.

No simultaneous native TUI and structured writer is allowed.

---

## 5. Canonical conversation and agent event model

### 5.1 Event envelope

Expand the existing generation/sequence protocol:

```rust
pub struct AgentEvent {
    pub owned_id: String,
    pub provider: AgentProvider,
    pub generation: u64,
    pub sequence: u64,
    pub timestamp_ms: u128,
    pub provider_session_id: Option<String>,
    pub turn_id: Option<String>,
    pub item_id: Option<String>,
    pub request_id: Option<String>,
    pub parent_item_id: Option<String>,
    pub kind: AgentEventKind,
    pub provider_meta: Option<serde_json::Value>,
}
```

`provider_meta` is bounded diagnostic/extension data. Svelte components never inspect raw Codex/Claude payloads to decide standard rendering.

### 5.2 Required event kinds

```text
session.started
session.loaded
session.state.changed
session.capabilities.updated
session.config-options.updated
session.config-value.updated
session.commands.updated
session.exited

turn.started
turn.completed
turn.interrupted
turn.failed

item.started
item.updated
item.completed
content.delta

plan.updated
task.started
task.progress
task.completed

request.approval.opened
request.approval.resolved
request.user-input.opened
request.user-input.resolved

children.updated
usage.updated
runtime.warning
runtime.error
```

### 5.3 Canonical item kinds

```text
user-message
assistant-message
reasoning
reasoning-summary
plan
command-execution
file-change
mcp-tool-call
dynamic-tool-call
web-search
image-view
image-generation
provider-subagent
assembly-agent
context-compaction
review
error
unknown
```

Each item has an explicit status: `queued`, `running`, `waiting`, `completed`, `failed`, `declined`, `cancelled`, or `unknown`.

### 5.4 Persistence and replay

Do not put the timeline or large tool output in `localStorage`.

Extend the existing app-managed append-only orchestration/event storage with a versioned agent-event stream, or add a sibling append-only JSONL ledger under the existing Application Support compatibility directory. Store large command output, images, and diffs as bounded app-managed artifacts referenced by ID/path. Session workspace snapshots retain only UI state and last acknowledged sequence.

Transcript files remain provider-native history and recovery evidence. They are not the only live event source for structured-owned sessions.

### 5.5 Incremental terminal transcript projection

Replace the current 500ms full-tail reread with a Rust-side incremental cursor:

```rust
pub struct TranscriptCursor {
    pub path: PathBuf,
    pub identity: TranscriptFileIdentity,
    pub offset: u64,
    pub partial_line: Vec<u8>,
}
```

It reads appended bytes, detects truncation/rotation, normalizes durable items, and emits canonical events. A bounded reconciliation snapshot repairs missed events. This adapter is used for terminal-owned sessions, imported history, and external/scanned sessions.

---

## 6. Configuration controls, not slash commands

### 6.1 Dynamic option contract

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'effort'
    | 'permission'
    | 'mode'
    | 'service-tier'
    | 'fast-mode'
    | 'boolean'
    | 'other';
  label: string;
  description: string | null;
  value: string | boolean | null;
  choices: AgentConfigChoice[];
  mutable: boolean;
  unavailableReason: string | null;
}
```

Known ACP semantic categories map to primary composer controls. Unknown options render under **More agent settings** without requiring a provider-specific Svelte component.

Primary composer row:

```text
[Provider] [Model ▾] [Effort ▾] [Permissions/Mode ▾] [Fast/Service tier] [More ▾]
```

The Provider selector is fixed for a running session unless the user starts/forks a new session. It is available in New Session and workflow role configuration.

### 6.2 Observed/pending/error state

A selection sends `session/set_config_option` or the provider-supported ACP operation. The UI shows `Applying…` until the provider emits the confirmed value. Rejection, timeout, session generation change, process exit, or mismatched confirmation restores the last observed value and shows the reason.

Never derive allowed model/effort combinations from hard-coded names. Preserve provider-advertised ordering and defaults.

### 6.3 Command taxonomy

```ts
export type AgentCommandSource = 'provider' | 'skill' | 'assembly';

export interface AgentCommandDescriptor {
  id: string;
  name: string;
  description: string;
  inputHint: string | null;
  source: AgentCommandSource;
  action: AgentCommandAction;
  availability: AgentCommandAvailability;
}
```

Keep:

- provider-advertised operations such as compact, review, goal, status, MCP/skills where supported;
- discovered skills;
- Assembly actions such as Open in Terminal, Return to Conversation, New Worktree, Open File, Session Library, Resources, Usage, and Workflow.

Omit native-TUI-only commands from the structured command palette:

- terminal theme/pets/keymap/vim/raw output/title/statusline;
- TUI-only quit/app/feedback/debug controls;
- commands already represented by model, effort, permissions, mode, or service-tier pickers.

Terminal-owned mode may expose **Open native command picker** rather than duplicating the TUI catalog.

---

## 7. Composer and screenshot attachments — top-priority vertical slice

### 7.1 One composer

Keep one `ConversationSurface` and one ownedId-keyed draft/attachment store. Split it into focused components, but do not create another conversation surface or store.

```text
components/conversation/
├── ConversationTimeline.svelte
├── ConversationComposer.svelte
├── AgentConfigBar.svelte
├── AgentCommandMenu.svelte
├── ConversationAttachmentStrip.svelte
├── TimelineItem.svelte
└── item renderers...
```

### 7.2 Paste behavior

On `paste`:

1. capture the active `ownedId` and runtime generation;
2. detect all supported image clipboard items;
3. display immediate local previews;
4. save each image through the existing validated Rust attachment vault;
5. replace temporary entries with canonical attachment IDs/paths;
6. discard the response if owner/generation changed;
7. preserve failed entries with a retry/remove action and plain reason.

Support PNG, JPEG, WebP, and GIF only when both MIME and decoded signature match. Enforce per-file, aggregate-size, and per-message-count limits. Managed files use owner-only permissions and opaque names. Blob URLs are never persisted.

### 7.3 Structured send

For ACP sessions, construct typed content blocks:

```text
text block(s)
image block(s), when provider advertises image input
resource/file link fallback, only when the provider supports it
```

Do not reduce images to prose paths when native image input is available. When a provider does not support images, block send for that attachment with a clear choice to remove it or Open in Terminal; never silently omit it.

### 7.4 Terminal-owned fallback

For the existing PTY writer, retain managed-file-path prompting with bracketed paste plus a separate Return. This fallback remains generation-guarded and clears attachments only after both writes succeed.

### 7.5 Browser capture attachment

Browser element grabs, annotations, and drawn screenshots use the same attachment contract. A browser feedback bundle may contain:

- screenshot/crop attachment;
- bounded element metadata JSON artifact;
- human note and Change/Question intent;
- source URL/title and capture generation.

Staging into the composer never auto-submits and never overwrites a nonempty draft without a merge preview.

---

## 8. Timeline, tools, plans, tasks, and sub-agents

### 8.1 Typed rendering

Assistant prose remains visually dominant. Tools are compact ordered rows that expand into details.

Required renderers:

- safe streaming Markdown with code-copy and file links;
- collapsible reasoning/reasoning summaries;
- command execution with command, cwd, duration, status, bounded stdout/stderr, and **Open terminal output**;
- file changes with exact changed paths and existing Monaco DiffEditor link;
- MCP/dynamic tool calls with input/output disclosure;
- plan steps with pending/running/completed state;
- task/TODO rows with progress and owner;
- approvals and structured questions inline at the item that requested them;
- web search/image items;
- context compaction and provider warnings;
- child/provider-subagent activity.

The chronological model preserves text/tool/reasoning ordering. Do not attach all tools to one assistant message after the fact.

### 8.2 Provider-internal children

`ProviderChildAgent` records provider-reported child identity, parent item, role/label, state, timestamps, and transcript/event availability.

Capabilities are honest:

- Inspect is always available when transcript/events exist.
- Cancel/steer is available only when that provider/ACP adapter exposes a correlated operation.
- No generic Kill button is invented from transcript metadata.

### 8.3 Assembly-managed agents

Assembly-managed workflow agents are separate `AgentRun` records with full lifecycle control. They may use Claude, Codex, or another ACP runtime independently of the parent provider. Their output is rendered in the workflow tree and can open in the standard conversation timeline.

---

## 9. Workflow and orchestration engine

### 9.1 Product boundary

ACP normalizes an individual provider session. Assembly owns multi-agent workflow semantics.

Create:

```text
tauri-svelte-preview/src-tauri/src/workflows/
├── mod.rs
├── definitions.rs
├── events.rs
├── reducer.rs
├── runner.rs
├── broker.rs
├── mcp_server.rs
├── budgets.rs
└── artifacts.rs

src/lib/shell/workflows/
├── workflowTypes.ts
├── workflowStore.svelte.ts
├── workflowService.ts
├── workflowViewModel.ts
├── workflowDefinitionValidation.ts
└── workflowTemplates.ts

src/lib/shell/components/workflows/
├── WorkflowWorkspace.svelte
├── WorkflowGraph.svelte
├── WorkflowRunTimeline.svelte
├── AgentRunTree.svelte
├── AgentRunInspector.svelte
├── WorkflowDefinitionEditor.svelte
└── WorkflowApprovalGate.svelte
```

Extend the existing append-only `orchestration.rs` authority or migrate its reducer behind this module. Do not create a second unrelated orchestration ledger.

### 9.2 Definitions

```ts
export interface WorkflowDefinition {
  id: string;
  version: number;
  name: string;
  description: string;
  entryNodeId: string;
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  defaultConcurrency: number;
  maxDurationMs: number;
  maxTotalAgents: number;
  maxDepth: number;
}

export type WorkflowNodeDefinition =
  | AgentNodeDefinition
  | ParallelNodeDefinition
  | ApprovalNodeDefinition
  | ReviewLoopNodeDefinition
  | DeterministicActionNodeDefinition
  | JoinNodeDefinition;
```

`AgentNodeDefinition` includes:

- role profile;
- provider runtime profile;
- model, effort, permission/mode, fast/service-tier config values;
- system/role instructions;
- task prompt template;
- cwd/worktree policy;
- allowed tools/MCP servers;
- token, turn, duration, and retry budgets;
- expected artifacts and completion contract.

### 9.3 Roles

Built-in templates are editable copies, never magic provider prompts:

- Orchestrator
- Implementer
- Code Reviewer
- Spec Compliance Reviewer
- Test/Verification Agent
- Security Reviewer
- Documentation Agent
- Release/PR Agent

A role may select a different provider/model than its parent. Unsupported option values fail validation before a run starts.

### 9.4 Bounded loops, not arbitrary hidden recursion

V1 supports DAG fan-out/fan-in plus explicit bounded `ReviewLoopNodeDefinition`:

```ts
export interface ReviewLoopNodeDefinition {
  id: string;
  implementerNodeId: string;
  reviewerNodeIds: string[];
  acceptancePolicy: 'all' | 'any' | 'named-gate';
  maxIterations: number;
  onExhausted: 'fail' | 'await-user' | 'continue-with-warning';
}
```

Each iteration is a visible event. No agent can spawn unbounded descendants. Global depth, child count, duration, turn, and token budgets are enforced by the broker.

### 9.5 App-owned MCP delegation tools

Expose an authenticated loopback MCP server only to Assembly-managed orchestrator sessions:

```text
assembly.spawn_agent
assembly.get_agent_status
assembly.send_agent_input
assembly.cancel_agent
assembly.list_agent_artifacts
assembly.publish_artifact
assembly.complete_task
assembly.request_approval
assembly.report_blocker
```

The tool schema accepts typed role/provider/config/worktree/task fields. It does not accept an arbitrary executable, shell command, environment map, or unbounded prompt body.

Flow:

```text
orchestrator tool call
  -> Assembly MCP companion
  -> WorkflowBroker validates workflow/node/budget/worktree policy
  -> AgentRuntimeManager starts child ACP session
  -> child events append to workflow ledger
  -> status/result correlates to parent tool-call ID
  -> parent receives bounded structured result plus artifact links
```

### 9.6 Worktree and file-lease policy

Parallel writers never share one uncontrolled working tree.

Policies:

- `read-only-current`: reviewers inspect the target worktree and cannot edit.
- `exclusive-current`: one writer owns the target worktree during the node.
- `new-worktree`: Assembly creates/uses an approved workflow worktree through the existing worktree service.
- `path-lease`: advanced mode only after deterministic non-overlapping path leases are proven.

Default workflow:

1. orchestrator plans without editing;
2. implementer writes in one exclusive worktree;
3. reviewers run read-only after implementation settles;
4. failed review returns bounded findings to implementer for the next visible iteration;
5. user approval gates any merge, push, PR mutation, deletion, or destructive action.

### 9.7 Run state

```text
queued
starting
running
waiting-for-child
waiting-for-input
waiting-for-approval
blocked
reviewing
retrying
completed
failed
cancelled
```

Every transition is append-only and includes workflow run, node, iteration, agent run, provider session, ownedId, worktree, timestamps, and bounded reason. Prose never determines state.

### 9.8 Controls

The Agent Operations workspace allows:

- pause scheduling;
- resume scheduling;
- cancel run;
- cancel/retry one Assembly-managed child;
- steer/send follow-up to a waiting child when supported;
- inspect live output and artifacts;
- open its worktree/session;
- approve/decline a gate;
- compare reviewer findings;
- promote a child to a normal persistent session;
- rerun from a safe checkpoint.

Provider-internal sub-agents appear in the same visual tree with a distinct **Provider child** label and only supported controls.

---

## 10. Shell layout, Paneviews, and session navigation

### 10.1 One layout system per purpose

Use the installed `dockview-core` package only:

- Center documents/workspaces: Dockview.
- Left/right/bottom vertical stacks: Dockview Paneview.
- Modals/popovers/hover cards/floating browser/full-screen overlays: overlay layer.

Create a generic side-region contract:

```ts
export type ShellPaneRegionId = 'left-work' | 'right-tools' | 'bottom-tools';

export interface ShellPaneRegistration {
  id: string;
  region: ShellPaneRegionId;
  title: string;
  minimumSize: number;
  defaultSize: number;
  collapsible: boolean;
  content: Snippet;
}
```

Create `ShellPaneviewHost.svelte` and `shell/layout/paneviewRegion.ts`. It captures/restores measured sizes, collapse state, ordering, and active/focused pane without destroying component instances.

### 10.2 Left work column

The left column becomes compact, project/worktree-first navigation inspired by the supplied Orca and ChatGPT references.

Structure:

```text
Projects / Workspaces
  repository
    worktree
      active agent/session rows

Working [Paneview]
Done    [Paneview]
```

Locked presentation:

- default width 248px; draggable range 216–320px;
- one-line compact rows, not full-width cards with permanent metadata blocks;
- primary row height 34–40px;
- status dot/icon, truncated title, provider/model hint, and relative activity;
- hover/focus card shows branch/worktree, model/effort, current task/plan step, PR/check state, ports, last activity, and exact path;
- selected row may open one accordion detail below it with actions and deterministic metadata;
- other rows remain compact and never shrink into unreadable slivers;
- Working and Done are independent Paneview panes with adjustable/collapsible size;
- Settled/archive may be a third pane only when the lifecycle package enables it;
- cards do not own backend IO.

### 10.3 Session Library moves out of the left work list

Add a center destination `sessionLibrary` with search/filter/grouping over resumable and archived provider sessions.

Features:

- provider, project/repository, worktree/cwd, branch, model, age, state, title/full-text search;
- compact list with hover details and an optional right inspection pane;
- Resume, Continue as New, Fork, Copy ID/command, Reveal Transcript;
- exact same-title disambiguation;
- no automatic resume on row expansion;
- virtualized/bounded results and explicit load-more;
- opening the library does not start a PTY, ACP adapter, LSP, browser, or Git scan.

The old `Find a session` drawer is removed from `SessionsColumn.svelte` after feature parity.

### 10.4 Right and bottom regions

Every tool section mounted inside the right or bottom region is a Paneview panel. This includes Problems, Source Control summaries, Worktrees, Context, Resources compact tree, Workflow inspector, and future panels. Pane headers own move/collapse/close actions; components retain their existing stores/services.

Do not convert menus, dialogs, status popovers, browser floating/full overlays, or the command palette into Paneview panels.

---

## 11. Browser: docked, floating, full-screen, collapsed

The existing master-plan native child-webview safety spike remains mandatory. This amendment expands the presentation and capture contract.

### 11.1 Presentation state

```ts
export type BrowserPresentationMode =
  | 'docked'
  | 'floating'
  | 'fullscreen'
  | 'collapsed';

export interface BrowserFloatingBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
```

- Docked: ordinary Browser center destination.
- Floating: one resizable overlay over the workbench, defaulting to roughly 72% width and 68% height within safe shell bounds, with persisted measured bounds.
- Fullscreen: fills the application content below native titlebar/window chrome and overlays left/right/center/bottom regions.
- Collapsed: browser child view is hidden; state remains warm and the global floating action island remains.

All four modes move/rebound the **same** native child view. URL, cookies/profile, forms, scroll, history, selected tab, viewport, captures, and annotations survive without reload.

### 11.2 Floating action island

Preserve the master plan's global bottom-right action surface, but make Browser behavior explicit:

- Browser action opens the last non-collapsed mode; first use opens Floating.
- Floating toolbar has Full Screen, Restore to Dock, and Collapse.
- Full Screen has Restore to Floating/Dock and Collapse.
- the island never covers the conversation composer or bottom Paneview strip;
- actions are context-aware and keyboard/VoiceOver accessible.

### 11.3 Browser toolbar

Match the evidenced toolbar behavior while using Assembly semantics:

- profile selector;
- back, forward, reload, address;
- Import entry point;
- **Grab page element**;
- **Annotate page element**;
- **Draw on screenshot**;
- development-only Devtools;
- Open in default browser;
- viewport/profile/settings overflow;
- Full Screen/Restore;
- Collapse.

Import Cookies remains disabled until a separate security decision defines the source, format, secret handling, and profile scope.

### 11.4 Grab page element

`Grab` arms the inspector, highlights one element, and produces:

- bounded selector/accessibility metadata;
- visible text snippet;
- element rectangle;
- screenshot crop when technically available;
- page URL/title and generation.

It opens a confirmation sheet with **Attach to conversation**, **Copy**, and **Cancel**. Attaching creates a browser-context attachment and never auto-sends.

### 11.5 Annotate page element

`Annotate` uses the same selection bridge, then opens a card anchored away from the selected element when possible. The card includes:

- human note;
- Change or Question intent;
- page/element summary;
- Add/Cancel;
- Cmd+Enter to add.

Saved annotations remain in a workspace/tab queue and can be edited, removed, copied, or staged into the exact owned conversation.

### 11.6 Draw on screenshot

1. capture the visible browser viewport through the approved native capture path;
2. temporarily hide/lower the native child view as needed;
3. open an HTML markup overlay with pen, arrow, rectangle, text, undo/redo, clear, crop, and cancel;
4. export a bounded PNG plus page metadata;
5. add it to the attachment queue or clipboard;
6. restore the same live browser view.

Drawing is not an arbitrary DOM/eval capability.

### 11.7 Feedback delivery

Browser feedback review shows exact queued items, screenshot thumbnails, note/intent, URL, and selected element. **Send to Conversation** performs an owner/generation check and stages typed attachments plus bounded Markdown into the existing composer. A nonempty draft opens Merge/Replace/Cancel; Replace is never the default.

---

## 12. Resource Manager and Workspace Space

### 12.1 Two-level product surface

Create two related but distinct surfaces over one native resource snapshot authority:

1. `ResourceManagerPopover.svelte`: compact status-bar/popover view.
2. `ResourceWorkspace.svelte`: full center destination.

The compact view groups:

- Assembly process totals;
- repositories/worktrees;
- owned sessions and ACP/native/tool terminals;
- language servers;
- browser views;
- ports;
- CPU/RSS sparklines;
- safe owned-process stop actions.

Use a bounded two-second refresh only while the resource popover or full workspace is visible. Hidden surfaces stop polling. Manual refresh remains available. Provider quota usage is not polled here.

### 12.2 Safety

A process is stoppable only when registry identity proves ownership and the backend revalidates PID, process group, generation, and owner immediately before stopping. External/unattributed processes are inspectable but never show Stop.

### 12.3 Workspace Space compact and full views

Compact footer inside Resource Manager:

- scanned disk bytes;
- reclaimable bytes;
- workspace count;
- last update;
- Scan/Cancel/Refresh;
- Review.

Review opens `WorkspaceSpaceWorkspace.svelte`, a full center destination with:

- treemap by worktree;
- selected-worktree top-level item breakdown;
- sortable/filterable table;
- active agents, terminals, dirty buffers, browser tabs, changed files, branch status, PR/issues, and protected reason;
- multi-select cleanup preview;
- exact safe-delete/archive authority from the existing worktree manager;
- primary checkout and active/dirty/unmerged/locked protections.

Disk scanning is cancellable and retains the last valid result while a refresh runs.

---

## 13. Usage popup and full analytics

Usage is a separate product module from Resource Manager.

### 13.1 Compact Usage popover

`UsagePopover.svelte` opens from a status-bar provider summary and includes:

- Detailed/Compact density switch;
- one row per enabled provider;
- provider icon, account/plan, reset countdown;
- all advertised rate-limit windows/buckets with used/remaining percentages;
- explicit refresh;
- Usage Details & History;
- Manage Accounts.

Quota/rate-limit values refresh only on open, explicit refresh, provider-auth change, or a known reset boundary. No background quota polling.

### 13.2 Full Usage workspace

Add center destination `usage`:

- agents spawned;
- time agents worked;
- workflows and child runs;
- PRs created/reviewed through Assembly;
- total/input/output/cache/reasoning tokens where known;
- estimated cost clearly labelled estimate;
- active days and session count;
- daily intensity heatmap;
- token mix;
- provider cards and provider-specific detail tabs;
- quota history and reset windows where the provider legally exposes them;
- unavailable/unsupported source explanations.

Data sources are separately labelled:

- live ACP usage events;
- durable transcript scan;
- provider quota/rate-limit endpoint/adapter;
- Assembly workflow ledger.

Never merge unknown values into a fabricated total.

---

## 14. AI assistance throughout the workbench

### 14.1 Shared assist registry

Extend Work Package 11 into a reusable assist system:

```ts
export interface AiAssistDescriptor {
  id: string;
  contexts: AiAssistContext[];
  title: string;
  inputSchema: AiAssistInputSchema;
  factSources: AgentFactSource[];
  outputKind: 'text' | 'field-patch' | 'action-proposal' | 'review';
  requiresPreview: boolean;
  requiresConfirmation: boolean;
}
```

One `AiAssistService` routes an assist to either:

- the current compatible structured session;
- a dedicated ephemeral ACP assistant role;
- a workflow role.

It does not create another permanent composer or conversation store.

### 14.2 Initial assist points

- PR: draft title/body, summarize changes, review code, explain failed checks, prepare review reply.
- Git: suggest commit message/branch name, summarize diff, prepare—but never auto-run—repair/reorganization.
- Run configurations: propose command/cwd/env from repository facts, then validate and save without auto-start.
- Browser: turn element grabs/annotations/drawings into a reviewed change request.
- Forms/settings: fill bounded fields from deterministic project facts and user text.
- Save-time checks: review a proposed record/config for omissions or contradictions.
- Workflows: generate a workflow draft/role configuration, then run schema/security/worktree validation before save.
- Sessions/worktrees: summarize stale work and propose safe next actions.

### 14.3 Save pipeline

AI never replaces deterministic validation.

```text
user edits
  -> deterministic schema/domain validation
  -> optional AI critique/suggestions
  -> user accepts/rejects field patches
  -> deterministic validation runs again
  -> typed native save/action confirmation
```

An AI warning cannot silently block a valid save unless the product field explicitly configures it as advisory-plus-required-user-acknowledgement. AI output is visibly generated and source-linked.

### 14.4 Security

Repository text, browser text, PR comments, tool output, and remote content are delimited untrusted context. Assist output cannot add shell arguments, environment values, provider endpoints, capabilities, or mutation targets outside typed allow-lists.

---

## 15. Revised work packages and parallel execution

### 15.1 Sequential contract wave

#### F0 — Runtime and ownership contract

Freeze:

- `AgentExecutionOwner` state machine;
- `AgentRuntimeManager` interfaces;
- canonical event/item/request/task/config contracts;
- sidecar manifest and packaging decision packet;
- native-terminal handoff protocol;
- resource ownership projection.

No provider/UI implementation starts before this contract passes fixture tests.

#### F1 — Shell Paneview and destination contracts

Freeze:

- center roster additions: `sessionLibrary`, `agents`, `resources`, `workspaceSpace`, `usage`;
- left/right/bottom Paneview registrations and persistence;
- overlay ownership for Browser Floating/Fullscreen;
- global action island context contract.

#### F2 — Workflow domain contract

Freeze workflow definition, role, node, run, event, broker, budget, worktree policy, and MCP tool schemas.

### 15.2 Parallel foundation lanes after F0–F2

| Lane | Scope | Owned paths | Heavy runner |
| --- | --- | --- | --- |
| A ACP runtime | Rust ACP client, sidecar supervision, provider catalog, process identity | `src-tauri/src/agent_conversation/runtime/**` | focused Rust |
| B Canonical conversation | Rust/TS event types, reducer, replay fixtures, transcript incremental cursor | existing `agent_conversation/**`, `shell/conversation/**` excluding UI files leased to C | focused Rust/Node |
| C Composer/timeline | typed timeline renderers, config bar, command menu, screenshot UI using mock runtime | `components/conversation/**`, `ConversationSurface.svelte` | Node |
| D Attachment vault | image validation, restore/delete/prune, ACP content-block mapping fixtures | `agent_conversation/attachments.rs`, attachment TS helpers/tests | focused Rust/Node |
| E Paneview shell | side-region Paneview host, compact work column, Session Library scaffolding | layout/session components and tests named in F1 packet | Node |
| F Browser native spike | child-view clipping, same-view teleport, capture/inspector feasibility | browser Rust/bridge spike only | native/Rust |
| G Resource/usage models | native snapshot adapters, usage source contracts, pure view models | resources/usage modules | focused Rust/Node |
| H Workflow core | definitions, reducer, event log, budgets, mock broker/MCP contract | workflows Rust/TS domain modules | focused Rust/Node |

At most two heavy lanes run, default one. The controller integrates shared seams serially and performs a SOL review before provider adapters are enabled.

### 15.3 Parallel feature lanes after foundation integration

| Lane | Scope | Dependency |
| --- | --- | --- |
| P1 Codex ACP | pinned sidecar, auth/session/config/tools/plans/subagents/images | A+B+D |
| P2 Claude ACP | pinned Node/adapter packaging, config/tools/tasks/nested children/images | A+B+D plus packaging proof |
| P3 Conversation UX | real runtime binding, model/effort/permission controls, screenshot acceptance | C + P1/P2 contracts |
| P4 Agent workflows | MCP broker, child ACP runs, review loop, controls, workflow UI | H + P1/P2 |
| P5 Sessions/navigation | compact worktree/session rail, hover cards, Paneview Working/Done, Session Library | E + runtime facts |
| P6 Browser product | Docked/Floating/Fullscreen/Collapsed, grab/annotate/draw, attachment delivery | F + D + P3 draft contract |
| P7 Resources/Space | compact/full process manager and workspace-space treemap | G + E |
| P8 Usage | compact roster and full analytics workspace | G + E + provider usage events |
| P9 AI assist | PR/forms/save/workflow/browser assistants and proposal registry | P3 + deterministic service contracts |

### 15.4 Serialized integration wave

The controller alone edits:

- `src/routes/next/+page.svelte`
- `ShellFrame.svelte`
- center Dockview roster
- root overlay host
- shared Paneview roster
- `src-tauri/src/main.rs`
- Cargo/package manifests and lockfiles
- Tauri capabilities/config
- shared settings schema
- command/palette registry

Integration order:

1. Paneview/center/overlay hosts.
2. Runtime manager and one mock ACP session.
3. Codex and Claude sidecars.
4. composer/config/image acceptance.
5. terminal handoff.
6. workflow broker and Agent Operations workspace.
7. browser capture delivery.
8. resources and usage.
9. AI assist points.
10. full native acceptance and milestone SOL review.

---

## 16. Focused verification

Add or extend tests for:

### Runtime

- one writer per ownedId;
- handoff rollback and stale-generation rejection;
- sidecar process-group cleanup;
- config discovery/confirmation/mismatch/timeout;
- ACP reconnect/snapshot recovery;
- transcript incremental append, rotation, truncation, and deduplication;
- tool-terminal identity separate from native CLI;
- future-provider generic rendering.

### Composer

- multi-image paste before/after owner switch;
- native image content blocks for both providers;
- unsupported image refusal;
- exact retry/remove/cleanup;
- restart preview recreation;
- no duplicate send;
- browser capture attachment merge.

### Timeline

- interleaved text/reasoning/tool/file/plan/task ordering;
- streaming partial Markdown/code fences;
- approval and user-input correlation;
- command output bounds;
- subagent parent linkage;
- raw provider fallback item.

### Workflow

- sequential and parallel nodes;
- fan-in ordering;
- bounded review loop;
- provider/model/mode validation;
- depth/agent/token/turn/duration/concurrency budgets;
- parent cancellation during child spawn/send/run;
- worktree exclusivity/read-only enforcement;
- approval gates;
- retry and stale result;
- restart/replay;
- cross-provider Claude orchestrator -> Codex implementer -> Claude reviewer and the reverse;
- no arbitrary shell/environment injection.

### Layout and navigation

- Paneview measured-size persistence;
- Working/Done independent collapse/resize;
- compact rows and hover details;
- Session Library no-start behavior;
- all side/bottom panels registered through Paneview;
- overlays remain above child webviews;
- per-session workspace restoration.

### Browser

- same native tab through four presentation modes without reload;
- element grab metadata/crop;
- annotation queue;
- screenshot draw export;
- stale workspace/tab/generation rejection;
- profile isolation;
- dialog/input z-order;
- staged-not-sent conversation delivery.

### Resources/Usage

- process ownership and external stop refusal;
- visible-only resource polling cleanup;
- disk scan cancellation and last-result retention;
- worktree deletion protections;
- provider quota source separation;
- no fabricated aggregate when a provider is unknown;
- usage analytics replay and provider filters.

### AI assistance

- deterministic validation before and after AI patch;
- hostile remote context remains data;
- preview/confirmation and cancel-no-mutation;
- stale fact refusal;
- field patch bounds;
- no auto-save, auto-send, auto-PR, auto-review submission, or destructive action.

---

## 17. Native acceptance scenarios

One recorded acceptance pass in the rebuilt Tauri app must prove all of the following.

### 17.1 Structured conversations

1. Start a new Codex structured session without opening an xterm.
2. Select an advertised model, effort, permission mode, and fast/service tier; values confirm from the runtime.
3. Paste two screenshots, remove one, send the other as an image content block, and see it in restored history.
4. Observe assistant prose, reasoning, command execution/output, file edits/diff, plan/tasks, approvals, and completion.
5. Repeat with Claude, including a permission request, TODO/task update, and nested provider child.
6. Switch sessions repeatedly while both continue; no output or controls cross ownedId.

### 17.2 Native CLI handoff

1. From an idle structured Codex session choose Open in Terminal.
2. Confirm the same native thread resumes in the existing terminal surface and only one writer exists.
3. Add a turn, stop the CLI, return to Structured, and see the imported turn exactly once.
4. Repeat with Claude.
5. Force a handoff failure and prove rollback leaves one usable owner and all history intact.

### 17.3 Workflow

Run a fixture workflow:

```text
Orchestrator
  -> Implementer
  -> parallel Code Review + Spec Compliance + Tests
  -> bounded revision loop when one reviewer fails
  -> user approval gate
  -> prepare PR title/body, but do not publish
```

Use at least two different providers across roles. Prove live graph state, child conversations, artifacts, cancel/retry, budgets, worktree safety, app restart/replay, and no uncontrolled child recursion.

### 17.4 Browser

1. Open the browser from the bottom action island into Floating mode.
2. move/resize it, switch to Full Screen, then restore to Docked without reloading the page;
3. Grab a real page element and attach its crop/metadata;
4. Annotate another element with Change intent;
5. Draw on a viewport screenshot;
6. stage all three into the exact conversation without auto-send;
7. prove workspace/profile isolation and Settings/confirmation dialogs above the native child view.

### 17.5 Resources and Usage

1. Open compact Resource Manager and inspect grouped app/worktree/session/ACP/LSP/browser processes.
2. attempt to stop an external process and confirm no action is offered;
3. open the full resource workspace and Workspace Space treemap/table;
4. cancel and rerun a disk scan; safely review a deletable disposable worktree;
5. open compact Usage with Detailed/Compact modes;
6. open full Usage analytics and verify provider-source labels, workflow/agent counts, token mix, and unknown-value behavior.

### 17.6 Navigation and AI assistance

1. Resize/collapse Working and Done Paneviews independently.
2. Hover compact session rows for facts and expand one selected detail.
3. Open Session Library and inspect/resume the correct same-title session without another session starting on row expansion.
4. Use AI to draft PR text, generate a run configuration, fill a bounded form, and critique a save.
5. Cancel each proposal and prove zero mutation; accept one safe fixture patch and prove deterministic validation reran before save.

---

## 18. Stop conditions

Stop and return a decision packet before implementation when:

- the selected ACP adapter cannot prove the required provider capability;
- Claude sidecar packaging cannot be made reproducible without an unapproved runtime dependency;
- a provider session cannot be loaded/resumed safely after native CLI handoff;
- child-webview z-order prevents HTML overlays/dialogs from receiving input;
- a new provider requires provider-specific UI for a capability that should be canonical;
- workflow parallel writers cannot be isolated through existing worktree authority;
- an action requires arbitrary shell/environment execution outside Run Configurations;
- a side panel cannot be represented with Dockview Paneview without destroying its existing service/store;
- a source cannot distinguish quota, token usage, and estimated cost honestly;
- an AI action cannot be expressed as a bounded previewable typed proposal;
- shared-seam WIP or a changed repository baseline makes path ownership ambiguous.

Do not resolve a stop condition by guessing, wiring the old duplicate provider path, scraping terminal pixels, hard-coding model catalogs, or silently weakening safety.

---

## 19. Completion definition

This amendment is complete only when:

- ACP-backed Codex and Claude conversations are the default and show the same canonical interaction model;
- no ACP session automatically creates a visible native terminal, but typed command/tool terminals render correctly;
- model, effort, permissions/mode, fast/service tier, and provider options are real dynamic controls;
- screenshot paste works end-to-end for both providers;
- tools, plans, tasks, approvals, structured questions, and provider children are first-class timeline items;
- Assembly can create and control cross-provider workflow agents with roles, parallelism, bounded loops, gates, budgets, artifacts, and replay;
- native CLI and structured mode hand off one writer safely;
- compact session navigation, Session Library, and all side/bottom Paneviews are proven;
- Browser Docked/Floating/Fullscreen/Collapsed modes, element grab, annotation, drawing, and conversation staging are proven;
- compact/full Resources, Workspace Space, and Usage surfaces are proven;
- AI assists exist throughout the workbench only as validated, previewed, auditable proposals;
- existing editor/Roslyn, Git, worktree, browser-profile, terminal, workspace snapshot, and safety contracts remain green.
