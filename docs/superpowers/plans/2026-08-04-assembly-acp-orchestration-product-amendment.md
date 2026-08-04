# Assembly ACP, Orchestration, and Product-Shell Amendment

**Date:** 2026-08-04  
**Status:** Approved planning amendment; implementation has not started  
**Repository baseline:** `4e8192e0cdce791f53e93af39a6d2d2e2c322911`  
**Applies to:** TSK-808, TSK-809, TSK-810, and the workflow/orchestration, browser, resource, usage, session-navigation, hosted-Git, and integrated-agent portions of the Assembly product wave.

> **Execution authority:** This amendment is mandatory alongside
> `2026-08-01-tsk-808-native-workbench-product-wave.md`. It supersedes conflicting guidance in
> Work Packages 2, 3, 8, 9, 10A, and 11, and supersedes the PTY-only runtime assumption in
> `2026-08-02-tsk-809-810-conversation-workbench.md`. The older files remain valuable repository
> analysis and acceptance evidence. When the documents disagree, this amendment wins for agent
> runtime ownership, conversation controls, attachments, nested agents, workflow orchestration,
> Paneview layout, browser presentation, resources, usage, session navigation, and AI assistance.

---

## 1. Locked product decisions

1. **Model, reasoning effort, permissions, collaboration/runtime mode, and supported provider
   options are first-class selectors in the composer.** They are not presented as slash commands.
   Their values come from the active runtime's advertised configuration and never from a static
   guessed model list.
2. **Only genuine prompt/provider commands and skills appear in the slash menu.** Terminal-UI-only
   commands such as theme, Vim mode, keymap, raw terminal rendering, terminal title, pets, local
   copy, and similar TUI concerns are omitted. Assembly implements applicable local actions as
   normal buttons, menus, keybindings, or command-palette actions.
3. **A structured ACP connection is not a PTY.** It is a bidirectional structured process/transport.
   Assembly's existing `TerminalRegistry` remains the PTY authority for native terminals and for
   terminal requests emitted by a structured agent.
4. **New Claude Code and Codex sessions start in structured mode by default** after the ACP adapter
   and packaging gates pass. Existing sessions started outside Assembly, or sessions explicitly
   opened in their native CLI, use the transcript-projection adapter until ownership returns to
   structured mode.
5. **One native conversation has exactly one writer.** Structured ACP and native CLI ownership may
   transfer, but they may never submit prompts, approvals, or interrupts concurrently.
6. **Screenshot paste is the first end-to-end conversation milestone.** Pasted and dropped images
   must preview, survive session switching, send through the exact active ACP session as image
   content when supported, and clean up safely. A path appended to prompt text is only a documented
   fallback for a provider/runtime that cannot accept images directly.
7. **Conversation output is one ordered, typed timeline across providers.** Assistant prose,
   reasoning, plans, tasks, commands, command output, file edits, diffs, MCP tools, approvals,
   questions, web activity, sub-agents, context compaction, warnings, and failures retain their
   position and lifecycle instead of being flattened into Markdown text.
8. **Provider-native sub-agents and Assembly workflow agents are both visible.** They share a common
   hierarchy/status model while retaining their origin, provider identity, parent relation, output,
   permissions, worktree, and cancellation rules.
9. **Workflow orchestration is an Assembly-owned product capability.** A model may request a
   delegation through an allow-listed tool, but the application validates, launches, limits,
   records, and controls the child runtime. The parent model never directly spawns an untracked OS
   process.
10. **Working and Done are Paneview panes.** The left navigation is no longer a hand-built flex
    column containing both active work and the machine-wide resume catalog. Active work remains
    compact on the left; session discovery becomes a separate Paneview/center workspace.
11. **Every collapsible panel inside a left, right, or tabbed workbench region uses Dockview's
    Paneview primitive.** Center destinations continue using Dockview. True overlays—dialogs,
    menus, the bottom-right action island, browser expanded mode, and annotation cards—are the only
    exceptions.
12. **The browser has three presentation states:** collapsed to the global action island, docked in
    its Browser destination, and expanded over the application shell while retaining the same live
    browser tab and profile.
13. **Browser context supports three distinct capture actions:** grab an element as context,
    annotate an element with Change/Question intent, and draw on a screenshot. All produce reviewable
    context attachments and never auto-submit a prompt.
14. **Resources and usage each have a compact surface and a full workspace.** Resource Manager and
    Workspace Space share authoritative ownership/safety data; provider usage and long-term usage
    analytics share collectors but remain separate views.
15. **AI assistance is embedded through typed, preview-first recipes rather than a second chat
    system.** PR drafting/review, check repair, run-configuration generation, form completion, save
    validation, cleanup proposals, and other assistance all target the existing conversation/runtime
    and deterministic application services.
16. **No provider-specific capability is invented for parity.** Unsupported features are hidden or
    disabled with a reason. The shared UI is a common semantic contract, not a claim that every
    provider has every feature.

---

## 2. Current repository reality and migration boundary

The merged repository already has the correct stable product identity and several strong
foundations:

- `ownedId` is the Assembly-owned key across session rail, terminal, workspace persistence, and
  conversation state.
- `nativeSessionId` and `ptySessionId` are provider/runtime metadata and may change without
  replacing the workspace.
- `TerminalRegistry` and `terminalService.ts` provide one terminal listener, persistent backend
  scrollback, process-group cleanup, reattach, background output, and tombstones.
- `ConversationSurface.svelte`, the reducer/store/service, transcript parser, attachment vault, and
  child transcript discovery already exist.
- Per-session Editor, Browser, Diff, conversation draft, child selection, scroll, and Dockview
  layout restoration already exist.

The current active conversation path is intentionally conservative:

```text
interactive Claude/Codex in PTY
        -> provider JSONL transcript
        -> 500 ms full-tail polling and flattening
        -> ConversationSurface
        -> prompt written back to the same PTY
```

The repository also contains an unwired provider process spike. Do not simply enable that spike:
doing so beside a running PTY would create two writers. The migration replaces the ownership model
first, then introduces one structured runtime under that ownership model.

