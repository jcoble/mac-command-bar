# Assembly ACP Runtime, Workflows, and Workbench Shell Amendment

**Date:** 2026-08-04  
**Status:** Planning authority for the product areas named below; implementation has not started.  
**Repository baseline:** `jcoble/mac-command-bar` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`.  
**Applies to:** TSK-808, TSK-809, TSK-810, the integrated-agent portions of TSK-766, and the browser/resource/session-shell requirements clarified on 2026-08-04.

> For implementation agents: retain every execution, worktree, model-routing, focused-test, native-proof, security, SQL, and cleanup rule from
> `2026-08-01-tsk-808-native-workbench-product-wave.md` unless this amendment explicitly replaces a product or architecture decision.
> This amendment narrows ambiguities; it does not authorize implementation before the controller refreshes the repository and writes exact task packets.

---

## 1. Plan authority and superseded decisions

This document amends both:

1. `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; and
2. `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`.

The older conversation plan remains historical discovery evidence. The master product-wave plan remains authoritative for all unaffected work packages. For conflicts in the following areas, this amendment wins:

- Work Package 2 shell composition, side panes, sessions rail, and resumable-session discovery;
- Work Package 3 session presentation;
- Work Package 8 browser presentation, capture, annotation, and conversation delivery;
- Work Package 9 resources, disk space, provider usage, and compact/full presentation;
- Work Package 10A conversation runtime, controls, attachments, tool rendering, plans, and child agents;
- Work Package 11 integrated AI and multi-agent workflow orchestration.

The most important replacement is this:

> The PTY is no longer the permanent sole writer for every future Assembly-created Claude or Codex conversation. Assembly enforces **one writer at a time**, and that writer may be either the structured ACP runtime or the existing native CLI/PT​​Y runtime. New Assembly-created agent conversations default to structured ownership. Existing, externally started, imported, or explicitly terminal-owned sessions continue to work through the current PTY plus durable-transcript projection.

The application must never run the structured writer and native CLI writer against the same provider-native conversation simultaneously.

---

## 2. Locked product decisions

1. **Model, effort, permissions, agent mode, fast/service tier, and other provider configuration are controls, not slash commands.** They appear as native selectors in the composer and Settings, populated from the active runtime’s advertised options.
2. **TUI-only commands are not copied into Assembly’s slash menu.** A command that only changes the Codex or Claude terminal UI—theme, keymap, Vim mode, terminal title, terminal pet, raw scrollback, or similar—is omitted unless Assembly has its own meaningful equivalent.
3. **Screenshot paste is the first conversation milestone.** Pasting one or more images into the composer must create previews, preserve order, survive a session switch, and send exact image content to the same structured provider session without a terminal detour.
4. **Tool calls, reasoning, plans, task/TODO progress, approvals, structured questions, command output, file changes, web searches, and subagents are first-class timeline items.** They are never flattened into one assistant Markdown blob.
5. **ACP is the common agent transport, not the workflow engine and not the terminal emulator.** Assembly owns workflows, roles, DAG/loop state, delegation, persistence, safety policy, and UI. Provider adapters own communication with Claude, Codex, and future agents.
6. **The existing terminal system remains a product capability.** A user can create a plain terminal, start a native CLI session, or hand a structured conversation to the native CLI. Raw terminal mode is not silently started merely because the user opens the Session tab.
7. **Subagents have two origins and one presentation model:** provider-native children reported by Claude/Codex, and Assembly-spawned cross-provider workflow children. Both appear in one parent-scoped agent tree, but their ownership and control capabilities remain explicit.
8. **The browser has four presentation states:** docked, floating, expanded over the whole workbench, and collapsed to the global bottom-right control. The same live browser tab/webview moves between states; no state transition creates a second page or reloads the page.
9. **Browser element feedback is a conversation attachment.** Grab element, annotate element, and draw on screenshot produce immutable, bounded context items that the user reviews before adding to a draft. Nothing auto-submits.
10. **Resources and provider usage each have compact and full surfaces.** The compact status/FAB surface answers “what is consuming resources or quota now?” The full center workspaces provide process trees, disk/worktree analysis, usage history, and safe actions.
11. **The left side is “My Work,” not the machine-wide session archive.** It shows active workspaces/worktrees and their owned sessions in compact rows. Resumable historical sessions move to a separate Session Library surface.
12. **Every stack of panels inside the left or right regions uses Dockview Paneview.** Center destinations use Dockview. A center destination that contains multiple independently resizable sections also uses Paneview internally. Custom flex-column pseudo-panes are not accepted.
13. **AI is integrated through one contribution and action framework.** PR drafting, review assistance, check repair, browser feedback, form completion, save preflight, run-configuration generation, and worktree cleanup all use the same deterministic-facts → AI proposal → preview/revalidation → typed execution boundary.
14. **Other providers are capability-gated.** Supporting a new provider requires an ACP adapter or an equivalent adapter implementing the same Assembly runtime interface. The UI does not claim images, model switching, approvals, tools, plans, or resume support until that runtime advertises them.

---

## 3. What an ACP session is—and is not

### 3.1 Structured ACP session

For Assembly, a structured ACP session consists of:

