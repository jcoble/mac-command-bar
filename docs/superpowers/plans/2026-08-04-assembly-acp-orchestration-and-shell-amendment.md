# Assembly ACP Runtime, Workflow Orchestration, and Shell Surface Amendment

**Date:** 2026-08-04  
**Status:** Active planning authority for the areas named below; implementation is still paused.  
**Baseline:** `origin/main` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`.  
**Applies to:**

- `2026-08-01-tsk-808-native-workbench-product-wave.md`
- `2026-08-02-tsk-809-810-conversation-workbench.md`

This amendment supersedes conflicting PTY-only conversation assumptions in those plans and expands the underspecified agent orchestration, browser, resource, usage, session-navigation, Paneview, and integrated-AI work. All unaffected safety, Git, editor, worktree, settings, Roslyn, testing, and dispatch requirements in the product-wave plan remain in force.

---

## 1. Product decisions locked by this amendment

1. **New Claude Code and Codex conversations are structured-first.** Assembly starts one ACP-backed provider session and renders its typed events. A PTY is not created merely because a conversation exists.
2. **ACP is a protocol connection, not an xterm session.** An ACP sidecar is a child process with JSON-RPC over stdio. It can report shell commands and terminal output as tool activity, but it is not the user-visible native CLI terminal.
3. **One native conversation has one writer at a time.** The writer is either Assembly's structured runtime or a native CLI in Assembly's existing PTY service. Switching views performs an explicit ownership handoff; it never runs both writers concurrently.
4. **The current PTY, transcript scanner, and durable provider logs remain valuable.** They become the terminal-owned projection, import, recovery, and read-only history path rather than the universal live conversation engine.
5. **Model, reasoning effort, permission/execution mode, fast/service tier, and similar provider settings are first-class selectors.** They are populated from ACP session configuration options and provider capabilities. They are not presented as slash commands.
6. **The structured slash menu contains only meaningful provider-advertised commands and Assembly-local actions.** Codex TUI-only commands such as terminal theme, keymap, raw-scrollback, pet, title-line, and exit aliases are omitted from structured mode.
7. **Pasting screenshots into the composer is release-blocking priority work.** Structured mode sends validated images as ACP image content blocks. Terminal-owned mode retains the managed-file-path fallback. Neither path duplicates or loses the attachment.
8. **Typed tool calls, plans/tasks, approvals, questions, command output, file changes, reasoning, and subagents are timeline items, not text inferred from prose.**
9. **ACP is not the workflow scheduler.** Assembly owns workflow definitions, run state, dependencies, retries, concurrency, approvals, and audit events. ACP is the provider-neutral transport used to execute agent nodes.
10. **Agent delegation is an app-owned typed tool path.** An orchestrator may request approved child roles through an Assembly MCP companion and delegation broker. The broker, not the model, validates provider, role, working directory, depth, budget, concurrency, and lifecycle.
11. **Native provider subagents and Assembly workflow agents are related but distinct.** Native subagents are observed and rendered from provider metadata. Workflow children are Assembly-owned sessions with explicit parent, node, attempt, and tool-use identities.
12. **Every left/right/bottom workbench group uses Dockview Paneview.** Custom flex accordions are not layout authorities. Center destinations remain Dockview panels. Dialogs, the global floating action island, tooltips, and full-shell overlays are the only intentional non-Paneview presentation surfaces.
13. **Session discovery is not mixed into the active-work sidebar.** Resume/search/history moves to a dedicated Session Library panel. The left side stays compact and centered on projects, workspaces, and active owned work.
14. **The Browser has one live state with multiple presentations:** docked, floating peek, and expanded full-workbench overlay. Presentation changes must not duplicate or reload the native child webview.
15. **Browser feedback supports both DOM-target annotations and screenshot markup.** Both become reviewable context attachments for the exact owned conversation; neither auto-submits.
16. **Resources, Workspace Space, and Usage each have a compact status/popover surface and a full Paneview/Dockview workspace.** Compact and full views consume one store and one native authority.
17. **AI assistance is integrated through one guarded assistance service.** PR drafting/review, run-configuration generation, browser feedback, form completion, pre-save review, worktree proposals, and configuration checks share the same fact/context and confirmation system. No feature creates a private model client or bypasses typed native services.

---

## 2. Research conclusions and implementation references

These projects are references for contracts and tested interaction patterns, not dependencies to fork wholesale.

| Reference | Proven pattern to reuse | Do not copy blindly |
| --- | --- | --- |
| `agentclientprotocol/rust-sdk` | Native Rust ACP client/agent types, stdio connections, capability negotiation, request/notification handling | Draft/unstable protocol features without version gates |
| `agentclientprotocol/codex-acp` | Codex app-server normalization; model, effort, fast mode, permissions, images, reasoning, plans, commands, file changes, MCP, reviews, subagents, usage | Its deliberately small slash-command catalog as Assembly's whole command system |
| `agentclientprotocol/claude-agent-acp` | Official Claude Agent SDK bridge; images, permissions, TODOs, edit review, terminals, commands, nested subagent metadata | Assuming Node is packaged; the current adapter requires a compatible Node runtime unless bundled separately |
| `pingdotgg/t3code` | Provider-neutral item lifecycle and content-delta taxonomy; adapters isolated from UI | Its full Effect server/event infrastructure in a local Tauri app |
| `recailai/jockey` | Tauri/Rust ACP host that discovers models, modes, config options, and available commands without an xterm | Its product/session database model as a drop-in replacement for `ownedId` |
| `xintaofei/codeg` | MCP companion -> token-auth broker -> child ACP session; parent tool-use linkage; depth, cancellation, per-agent defaults, terminal outcomes | One-shot delegation as the final workflow model; Assembly also needs persistent child sessions and graph runs |
| `stablyai/orca` | Persistent browser state, DOM grab/annotation, screenshot markup, compact/full resource and usage surfaces, workspace-space treemap, compact worktree rows and hover detail | Electron webview/process code; Assembly must implement the behavior through Tauri child webviews and Rust authorities |

The concrete direction is therefore:

- **Jockey-style ACP hosting** for provider discovery and session controls.
- **T3-style canonical event shapes** reduced to Assembly's actual needs.
- **CodeG-style delegation broker** extended into a durable workflow engine.
- **Orca-style interaction states** reimplemented against Assembly's Tauri, Dockview, Paneview, PTY, worktree, and owned-session architecture.

---

## 3. Runtime architecture

### 3.1 Preserve existing authorities

The following remain authoritative and are extended, not replaced:

- `ownedId` for the complete Assembly session/workspace identity.
- `TerminalRegistry` and `terminalService.ts` for user-visible native CLI terminals.
- `sessionWorkspaces.ts` for per-ownedId editor/browser/diff/layout restoration.
- the current attachment vault and its canonical owner-scoped paths.
- provider transcript scanners for discovery, terminal projection, import, and recovery.
- the existing append-only orchestration event abstraction for audit and workflow replay.
- Dockview for center destinations and Paneview for side/bottom groups.

Provider-native IDs remain metadata beneath `ownedId`:

```ts
export interface ProviderSessionBinding {
  provider: 'codex' | 'claude' | string;
  runtimeKind: 'acp' | 'terminal-transcript';
  nativeSessionId: string | null;
  runtimeInstanceId: string | null;
  adapterVersion: string | null;
  protocolVersion: string | null;
}
```

### 3.2 Explicit execution ownership

Extend `OwnedSession` additively:

```ts
export type AgentExecutionOwner =
  | { kind: 'structured'; runtimeInstanceId: string }
  | { kind: 'terminal'; ptySessionId: string }
  | { kind: 'transitioning'; from: 'structured' | 'terminal'; to: 'structured' | 'terminal' }
  | { kind: 'stopped' };