### 2.1 Required migration principle

```text
                         Assembly ownedId
                               |
                    AgentRuntimeManager
                               |
          +--------------------+--------------------+
          |                                         |
 Structured owner                              Terminal owner
 ACP adapter connection                     Existing native PTY
 typed live events                           Incremental transcript projection
          |                                         |
          +--------------------+--------------------+
                               |
                    Canonical AgentEvent stream
                               |
                    One conversation timeline
```

Transcript projection remains necessary for:

- sessions created outside Assembly;
- a conversation currently owned by its native CLI;
- archived/read-only history;
- recovery after an adapter crash;
- importing changes made during terminal ownership;
- provider-native child transcripts that are durable but no longer live.

It is no longer the only live provider protocol.

---

## 3. ACP runtime and PTY ownership

### 3.1 Rust modules

Refactor the existing `src-tauri/src/agent_conversation` module instead of creating a parallel
conversation subsystem:

```text
src-tauri/src/agent_conversation/
  mod.rs
  manager.rs
  ownership.rs
  capabilities.rs
  events.rs
  event_buffer.rs
  attachments.rs
  handoff.rs
  providers/
    mod.rs
    acp.rs
    acp_process.rs
    adapter_catalog.rs
    codex_direct.rs        # optional later optimization, not first delivery
  transcript/
    mod.rs
    watcher.rs
    codex.rs
    claude.rs
```

Core shape:

```rust
pub enum AgentExecutionOwner {
    Structured,
    Terminal,
    TransitioningToStructured,
    TransitioningToTerminal,
    Stopped,
}

pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider_id: String,
    pub native_session_id: Option<String>,
    pub generation: u64,
    pub owner: AgentExecutionOwner,
    pub state: AgentSessionState,
    pub capabilities: AgentCapabilities,
    pub active_turn_id: Option<String>,
    pub recent_events: VecDeque<AgentEvent>,
}
```

`AgentRuntimeManager` is keyed by `ownedId`. It owns structured adapter processes, generation
fences, cancellation, capability negotiation, request/response routing, terminal requests, and
explicit ownership handoff. `TerminalRegistry` remains a separate process authority and is injected
into the manager where ACP terminal operations need a real PTY.

### 3.2 ACP adapter strategy

Use the official Rust ACP SDK as Assembly's client boundary. Run pinned, tested adapters rather
than resolving `@latest` during a session:

- **Codex:** package a pinned `codex-acp` macOS sidecar. It drives Codex app-server and exposes
  structured messages, reasoning, plans, commands, edits, approvals, model/effort/mode options,
  images, usage, review, MCP, and sub-agent metadata.
- **Claude Code:** package a pinned `claude-agent-acp` adapter. It drives the official Claude Agent
  SDK and exposes messages, thoughts, tools, permission requests, images, TODO/task updates,
  terminals, slash commands, edit review, MCP, and nested sub-agent metadata.
- **Future providers:** implement or install another ACP adapter behind the same provider catalog;
  no Svelte timeline rewrite is allowed for a common capability.

Claude adapter distribution has a required packaging gate because its current package uses a Node
runtime. The implementation packet must choose and prove one of these before product code depends
on it:

1. a reviewed standalone adapter binary;
2. a bundled, version-pinned Node runtime plus adapter assets; or
3. an explicit supported local Node requirement with a clear availability check.

Do not invoke `npx` at session start and do not download runtime code silently.

### 3.3 ACP does not imply a visible terminal

The ACP adapter process communicates over structured stdio/JSON-RPC. It is not shown in xterm and
is not registered as a user terminal. When an agent emits an ACP terminal request or command tool:

1. validate the exact `ownedId`, generation, cwd, environment scope, and provider request ID;
2. create or attach a PTY through the existing `TerminalRegistry` when interactive terminal output
   is required;
3. return terminal identity/output through the ACP client contract;
4. render the command/tool item in the conversation timeline;
5. expose **Open terminal** to reveal that exact existing PTY, not spawn another command;
6. close or retain it according to the provider request and Assembly's process policy.

Background tool commands that do not require interaction may remain structured tool executions and
never create an xterm tab. Terminal creation is capability- and request-driven, not automatic for
every ACP session.

### 3.4 Native CLI handoff

A raw terminal button performs a state transition, not a CSS toggle:

```text
StructuredOwned
  -> finish/interrupt active turn
  -> flush and snapshot canonical events
  -> release structured writer
  -> start native CLI with exact provider resume identity
  -> TerminalOwned

TerminalOwned
  -> stop/detach native CLI after confirmation when needed
  -> read/import provider turns written while terminal-owned
  -> resume ACP session using the same native identity
  -> reconcile by native item identity
  -> StructuredOwned
```

Rules:

- no transition starts while an approval or user-input request is unresolved;
- a failed transition restores the prior owner;
- ownership and generation persist with the `OwnedSession` record;
- UI remains readable during transition and never clears history;
- **Fork to terminal** creates a provider-supported fork or a new related session and does not
  surrender the current writer;
- concurrent writes are a test failure even if both provider processes appear healthy.

A later Codex-specific experiment may share a managed app-server daemon between Assembly and a
remote Codex TUI. It is an optimization only after exact thread, subscription, approval, and
single-writer behavior is proven. It is not required for the first ACP delivery.

---

## 4. Capability-driven controls and command discovery

### 4.1 Configuration selectors

The composer exposes a provider identity control followed by capability-driven controls such as:

```text
[Codex] [GPT-5.6] [High] [Agent] [Fast] [More]
```