```text
Assembly Rust AgentRuntimeManager
        │
        ├── ACP client connection over stdio
        │       └── pinned provider adapter process
        │               ├── Codex ACP → Codex app-server → Codex thread
        │               └── Claude ACP → Claude Agent SDK → Claude session
        │
        ├── normalized AgentEvent stream
        ├── provider-native session/thread ID
        ├── advertised capabilities/configuration
        └── no xterm/PT​​Y unless a tool terminal is explicitly presented
```

Starting a structured session launches an adapter/provider process and creates or resumes a provider-native conversation. It **does not create an Assembly terminal tab or xterm instance**.

An agent may run shell commands. ACP can report those command/tool events and may expose interactive or background terminal handles depending on the adapter. Those are **tool terminals owned by the structured runtime**, not the same thing as opening the provider’s full native TUI.

### 3.2 Terminal-owned session

The current terminal path remains:

```text
Assembly TerminalRegistry
        └── login shell PTY
                └── interactive `claude`, `codex`, or another CLI
                        └── durable transcript projected into ConversationTimeline
```

The PTY remains authoritative while terminal-owned. Assembly may render its transcript, but controls that require a structured request are disabled or handed to the terminal UI honestly.

### 3.3 Ownership handoff

Each owned session has one execution owner:

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';
```

Handoff rules:

- **Open in native CLI:** finish or interrupt the active structured turn, persist/reconcile events, close or detach the structured writer, then launch the native CLI with the exact provider-native session ID.
- **Return to Conversation:** stop/detach the native CLI, import transcript changes, resume/load the same provider-native conversation through ACP, reconcile by provider item identity, then acquire structured ownership.
- **Fork to terminal:** create a provider-native fork when supported, or create a new terminal-owned continuation with an explicit relationship record. The original structured session remains untouched.
- A failed handoff restores the prior owner and its usable UI. `transitioning-*` never lasts across an application restart without a recovery receipt.
- The baseline implementation is stop/resume ownership transfer. A future shared Codex app-server/remote-TUI optimization is allowed only after a separate native proof shows one thread, no duplicate writers, and exact history/settings reconciliation.

---

## 4. Provider runtime architecture

### 4.1 Rust module layout

Refactor the existing `src-tauri/src/agent_conversation` module into one runtime boundary rather than adding another parallel subsystem:

```text
ta​​uri-svelte-preview/src-tauri/src/agent_conversation/
├── mod.rs
├── manager.rs
├── runtime.rs
├── capabilities.rs
├── protocol.rs
├── event_buffer.rs
├── ownership.rs
├── handoff.rs
├── attachments.rs
├── providers/
│   ├── mod.rs
│   ├── acp_client.rs
│   ├── process_supervisor.rs
│   ├── codex.rs
│   └── claude.rs
└── transcript/
    ├── mod.rs
    ├── watcher.rs
    ├── codex.rs
    └── claude.rs
```

Do not create a second Tauri state object for conversations. Evolve the existing `AgentConversationRegistry` into `AgentRuntimeManager`, keyed by `ownedId`.

### 4.2 Runtime interface

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

The concrete ACP runtime uses the official Rust ACP SDK. Provider adapters are pinned and version-reported. Do not invoke an unbounded `npx ...@latest` when a session starts.

### 4.3 Provider packaging

- **Codex:** package a pinned `codex-acp` macOS sidecar. It already maps Codex app-server models, reasoning effort, fast mode, approval/sandbox, images, commands, file changes, permission requests, MCP tools, terminal output, reasoning, plans, web search, reviews, token usage, and subagents.
- **Claude:** package or launch a pinned `claude-agent-acp` build backed by the official Claude Agent SDK. The implementation packet must decide and prove the Node 22/runtime distribution strategy before product acceptance.
- **Future providers:** register an adapter descriptor containing executable, version probe, auth methods, capabilities, and migration policy. No provider-specific logic enters Svelte components.

### 4.4 Capability model

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

Capabilities are observed runtime facts. Missing or unknown means unavailable, not “probably supported.”

---

## 5. Native configuration controls, not slash commands

### 5.1 Config option contract

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

1. Values come from ACP session configuration updates or provider discovery, never a hard-coded model list.
2. Selecting a value calls `session/set_config_option` or the adapter’s equivalent.
3. The control displays `Applying…` until the provider confirms the effective value.
4. If the selected model does not support the prior effort, use the provider-advertised fallback/default and explain the change.
5. Unknown config options appear under **More**, preserving future compatibility without provider-specific UI code.
6. Terminal-owned sessions may expose read-only observed metadata plus **Open native picker** when the PTY can safely handle it. They do not pretend to offer the same direct mutation path.

### 5.2 Command catalog

The command menu merges only:

- provider-advertised commands that are meaningful through the active structured runtime;
- discovered skills/prompts;
- Assembly-local workflow/navigation actions.

Examples that belong in the menu when supported:

```text
/review
/compact
/status
/mcp
/skills
/goal
/plan
/new-workflow
/open-terminal
```

Examples omitted as provider-TUI-only:

```text
/theme
/pets
/vim
/keymap
/title
/statusline
/raw
```

`/model`, `/permissions`, `/agent`, and effort/service-tier choices are not primary slash commands in Assembly. Their native controls may expose aliases for keyboard users, but selecting the alias focuses the control; it does not send text to the agent.

No catalog selection auto-submits. Every item declares one exact action kind:

```ts
export type ConversationCommandAction =
  | { kind: 'provider-command'; name: string }
  | { kind: 'focus-config'; optionId: string }
  | { kind: 'assembly-action'; actionId: string }
  | { kind: 'insert-prompt'; text: string }
  | { kind: 'insert-skill'; invocation: string };