export type AgentActivityState =
  | 'starting'
  | 'idle'
  | 'working'
  | 'waiting-for-approval'
  | 'waiting-for-input'
  | 'paused'
  | 'completed'
  | 'failed';
```

Rules:

1. `executionOwner` is persisted with the owned-session record.
2. The Rust manager enforces one writer lease per `ownedId` and native session ID.
3. A frontend mode toggle cannot change ownership on its own; it invokes a typed handoff command.
4. A failed handoff restores the previous owner and its visible surface.
5. Stale generation or lease tokens cannot send prompts, interrupt turns, answer approvals, or apply config changes.
6. Merely switching Assembly sessions never stops or resumes a provider.

### 3.3 Structured session lifecycle

Create/extend under `src-tauri/src/agent_conversation/`:

```text
manager.rs
acp_client.rs
acp_sidecar.rs
capabilities.rs
canonical_events.rs
event_buffer.rs
handoff.rs
transcript_projection.rs
providers/codex.rs
providers/claude.rs
```

`AgentConversationManager`, keyed by `ownedId`, owns:

- pinned adapter process supervision;
- ACP initialize/auth/new/load/resume/close calls;
- prompt, cancel, permission response, user-input response, and config-option calls;
- native provider/session IDs;
- monotonic generation and sequence;
- bounded recent-event snapshots for frontend resync;
- durable normalized-event append;
- structured/terminal writer leases;
- crash/restart and handoff state.

New-session sequence:

1. The user chooses provider, project/worktree, model, effort, and permission mode in New Session.
2. Rust starts the pinned ACP adapter and performs `initialize`.
3. Assembly reads capabilities and configuration options.
4. Rust calls new session with cwd, selected options, additional directories, and client capabilities including images and nested subagent transcripts when supported.
5. The provider returns the native session ID before Assembly marks the session ready.
6. The new `OwnedSession` is stored as structured-owned and the Session panel opens without creating an xterm view.
7. A user-selected **Open native CLI** action may later transfer ownership.

Resume sequence is the same except Assembly loads/resumes the native session ID and imports provider history before accepting input.

### 3.4 ACP sidecar packaging

Add one provider manifest:

```rust
pub struct AgentAdapterManifest {
    pub id: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub expected_version: String,
    pub supported_platforms: Vec<String>,
    pub environment_allowlist: Vec<String>,
}
```

Requirements:

- Pin adapter and provider versions in source and build metadata.
- Never run `npx ...@latest` at session start.
- Bundle the Codex ACP standalone macOS binary or build an equivalent pinned sidecar.
- Run a bounded packaging spike for Claude Agent ACP: either package a standalone executable or bundle a known Node 22+ runtime and the pinned adapter. Do not depend on an arbitrary GUI-app `PATH` without a visible unsupported-state explanation.
- Verify adapter hash/version on launch and expose it in diagnostics.
- Capture bounded stderr rings; never place secrets or complete environments in diagnostics.
- Capability-gate every optional feature and preserve unknown ACP `_meta` values for forward compatibility.

### 3.5 ACP terminal semantics

ACP tool activity may request a terminal or emit terminal output. That is not the same as opening the provider's native CLI.

- Tool terminals are child resources of a timeline item and are rendered inline or opened in a tool-output Paneview.
- If ACP asks the client to create a terminal, route it through a restricted `AgentToolTerminalRegistry` that records the parent ownedId, turn, item, command, cwd, and lifecycle.
- Tool terminals cannot receive arbitrary user keystrokes unless the protocol explicitly asks for interaction and the user opens that terminal.
- Native CLI handoff continues to use the existing top-level `TerminalRegistry` and login-shell behavior.
- Resource Manager identifies tool terminals separately from user-visible session PTYs.

### 3.6 Structured <-> native CLI handoff

State machine:

```text
StructuredOwned
  -> PreparingTerminal
  -> TerminalOwned
  -> PreparingStructured
  -> StructuredOwned