The exact controls are data, not hard-coded component branches:

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'reasoning'
    | 'permission'
    | 'collaboration'
    | 'service-tier'
    | 'boolean'
    | 'unknown';
  label: string;
  description: string | null;
  value: string | boolean | null;
  choices: AgentConfigChoice[];
  mutable: boolean;
  disabledReason: string | null;
}
```

Requirements:

- preserve provider-advertised choice order;
- apply a selection through ACP `session/set_config_option`, mode APIs, or the adapter's declared
  equivalent;
- display `Applying…` until the adapter confirms the effective value;
- retain observed and requested values separately;
- reject stale generation responses;
- when changing the model invalidates the selected effort, use the provider-advertised default or
  supported fallback and show the resulting confirmed pair;
- unknown option categories appear under **More**, never disappear silently;
- terminal-owned sessions show recorded values and route supported changes to the native picker, or
  disable them with the exact reason.

### 4.2 Slash-command policy

The command menu merges only:

1. provider-advertised prompt commands;
2. discovered provider/project skills;
3. Assembly prompt templates that intentionally insert text into the composer.

It does **not** mirror a native TUI command enum. Local workbench actions belong in normal UI:

- model, effort, permissions, plan/runtime mode -> composer selectors;
- copy -> message/code item actions;
- diff -> Diff workspace;
- terminal/raw -> ownership/handoff action;
- new/resume/fork/archive -> session actions;
- theme/keymap/Vim/title/statusline -> Settings;
- resources/usage/browser -> their own workspaces and status controls.

Each command descriptor declares whether selection inserts text, opens a provider form, invokes a
structured provider command, or runs an Assembly prompt template. Navigating the menu never runs a
command and selecting a skill never auto-submits.

---

## 5. Canonical conversation and task model

### 5.1 Ordered timeline

Replace the message-only transcript projection with ordered typed items:

```ts
export type AgentTimelineItem =
  | UserMessageItem
  | AssistantMessageItem
  | ReasoningItem
  | PlanItem
  | TaskItem
  | CommandItem
  | FileChangeItem
  | McpToolItem
  | DynamicToolItem
  | WebSearchItem
  | ImageItem
  | SubAgentItem
  | ApprovalItem
  | UserInputItem
  | ContextCompactionItem
  | WarningItem
  | ErrorItem;
```

Every item has stable Assembly and provider IDs, turn ID, parent item ID when applicable, status,
sequence, timestamps, and optional namespaced provider metadata. Content deltas identify a channel:
assistant, reasoning, reasoning summary, plan, command output, or file-change output.

Do not store assistant prose in one list and tools in another. The timeline must preserve:

```text
assistant text -> command -> command output -> reasoning summary -> edit -> assistant text
```

### 5.2 Canonical runtime events

```ts
export type AgentEventType =
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
  | 'task.started'
  | 'task.progress'
  | 'task.completed'
  | 'usage.updated'
  | 'children.updated'
  | 'runtime.warning'
  | 'runtime.error';
```

Rust normalizes ACP updates and transcript imports into this contract. Svelte never switches on raw
Codex app-server or Claude SDK message shapes. Raw frames may be retained in a bounded diagnostic
ring and exported only through an explicit diagnostic action.

### 5.3 Durability

For this wave, do not introduce an unrelated application database migration. Keep provider-native
history authoritative and add:

- bounded append-only normalized event receipts per `ownedId` for live replay/recovery;
- an atomic compact snapshot containing last applied sequence, active items, configuration,
  attachments, child relations, and ownership state;
- the existing append-only orchestration log for workflow/audit events, extended additively;
- bounded raw diagnostic logs with redaction.

Reconciliation imports provider history by native item identity and is idempotent. If measured
startup/search requirements later justify an indexed store, that is a separate migration packet
with explicit schema and performance evidence—not an accidental side effect of the ACP work.

---

## 6. Screenshot and context attachments — priority vertical slice

### 6.1 Supported sources

The composer accepts:

- clipboard images;
- file-picker images;
- files dragged into the composer;
- browser element captures;
- browser screenshot markup;
- selected editor/diff/problem/PR context through typed context attachments.

```ts
export interface ConversationAttachment {
  id: string;
  ownedId: string;
  kind: 'image' | 'browser-element' | 'browser-markup' | 'file' | 'selection' | 'diff' | 'problem';
  mimeType: string | null;
  managedPath: string | null;
  displayName: string;
  byteLength: number | null;
  source: ConversationAttachmentSource;
  unavailableReason: string | null;
}
```

### 6.2 Image send path

For a structured ACP session with image capability:

1. validate/copy the bytes through the existing Rust attachment vault;
2. create the local preview immediately;
3. persist only managed identity/path metadata, never a blob URL;
4. on Send, revalidate owner, generation, file signature, size, and existence;
5. transmit an ACP image content block or the adapter's declared image representation;
6. include the text prompt in the same prompt request;
7. clear/revoke only after the adapter accepts the prompt;
8. keep the exact draft and previews on failure.

Fallback order when images are unsupported:

1. provider-supported resource/file link to the managed file;
2. documented prompt path reference when the provider can read the local workspace path;
3. disabled Send for that attachment with a clear provider-specific reason.

Never silently discard an image or claim it was visually supplied when only its filename was sent.

### 6.3 Safety and cleanup

Keep PNG, JPEG, GIF, and WebP validation; add aggregate per-draft/session limits, owner-only file
permissions, exact delete/prune commands, symlink refusal, stale-generation checks, and unavailable
attachment restoration. Browser captures and markup are immutable snapshots; navigating the page
later does not retarget them.

The first native acceptance recording for the ACP lane must paste and send a screenshot through
both Codex and Claude structured sessions before model menus, workflows, or visual polish can close.

---

## 7. Tool calls, approvals, terminals, plans, and nested agents

### 7.1 Typed renderers

Conversation components become a typed hierarchy:

```text
ConversationTimeline
  UserMessageItem
  AssistantMessageItem
  ReasoningItem
  PlanItem
  TaskItem
  CommandItem
  FileChangeItem
  ToolItem
  ApprovalItem
  UserInputItem
  SubAgentItem
  ErrorItem