```

---

## 6. Canonical conversation and task timeline

### 6.1 Ordered item model

Replace the message-only transcript shape with an ordered item timeline:

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
  | ContextCompactionItem
  | WarningItem
  | ErrorItem;
```

Every item includes stable app ID, provider item ID, turn ID, parent item ID when nested, lifecycle status, timestamps, and optional provider metadata. Text, reasoning, command output, plan output, and file-change output stream as distinct channels.

### 6.2 Canonical events

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

Keep the repository’s existing `ownedId`, generation, monotonic sequence, stale-generation rejection, and snapshot resynchronization rules. Persist a bounded raw provider frame only for diagnostics; the UI and workflow engine consume canonical events.

### 6.3 Terminal transcript projection

The current JSONL parser becomes `TerminalTranscriptProjection`, implementing the same event output with explicitly limited capabilities.

Replace the 500 ms full-tail frontend poll with a Rust incremental watcher:

- cache canonical transcript path;
- retain file identity, byte offset, and partial line;
- parse only appended records;
- detect rotation/truncation;
- emit normalized events;
- occasionally reconcile with a bounded snapshot;
- never scrape terminal pixels.

This projection supports imported/external/native-CLI sessions and handoff reconciliation. It is not the primary runtime for newly created structured sessions.

---

## 7. Screenshot and image attachments—priority zero

### 7.1 Composer behavior

1. `Cmd+V` with one or more clipboard images prevents text paste only for the consumed image files.
2. Each image appears immediately as an ordered removable preview.
3. The draft, image order, and managed attachment IDs are scoped by `ownedId` and generation.
4. A failed image write leaves the text draft and all previously saved attachments intact.
5. Send is disabled while any image is still validating/writing.
6. Images survive switching sessions and application restart when their managed file still exists.
7. Removing a preview deletes only that owned managed file after canonical containment validation.
8. A successful send clears previews only after the provider accepts the prompt request.

### 7.2 Structured ACP send

When `capabilities.images` is true, send each image as an ACP image content block in the same prompt, preserving order with text and embedded browser context.

When a provider supports resource/file links but not direct image blocks, Assembly may send an app-managed canonical file resource only after explicit capability negotiation. Otherwise the UI explains that the provider does not accept images; it does not silently convert the screenshot into a path-shaped text prompt.

### 7.3 Limits and safety

Retain and extend the existing Rust vault:

- PNG, JPEG, WebP, and GIF only when the chosen provider supports that type;
- byte-signature verification;
- per-file, per-draft aggregate, and per-session retained limits;
- opaque file names;
- owner-only permissions;
- no symlink traversal;
- canonical owned-session root containment;
- no user-owned file deletion;
- no blob URL persistence;
- explicit pruning receipts.

### 7.4 Browser/drawing integration

Browser element captures and drawn screenshots use the same attachment pipeline. They carry a separate bounded context record beside the image so the model receives both visual evidence and exact page/element metadata.

---

## 8. Tool calls, plans, tasks, approvals, and child agents

### 8.1 Timeline presentation

- Assistant prose remains the dominant reading surface.
- Reasoning is a collapsible row, open while streaming and collapsed after completion unless the user leaves it open.
- Commands/tools are compact status rows with expandable input, output, duration, cwd, exit code, and file locations.
- File-change items show file count and line totals and open the existing Diff workspace.
- Plans render steps with pending/in-progress/completed/failed states and an explicit **Implement**, **Revise**, or **Dismiss** action only when the runtime advertises it.
- Provider TODO/task updates are first-class `TaskListItem`s, not inferred from Markdown checkboxes.
- Approval and structured-input requests appear inline with only the choices provided by the runtime.
- A request is rejected after generation change, expiry, turn completion, or owner handoff.

### 8.2 Unified agent tree

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

Provider-native children are controlled only through provider-supported actions. Imported transcript children remain read-only. Assembly-spawned workflow children can be paused/cancelled/retried through the workflow engine because Assembly owns those ACP sessions.

The UI clearly differentiates **Open transcript** from **Cancel Assembly-owned task**. It never presents a kill button for a child that the app does not own.

---

## 9. Multi-agent workflow and orchestration system

ACP supplies sessions and events. Assembly supplies orchestration.

### 9.1 Two orchestration modes

#### Deterministic workflow mode

Assembly executes a versioned DAG/loop definition. The model does not decide what node runs next unless the definition contains an explicit AI decision node.

Use this for repeatable flows such as:

```text
Spec check
   ↓
Implementation ───────┐
   ↓                  │
Tests                 │ parallel
   ↓                  │
Code review ──────────┘
   ↓
Spec compliance
   ↓
Fix loop (max 2)
   ↓
Human approval
```

#### Agent-directed delegation mode

An orchestrator agent can call an Assembly-provided MCP delegation service. The service validates a role and task, creates a fresh child ACP session, links it to the parent tool call/workflow run, and returns a structured task handle/result.

This follows the proven broker pattern:

```text
parent agent
  └── MCP tool: delegate_to_role
        └── Assembly DelegationBroker
              └── AgentRuntimeManager.spawn(role configuration)
                    └── child ACP session
                          └── canonical events + terminal result
```

The baseline tools are:

```text
delegate_to_role
get_delegation_status
cancel_delegation
continue_delegation       # only after v1 one-shot behavior is proven
close_delegation
```

A parent cannot provide an arbitrary executable, adapter path, environment dump, permission bypass, or unrestricted working directory. It selects an allow-listed role and passes bounded task text/context.

### 9.2 Workflow types

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

### 9.3 Runtime state

```ts
export interface WorkflowRun {
  id: string;
  definitionId: string;
  definitionVersion: number;
  ownedWorkspaceId: string;
  rootOwnedSessionId: string | null;
  status: 'queued' | 'running' | 'waiting' | 'paused' | 'completed' | 'failed' | 'cancelled';
  currentNodeIds: string[];
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
  steps: WorkflowStepRun[];
  agents: WorkflowAgentRun[];
  artifacts: WorkflowArtifact[];
  decisions: WorkflowDecision[];
}
```

Every transition appends an event to the existing orchestration ledger. Do not create a second unrelated run log. Extend the current append-only event schema additively and reconstruct current state through a pure reducer.

Events include:

```text
workflow.created
workflow.started
workflow.paused
workflow.resumed
workflow.completed
workflow.failed
step.queued
step.started
step.waiting
step.completed
step.failed
step.retry-scheduled
agent.spawn-requested
agent.started
agent.progress
agent.waiting
agent.completed
agent.failed
agent.cancelled
approval.requested
approval.resolved
artifact.recorded
decision.recorded
```

### 9.4 Workflow Center UI

Register a permanent **Workflows** center destination. Its internal composition uses Paneview:

- **Run list/definitions** pane;
- **Workflow graph and active loop** pane;
- **Agent tree** pane;
- **Selected step/agent transcript and artifacts** pane;
- **Decision/approval queue** pane.

The user can:

- create and clone a workflow definition;
- choose providers/models/effort/modes per role from runtime capability catalogs;
- start, pause, resume, cancel, and retry an owned run;
- open any child transcript;
- see which loop iteration, node, barrier, or approval is active;
- inspect exact input context, output, artifacts, and failure reason;
- intervene by revising a pending task or approving a gate;
- save a successful run as a reusable workflow template.

No UI state is inferred from agent prose. “Implementing,” “Reviewing,” and “Waiting for approval” come from workflow/runtime events.

### 9.5 Example built-in templates

Ship templates only after their contracts pass fixtures:

1. **Implement → Test → Review → Spec compliance → Fix loop**
2. **Investigate bug → reproduce → root cause review → implement → regression test**
3. **Draft PR → review diff → run checks → prepare PR text → human publish**
4. **UI audit → browser annotations → implement → visual review**

Templates are editable copies. They never silently publish, merge, delete worktrees, or bypass approvals.

---

## 10. Browser—Orca-inspired, Assembly-owned implementation

The existing master plan’s native child-webview security and profile isolation remain in force. This amendment adds the precise interaction and state model evidenced by Orca.

### 10.1 Presentation state

```ts
export type BrowserPresentationMode =
  | 'docked'
  | 'floating'
  | 'expanded'
  | 'collapsed';
```

- **Docked:** normal Browser center destination.
- **Floating:** a resizable/movable overlay larger than the docked pane, preserving the rest of the workbench behind it. The same native page is teleported to measured floating bounds.
- **Expanded:** fills the entire Assembly content area under macOS title-bar chrome and overlays sessions, editor, and side panes.
- **Collapsed:** hides browser chrome/webview while retaining tabs/profile/navigation/annotations; the global bottom-right control remains.

Transitions hide the child view before moving it, measure final bounds, then reveal the same page. URL, history, cookies, forms, scroll position, viewport preset, selected tab, and annotations survive.

### 10.2 Toolbar actions

The toolbar has, capability permitting:

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
Overflow / browser settings
Float / Restore / Expand
Collapse
```

“Import cookies” remains disabled until a separate security design defines source, format, scope, secret handling, and deletion.

### 10.3 Three capture modes

#### Grab page element

Captures bounded DOM/accessibility metadata plus an optional exact element screenshot. Intended for quickly attaching context without adding a note.

#### Annotate page element

Highlights a selected element and opens a card containing:

- page title and URL;
- accessible name;
- tag/role;
- bounded selector;
- bounded text snippet;
- note;
- `Change` or `Question` intent.

Add queues the item; it does not send.

#### Draw on screenshot

Captures the visible page viewport, opens a markup overlay with pen/shape/arrow/text/undo/clear tools, and saves the marked image through the conversation attachment vault. The image retains page URL, viewport, and capture timestamp metadata.

### 10.4 Conversation delivery

A browser context attachment is immutable:

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
  accessibleName: string | null;
  textSnippet: string | null;
  note: string | null;
  intent: 'change' | 'question' | null;
  imageAttachmentId: string | null;
  sourceHash: string;
  createdAt: string;
}
```