```

**Open native CLI**:

1. Reject while an approval or form response is being submitted.
2. Ask whether to finish or interrupt an active turn.
3. Flush/persist canonical events and native ID.
4. Release the structured writer lease and close/detach the ACP session connection.
5. Start one existing PTY with the exact provider resume command in the same cwd/worktree.
6. Start the incremental transcript projection and mark terminal-owned only after PTY start succeeds.

**Return to Conversation**:

1. Stop or detach the native CLI with a visible confirmation if it is active.
2. Read only transcript bytes appended since the terminal handoff and import them by native item ID.
3. Start/resume the ACP adapter and acquire the structured lease.
4. Reconcile provider history with the canonical timeline.
5. Switch presentation only after the provider confirms the current session ID and options.

Codex remote app-server/TUI sharing may be researched as a later optimization. The initial implementation does not depend on two simultaneous clients sharing one thread.

### 3.7 Incremental transcript projection

Replace the active 500 ms full-tail reread with a Rust watcher:

```rust
pub struct TranscriptCursor {
    pub canonical_path: PathBuf,
    pub file_identity: FileIdentity,
    pub byte_offset: u64,
    pub partial_line: Vec<u8>,
    pub generation: u64,
}
```

- Parse only appended complete JSONL lines.
- Detect truncation, rotation, archive moves, and inode/file-ID changes.
- Emit the same canonical events as ACP where durable evidence exists.
- Mark unsupported live capabilities false rather than inventing approval or turn control.
- Reconcile with a bounded snapshot after rotation, return from terminal, or detected sequence gap.
- Keep the last valid timeline on read failure.

---

## 4. Canonical conversation model

### 4.1 Event and item taxonomy

Expand the existing generation/sequence protocol instead of creating a second store.

```ts
export type CanonicalAgentItemType =
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

export type CanonicalContentChannel =
  | 'assistant'
  | 'reasoning'
  | 'reasoning-summary'
  | 'plan'
  | 'command-output'
  | 'file-change-output';
```

Every item carries:

- stable Assembly item ID;
- provider item ID and raw source reference when known;
- ownedId, turn ID, parent item/tool-use ID, workflow node/attempt IDs when applicable;
- status `pending | in-progress | completed | failed | declined | cancelled`;
- start/update/end timestamps;
- bounded provider `_meta` extension data;
- typed locations, command, diff, task steps, or result rather than prose parsing.

Events include session/turn/item lifecycle, content deltas, approvals, structured user input, config updates, usage, children, warnings, and errors. The reducer must replay deterministically and reject duplicates, stale generations, and gaps exactly as the existing reducer does today.

### 4.2 Durable event authority

Extend the existing append-only orchestration storage rather than introducing a parallel transcript database in the first implementation wave.

- `schemaVersion`, event ID, ownedId, workflow/run/node/attempt IDs, provider/native IDs, sequence, and timestamp are required.
- Append is serialized through one Rust queue and flushed before acknowledging consequential state transitions.
- A compact projection snapshot is written by temp-file + fsync + atomic rename every bounded number of events.
- Startup loads the snapshot then replays the later log suffix.
- Compaction creates an archived immutable log before replacing the active segment.
- Provider transcript bodies are not copied wholesale; only normalized timeline events and bounded raw references are retained.
- A later SQLite migration requires a separate measured decision; it is not smuggled into an ACP lane.

### 4.3 Conversation UI

Split `ConversationSurface.svelte` into composition components while retaining one store/service:

```text
ConversationHeader.svelte
ConversationTimeline.svelte
ConversationTimelineItem.svelte
ConversationComposer.svelte
ConversationConfigBar.svelte
ConversationCommandMenu.svelte
ConversationTaskPanel.svelte
ConversationAgentTree.svelte
ConversationApprovalCard.svelte
ConversationUserInputCard.svelte
```

Required behavior:

- Real sanitized Markdown, selectable text, code highlighting, copy actions, file/line links, tables, and task lists.
- Compact disclosure rows for tools, commands, file changes, and reasoning; narration remains dominant.
- Plans/tasks remain visible as structured steps with current status and links to workflow nodes where applicable.
- Approvals and provider questions appear inline and target the exact request ID/generation.
- Long output is bounded/virtualized and can open in a dedicated Paneview.
- Auto-follow only while the user is near the bottom; otherwise show **Jump to latest**.
- Parent and every child retain independent scroll positions.

### 4.4 Model, effort, permissions, and modes

`ConversationConfigBar` renders ACP session configuration options dynamically.

```ts
export interface AgentConfigOption {
  id: string;
  category: 'model' | 'reasoning' | 'permission' | 'mode' | 'service-tier' | 'boolean' | 'unknown';
  label: string;
  description: string | null;
  value: string | boolean | null;
  choices: Array<{ value: string; label: string; description: string | null }>;
  mutable: boolean;
  source: 'acp' | 'provider-extension' | 'terminal-observed';
}
```

- Main controls are Provider, Model, Effort/Thinking, Permission/Execution mode, and optional Fast/Service tier.
- Unknown options live under **More** without being discarded.
- A value changes visually only after the ACP response/update confirms it.
- If switching model invalidates effort, use the provider-advertised default and show the change.
- Terminal-owned sessions show observed values and an **Open provider picker in terminal** fallback where safe.
- New-session defaults are separate from live-session values.

### 4.5 Commands

The slash/command surface merges only:

1. commands advertised by the active ACP session;
2. discovered provider skills/prompts using their supported invocation syntax;
3. Assembly-local actions that make sense in a composer, such as attaching context or opening the terminal.

Do not import the full Codex TUI enum. Do not show TUI-local commands for keymaps, terminal theme, raw scrollback, pets, title/status line, app handoff, or exit aliases in structured mode. Model, effort, permission, mode, and service tier remain selectors. Selecting a command inserts or executes according to its typed descriptor and never auto-sends merely because the menu row was highlighted.

### 4.6 Screenshot and image attachments — priority packet

Keep and expand the existing Rust attachment vault.

- Accept clipboard paste, file picker, drag/drop, browser capture, and browser markup.
- Validate decoded PNG/JPEG/GIF/WebP signatures, per-file limit, aggregate count/bytes, canonical owner-scoped destination, and symlink/path escape.
- Persist attachment IDs and managed paths, never blob URLs.
- Show immediate preview and upload/preparation state.
- Restore previews across app restart; a missing file becomes removable unavailable context.
- Structured ACP send creates one prompt containing text blocks plus actual image content blocks.
- Terminal send appends exact managed paths once through bracketed paste.
- Clear and delete managed attachments only after the matching send succeeds.
- A failed or stale send restores the exact draft and attachment order.
- Capture `ownedId`, generation, and writer lease before asynchronous paste/save/send and reject mismatches.

Native acceptance is not complete until two screenshots can be pasted, one removed, the other sent to both a Codex and Claude structured session, and the provider demonstrably receives the image without a duplicate process or path-only placeholder.

### 4.7 Subagents and tasks

Two sources feed one recursive tree:

- provider-native subagent metadata/events through ACP and provider `_meta`;
- Assembly workflow/delegation child sessions from the workflow engine.

Each child has parent, relation, provider, model/effort, role, status, task, timestamps, workflow node/attempt, native/session IDs, transcript availability, and output summary. Provider-native unknowns remain unknown. Workflow children are controllable only through workflow actions; provider-native children are read-only unless the provider advertises an exact control.

Selecting a child opens its typed timeline beneath the same owned workspace. The tree supports recursive expansion, keyboard navigation, cycles/depth caps, and stale-response protection.

---

## 5. Workflow orchestration and agent control

### 5.1 Boundary

The workflow engine is a deterministic Rust service. An LLM can recommend or request allowed transitions, but prose never determines run state. Every transition is a typed command validated against the current workflow definition and run projection.

Create under `src-tauri/src/orchestration/workflow/`:

```text
definition.rs
commands.rs
decider.rs
events.rs
projection.rs
scheduler.rs
worker.rs
delegation.rs
artifacts.rs
recovery.rs
```

Frontend:

```text
shell/workflows/workflowTypes.ts
shell/workflows/workflowService.ts
shell/workflows/workflowStore.svelte.ts
components/workflows/WorkflowWorkspace.svelte
components/workflows/WorkflowGraph.svelte
components/workflows/WorkflowRunTree.svelte
components/workflows/WorkflowNodeDetail.svelte
components/workflows/WorkflowEventTimeline.svelte
components/workflows/WorkflowTemplateEditor.svelte
```

### 5.2 Durable types

```ts
export interface WorkflowDefinition {
  id: string;
  version: number;
  name: string;
  description: string;
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdgeDefinition[];
  policy: WorkflowPolicy;
}