```

Command and tool rows remain compact in the narration flow and expand to exact command, cwd,
status, duration, bounded stdout/stderr, affected files, and diagnostics. File changes open the
existing Diff destination. Paths and locations use the existing editor/open-file bus.

### 7.2 Approvals and elicitation

An approval request carries provider request ID, generation, turn/item identity, operation class,
summary, consequences, and options. Render only options supplied by the runtime plus Assembly's
explicit cancellation behavior. Responses are rejected after expiry, owner change, or generation
change.

Structured user questions/forms are first-class timeline items. They are not converted into prose
and guessed back into fields.

### 7.3 Provider-native sub-agents

Live ACP sub-agent metadata creates or updates a recursive child entry. Durable transcript discovery
fills history for terminal-owned or completed sessions. One child may contain its own typed
messages, reasoning, tools, tasks, and descendants.

```ts
export interface AgentNode {
  id: string;
  origin: 'provider-native' | 'assembly-workflow';
  providerId: string;
  nativeSessionId: string | null;
  parentId: string | null;
  launchItemId: string | null;
  role: string | null;
  title: string;
  state: 'queued' | 'starting' | 'running' | 'waiting' | 'completed' | 'failed' | 'cancelled';
  depth: number;
  worktreeId: string | null;
  startedAt: string | null;
  updatedAt: string | null;
  outputAvailable: boolean;
}
```

Provider-native children are controlled only through capabilities the provider explicitly exposes.
Transcript-only children remain read-only. Workflow-managed children use Assembly's workflow
controls described below.

---

## 8. Assembly workflow and orchestration engine

The existing plan's guarded-agent recipes are useful but too narrow. Assembly needs a durable,
observable multi-agent workflow layer that can run Claude, Codex, and future ACP providers in the
same workflow.

### 8.1 Definition model

```ts
export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
  version: number;
  trigger: 'manual' | 'session-action' | 'pull-request' | 'check-failure' | 'run-configuration';
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  defaults: WorkflowDefaults;
  limits: WorkflowLimits;
}

export interface WorkflowNodeDefinition {
  id: string;
  role: 'orchestrator' | 'implementer' | 'reviewer' | 'spec-compliance' | 'tester' | 'fixer' | 'custom';
  label: string;
  providerId: string;
  modelOption: string | null;
  reasoningOption: string | null;
  permissionMode: string | null;
  promptTemplate: string;
  worktreePolicy: 'inherit' | 'new-worktree' | 'read-only-checkout' | 'none';
  inputBindings: WorkflowInputBinding[];
  outputContract: WorkflowOutputContract;
  retryPolicy: WorkflowRetryPolicy;
  approvalGate: WorkflowApprovalGate | null;
}

export interface WorkflowEdgeDefinition {
  from: string;
  to: string;
  condition: 'success' | 'failure' | 'findings' | 'changes-requested' | 'always' | 'expression';
  expression: string | null;
}
```

Default templates may include:

- orchestrator -> implementer -> tests -> code review -> spec compliance -> fix loop -> final
  review;
- implementer and independent reviewer in parallel, then adjudicator;
- PR review -> findings triage -> fixer -> check rerun;
- plan/spec audit with multiple read-only specialist roles;
- browser feedback -> implementer -> screenshot/browser verification.

Templates are editable definitions, not hard-coded one-off prompts.

### 8.2 Run model

```ts
export interface WorkflowRun {
  id: string;
  definitionId: string;
  definitionVersion: number;
  ownedId: string;
  projectRoot: string;
  worktreeId: string | null;
  state: 'queued' | 'running' | 'waiting' | 'paused' | 'completed' | 'failed' | 'cancelled';
  activeNodeIds: string[];
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
  budget: WorkflowBudgetState;
}

export interface WorkflowAgentRun {
  id: string;
  workflowRunId: string;
  nodeId: string;
  parentAgentRunId: string | null;
  ownedId: string;
  providerId: string;
  nativeSessionId: string | null;
  state: AgentNode['state'];
  attempt: number;
  worktreeId: string | null;
  currentTurnId: string | null;
  progress: string | null;
  outputs: WorkflowArtifactReference[];
}
```

Persist workflow definitions as versioned JSON under app data and append run events to the existing
orchestration authority. A run always records the exact definition version and resolved provider
configuration used.

### 8.3 Orchestration broker

Add one Rust `WorkflowCoordinator` over `AgentRuntimeManager`; do not let components launch
providers directly.

Responsibilities:

- validate workflow graph, role/provider availability, config choices, roots, and limits;
- allocate or validate worktrees before starting an editing node;
- spawn fresh or resumable ACP sessions through the runtime manager;
- link parent, workflow, node, provider session, worktree, and launch tool IDs;
- deliver bounded structured inputs and artifacts;
- observe turn/task/tool state and advance eligible edges;
- enforce maximum depth, maximum concurrent agents, maximum attempts, time budget, and token/cost
  budget when usage is available;
- pause for user decisions and approvals;
- cancel descendants when policy requires it;
- return final output/artifacts to an orchestrator node without hiding the full child transcript;
- emit append-only receipts for every state change.

### 8.4 Model-accessible delegation tools

Assembly may expose a small local MCP/ACP tool surface to an orchestrator:

```text
spawn_workflow_agent
get_workflow_agent_status
read_workflow_agent_result
send_workflow_agent_message
cancel_workflow_agent
list_workflow_artifacts
request_workflow_decision
```

Every call enters `WorkflowCoordinator`; the model cannot supply an executable path, arbitrary
adapter command, hidden environment, unrestricted cwd, or unbounded prompt. `spawn_workflow_agent`
resolves only registered workflow roles/provider profiles and applies the definition's limits.

This permits a parent agent to coordinate dynamically while keeping OS/process ownership,
permissions, visibility, cancellation, and audit in Assembly.

### 8.5 Agent Control workspace

Add a center **Agent Control** destination with Paneviews:

```text
Agent Control Dockview destination
  Workflow / plan graph Paneview
  Active agents Paneview
  Selected agent live timeline Paneview
  Decisions and approvals Paneview
  Artifacts and changes Paneview
  Run event log Paneview