The feedback panel supports edit, remove, Copy All, Add to Conversation, and Ask Workflow. Adding to Conversation targets the exact captured `ownedId` and generation, opens a merge preview when the draft is nonempty, and never auto-submits.

Ask Workflow can create a workflow input artifact or start the built-in UI-audit flow only after the user reviews the generated workflow request.

### 10.5 Reference implementations to study, not copy blindly

Inspect these Orca modules during task-packet preparation:

- `stablyai/orca/src/renderer/src/components/browser-pane/BrowserPane.tsx`;
- its grab/annotation/markup helpers and persistent webview registry;
- its toolbar/profile/viewport components.

Reuse product patterns only after license and dependency review. Assembly remains Tauri/Svelte and uses its existing browser, attachment, conversation, and overlay boundaries.

---

## 11. Resources, disk space, and provider usage

### 11.1 Compact Resource Manager

A status-bar segment and global-action entry open a compact popover containing:

- total CPU and RSS;
- expandable repository/worktree/session process tree;
- owned terminal and language-server rows;
- listening ports;
- sparklines from bounded recent samples;
- safe stop action only for registry-owned processes;
- inactive workspace count;
- compact **Space** section with scanned/reclaimable bytes and Scan/Review actions.

The compact surface never becomes the sole place to understand or manage resources.

### 11.2 Full Resource workspace

Register **Resources** as a center destination using Paneview:

- process/resource tree;
- selected process/session details;
- language-server policy and logs;
- ports;
- workspace disk usage;
- provider quota summary.

The existing resource ownership and process-safety rules from Work Package 9 remain unchanged.

### 11.3 Full Space workspace

The disk analysis view includes:

- scanned bytes;
- reclaimable bytes;
- workspace count;
- updated time;
- treemap by worktree/top-level folder;
- selected workspace item breakdown;
- searchable/sortable table;
- active-agent, terminal, dirty buffer, browser tab, Git, PR, and checkpoint facts;
- selection plus confirmation-gated cleanup.

A worktree with active agents, live terminals, dirty editor buffers, uncommitted work, or unknown ownership is protected or requires the existing exact confirmation policy. The Space view delegates deletion to the one worktree-safety service.

### 11.4 Compact Usage popup

The status bar opens an Orca-like provider roster:

- provider icon and name;
- plan/account label when available;
- each active quota window;
- used or remaining percentage according to preference;
- reset countdown;
- detailed/compact density switch;
- refresh;
- Usage details & history;
- Manage accounts.

Unavailable data shows the exact reason. The popup does not infer plan limits from token logs.

### 11.5 Full Usage and analytics workspace

Register **Usage** as a center destination or a Resources subdestination after the shell roster decision. It contains:

- agents spawned;
- active/completed time;
- PRs created;
- total sessions/turns/events;
- provider usage tabs;
- recent activity heatmap;
- token mix/input/output/cache/reasoning;
- session and project breakdown;
- rate-limit history when observed;
- account management.

Cost is always labelled **estimated** and appears only when a versioned pricing table and provider/model identity are known. Missing model or pricing produces no fabricated dollar figure.

### 11.6 Reference modules

Study these Orca patterns during task-packet preparation:

- `ResourceUsageStatusSegment.tsx`;
- `WorkspaceSpaceCompactPanel.tsx`;
- `WorkspaceSpaceManagerPanel.tsx`;
- `UsageRosterPanel.tsx`;
- `StatsPane.tsx` and provider usage panes.

Assembly reuses concepts, not Electron-specific process/webview code.

---

## 12. Shell composition, Paneview, and session navigation

### 12.1 Layout technology contract

- **Center destinations:** existing Dockview.
- **Left/right/bottom stacked regions:** Paneview from the Dockview ecosystem already present in the project; do not introduce another layout library.
- **Internal multi-section workspaces:** Paneview when sections are independently resizable/collapsible/reorderable.
- **Dialogs, menus, hover cards, browser floating/expanded surfaces, and the global FAB:** existing overlay/portal layer, not Dockview panels.

Create one adapter rather than direct Paneview calls throughout the product:

```text
shell/layout/workbenchPaneview.ts
components/layout/WorkbenchPaneviewHost.svelte
components/layout/WorkbenchPaneHeader.svelte
```

The adapter owns IDs, persistence version, min/max size, collapsed state, move/reorder, focus, accessibility, parking, and restore. It does not own feature data.

### 12.2 Left rail: My Work

The left side contains only current work, grouped:

```text
Project
  Worktree/workspace
    owned agent session
    owned agent session
```

Rows are compact, not full-width cards with permanent detail blocks. A row shows:

- agent/provider icon;
- concise title;
- state dot/label;
- model only when known;
- branch/task/PR indicator;
- relative activity;
- attention/approval indicator.