export type WorkflowNodeKind =
  | 'agent-task'
  | 'human-approval'
  | 'deterministic-check'
  | 'run-configuration'
  | 'fan-out'
  | 'join'
  | 'finish';

export interface WorkflowNodeDefinition {
  id: string;
  kind: WorkflowNodeKind;
  title: string;
  roleId: string | null;
  providerSelector: ProviderSelector | null;
  modelSelector: ModelSelector | null;
  effort: string | null;
  permissionMode: string | null;
  workingDirectoryPolicy: 'same-worktree' | 'new-worktree' | 'read-only-root';
  promptTemplate: string | null;
  inputBindings: WorkflowInputBinding[];
  successCriteria: WorkflowSuccessCriterion[];
  retry: { maxAttempts: number; backoffMs: number; retryableCodes: string[] };
  timeoutMs: number | null;
  concurrencyGroup: string | null;
}

export interface WorkflowRunProjection {
  id: string;
  definitionId: string;
  definitionVersion: number;
  status: 'queued' | 'running' | 'waiting' | 'paused' | 'completed' | 'failed' | 'cancelled';
  rootOwnedId: string;
  projectRoot: string;
  worktreeRoot: string;
  currentNodeIds: string[];
  createdAt: string;
  updatedAt: string;
}

export interface WorkflowNodeAttempt {
  id: string;
  runId: string;
  nodeId: string;
  attempt: number;
  status: 'queued' | 'starting' | 'running' | 'waiting' | 'succeeded' | 'failed' | 'cancelled';
  ownedId: string | null;
  parentAttemptId: string | null;
  parentToolUseId: string | null;
  provider: string | null;
  nativeSessionId: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  failureCode: string | null;
}
```

### 5.3 Scheduler rules

- One serialized command queue decides events from command + current projection.
- Side effects run in drainable workers after the event is durably appended.
- Per-provider, per-worktree, and named concurrency-group limits are explicit.
- A node cannot run before all incoming edge conditions are satisfied.
- Retries create a new attempt; they never rewrite a failed attempt.
- Cancellation propagates from workflow -> node attempts -> ACP turn/session according to policy.
- Parent cancellation while a child is spawning is fenced and deterministic.
- App restart replays events, marks uncertain in-flight operations `reconciling`, asks the provider/runtime for session state, then resumes or surfaces a decision. It never silently reruns a consequential node.
- Human approvals and destructive native actions pause the graph and name exactly what is waiting.
- Loops are explicit graph edges with a maximum iteration count and stop criteria; an orchestrator cannot create an unbounded loop through prose.

### 5.4 Delegation broker

Expose a local, token-authenticated MCP companion to structured providers with typed tools:

```text
delegate_agent
get_agent_status
continue_agent
cancel_agent
publish_agent_artifact
request_workflow_approval
```

`delegate_agent` arguments are bounded and schema-validated:

```ts
export interface DelegateAgentRequest {
  workflowRunId: string;
  parentAttemptId: string;
  parentToolUseId: string;
  roleId: string;
  task: string;
  workingDirectory: string | null;
  requestedProvider: string | null;
}
```

The broker:

1. authenticates the companion and resolves the exact parent ACP connection;
2. verifies the workflow/run/node is allowed to delegate;
3. validates role, provider allow-list, model/effort defaults, depth, concurrency, budget, and canonical cwd/worktree;
4. appends `delegation-requested` and reserves a child attempt;
5. creates a new Assembly `ownedId` and structured ACP session;
6. sends the child prompt with immutable parent/workflow context;
7. streams child status/output to the Agent Control UI;
8. returns a bounded running/completed/failure report to the parent tool call;
9. retains the child session for inspection or continuation according to workflow policy.

The first release supports persistent children but allows a definition to request one-shot semantics. Every result carries child ownedId, attempt, duration, token usage when provided, and an artifact/output reference. Depth, max child count, time, and token/cost budgets are mandatory.

### 5.5 Workflow templates

Ship editable, versioned templates rather than hard-coded orchestration prompts:

1. **Implement -> Code Review -> Spec Compliance -> Test/Fix loop**
2. **Investigate -> Propose Plan -> Human Approval -> Implement**
3. **Fix failing checks -> Review diff -> Prepare PR**
4. **Parallel implementation lanes -> Join -> Integration review**

A template defines roles, provider/model defaults, worktree policy, max parallelism, retries, and approval gates. Role examples include orchestrator, implementer, reviewer, spec-compliance reviewer, tester, debugger, and documentation reviewer.

The orchestrator may choose among pre-authorized branches or request a delegation, but it cannot bypass gates, rewrite the graph, choose an unapproved provider, or issue arbitrary shell commands. Shell work uses existing validated Run configurations or provider tool calls under their permission policy.

### 5.6 Agent Control workspace

Add a center Dockview destination **Agent Control** and optional compact right Paneview summary.

The full workspace shows:

- workflow/run selector and template version;
- graph with active, waiting, succeeded, failed, and cancelled nodes;
- hierarchical run/agent tree;
- each agent's provider, model, effort, role, cwd/worktree, elapsed time, current item, attempt, token usage, and last event;
- plan/task progress and loop iteration;
- approval/attention queue;
- event timeline and artifacts;
- actions: Open conversation, Open terminal when owned, Open diff, Pause run, Resume, Cancel, Retry failed node, Approve/Decline, Continue child, and Copy handoff.

No status is inferred from assistant text. The compact summary may say `2 running · 1 review · waiting for approval`, but every count comes from workflow projections and provider events.

---

## 6. Browser presentation, inspection, annotation, and markup

This section expands product-wave Work Package 8.

### 6.1 One browser state, four presentations

```ts
export type BrowserPresentationMode =
  | 'collapsed'
  | 'docked'
  | 'floating'
  | 'expanded';