```

The user can:

- inspect the current loop/phase and why a node is waiting;
- expand every nested agent and read live output;
- focus its associated session/worktree/editor/diff;
- pause/resume/cancel a run;
- cancel an Assembly-managed child;
- approve a gate or answer a structured question;
- retry one failed node within policy;
- message an active workflow agent through a guarded coordinator operation;
- compare outputs from parallel reviewers;
- open artifacts and file changes;
- save a successful run as a reusable workflow template.

A provider-native child that Assembly did not launch appears in the hierarchy, but its controls are
limited to provider capabilities. The UI labels this distinction.

### 8.6 Worktree and mutation policy

- Only one editing agent owns a worktree at a time unless a workflow definition explicitly selects
  a reviewed shared-worktree policy.
- Review/spec/test nodes default to read-only or their own checkout.
- Worktree creation/removal delegates to the existing worktree safety authority.
- Agent text is never proof that a test passed, a file is clean, or a PR is mergeable; deterministic
  services produce those facts.
- No workflow node gains an arbitrary shell action. User-created Run configurations and provider
  tool permissions remain the allowed command paths.
- Destructive or remote operations retain their existing preview, confirmation, and expected-state
  gates.

---

## 9. Browser workspace and context capture

The existing browser work package already has the correct child-webview, profile-isolation, global
FAB, full-shell overlay, and annotation direction. Keep it, and add the following explicit
contracts.

### 9.1 Presentation

One live browser workspace supports:

```text
collapsed -> bottom-right global action island only
docked    -> Browser Dockview destination
expanded  -> root overlay covering the app work area, not macOS window chrome
```

Docked and expanded use the same native tab/profile. Changing presentation never reloads the page
or duplicates the webview. Expanded chrome remains under dialogs/confirmations and above the
browser webview.

### 9.2 Toolbar contract

The upper toolbar includes, when supported:

- profile and profile settings;
- Import context/cookies only after an explicit security design;
- viewport presets and custom viewport;
- Grab page element;
- Annotate page element;
- Draw on screenshot;
- development-only Devtools;
- Open in default browser;
- overflow settings;
- Expand/Restore and Collapse.

### 9.3 Three context-capture flows

**Grab element**

- user arms picker and selects a real DOM element;
- inspector returns bounded selector, accessibility name, text snippet, URL, viewport/page rect,
  and screenshot crop when available;
- confirmation sheet lets the user copy or attach the exact context;
- no change/question note is required.

**Annotate element**

- same bounded selection plus an annotation card;
- note is required;
- intent is Change or Question;
- queue supports edit, remove, Copy All, attach to composer, or Ask Agent;
- merely selecting the element sends nothing.

**Draw on screenshot**

- capture the current visible viewport through a browser-owned screenshot operation;
- open an HTML/canvas markup overlay with pen, arrow, rectangle, text, undo/redo, clear, cancel, and
  done;
- preserve original screenshot plus bounded vector markup metadata when practical;
- export one validated managed PNG/WebP and optional annotation summary;
- attach through the normal conversation attachment service.

All three create immutable `ConversationAttachment` or `BrowserAnnotation` records scoped to
workspace/tab/generation. They stage into the matching owned conversation and never replace a
nonempty draft without a merge preview.

### 9.4 Browser automation boundary

Browser automation for workflow verification uses a typed, allow-listed browser service:

```text
navigate
wait-for
query accessible element
click/type/select
capture screenshot
read bounded console/network failure summaries
```

It is separate from the interactive inspector and does not expose arbitrary JavaScript eval,
cookies, storage, page HTML, or credentials. Workflow browser steps show every action and artifact
in Agent Control.

---

## 10. Resource Manager, Workspace Space, and Usage

### 10.1 Compact Resource Manager

A status-bar/resource control opens a compact hierarchy grouped by:

```text
application
repository/project
worktree
owned session
agent/terminal/browser/language server
```

Rows show bounded CPU/RSS, ports, state, ownership, and a sparkline when historical samples exist.
Only registry-proven owned processes expose Stop. Stopping requires the existing exact-target
revalidation and confirmation policy.

The compact footer links to:

- review inactive workspaces;
- Workspace Space scan/review;
- full Resource Manager workspace;
- language-server policy/settings.

### 10.2 Full Resource Manager workspace

Register a center **Resources** destination using Paneviews for:

- process/workspace tree;
- selected process details and logs;
- ports and services;
- language servers and warm-root policy;
- browser/terminal ownership;
- disk/reclaimable summary;
- pressure events and cleanup receipts.

It uses the one-shot native snapshot and existing registries described in Work Package 9. It does
not become a second process registry.

### 10.3 Workspace Space workspace

Keep Workspace Space as a specialized full-page/center workspace sharing the existing worktree
safety facts:

- total scanned and reclaimable bytes;
- workspace treemap with zoom;
- selected workspace top-level breakdown;
- filter/sort/table;
- active agents, open terminals, dirty buffers, browser tabs, Git state, PR/check context;
- select and delete only through existing safe worktree removal flows;
- `Keep: main`, `Keep: active`, blocked, unavailable, and reclaimable reasons are deterministic.

### 10.4 Usage popover

The compact Usage control shows one provider row with:

- account/provider identity;
- current plan/source;
- relevant rate-limit windows;
- used or remaining display preference;
- next reset;
- refresh and account actions;
- Detailed/Compact density.

It links to **Usage details & history**.

### 10.5 Usage Analytics workspace

Register a center **Usage** destination with:

- agents spawned, agent-active time, PRs created, and tracking start;
- provider selector plus overview;
- token/input/output/cache/reasoning mix where the source records it;
- sessions, turns/events, active days, context use, and rate-limit history;
- model/provider breakdown;
- estimated cost clearly labelled as estimated and based on a versioned price source;
- source health, last refresh, and unavailable reasons.

Do not mix subscription percentage windows with token/cost analytics in one number. Collectors are
provider-specific adapters that output one normalized schema and never read or expose credentials.

---

## 11. Paneview shell and session navigation

### 11.1 Layout primitive rule

The installed `dockview-core` package includes Paneview primitives. Create one small Svelte host
adapter and use it consistently:

```text
shell/layout/paneviewHost.ts
components/layout/PaneviewHost.svelte
shell/layout/paneviewPersistence.ts
```

A Paneview panel descriptor contains stable ID, title, component, minimum size, initial size,
expanded state, close policy, and persistence scope. All Paneview layouts have versioned
serialization and safe default restoration.

Use:

- Dockview for center destinations/tabs and movable editor-style groups;
- Paneview for stacked/collapsible panes inside left, right, bottom, and center workspaces;
- overlays only for transient or full-shell presentation that cannot participate in document
  layout.

Do not wrap every leaf component in its own layout engine. Paneview owns meaningful resizable panes,
not every toolbar or card.

### 11.2 Left navigation

Replace the current wide card-first column with a compact active-work navigation inspired by the
best aspects of project-grouped chat histories and worktree rails:

```text
Assembly navigation
  primary destinations
  projects/workspaces
    compact active session rows
  Working Paneview
  Done Paneview