Hover/focus opens a bounded hover card with basic information: full title, folder/worktree, provider/model/effort, state, last activity, latest turn preview, PR/check hint, and primary actions.

Click selects the session. A chevron/explicit details action expands an accordion row with full metadata and lifecycle actions. Expansion does not make every sibling wider or taller.

Working and Done are separate Paneview panes, each independently collapsible/resizable and preserving its own scroll. Settled/archived work may be a third pane after lifecycle migration.

### 12.3 Session Library is separate

Move machine-wide scanned/resumable sessions out of the left My Work column.

Register a **Session Library** center or right-tool destination containing:

- provider/project/worktree filters;
- search;
- grouped session list;
- compact rows;
- preview/details pane;
- exact Resume, Fork, Inspect, Copy ID, Reveal transcript, and Open folder actions;
- pagination/virtualization for hundreds of records.

Opening Session Library does not adopt or start anything. Resume is an explicit action and chooses structured or terminal ownership according to provider capability and user choice.

### 12.4 Right-side Paneview roster

Files, Source Control, Worktrees, Problems, Context, Run Configurations, Workflow Inspector, and future tools are Paneview panes or Paneview-backed destinations. Each can collapse, resize, reorder within an allow-listed region, and preserve state.

A component hidden by Paneview remains parked when it owns expensive state; hiding it does not destroy terminals, Monaco models, browser tabs, or active workflow runs.

### 12.5 Orca/Codex reference patterns

Study Orca’s modular worktree card files—hover identity, agents, metadata, ports, review, and detail sections—and Codex’s compact project/session list. Do not copy their store architecture. Assembly keeps `OwnedSession`, session workspaces, and the existing rail store as authority.

---

## 13. AI integrated throughout the workbench

### 13.1 One contribution registry

Create one app-level AI contribution contract:

```ts
export interface AiContribution {
  id: string;
  contexts: AiContextKind[];
  label: string;
  description: string;
  requiredFacts: AgentFactKind[];
  requiredCapabilities: string[];
  output: 'suggestion' | 'draft' | 'review' | 'workflow-request' | 'action-proposal';
  buildRequest(context: AiContributionContext): AiRequest;
  apply(result: AiResult, context: AiContributionContext): Promise<AiApplyResult>;
}
```

Contributions use the existing conversation runtime or workflow engine. They do not spawn hidden one-off CLIs independently.

### 13.2 Initial contributions

- Draft PR title/body from deterministic Git/PR facts.
- Review current changes and prepare findings.
- Address review comments through a workflow.
- Diagnose/fix failed checks through a workflow.
- Suggest a run configuration and save it after preview.
- Explain a setting or form field.
- Help fill a bounded form using visible fields only.
- Save preflight: deterministic validation first, optional AI review second, advisory by default.
- Browser feedback → implementation workflow.
- Worktree/session cleanup summary and proposal.
- Git repair/reorganization/squash proposal.

### 13.3 Safety and truth

- Deterministic services remain the source of truth.
- AI receives bounded fact snapshots with source references and stale times.
- Remote PR text, browser text, repository content, and tool output are labelled untrusted data.
- AI output is visibly generated and never replaces status/ownership/check facts.
- Consequential actions require a typed proposal, preview, revalidation, and existing confirmation boundary.
- Save preflight cannot silently mutate unrelated fields, block saving indefinitely, or run without an explicit project policy.
- No arbitrary shell execution enters through the AI contribution registry.

---

## 14. Required changes to existing work packages

### 14.1 Replace Work Package 10A with four deliverables

#### WP10A-1 — ACP runtime and canonical protocol

- official Rust ACP client;
- pinned Codex/Claude adapters;
- capability/config discovery;
- canonical events/items;
- crash/reconnect/load/resume;
- no terminal creation;
- new structured sessions default.

#### WP10A-2 — Conversation experience

- proper Markdown;
- ordered typed timeline;
- screenshot paste first;
- model/effort/mode/permissions selectors;
- commands/skills menu;
- tool/plan/task/approval/input rendering;
- scrolling/virtualization;
- per-session persistence.

#### WP10A-3 — Terminal projection and ownership handoff

- incremental transcript watcher;
- external/imported session projection;
- structured ↔ native CLI ownership transfer;
- fork to terminal;
- reconciliation and failure rollback.

#### WP10A-4 — Unified child-agent tree

- provider-native child events/transcripts;
- Assembly workflow children;
- parent/role/origin state;
- read-only versus controllable actions;
- nested output and artifacts.

### 14.2 Expand Work Package 11 into workflow orchestration

The prior guarded-action work remains, but it is not sufficient. Add:

- workflow definition/versioning;
- workflow run reducer and append-only events;
- role catalog;
- ACP child-session spawner;
- MCP delegation broker;
- budgets/depth/parallelism/retries;
- pause/cancel/retry/approval;
- Workflow Center;
- built-in templates;
- cross-provider child support;
- integration with browser feedback, PRs, checks, tests, and worktrees.

### 14.3 Amend Work Package 8

Add floating mode, exact toolbar actions, grab/annotate/draw modes, same-webview teleport, marked-screenshot attachments, and workflow delivery.