```

- **Collapsed:** only the global bottom-right action island remains.
- **Docked:** Browser center destination inside Dockview.
- **Floating:** a resizable, draggable, bottom-right workbench overlay using roughly 55-70% width and 55-70% height, with bounded minimums and snap-back. It overlays workbench content without replacing the active center tab.
- **Expanded:** fills the Assembly workbench below native macOS window chrome and overlays rails/panes.

All presentations use the same native child webview/tab/profile. The implementation hides the child view, moves/rebounds the HTML chrome, measures final bounds, then shows that same child. URL, scroll, forms, cookies, history, profile, viewport, annotation queue, and selected tab must not reload.

The floating and expanded controls mirror the evidenced actions:

- Import/context action;
- Grab page element;
- Annotate page element;
- Draw on screenshot;
- Devtools when development policy allows;
- Open in default browser;
- profile, cookie-import-disabled/security state, viewport, and browser settings menu;
- Maximize/Restore and Collapse.

### 6.2 Element grab vs annotation

**Grab page element** captures bounded context for direct copy/attachment:

- URL/title;
- selector, tag, role, accessible name, bounded text;
- viewport/page rectangle and device scale;
- optional cropped screenshot;
- source hash and capture time.

It opens a confirmation sheet with Copy, Attach to draft, and Cancel. It does not create a feedback note automatically.

**Annotate page element** arms the inspector, outlines the hovered/selected element, then opens a card anchored away from the target when possible. The card contains:

- selected element identity;
- note textarea;
- Change/Question intent;
- Add and Cancel;
- `Cmd+Enter` Add.

Add queues the immutable annotation. It does not send it.

### 6.3 Draw on screenshot

`Draw on screenshot`:

1. captures the active visible page viewport through the native browser authority;
2. stores the unmodified screenshot in the owner-scoped attachment vault;
3. opens a main-webview markup canvas above the hidden child view;
4. supports pen, arrow, rectangle, highlight, text, color, stroke size, undo/redo, clear, crop, cancel, and save;
5. saves a new annotated image while preserving the original until the user discards it;
6. queues the markup as a `BrowserContextAttachment` with URL, viewport, timestamp, and optional note.

Drawing is not attempted directly over the child WKWebView. Main-webview markup guarantees z-order, input, accessibility, and exact image output.

### 6.4 Conversation attachment

```ts
export interface BrowserContextAttachment {
  id: string;
  workspaceId: string;
  tabId: string;
  generation: number;
  kind: 'element' | 'annotation' | 'markup';
  pageUrl: string;
  pageTitle: string;
  selector: string | null;
  accessibleName: string | null;
  textSnippet: string | null;
  note: string | null;
  intent: 'change' | 'question' | null;
  imageAttachmentId: string | null;
  sourceHash: string;
}
```

Copy All produces bounded Markdown. **Attach to Conversation** adds typed text/image blocks to the exact ownedId draft through the one conversation service. Existing nonempty drafts require a merge preview. Owner/generation/hash mismatch retains the queue and explains why. Nothing auto-submits.

### 6.5 Native spike and security remain blocking

The product-wave child-WKWebView clipping, z-order, input, profile isolation, HTTP/S validation, and dialog layering spike remains mandatory. Remote child views receive no Tauri capabilities. The inspector exposes bounded metadata only—not arbitrary eval, HTML, cookies, storage, or tokens.

---

## 7. Resource Manager, Workspace Space, and Usage

This section expands Work Package 9 and separates three user concepts that currently risk being mixed.

### 7.1 Resource Manager

**Compact entry:** a status-bar segment showing total Assembly-owned RSS, CPU, and active process/session count.  
**Popover:** grouped project -> workspace/worktree -> session/tool/LSP/browser rows with sparklines, CPU, RSS, port, bound/unbound state, and exact ownership.  
**Full workspace:** center Dockview panel **Resources** with filters, process tree, language servers, ports, logs, ownership evidence, and safe stop/restart actions.

Rules:

- Sample at two seconds only while the popover/full workspace is visible; use a slower bounded cadence only when a setting explicitly enables background alerts.
- Join OS snapshot to Terminal/LSP/browser/provider registries once in Rust.
- External or uncertain processes have no Stop action.
- Stop revalidates PID, PGID, registry generation, owner, and cwd, then asks for confirmation when state could be lost.
- Tool terminals and provider sidecars appear separately from user-visible PTYs.
- Compact and full views share one store and sample ring.

### 7.2 Workspace Space

**Compact section:** inside Resource popover, showing scanned, reclaimable, workspaces, and updated time with Scan/Cancel/Review.  
**Full workspace:** center Dockview panel **Space** with treemap, selected-workspace breakdown, filter/sort table, safety states, and confirmation-gated cleanup.

The scan returns exact workspace/worktree ownership, top-level item sizes, build/cache classes, protected reasons, active agents, terminals, dirty buffers, browser tabs, Git state, PR/issue links, and reclaimable classification. Scan cancellation keeps the last result. Delete delegates to the existing worktree safety authority; Space never invents another removal path.

### 7.3 Provider Usage and Assembly analytics

**Compact entry:** status-bar usage roster for Claude, Codex, and later providers.  
**Popover:** Detailed/Compact density, each provider's authoritative windows/buckets/reset times and account state, refresh, Usage details & history, and Manage Accounts.  
**Full workspace:** center Dockview panel **Usage** with:

- agents spawned, time agents worked, PRs created;
- provider selector and overview;
- daily intensity;
- input/output/cache/reasoning mix where provider data supports it;
- per-provider sessions, turns/events, model, rate windows, and unavailable reason;
- no invented dollar cost when pricing or token classification is unknown.

Sources are explicit: ACP token/rate events, provider account/rate APIs exposed by the adapter, and Assembly's own canonical event log. Local transcript estimates are labelled estimates. History uses bounded daily aggregates derived from canonical events; raw secrets and provider responses are never stored.

---

## 8. Shell navigation and Paneview contract

This section amends the current session-card and shell-layout packets.

### 8.1 Left navigation

The left side becomes a compact project/workspace and active-work navigator inspired by the supplied Orca/ChatGPT references, without copying their product taxonomy.

- Default width target: 220-280 CSS pixels; compact rows around 28-34 pixels.
- Projects/repositories are headings; workspaces/worktrees and active sessions nest beneath them.
- The selected workspace/session is one concise row, not a full-width dashboard card.
- Hover/focus opens a bounded HoverCard with provider, model/effort, role, branch/worktree, activity, age, last turn, PR/check state, ports, and context usage when known.
- A chevron opens an inline accordion with the existing explicit actions and full metadata.
- Hover is supplemental; keyboard/focus and explicit expansion expose the same facts.
- Long names truncate; full identity is always available in tooltip/HoverCard.

### 8.2 Working, Done, and Settled are Paneview panels

Replace custom flex pane ownership with a left-region Dockview Paneview instance:

```text
assembly-left-paneview
  working-sessions
  done-sessions
  settled-sessions