```

Requirements:

- default width is narrow enough to leave the editor dominant;
- rows show provider/status icon, short title, project/worktree relationship, and activity state;
- hover/focus card shows model/effort, branch, task/PR, worktree path, last activity, current
  workflow/agent role, ports/checks, and quick actions;
- click selects the workspace;
- disclosure/accordion opens full details and actions without making every row permanently tall;
- Working and Done are independent Paneview panels with persisted size/expanded state;
- Settled may be a third Paneview after its lifecycle semantics land;
- row text and hover information come from deterministic session facts, not title parsing;
- keyboard and VoiceOver receive the same information as hover.

### 11.3 Session Catalog is separate

The machine-wide resumable-session finder is removed from the active-work column. Add a separate
**Sessions** center destination or right-side Paneview workspace with:

- provider, project, worktree, state, model, and date filters;
- search and server/filesystem-side paging;
- project grouping;
- compact rows and a detail Paneview;
- exact resume, fork, inspect, reveal transcript, copy ID, and adopt actions;
- no automatic launch merely from expanding/selecting a row.

The active left rail includes a single **Find sessions** action that opens this destination.

### 11.4 Panel roster

Every panel is registered through one typed roster describing destination, region, layout kind,
persistence scope, activation data, and capabilities. Existing singleton services/stores remain
owners; Paneviews never create a second Git, browser, resource, session, workflow, or conversation
store.

---

## 12. AI assistance throughout Assembly

Create a shared recipe contract, not provider-specific buttons that each invent their own prompt
and mutation path:

```ts
export interface AiAssistRequest<TFacts, TProposal> {
  id: string;
  kind: AiAssistKind;
  ownedId: string;
  providerProfileId: string;
  facts: TFacts;
  factsHash: string;
  promptTemplateId: string;
  outputSchema: JsonSchema;
  applyPolicy: 'preview-only' | 'field-by-field' | 'confirm-once';
}