### 14.4 Amend Work Package 9

Add compact/full Resource, full Space treemap/table, compact Usage roster, and full Usage analytics. Preserve one resource/process/worktree safety authority.

### 14.5 Amend Work Packages 2 and 3

Adopt Paneview for all side-region stacks, convert Working/Done into Paneview panes, implement compact My Work rows plus hover cards, and move machine-wide resumable sessions to Session Library.

---

## 15. Parallel execution plan

The shared seams land first. After that, maximize parallelism without allowing competing owners.

### Wave 0 — planning and native spikes, sequential

1. Refresh `origin/main`, re-anchor files/symbols, and record dirty paths.
2. Prove packaged ACP sidecars and authentication for one disposable Codex and Claude session.
3. Prove image prompt round-trip through each adapter.
4. Prove Tauri child-webview docked/floating/expanded z-order and bounds.
5. Freeze canonical `AgentEvent`, `AgentTimelineItem`, `AgentCapabilities`, `AgentConfigOption`, `ExecutionOwner`, Paneview registration, workflow-event, and attachment contracts.

No broad UI lane starts before these contracts are committed.

### Wave 1 — independent foundation lanes

| Lane | Scope | May run with |
| --- | --- | --- |
| A ACP runtime | Rust ACP manager, sidecars, capabilities, events, fixtures | B, C, D, E, F |
| B Conversation renderer | Typed timeline, Markdown, composer shell, image previews using mocks | A, C, D, E, F |
| C Workflow domain | Pure definitions, reducer, scheduler, role/budget validation | A, B, D, E, F |
| D Paneview shell | Adapter, left/right hosts, My Work row/hover model, Session Library shell | A, B, C, E, F |
| E Browser presentation | Floating/expanded state, toolbar, annotation/drawing models; native browser lane serialized as needed | A, B, C, D, F |
| F Resources/usage | Pure view models, compact/full surfaces, disk/usage contracts | A, B, C, D, E |

Only the controller edits shared manifests, Tauri `main.rs`, `/next/+page.svelte`, ShellFrame, global center roster, and shared settings schema.

### Wave 2 — runtime integrations

| Lane | Dependency |
| --- | --- |
| A2 Codex ACP integration | A contracts + sidecar proof |
| A3 Claude ACP integration | A contracts + sidecar/Node proof |
| B2 Screenshot send | A2/A3 capabilities + current attachment vault |
| C2 Delegation broker | A runtime + C workflow domain |
| D2 Session rail migration | D adapter + current session store |
| E2 Native browser capture | E models + child-webview proof + B attachment contract |
| F2 Native resource/usage data | F models + existing terminal/LSP/worktree registries |

Codex and Claude integration lanes may run in parallel because they own separate provider adapter files and fixtures. Native Rust test runners remain serialized according to the master plan.

### Wave 3 — product integration

- Workflow Center consumes C2 and the unified agent tree.
- Conversation consumes live A2/A3 events and B2 attachments.
- Browser feedback targets the one conversation/workflow service.
- PR/check/browser/run/worktree AI contributions register through one contribution framework.
- Resource and Usage compact surfaces mount in status/FAB; full workspaces enter the center roster.
- Paneview migration mounts all left/right/internal panes without recreating feature stores.

### Milestone reviews

- **M-ACP:** protocol, capabilities, adapter packaging, auth, image proof, no duplicate writer.
- **M-Conversation:** image paste, controls, tools/plans/tasks, session isolation, Markdown/security.
- **M-Workflow:** DAG/loop semantics, delegation safety, budgets, cancellation, append-only replay.
- **M-Shell:** Paneview persistence, compact rail, Session Library separation, hover/focus/accessibility.
- **M-Browser:** same page across docked/floating/expanded, capture/annotation/drawing, z-order/security.
- **M-Resource:** process ownership, Space safety, usage truth, compact/full navigation.
- **M-Integrated:** AI proposal/revalidation boundaries, cross-feature workflows, native regression suite.

Every milestone receives the read-only SOL-medium review required by the master plan.

---

## 16. Test and acceptance matrix

### 16.1 ACP/runtime fixtures

Record redacted event traces for both providers covering:

- new/load/resume;
- model and effort discovery/change;
- permission/mode change;
- image prompt;
- assistant text and reasoning;
- command execution/output;
- file changes/diff;
- plan and task/TODO update;
- approval allowed/denied;
- structured user question;
- MCP call;
- subagent start/progress/completion;
- interruption;
- context compaction;
- adapter crash/reconnect;
- app restart/replay.

Run the same canonical reducer assertions against both providers.

### 16.2 Conversation native proof

One recording must prove:

1. Start a new structured Codex session with no Assembly PTY/xterm created.
2. Start a new structured Claude session with no Assembly PTY/xterm created.
3. Choose model, effort, and permissions using native selectors and observe confirmed values.
4. Paste two screenshots, remove one, send the other, and prove the provider receives the image.
5. Stream reasoning, tool calls, command output, file changes, plans/tasks, approvals, and final text in order.
6. Run Codex and Claude concurrently while switching sessions; drafts, images, scroll, controls, and children remain isolated.
7. Open native CLI for one session, prove the structured writer releases ownership, add a turn, return to Conversation, and reconcile exactly once.
8. Restart Assembly and restore both sessions without duplicate provider or terminal processes.