```

Each is a registered Paneview panel with header, count, collapse state, minimum/maximum size, independent scroll, reorder policy, and persisted layout. Lifecycle state is still user-owned and independent from process state. Collapsing or moving a pane does not mutate sessions.

The current session-card view model and actions are reused in compact-row form. No new session store or route is created.

### 8.3 Session Library is separate

Remove **Find a session** from `SessionsColumn.svelte`. Add a dedicated **Session Library** center Dockview destination, optionally mirrored as a right Paneview panel at narrow scope.

It provides:

- search across provider, title, first prompt, project, branch, task, PR, model, and date;
- project/provider/date/status filters;
- compact rows with HoverCard details;
- exact Resume, Continue in New, Fork, Inspect transcript, Copy ID/path, and Reveal actions;
- pagination/virtualization and stale/missing-worktree reasons;
- no agent launch on row expansion or hover.

### 8.4 All side/bottom panel groups use Paneview

- Left, right, and bottom regions each own one Paneview registry/layout.
- Explorer, Source Control, Worktrees, Problems, Context, Run configurations, Workflow summary, and other side tools are Paneview panels, not custom collapsible flex children.
- Panel IDs are stable and versioned in layout persistence.
- Center workspaces remain Dockview panels: Session, Editor, Browser, Diff, Git Graph, Pull Requests, Agent Control, Session Library, Resources, Space, Usage, Markdown, and any later full workspace.
- A center workspace may use an internal Paneview only for genuinely resizable subpanels.
- The global floating action island, browser floating/expanded chrome, dialogs, menus, HoverCards, and notifications remain overlay primitives rather than Paneview panels.

### 8.5 Global floating action island

Retain the product-wave global bottom-right action surface. Add context actions only through the central descriptor registry. It opens Browser, Resources, Space, Usage, Session Library, and Agent Control using their existing service/panel IDs. It never implements those features itself.

---

## 9. AI integrated throughout Assembly

### 9.1 One assistance service

Create one guarded `AgentAssistService` over the structured runtime/context system:

```ts
export interface AssistActionDescriptor<Input, Output> {
  id: string;
  title: string;
  inputSchema: string;
  outputSchema: string;
  contextBuilder(input: Input): AgentWorkbenchContext;
  outputDestination: AssistOutputDestination;
  requiresPreview: boolean;
  requiresConfirmation: boolean;
}
```

Surfaces register descriptors; they do not instantiate providers. The service:

- captures bounded deterministic facts and untrusted-context boundaries;
- chooses the configured provider/model/effort through the same runtime manifest;
- creates or reuses an explicit assistance session/turn;
- validates structured output;
- presents source-linked preview;
- applies only through the existing typed service after revalidation/confirmation;
- writes an append-only audit receipt.

### 9.2 Initial assistance actions

- Draft PR title/body from Git facts and selected diff.
- Review a PR/diff and prepare comments without publishing.
- Explain/fix failed checks through a workflow template.
- Generate a Run configuration proposal; save only after preview.
- Turn browser annotations/markup into a structured implementation prompt.
- Suggest worktree cleanup/reorganization without deleting.
- Fill selected form fields from visible deterministic facts.
- **Check before save:** run deterministic validators first, then optional AI advisory checks. AI warnings never masquerade as schema/business-rule errors and never block a save unless the user/config explicitly chooses a guarded policy.
- Validate settings/config combinations and explain consequences.
- Summarize old sessions, worktrees, projects, and workflow runs with fact hashes and stale markers.

No action auto-publishes, auto-merges, deletes, rewrites Git history, changes permissions, or executes arbitrary shell text.

---

## 10. Revised work-package and parallelization plan

### Wave 0 — bounded decision/proof spikes, sequential

1. ACP Rust SDK connection spike with one echo/test agent.
2. Pinned Codex ACP sidecar packaging and real session spike.
3. Claude Agent ACP packaging/Node decision and real session spike.
4. Structured/native single-writer handoff proof on disposable sessions.
5. Tauri child-webview docked/floating/expanded z-order/input proof.
6. Dockview Paneview nested-left/right/bottom persistence proof.
7. Delegation companion/broker proof with parent -> one child ACP session -> typed result.

Each spike ends with a written decision and process cleanup receipt. No broad UI implementation begins before its relevant spike passes.

### Wave 1 — shared contracts, controller-owned and sequential

Freeze:

- `OwnedSession` execution owner/binding fields and migration;
- canonical event/item/request/config types in Rust and TypeScript;
- ACP client capability declaration;
- event append/snapshot/replay contract;
- workflow definition/run/node/event schemas;
- stable center panel and Paneview panel IDs;
- attachment/context block schema;
- provider manifest and packaging locations.

Controller-only shared files include `main.rs`, `Cargo.toml`, `package.json`, `+page.svelte`, root shell roster/layout types, and generated bridge types. Lanes return adapters/registrations; the controller integrates shared seams serially.

### Wave 2 — independent foundations after Wave 1

| Lane | Primary ownership | Dependencies | Parallel with |
| --- | --- | --- | --- |
| ACP runtime host | `agent_conversation/acp_*`, sidecar supervisor, capability state | Wave 1 | Paneview, Browser, Resources, Workflow core |
| Paneview/session navigation | shell layout, compact rows, HoverCards, Session Library | Wave 1 | ACP, Browser, Resources, Workflow core |
| Browser native/presentation | browser Rust + browser frontend directory | browser spike + Wave 1 | ACP, Paneview, Resources, Workflow core |
| Resources/Space/Usage authorities | resource scanners/stores/full workspaces | Wave 1 | ACP, Paneview, Browser, Workflow core |
| Workflow event engine | orchestration/workflow Rust + pure TS types | delegation spike + Wave 1 | ACP, Paneview, Browser, Resources |
| Conversation visual system | timeline/Markdown/item components with fixture adapter | canonical types | provider adapters, Paneview, Browser, Resources |

No lane edits another lane's directory or shared registration files.

### Wave 3 — provider and orchestration integrations

- Codex ACP adapter integration.
- Claude ACP adapter integration.
- Image attachment send/restore path.
- Typed tool/approval/input/task/subagent mapping.
- Transcript incremental projection and handoff reconciliation.
- Delegation broker + MCP companion.
- Workflow scheduler/workers/recovery.
- Agent Control workspace.
- Browser context attachments to conversation.
- Provider usage/rate data to Usage workspace.

Codex and Claude lanes run in parallel after the ACP host is stable. Workflow scheduler and Agent Control may run in parallel only after workflow events/projections are frozen. Browser-to-conversation integration waits for both attachment and browser contracts but does not wait for resource work.

### Wave 4 — guarded AI integrations and product workflows

Parallel recipe lanes may implement PR assistance, run-configuration assistance, browser-feedback assistance, pre-save advisory checks, and worktree/session summaries. They share only `AgentAssistService` descriptors and cannot edit its core or provider runtime.

Workflow templates are separate data/test files and may be built in parallel after the workflow schema is fixed.

### Wave 5 — serialized integration and acceptance

1. Controller integrates root registrations/manifests.
2. One SOL-medium architecture/code review covers runtime ownership, workflow recovery, and security.
3. Focused tests run by lane; heavy Rust/Tauri integration runs serially.
4. Rebuilt native Tauri acceptance covers the complete scenarios below.
5. Only after evidence passes are TSK-808/809/810 and routed sub-tasks considered for closure.

### File-overlap and runner rules

- At most one controller edits each shared seam at a time.
- At most two heavy build/test runners; default schedule uses one.
- Provider integration tests run serially per real provider account/session.
- Browser child-view, Roslyn, and process/resource native proofs are never run concurrently.
- Every lane reports changed files, focused tests, process cleanup, remaining risk, and whether its branch/worktree is clean.

---

## 11. Required native acceptance scenarios

### 11.1 Structured conversation

1. Start fresh Codex and Claude sessions from Assembly with no top-level PTY created.
2. Verify model, effort, permissions/mode, and optional fast tier are authoritative selectors.
3. Paste two screenshots, remove one, send the other, and prove the provider receives the image.
4. Observe reasoning, plan/tasks, command/tool output, file changes, approval, structured question, token usage, and turn completion as typed items.
5. Switch sessions repeatedly while both stream; drafts, attachments, config, scroll, tasks, and children remain isolated.
6. Restart Assembly and restore timeline/session IDs without starting duplicate providers.

### 11.2 Terminal handoff

1. Open one structured session in native CLI.
2. Prove the ACP writer is released before PTY input is accepted.
3. Add a CLI turn, return to Conversation, and reconcile exactly once.
4. Fail a handoff deliberately and prove the previous writer/surface is restored.
5. Process inventory never shows two writers for the same native session.

### 11.3 Subagents and workflow

1. Run Implement -> parallel Code Review and Spec Compliance -> Test/Fix -> Join.
2. Use at least one Codex and one Claude child in the same workflow.
3. Show every child, role, model/effort, node, attempt, current item, elapsed time, and output.
4. Exercise approval wait, one retry, cancellation during spawn, app restart/recovery, max-depth refusal, and concurrency limit.
5. Parent receives a typed delegation result linked to the exact tool call and child ownedId.
6. Native provider subagents appear in the same tree but are labelled provider-native and are not given unsupported controls.

### 11.4 Browser

1. Open Browser from the floating island into the floating overlay, then maximize and restore without reload.
2. Grab one element and attach its exact bounded context.
3. Annotate another with Change/Question and queue it without sending.
4. Draw arrows/text/highlight on a viewport screenshot and attach the resulting image.
5. Merge attachments into a nonempty conversation draft through preview and submit manually.
6. Prove child-webview z-order, scrolling, profile isolation, dialogs, minimum window, keyboard, reduced motion, and VoiceOver.

### 11.5 Paneview/navigation

1. Working, Done, and Settled resize/collapse/reorder as Paneview panels and survive restart.
2. Compact rows fit the narrower left rail; HoverCards and accordions expose the same facts by keyboard.
3. Session Library searches/resumes exact same-title sessions from different worktrees.
4. Every right/bottom group is a Paneview panel; no custom panel silently disappears during layout restore.

### 11.6 Resources, Space, and Usage

1. Resource popover and full workspace show identical totals and exact ownership.
2. External process stop is unavailable; owned stop revalidates and affects one target.
3. Space scan cancels while preserving last results; treemap/table agree; cleanup uses existing worktree confirmations.
4. Usage popover/full workspace agree on authoritative provider windows/reset times and label unavailable/estimated data honestly.
5. Resource/usage sampling stops or slows according to visibility policy and leaves no leaked timers/processes.

### 11.7 Integrated AI

1. Draft a PR, generate a Run configuration, transform browser feedback, and run a pre-save advisory check through one assistance service.
2. Cancel every preview and prove zero mutation.
3. Confirm one disposable action and prove typed revalidation plus one audit receipt.
4. Feed hostile page/review/form text and prove it remains delimited untrusted data.
5. Change source facts and prove summaries/proposals become stale rather than remaining authoritative.

---

## 12. Change map against the earlier plans

### Product-wave plan

- **Architecture and decision summary:** amend “do not add another agent runtime” to mean one upgraded `AgentConversationManager`; do not keep both the dead provider registry and ACP runtime active.
- **Session/layout packets:** replace custom Working/Done collapsibles and in-column Find drawer with Paneview panels plus Session Library.
- **Work Package 8 Browser:** retain native Tauri child-webview security work; add floating mode, exact evidenced toolbar, element Grab, screenshot markup, and typed image/context attachments.
- **Work Package 9 Resources:** retain process/LSP ownership; add explicit compact -> popover -> full Resources, Space, and Usage surfaces and sampling/history policies.
- **Work Package 10A Conversation:** replace PTY-as-only-writer implementation with structured-first ACP plus explicit terminal handoff. Keep ownedId isolation, attachment vault, transcript safety, and child authorization.
- **Work Package 11 Guarded Agent:** retain fact/action/confirmation protections; add the durable workflow engine, delegation broker, Agent Control workspace, templates, retries, loops, and recovery.
- **Parallel dispatch manifest:** use Waves 0-5 above and preserve the product-wave heavy-runner/file-ownership rules.

### TSK-809/810 plan

The older file remains discovery evidence for transcript parsing, attachments, session isolation, and child safety. Its PTY-only architecture and read-only-only child presentation are superseded where this amendment explicitly provides ACP structured ownership and workflow-controlled children. Provider-native child sessions remain read-only unless the provider exposes a typed control.

---

## 13. Stop conditions

Stop and return a decision packet before implementation when:

- Codex or Claude ACP cannot be packaged reproducibly on the target macOS architectures;
- an adapter does not expose image prompt capability needed for screenshot acceptance;
- a provider config option cannot be confirmed authoritatively;
- native CLI handoff cannot prevent concurrent writers;
- Tauri child webviews cannot satisfy z-order/input isolation in floating and expanded modes;
- Paneview cannot preserve the required nested left/right/bottom layout without replacing Dockview;
- delegation cannot link parent tool call, child ownedId, and terminal outcome deterministically;
- workflow recovery would rerun a consequential node without proof;
- a requested AI integration would need arbitrary shell, implicit mutation, unbounded page/repository text, or a second provider client.

Do not fall back silently to decorative controls, text parsing, duplicate processes, or a second store/runtime. Record the missing capability and present an honest reduced mode.