export interface AiAssistProposal<T> {
  requestId: string;
  factsHash: string;
  value: T;
  explanation: string | null;
  warnings: string[];
  sourceRefs: AgentFactReference[];
}
```

First recipes:

- draft PR title/body;
- review code or PR and prepare findings;
- address review comments;
- diagnose/fix failed checks;
- propose run configuration;
- summarize session/worktree/project/PR;
- fill selected form fields from visible deterministic facts;
- pre-save validation/advisory check;
- browser feedback implementation plan;
- worktree cleanup/reorganization proposal.

Rules:

- every request identifies one existing owned session/provider profile;
- remote/repository/page text is delimited as untrusted context;
- structured output is schema-validated before display;
- stale `factsHash` prevents applying the proposal;
- deterministic validation remains authoritative;
- no recipe silently saves, publishes, merges, deletes, pushes, or starts a run;
- the user can accept fields individually when appropriate;
- consequential operations go through existing prepare/confirm/revalidate services;
- receipts link the proposal, accepted values, deterministic action, and result without recording
  secrets.

This service is reusable from PRs, Settings/forms, browser feedback, run configurations, Problems,
Git, worktrees, and Agent Control. It does not create another transcript or assistant pane.

---

## 13. Required changes to existing work packages

### 13.1 Work Packages 2 and 3 — shell and sessions

Add before card polish:

- Paneview host/persistence contract;
- narrow active-work navigation;
- Working/Done Paneview conversion;
- separate Sessions destination;
- hover/focus details and accordion details;
- panel roster migration.

Remove the assumption that Working, Done, and Find remain three hand-built sections in one
`SessionsColumn`.

### 13.2 Work Package 8 — browser

Keep the child-webview spike, isolation, global action island, and annotation queue. Add explicit
Grab, Annotate, Draw-on-screenshot flows, typed conversation attachments, workflow browser
automation, and Browser Paneviews. The current browser plan remains authoritative for security,
clipping, profiles, z-order, and no-arbitrary-eval constraints.

### 13.3 Work Package 9 — resources and usage

Split presentation into compact Resource, full Resources, Workspace Space, compact Usage, and full
Usage destinations. Share collectors and deterministic ownership facts; do not merge all surfaces
into one oversized component.

### 13.4 Work Package 10A — conversation

Replace the PTY-only implementation order with:

1. ownership state and canonical rich event contract;
2. ACP client/adapter packaging spike;
3. image attachment vertical slice for Codex and Claude;
4. typed timeline, tools, approvals, tasks, plans, and terminal bridge;
5. dynamic selectors and genuine command discovery;
6. live nested sub-agents plus transcript reconciliation;
7. explicit native CLI handoff;
8. remove dead/unwired provider spike and 500 ms full-tail polling after parity/recovery tests.

The attachment vault, `ownedId`, terminal service, workspace snapshots, and existing conversation
store are extended, not replaced.

### 13.5 Work Package 11 — integrated agent

Keep its deterministic facts, preview/confirmation, and audit constraints. Expand it into:

- workflow definition/run models;
- `WorkflowCoordinator` and model-accessible delegation broker;
- Agent Control destination;
- role templates and cross-provider profiles;
- live nested output, decisions, artifacts, retries, pause/cancel;
- reusable AI assist recipes.

The guarded action registry remains the only mutation route. Workflow orchestration coordinates
agents; it does not grant them a general application automation backdoor.

---

## 14. Parallel delivery plan

Shared seams are frozen in Wave 0. Parallel work starts only after their types and owners are
committed.

### Wave 0 — controller-owned contracts, sequential

1. Re-anchor current branch/SHA and verify no overlapping conversation/shell WIP.
2. Commit execution-owner, capabilities, canonical event/item, attachment, Paneview descriptor,
   workflow definition/run, and AI assist interfaces.
3. Add additive persisted-record migrations and fixture shapes.
4. Define adapter packaging/version manifest and test fixtures.
5. Assign exact file ownership to lanes.

### Wave 1 — independent foundations, parallel

| Lane | Scope | Shared files forbidden after dispatch |
| --- | --- | --- |
| A | Rust ACP client/process supervisor and adapter catalog | Svelte conversation/store files |
| B | Incremental transcript watcher and transcript-to-event import | ACP process supervisor |
| C | Paneview Svelte host, serialization, and panel roster | conversation/runtime files |
| D | Browser child-webview/clipping/overlay spike | session/conversation runtime |
| E | Resource/process/disk/usage collector contracts | shell layout and conversation |
| F | Workflow graph validation, pure reducer, limits, fixtures | provider process manager |

### Wave 2 — product verticals, parallel with controller integrations

| Lane | Scope | Dependency |
| --- | --- | --- |
| A1 | Codex ACP adapter integration and fixtures | Wave 1A |
| A2 | Claude ACP adapter packaging/integration and fixtures | Wave 1A |
| C1 | Compact active-work navigation + Working/Done Paneviews | Wave 1C |
| C2 | Separate Sessions destination + detail Paneviews | Wave 1C |
| D1 | Browser docked/expanded/collapsed presentation | Wave 1D + 1C |
| D2 | Grab/Annotate/Markup context attachments | Wave 1D + attachment contract |
| E1 | Resource compact + Resources workspace | Wave 1E + 1C |
| E2 | Usage compact + Usage Analytics workspace | Wave 1E + 1C |
| F1 | Workflow coordinator and delegation broker | Wave 1F + 1A |

The controller alone integrates the canonical runtime into
`conversationTypes.ts`, `conversationReducer.ts`, `conversationStore.svelte.ts`,
`conversationService.ts`, `ConversationSurface.svelte`, `OwnedSession`, `sessionWorkspaces.ts`, and
`/next/+page.svelte` after returned lane tests pass.

### Wave 3 — rich conversation and orchestration, partially parallel

1. Screenshot paste/send first, against both provider adapters.
2. Typed timeline and Markdown/code rendering.
3. Tools, approvals, structured questions, task/plan updates, terminal bridge.
4. Dynamic model/effort/permissions/mode selectors.
5. Provider-native sub-agent hierarchy and live output.
6. Agent Control UI over workflow reducer/coordinator.
7. CLI ownership handoff and transcript reconciliation.
8. Browser context -> composer/workflow proof.
9. Resource/usage/session Paneview integration proof.

Timeline item renderers, workflow panes, provider fixtures, and browser/resource UI may proceed in
parallel after the canonical contracts freeze. Process ownership, reducer integration, and
workspace persistence remain controller-owned sequential seams.

### Wave 4 — AI-assisted product workflows

Parallel recipe lanes may implement PR drafting/review, checks, run configurations, forms/save
validation, cleanup, and browser-feedback recipes against the frozen `AiAssistRequest` contract.
One controller lane integrates recipes into existing workspaces and action confirmation services.

### Integration gates

After each wave:

- focused lane tests first;
- one serialized TypeScript/Svelte check;
- one serialized Rust test/build pass;
- one real Tauri acceptance session;
- read-only SOL-medium review of architecture, concurrency, safety, and scope;
- no next wave until duplicate runtime/process/store and stale-generation checks pass.

---

## 15. Acceptance matrix

### 15.1 ACP and composer

1. Start fresh structured Codex and Claude sessions; each receives and persists its native identity.
2. Paste two screenshots, remove one, switch sessions, return, and send the retained image plus text
   exactly once through each provider's image-capable prompt path.
3. Model, effort, permissions/mode, and provider-specific options populate from runtime data,
   confirm after change, and remain isolated per session.
4. Unsupported options display a reason and do not fall back to invented defaults.
5. Slash menu contains provider commands/skills but none of the excluded TUI-local commands.

### 15.2 Timeline, tools, and children

1. One turn renders reasoning, command, streamed output, file edit, diff, task/plan, approval,
   structured question, and final assistant text in exact order.
2. Command **Open terminal** reveals the exact PTY created for that tool without rerunning it.
3. Approval accept/deny targets the exact request and stale responses are refused.
4. Codex and Claude nested agents show identity, role, provider, state, relation, live output,
   tools, and descendants; completed transcript fallback remains readable.

### 15.3 Ownership

1. Open a structured conversation in native CLI and return to structured mode.
2. Process and event receipts prove there was never more than one writer.
3. History written in terminal mode appears once after reconciliation.
4. A failed handoff restores the prior owner and does not lose draft, history, attachments, or
   workspace state.

### 15.4 Workflow orchestration

Run a disposable workflow:

```text
orchestrator
  -> implementer
  -> tests
  -> reviewer + spec-compliance in parallel
  -> fixer when findings exist
  -> final reviewer