### 16.3 Workflow native proof

Use a disposable repository and run:

```text
Implement role → tests role → code-review role → spec-compliance role → one fix iteration → approval
```

Prove:

- at least two roles use different providers when configured;
- parallel nodes run concurrently up to the configured limit;
- the graph and agent tree show exact node/loop state;
- child transcripts and artifacts open;
- cancellation stops only Assembly-owned children;
- depth/agent/turn/retry/time budgets refuse excess work;
- a failed node follows the selected failure policy;
- pause/restart replays the append-only run state;
- no child receives unapproved arbitrary execution capability;
- the final publish/merge remains human-confirmed.

### 16.4 Browser proof

Prove one live page through docked → floating → expanded → docked → collapsed without reload or state loss. Capture an element, annotate another, draw on a screenshot, add all three to the matching conversation, and send only after explicit review. Show dialogs above the child view and prove no stale workspace/tab/generation event lands.

### 16.5 Shell proof

- Working and Done are real Paneview panes.
- Left rows remain compact at minimum width.
- Hover/focus shows bounded basic information.
- Accordion details do not resize unrelated rows.
- Session Library contains machine-wide resumable sessions and does not start one on selection.
- Left/right Paneview order, size, collapse, and scroll restore after restart.
- Parking a pane does not kill a terminal, editor, browser, or workflow.

### 16.6 Resource/usage proof

- Compact Resource popup opens the exact session/worktree/process tree.
- External/unowned processes have no stop action.
- Full Space view matches disposable worktree disk totals and protects active/dirty work.
- Compact Usage and full Usage show the same provider windows/reset values.
- Unknown plan/model/pricing produces no fabricated quota or cost.

---

## 17. Migration and deletion policy

1. Keep the current PTY/transcript implementation working while the canonical timeline and ACP runtime land.
2. Introduce structured ownership only for newly created sessions behind a capability/feature gate.
3. Import existing transcript history into the canonical timeline by stable provider item identity.
4. Enable structured resume for existing sessions only after provider-specific resume fixtures pass.
5. Add terminal handoff only after single-writer tests and native process receipts pass.
6. Remove the dead/unwired handwritten provider process code only after ACP has equivalent tested coverage.
7. Remove the 500 ms frontend transcript polling only after the incremental Rust watcher handles live, rotated, archived, and missing transcripts.
8. Remove hard-coded slash/model/permission lists only after runtime config and command discovery are live.
9. Never migrate or delete user transcript, workspace, layout, browser, or attachment state merely to simplify implementation.

---

## 18. Research references for task-packet writers

Inspect these repositories and exact areas while writing bounded implementation packets:

- `agentclientprotocol/rust-sdk` — official Rust client/session/proxy patterns;
- `agentclientprotocol/codex-acp` — config options, images, plans, tools, permissions, subagents, standalone binaries;
- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK, images, permissions, TODOs, nested subagents, terminals;
- `pingdotgg/t3code/packages/contracts/src/providerRuntime.ts` — canonical provider-neutral event taxonomy;
- `pingdotgg/t3code/apps/web/src/components/ChatMarkdown.tsx` — safe rich streaming Markdown patterns;
- `recailai/jockey/src-tauri/src/acp` — ACP runtime model/mode/config/command discovery in Tauri/Rust;
- `xintaofei/codeg/src-tauri/src/acp/delegation` — app-owned delegation broker, linked child sessions, budgets, status, cancel/disconnect;
- `stablyai/orca` browser-pane, resource-manager, workspace-space, usage-roster/stats, and modular worktree-card surfaces.

These are architecture and behavior references. Reuse code only after license, dependency, security, and compatibility review.

---

## 19. Completion definition

The amended product wave is complete only when:

- Assembly-created Claude and Codex conversations run as structured ACP sessions by default without creating terminal tabs;
- model, effort, permissions/mode, fast/service-tier, and provider options are truthful native controls;
- screenshot paste works end-to-end for both providers;
- tools, reasoning, plans, tasks, approvals, structured questions, file changes, and subagents render as typed items;
- terminal-owned/imported sessions still work and ownership handoff never creates duplicate writers;
- workflow definitions can orchestrate role-specific, cross-provider child agents with bounded loops, visible state, controllable Assembly-owned children, and append-only replay;
- browser docked/floating/expanded/collapsed modes preserve one live page and element/drawing feedback reaches the exact conversation or workflow only after review;
- compact/full Resource, Space, and Usage surfaces are accurate and safe;
- the left rail is compact My Work, Session Library is separate, and all left/right/internal panel stacks use Paneview;
- AI assistance throughout GitHub, browser, forms, saves, runs, resources, and worktrees uses deterministic facts and confirmed typed actions;
- the native acceptance recordings, process trees, security fixtures, focused tests, and milestone reviews all pass without regressing the current terminal, Dockview, Monaco, Roslyn, Git, worktree, browser, or persisted-session behavior.