```

Prove:

- exact provider/model/effort/permissions for every node;
- tracked parent/child/worktree links;
- live output and artifacts visible in Agent Control;
- parallel nodes obey concurrency and checkout policy;
- findings advance the correct conditional edge;
- pause/resume, one-node retry, and cancellation work;
- an approval gate pauses before consequence;
- final result returns to the orchestrator while child transcripts remain inspectable;
- app restart replays the run without rerunning completed nodes.

### 15.5 Browser

1. Global action island opens Browser from collapsed state.
2. Same live page moves docked -> expanded -> docked without reload.
3. Grab one element, annotate another, and draw on a screenshot.
4. Attach all three to the matching conversation; owner/draft conflicts retain the queue.
5. A workflow browser verification step records actions and screenshot artifacts without arbitrary
   eval or credential access.

### 15.6 Shell, resources, usage, and sessions

1. Working and Done resize/collapse/restore as independent Paneview panels.
2. Left rows remain compact; hover, keyboard focus, and accordion expose equivalent deterministic
   details.
3. Session Catalog opens separately and selection alone never launches a session.
4. Resource compact view and full workspace show the same ownership/totals; only owned processes
   can be stopped.
5. Workspace Space treemap/table uses the same safe worktree delete authority.
6. Usage compact and full analytics agree on source/reset values while keeping subscription limits
   distinct from token/cost estimates.
7. Paneview and Dockview layouts restore per session/workspace without duplicating panel services.

### 15.7 AI assistance

1. Draft a PR title/body, review code, and propose a run configuration through the shared recipe
   contract.
2. Fill selected form fields and run pre-save advisory validation without silently saving.
3. Change deterministic facts after proposal generation; stale `factsHash` prevents apply.
4. Hostile PR/page/repository text remains delimited data.
5. A confirmed consequential action produces one audit receipt; cancel produces no mutation.

---

## 16. Focused tests to add

At minimum:

```text
scripts/agentOwnership.test.mjs
scripts/agentCapabilities.test.mjs
scripts/agentConfigOptions.test.mjs
scripts/conversationRichTimeline.test.mjs
scripts/conversationImageAttachments.test.mjs
scripts/conversationCommandPolicy.test.mjs
scripts/conversationTerminalBridge.test.mjs
scripts/conversationHandoff.test.mjs
scripts/agentHierarchy.test.mjs
scripts/workflowDefinition.test.mjs
scripts/workflowReducer.test.mjs
scripts/workflowCoordinator.test.mjs
scripts/workflowLimits.test.mjs
scripts/workflowRecovery.test.mjs
scripts/paneviewPersistence.test.mjs
scripts/activeWorkNavigation.test.mjs
scripts/sessionCatalog.test.mjs
scripts/browserMarkup.test.mjs
scripts/browserContextAttachments.test.mjs
scripts/resourceViewModel.test.mjs
scripts/providerUsage.test.mjs
scripts/usageAnalytics.test.mjs
scripts/aiAssistRecipes.test.mjs
```

Rust suites cover ACP process supervision, generation fences, adapter fixtures, terminal bridge,
attachment content blocks, ownership transfer, incremental transcript cursor/rotation, workflow
coordinator limits/cancellation/recovery, browser capture safety, resource ownership, and redaction.

No lane closes from browser-preview mocks alone. ACP processes, images, PTYs, native webviews,
process resources, and handoffs require rebuilt Tauri evidence.

---

## 17. Reference implementations and adoption decisions

These are implementation references, not blanket dependencies or permission to copy architecture
wholesale:

| Reference | Useful pattern | Assembly decision |
| --- | --- | --- |
| `agentclientprotocol/rust-sdk` | Native ACP client/session/transport types | Use as the Rust protocol boundary |
| `agentclientprotocol/codex-acp` | Codex app-server normalization, config options, images, tools, plans, subagents | Package a pinned adapter; keep Assembly canonical events |
| `agentclientprotocol/claude-agent-acp` | Claude Agent SDK, permissions, images, tasks, terminals, nested transcripts | Package after explicit runtime/distribution proof |
| `pingdotgg/t3code/packages/contracts/src/providerRuntime.ts` | Provider-neutral event/item/request taxonomy | Adapt the useful semantic taxonomy, not Effect/server architecture wholesale |
| `recailai/jockey/src-tauri/src/acp/runtime_state.rs` | Runtime-discovered models, modes, config options, and commands | Use UI-driven discovery; do not hard-code provider lists |
| `xintaofei/codeg/src-tauri/src/acp/delegation/` | App-owned delegation broker, depth, cancellation, linked child sessions | Use as a reference for WorkflowCoordinator and model-accessible tools |
| `stablyai/orca/.../BrowserPane.tsx` | Persistent browser, element grab/annotation, markup, profiles, viewport controls | Recreate against Tauri child webviews and Assembly context/attachment contracts |
| `stablyai/orca/.../ResourceUsageStatusSegment.tsx` | Compact resource hierarchy and ownership controls | Split compact Resource surface from full Resources workspace |
| `stablyai/orca/.../WorkspaceSpaceManagerPanel.tsx` | Treemap, inspection, table, reclaim decisions | Reuse product pattern over Assembly's existing worktree safety authority |
| `stablyai/orca/.../UsageRosterPanel.tsx` and `StatsPane.tsx` | Compact provider limits plus full analytics | Use normalized provider collectors and Assembly Paneviews |
| `stablyai/orca/.../WorktreeCard*` | Compact row, hover identity, optional detail sections | Apply to narrow active-work navigation without importing Orca state architecture |
| `mathuo/dockview/packages/dockview-core/src/paneview/` | Paneview layout and persistence primitive | Use through one Svelte host adapter for all stacked workbench regions |

License and attribution review is required before copying any nontrivial implementation. Prefer
protocols, interfaces, tests, and independently implemented behavior.

---

## 18. Completion definition

This amendment is complete only when:

- Codex and Claude structured sessions run through one ACP-based manager with dynamic selectors,
  real image attachments, typed tools/tasks/plans, approvals, terminal bridge, and live nested
  output;
- native CLI handoff preserves one writer and exact history;
- workflows can coordinate configurable cross-provider roles with visible loops, artifacts,
  decisions, limits, pause/cancel/retry, and restart recovery;
- browser element grab, annotation, and screenshot markup stage exact context into the matching
  conversation or workflow;
- Resources, Workspace Space, Usage, Agent Control, and Sessions are real workspaces with compact
  entry surfaces where specified;
- active navigation is narrow, Working/Done and other stacked regions use Paneview, and machine-wide
  session discovery is separate;
- AI assistance across PRs, checks, forms, saves, run configurations, browser feedback, and cleanup
  uses one preview-first, schema-validated, stale-safe recipe system;
- existing Terminal, Monaco/Roslyn, Git, worktree safety, Dockview workspace restoration, and
  provider-native history remain intact.
