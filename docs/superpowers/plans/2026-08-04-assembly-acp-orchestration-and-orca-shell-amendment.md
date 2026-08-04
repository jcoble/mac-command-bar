# Assembly ACP Runtime, Workflow Orchestration, and Orca-Inspired Shell Amendment

**Date:** 2026-08-04  
**Status:** Superseding product and implementation amendment; implementation is not authorized by this document alone.  
**Repository baseline reviewed:** `jcoble/mac-command-bar` at `beebda6c4dad60cd2782374a36f24737cefda605`; the product-source diff from the earlier `4e8192e0cdce791f53e93af39a6d2d2e2c322911` review baseline is empty.
**Primary affected tasks:** TSK-808, TSK-809, TSK-810, the TSK-766 orchestration outcome, and the browser/resource/session portions already routed into the TSK-808 master plan.

## 0. Authority and supersession

This amendment was produced after re-reading the latest repository implementation and the two newest Superpowers plans, then comparing them with the clarified product requirements and the supplied Orca/Codex/ChatGPT screenshots.

It is the execution authority where it conflicts with:

- `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`;
- `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`.

The older plans remain useful for already-decided contrast, Git, editor, Roslyn, worktree, settings, native-browser safety, and native acceptance details. They are not deleted. The following clauses are specifically superseded:

1. The master-plan architecture sentence forbidding an agent runtime. Assembly must have **one** app-owned `AgentRuntimeManager`; the prohibition becomes “do not create a second or competing agent runtime.”
2. Work Package 10A’s permanent-PTY-writer design. The PTY transcript mirror becomes the adapter for terminal-owned sessions, imported sessions, and recovery—not the only live conversation architecture.
3. Work Package 10A’s model/effort/approval slash-command controls. These become capability-driven UI selectors backed by ACP session configuration options.
4. Work Package 10A’s path-only screenshot submission. Images become first-class ACP prompt content when the active provider advertises image input; managed paths remain the terminal-owned fallback.
5. Work Package 10A’s read-only-only child discovery as the complete subagent solution. Assembly must support both live provider-native subagents and Assembly-managed workflow agents.
6. Work Package 11’s narrow proposal-only “integrated agent.” It remains the safety model for consequential mutations, but is expanded into an app-owned workflow engine, agent control center, and reusable AI assistance recipes.
7. Work Package 3’s combined “My work” plus resumable-session drawer in one left column. Active work remains left; the complete session/history library moves to a separate Paneview/Dockview surface.
8. Work Packages 2 and 3’s hand-built left/right accordion layout. Primary left/right sections become Dockview Paneview panels.
9. Work Package 8’s three-state browser presentation. It becomes a four-state docked/floating/maximized/collapsed browser using one persistent native page.
10. Work Package 9’s combined resources/provider-usage presentation. Compact resource and usage popovers each open their own full center workspace.
11. The master plan’s parallel schedule for conversation and integrated-agent work. The schedule in section 18 of this amendment replaces it for these lanes.
12. The master plan’s “first implementation action.” Section 21 of this amendment is now the first action for the affected lanes.

When a packet below says “preserve the master plan,” every unaffected safety, test, accessibility, cleanup, worktree, SQL, native-proof, and controller-owned-seam rule remains binding.

---

## 1. Direct product answers

### 1.1 Does an ACP session create a terminal session?

No. An ACP session is a structured JSON-RPC connection between Assembly, acting as the ACP client, and an agent process or adapter, acting as the ACP agent. The client normally starts the adapter as a child process with piped stdin/stdout. That process is not an xterm instance and is not the native Claude Code or Codex TUI.

An ACP agent can report command/tool activity and may ask the client to provide terminal execution facilities. Those are **tool terminals** owned by Assembly and associated with a tool call. They may be shown as expandable command output or optionally opened in a terminal pane. They are not the same thing as the user-facing raw `claude` or `codex` TUI.

Assembly therefore keeps three distinct concepts:

```text
Structured agent session
  ACP JSON-RPC session; default rich conversation experience.

Tool terminal
  Client-managed process/output used by a command tool call inside an ACP turn.

Native CLI terminal
  Existing interactive PTY running the real Claude Code or Codex TUI.
```

A normal structured session creates the first and may create the second. It creates the third only when the user explicitly chooses **Open in native CLI**, when a provider capability is unavailable, or when recovery requires it.

### 1.2 Can ACP provide the requested Codex/T3-style controls and content?

Yes, with capability negotiation and provider-specific extensions where the stable protocol does not yet have a universal shape.

For Codex and Claude, the reviewed official adapters already provide enough surface to build the requested product:

- models and reasoning effort;
- session modes/permissions;
- optional model configuration such as Fast mode;
- text and image input;
- assistant text and thoughts/reasoning;
- tool calls, command output, file changes, and approval requests;
- plans/TODOs/task progress where the provider exposes them;
- session load/resume/close capabilities;
- provider-advertised commands and skills;
- subagent activity and nested transcripts through standard updates plus namespaced metadata.

Assembly must render these from the runtime’s advertised capabilities. It must not hard-code a model catalog, assume every provider has the same effort values, or show a control that does not affect the active session.

### 1.3 Can the same architecture support future providers?

Yes. ACP is the default provider boundary, not a Codex/Claude-specific UI. Future providers enter through a curated `AgentProviderManifest` and the same `AgentRuntimeAdapter` contract. Provider differences remain in capabilities and namespaced metadata, while the visible timeline uses one app-owned canonical model.

ACP does not itself define a complete multi-agent workflow product. Assembly owns workflow definitions, scheduling, worktree policy, retries, gates, budgets, and run history. ACP is the transport used to run each participating agent.

---

## 2. Current repository reality that this plan extends

The current `main` implementation has strong foundations that remain:

- `ownedId` is the stable app identity; provider session IDs and PTY IDs are metadata.
- `TerminalService` and Rust `TerminalRegistry` already provide one listener, reattach, bounded scrollback, tombstones, exact process ownership, and safe explicit close.
- per-session workspaces already capture editor, explorer, browser, diff, center layout, conversation draft/mode/child selection, and scroll position.
- the Rust transcript scanners already discover Codex and Claude sessions, exclude helper/subagent sessions from the top-level rail, and derive titles, metadata, message counts, and resumable commands.
- the current conversation store is keyed by `ownedId`, has generation/sequence protection, and already includes attachment and child state.
- the attachment vault validates image signatures, size, canonical managed paths, and safe owned-session directories.
- the browser master-plan lane already contains strong child-webview, annotation, isolation, and cleanup requirements.
- the orchestration event ledger is append-only and already has run, agent, step, artifact, link, attention, and decision concepts.

The current active conversation path is nevertheless a terminal transcript projection:

```text
bare interactive `claude` or `codex` in a PTY
  -> provider JSONL transcript
  -> 500 ms full-tail polling
  -> flattened user/assistant messages
  -> structured overlay
  -> composer writes bracketed paste + Return back to the PTY
```

The repository also contains an intentionally unwired handwritten `codex app-server`/`claude -p stream-json` provider spike. It must not be independently activated beside the PTY. Its useful protocol discoveries can be preserved as tests or removed after the ACP runtime replaces it.

The current projection cannot be the final rich client because it discards most typed provider behavior and cannot authoritatively control an active turn. The target architecture replaces that limitation without discarding the terminal system.

---

## 3. Locked product decisions

1. **New Codex and Claude sessions start structured by default through ACP.**
2. **No permanent user-visible terminal is created for an ordinary ACP session.** Tool terminals are separate and scoped to tool calls.
3. **The native CLI remains first class.** The user can hand a session to the raw Codex/Claude TUI and later return it to the structured view.
4. **One writer per native provider conversation.** A structured ACP runtime and a native CLI never submit to the same provider session concurrently.
5. **Existing externally created or already-running CLI sessions remain usable.** Assembly imports/mirrors their transcripts and may perform an explicit terminal-to-structured handoff when the provider can resume them safely.
6. **`ownedId` remains the primary application identity.** ACP session IDs, Codex thread IDs, Claude session IDs, PTY IDs, tool-terminal IDs, workflow-run IDs, and parent/child IDs never replace it.
7. **Model, effort/thought level, mode/permissions, Fast/service tier, and similar settings are UI controls—not slash commands.** They come from runtime-advertised session configuration options.
8. **The slash menu contains only genuine provider commands and Assembly actions.** TUI-only commands such as theme, raw terminal rendering, terminal title, pet, keymap, or app handoff are excluded unless Assembly implements an equivalent local action.
9. **Screenshot paste is a top-priority first-class input path.** Clipboard images appear immediately as previews and are sent as ACP image content blocks when supported.
10. **Tool calls, approvals, plans, task lists, reasoning, file changes, and terminals are typed timeline items.** They are not flattened into assistant Markdown.
11. **Subagents have two provenances:** provider-native subagents observed through ACP, and Assembly-managed workflow agents created by the workflow engine. The UI identifies which is which.
12. **Workflow orchestration is app-owned and deterministic.** An LLM may serve as the selected orchestrator, but it does not become the scheduler, persistence layer, worktree manager, or permission authority.
13. **The left rail is compact active work, not the complete session archive.** Resumable/history discovery moves to its own registered surface.
14. **Primary left and right sections use Dockview Paneview.** Center destinations remain Dockview panels. A hand-built accordion is not a substitute for a primary pane.
15. **The browser has docked, floating, maximized, and collapsed states using the same persistent native page.**
16. **Browser grab, annotate, and draw are separate actions.** All produce reviewable context that can be attached to the exact conversation; none auto-submits.
17. **Resource and usage each have a compact popover and a full center workspace.**
18. **AI assistance is reusable across the product.** PR creation/review, forms, save validation, browser feedback, Git, Problems, run configurations, and cleanup use typed recipes over deterministic context.
19. **Consequential actions remain previewed, confirmed, revalidated, and audited.** AI output never bypasses the existing Git, worktree, PR, filesystem, terminal, or provider permission boundary.
20. **No provider feature is claimed from a screenshot, transcript guess, or stale catalog.** The active runtime’s capability snapshot is authoritative.

---

## 4. Research decisions and reusable implementation references

### 4.1 ACP foundation

Use the official Rust SDK as the client transport:

- `agentclientprotocol/rust-sdk`
- stable session config options for model, mode, thought level, and arbitrary selectors;
- stable session list/resume/close capabilities;
- image prompt capability negotiation;
- permission and tool-call request/response flow;
- provider-advertised command updates.

Use pinned official adapters initially:

- `agentclientprotocol/codex-acp` — Codex app-server mapping for models, effort, Fast mode, permissions/sandbox, images, commands, file changes, reasoning, plans, reviews, web search, token/rate usage, and subagents;
- `agentclientprotocol/claude-agent-acp` — Claude Agent SDK mapping for models/options, images, tool permissions, TODOs, edit review, terminals, custom commands, MCP, and nested subagent transcripts.

Do not invoke `npx ...@latest` per session. Pin the adapter version. Prefer packaged sidecars with recorded hashes. Codex ACP already has a standalone-binary build route. Claude ACP currently requires a compatible Node runtime; Work Package A0 must choose and prove either a packaged executable or a bundled/pinned Node 22 runtime before product implementation.

### 4.2 Canonical event and provider design

Use T3 Code as a taxonomy and adapter reference, not as a wholesale architecture transplant:

- `pingdotgg/t3code/packages/contracts/src/providerRuntime.ts`;
- provider adapters under `apps/server/src/provider`;
- `ChatMarkdown.tsx` for safe, streaming Markdown behavior.

Assembly needs the same broad separation of:

- session/thread/turn lifecycle;
- item lifecycle;
- assistant/reasoning/plan/content deltas;
- commands, file changes, MCP calls, web search, and subagents;
- approvals and structured user input;
- task/TODO progress;
- token/rate metadata;
- raw provider frames retained only for diagnostics.

Assembly does **not** need T3’s entire Effect RPC server, SQL event store, or React client.

### 4.3 Tauri ACP client reference

Use Jockey as a focused Tauri/Rust ACP-client reference:

- `recailai/jockey`;
- session config discovery and mutation;
- model/mode/command updates;
- permission handling;
- session process ownership and multi-agent client composition.

Reuse patterns only after checking them against Assembly’s existing `ownedId`, PTY, Dockview, and workspace contracts.

### 4.4 Delegation/workflow reference

Use CodeG’s delegation broker as evidence that ACP sessions can be coordinated through an app-owned broker and optional MCP companion:

- `xintaofei/codeg/src-tauri/src/acp/delegation/types.rs`;
- delegation broker/spawner/listener modules;
- parent connection and tool-use correlation;
- per-agent model/mode/config defaults;
- background status/cancel tools;
- depth limits, child conversation IDs, token/turn budgets, and stable terminal outcomes.

Assembly’s workflow engine is broader and deterministic. The CodeG pattern is specifically useful for an LLM orchestrator that calls tools such as `delegate_agent`, `workflow_status`, and `cancel_agent` without gaining direct access to Assembly’s process or worktree internals.

### 4.5 Orca interaction reference

Use Orca’s public implementation for product behavior and component decomposition, not its Electron runtime:

- `stablyai/orca/src/renderer/src/components/browser-pane/BrowserPane.tsx` and adjacent browser modules for persistent pages, grab mode, annotations, markup, viewport/profile controls, devtools, external open, and annotation delivery;
- resource and workspace-space panels for compact/full resource presentation and treemap cleanup;
- usage roster/stats panels for compact quota and full analytics presentation;
- `WorktreeCard*` modules for compact rows, hover identity, agents, metadata, status badges, and expandable detail.

Assembly must implement equivalent behavior over Tauri child WKWebViews, Rust process registries, existing worktree safety, Svelte, Dockview, and Paneview.

---

## 5. Target runtime architecture

### 5.1 Stable identity and execution ownership

Extend `OwnedSession` additively:

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';

export type AgentRuntimeState =
  | 'starting'
  | 'ready'
  | 'working'
  | 'waiting-approval'
  | 'waiting-input'
  | 'interrupting'
  | 'failed'
  | 'closed';

export interface OwnedAgentRuntimeFields {
  executionOwner: AgentExecutionOwner;
  runtimeState: AgentRuntimeState;
  providerInstanceId: string | null;
  nativeSessionId: string | null;
  activeTurnId: string | null;
  capabilityRevision: number;
  lastRuntimeError: string | null;
}
```

Migration rules:

- an existing live agent PTY becomes `executionOwner: 'terminal'`;
- an exited agent session becomes `stopped`;
- a new structured session becomes `structured` only after ACP initialization and `session/new` succeed;
- failed migration never erases `ptySessionId`, `nativeSessionId`, transcript, draft, workspace, or attachment state.

### 5.2 One Rust `AgentRuntimeManager`

Refactor the existing `src-tauri/src/agent_conversation` authority rather than adding a competing runtime.

Target module shape after re-anchor:

```text
src-tauri/src/agent_conversation/
├── mod.rs
├── manager.rs
├── protocol.rs
├── capabilities.rs
├── journal.rs
├── handoff.rs
├── attachments.rs
├── terminal_projection.rs
├── transcript/
│   ├── mod.rs
│   ├── claude.rs
│   └── codex.rs
└── providers/
    ├── mod.rs
    ├── acp.rs
    ├── acp_client.rs
    ├── process.rs
    └── direct_codex.rs        # optional later optimization only
```

Core state:

```rust
pub struct AgentRuntimeManager {
    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    providers: Arc<ProviderRegistry>,
}

pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider: AgentProvider,
    pub provider_instance_id: String,
    pub native_session_id: Option<String>,
    pub generation: u64,
    pub owner: ExecutionOwner,
    pub state: AgentRuntimeState,
    pub capabilities: AgentCapabilities,
    pub next_sequence: u64,
    pub active_turn_id: Option<String>,
    pub runtime: Option<StructuredRuntimeHandle>,
    pub terminal_projection: Option<TerminalProjectionHandle>,
    pub recent_events: VecDeque<AgentEvent>,
}
```

The manager owns:

- provider sidecar lifecycle and stderr diagnostics;
- ACP initialize/new/load/resume/close;
- prompt, image, cancellation, steering, permission, and user-input responses;
- session config updates;
- normalized event emission;
- provider-native IDs;
- structured/terminal writer lease;
- recovery and handoff;
- bounded recent-event snapshots and rebuildable app journal.

It does not own editor, Git, worktrees, browser, Dockview, or the main terminal registry.

### 5.3 Provider registry and future providers

```rust
pub struct AgentProviderManifest {
    pub id: String,
    pub display_name: String,
    pub transport: ProviderTransport,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub version: String,
    pub content_hash: String,
    pub trusted_source: ProviderSource,
}

pub enum ProviderTransport {
    AcpStdio,
    AcpWebSocket,
    BuiltIn,
}
```

Initial manifests are Codex and Claude. A future provider is enabled only when:

- its manifest is trusted or explicitly configured;
- executable path and hash/version are recorded;
- ACP initialization succeeds;
- capabilities are displayed before session start;
- unsupported features degrade honestly;
- it cannot inherit Tauri IPC, app credentials, or arbitrary unlisted workspace roots.

The ACP Registry can later aid discovery, but automatic install/update is not part of the first runtime packet.

### 5.4 Structured runtime contract

```rust
#[async_trait]
pub trait AgentRuntimeAdapter: Send + Sync {
    async fn initialize(&mut self, input: InitializeAgentInput)
        -> Result<AgentCapabilities, AgentRuntimeError>;
    async fn new_session(&mut self, input: NewAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn load_session(&mut self, input: LoadAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn resume_session(&mut self, input: ResumeAgentSession)
        -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn prompt(&mut self, input: AgentPrompt)
        -> Result<StartedTurn, AgentRuntimeError>;
    async fn steer(&mut self, input: AgentSteeringInput)
        -> Result<(), AgentRuntimeError>;
    async fn cancel_turn(&mut self, turn_id: Option<&str>)
        -> Result<(), AgentRuntimeError>;
    async fn set_config(&mut self, option_id: &str, value: AgentConfigValue)
        -> Result<Vec<AgentConfigOption>, AgentRuntimeError>;
    async fn respond_permission(&mut self, input: PermissionResponse)
        -> Result<(), AgentRuntimeError>;
    async fn respond_user_input(&mut self, input: UserInputResponse)
        -> Result<(), AgentRuntimeError>;
    async fn close_session(&mut self) -> Result<(), AgentRuntimeError>;
}
```

ACP-specific protocol objects remain inside the adapter. The frontend receives Assembly types only.

### 5.5 Tool terminals are separate from native CLI terminals

Extend the existing `TerminalRegistry` with an explicit terminal kind and optional tool identity.
It remains the one process/output registry for user PTYs and Assembly-owned tool terminals;
`AgentRuntimeManager` requests tool-terminal creation through a typed registry API and does not
own a second process map, listener, scrollback store, or cleanup authority.

```rust
pub enum TerminalKind {
    UserPty,
    AgentTool,
    RunConfiguration,
    BrowserAutomation,
}

pub struct ToolTerminalIdentity {
    pub owned_id: String,
    pub turn_id: String,
    pub tool_call_id: String,
    pub terminal_id: String,
}
```

Rules:

- tool terminals are created only from an authenticated current-generation ACP request;
- command, cwd, environment-key policy, and workspace roots are validated at the client boundary;
- streamed output is a typed timeline item;
- optional “Open terminal” reuses the same process/output and does not rerun it;
- closing a conversation closes or detaches only its owned tool terminals according to provider capability;
- tool terminals never set `OwnedSession.ptySessionId` and never masquerade as native CLI ownership.

---

## 6. Canonical capabilities, events, and timeline

### 6.1 Capability snapshot

```ts
export interface AgentCapabilities {
  revision: number;
  provider: AgentConversationProvider;
  implementation: { name: string; version: string };
  session: {
    list: boolean;
    load: boolean;
    resume: boolean;
    close: boolean;
    steering: boolean;
  };
  prompt: {
    text: boolean;
    image: boolean;
    embeddedContext: boolean;
    resourceLinks: boolean;
  };
  interaction: {
    permissions: boolean;
    structuredUserInput: boolean;
    toolTerminals: boolean;
    plans: boolean;
    tasks: boolean;
    subagents: boolean;
  };
  configOptions: AgentConfigOption[];
  commands: AgentCommandDescriptor[];
}
```

Unknown ACP categories are retained and rendered in More Options. Recognized categories map as follows:

- `model` -> primary model picker;
- `thought_level` -> effort/reasoning picker;
- `mode` -> execution/permission mode picker;
- model-related custom or future `model_config` -> model popover secondary controls;
- unrecognized category -> generic, labelled option in More Options.

Do not duplicate legacy ACP `modes` when config options are supplied.

### 6.2 Canonical event model

```ts
export type AgentEventType =
  | 'session.started'
  | 'session.config.updated'
  | 'session.state.changed'
  | 'session.closed'
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
  | 'children.updated'
  | 'usage.updated'
  | 'rate-limits.updated'
  | 'runtime.warning'
  | 'runtime.error';

export type AgentItemType =
  | 'user-message'
  | 'assistant-message'
  | 'reasoning'
  | 'plan'
  | 'task-list'
  | 'command'
  | 'file-change'
  | 'mcp-tool'
  | 'web-search'
  | 'image-view'
  | 'image-generation'
  | 'subagent'
  | 'review'
  | 'context-compaction'
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

Each event carries:

- `ownedId`;
- provider and provider instance;
- generation and monotonically increasing sequence;
- timestamp;
- native session/turn/item/request IDs when present;
- canonical payload;
- optional namespaced provider metadata;
- optional redacted raw-frame reference for diagnostics, never arbitrary secrets.

### 6.3 Ordered item projection

The timeline stores ordered items, not one assistant blob with a detached tool list. A turn such as assistant text -> command -> reasoning -> file change -> assistant text must remain in that order.

The reducer must support:

- incremental content deltas;
- item start/update/complete;
- authoritative completed text replacing or reconciling streamed text without truncation;
- request lifecycle;
- stale generation/sequence rejection;
- snapshot repair after a missed event;
- provider-history import without duplicate native item IDs;
- parent/child grouping without flattening child output into the parent message.

### 6.4 Provider transcript projection

Refactor `transcript.rs` into an incremental terminal-owned adapter:

- cache canonical path plus file identity;
- retain read offset and partial final line;
- parse only appended JSONL;
- detect truncate/rotation/archive move;
- emit the same canonical `AgentEvent` shape where durable transcript evidence exists;
- periodically reconcile with a bounded snapshot;
- never invent a live approval, active turn, or tool state absent from durable evidence.

This removes the frontend 500 ms full-tail polling loop. Rust emits update events to hidden and visible sessions alike.

---

## 7. Conversation experience

### 7.1 Component structure

Refactor the current single large `ConversationSurface.svelte` into composition over the same store/service:

```text
ConversationSurface.svelte
├── ConversationHeader.svelte
├── AgentConfigBar.svelte
├── ConversationTimeline.svelte
│   └── TimelineItem.svelte
│       ├── UserMessageItem.svelte
│       ├── AssistantMessageItem.svelte
│       ├── ReasoningItem.svelte
│       ├── PlanItem.svelte
│       ├── TaskListItem.svelte
│       ├── CommandItem.svelte
│       ├── FileChangeItem.svelte
│       ├── ToolItem.svelte
│       ├── SubagentItem.svelte
│       ├── ApprovalItem.svelte
│       ├── UserInputItem.svelte
│       └── ErrorItem.svelte
├── ConversationComposer.svelte
├── AgentCommandMenu.svelte
└── ConversationAgentTree.svelte
```

No second store or provider service is introduced.

### 7.2 Model, effort, permissions, and options

The composer footer mirrors Codex/T3 behavior:

```text
[Provider] [Model ▾] [Effort ▾] [Mode/Permissions ▾] [Fast/Options ▾]
```

Rules:

- values and choices come from the current capability snapshot;
- changing a model sends `session/set_config_option` through the runtime manager;
- the returned full option list replaces current state because model changes can alter effort/options;
- pending state remains visible until confirmed;
- failure restores the previous confirmed value;
- changing one session cannot affect another;
- unsupported selectors are hidden or read-only with the reason;
- provider identity is shown but not offered as an in-place switch unless the session is forked into a new provider-backed session.

### 7.3 Command menu

The command menu merges:

1. provider-advertised commands;
2. discovered provider skills/prompts;
3. Assembly-local actions that make sense in the conversation.

It deliberately excludes native-TUI-only commands. Examples of valid entries:

```text
Provider
  /review
  /compact
  /status
  /mcp
  /skills

Assembly
  /terminal
  /conversation
  /new-worktree
  /open-file
  /rename-session
  /archive-session
```

Model, effort, permissions, mode, service tier, theme, keymap, Vim mode, raw terminal rendering, terminal title/statusline, pets, app handoff, and debug commands are not normal slash rows. They are UI settings, local actions, or development-only controls.

Selecting a row inserts or invokes only according to its descriptor. Navigation never auto-runs a command.

### 7.4 Screenshot and image input — first-priority packet

The existing managed attachment vault remains the storage and validation boundary.

Composer behavior:

1. `Cmd+V` examines clipboard files before plain text.
2. Supported images are validated and stored under the exact `ownedId`.
3. A preview appears immediately with name, size, remove, and optional annotate action.
4. Send builds an ACP prompt containing ordered text/image content blocks when `prompt.image` is supported.
5. The image bytes/file are transmitted through the adapter’s supported image form. A filesystem path is not substituted when the protocol accepts real image content.
6. For terminal-owned sessions, the fallback stages the managed path in the prompt exactly once and sends through the PTY only after confirmation of the exact owner/generation.
7. Success clears and revokes previews. Failure preserves the draft and every attachment.
8. App restart restores managed attachment metadata and regenerates preview URLs.
9. Cleanup deletes only owner-validated managed files and never a user-owned original.

Acceptance requires Codex ACP and Claude ACP to receive the same pasted PNG/JPEG/WebP fixture and reference visible image content in their answer.

### 7.5 Markdown and code

Use the Work Package 10 safe Markdown pipeline or a shared chat-specific adapter over it:

- GFM, tables, lists, task lists, links, fenced code, inline code, and blockquotes;
- streaming-safe partial fences;
- sanitized HTML policy;
- code-block copy and filename metadata;
- file/path links open through the existing editor bus after root validation;
- external HTTP/S links use the approved opener;
- syntax colors reuse the already-loaded Monaco/VS Code language/theme services where practical;
- large output uses bounded rendering and virtualization.

### 7.6 Plans and tasks

Plan and TODO/task updates are first-class items.

```ts
export interface AgentPlanStep {
  id: string;
  title: string;
  detail: string | null;
  state: 'pending' | 'in-progress' | 'completed' | 'failed' | 'blocked';
  ownerAgentId: string | null;
  startedAt: string | null;
  completedAt: string | null;
}
```

The user can:

- expand/collapse a plan;
- see the current step and elapsed time;
- open the assigned agent/worktree;
- approve/revise a provider-proposed plan when supported;
- copy plan text;
- distinguish provider task updates from Assembly workflow nodes.

Assembly never parses ordinary prose into authoritative task completion.

### 7.7 Subagents

Provider-native child records include:

- provider-native ID;
- parent native item/tool ID;
- role/label;
- relationship/provenance;
- status and timestamps;
- nested messages, thoughts, and tools when supplied;
- provider metadata preserved under a namespace.

The tree is live for ACP-owned sessions and transcript-backed for terminal-owned history. Opening a provider-native child is read-only unless the provider explicitly advertises direct-input support for that child. Unknown support means read-only.

### 7.8 Scrolling and long sessions

- auto-follow only when the reader is within 80px of the bottom;
- show Jump to latest when the user has scrolled up;
- preserve parent and child scroll independently;
- use stable item keys and row virtualization for long histories;
- never rebuild the whole timeline on every delta;
- switching sessions does not stop background streaming.

---

## 8. Structured/native-terminal ownership handoff

### 8.1 State machine

```text
StructuredOwned
  -> PreparingTerminal
  -> TerminalOwned
  -> PreparingStructured
  -> StructuredOwned
```

`PreparingTerminal`:

1. block new structured prompts;
2. finish or interrupt the active turn according to user choice;
3. flush normalized events and capture provider/native IDs;
4. close/unsubscribe the structured writer without deleting the provider session;
5. start the existing PTY with the exact native resume command;
6. prove the new PTY owns the same native conversation;
7. begin incremental transcript projection;
8. mark terminal ownership only after the TUI is live.

`PreparingStructured`:

1. block PTY composer writes;
2. ask the user to exit or explicitly terminate the native TUI when still active;
3. stop transcript projection after the final append is processed;
4. resume/load the provider session through ACP;
5. import/reconcile turns added by the native TUI;
6. prove native IDs and history boundary;
7. acquire the structured writer lease;
8. mark structured ownership.

Failure rolls back to the previous usable owner. It never leaves two writers or loses the terminal scrollback.

### 8.2 Fork to terminal/provider

Separate actions:

- **Open in native CLI** transfers the same conversation;
- **Fork to native CLI** creates a new provider-native branch and leaves the structured source open;
- **Fork with provider…** creates a new Assembly session with copied bounded context and a selected provider/model; it is not claimed as the same provider thread.

### 8.3 Codex shared app-server optimization

A later Codex-specific spike may connect both Assembly and the Codex TUI to one app-server daemon using the TUI’s remote endpoint support. This is an optimization only after proving:

- one thread identity;
- one active writer lease;
- exact subscription/unsubscription behavior;
- no duplicate rollout;
- approvals and settings remain synchronized;
- clean disconnect/reconnect.

The first implementation does not depend on this optimization.

---

## 9. App-owned workflow orchestration

### 9.1 Why ACP alone is not the workflow engine

ACP runs and observes an agent session. It does not define Assembly’s product-level workflow DAG, role catalog, worktree allocation, retry policy, approval gates, provider routing, budgets, or durable run history.

Assembly must own those concepts and use ACP to execute each node.

### 9.2 Workflow definitions

```ts
export interface WorkflowDefinitionV1 {
  version: 1;
  id: string;
  name: string;
  description: string;
  trigger: WorkflowTrigger;
  inputs: WorkflowInputDefinition[];
  roles: AgentRoleDefinition[];
  nodes: WorkflowNodeDefinition[];
  edges: WorkflowEdge[];
  concurrency: WorkflowConcurrencyPolicy;
  budgets: WorkflowBudgetPolicy;
  completion: WorkflowCompletionPolicy;
}

export interface AgentRoleDefinition {
  id: string;
  name: string;
  purpose: string;
  providerPolicy: ProviderSelectionPolicy;
  modelPolicy: ModelSelectionPolicy;
  effortPolicy: ConfigSelectionPolicy;
  permissionPolicy: ConfigSelectionPolicy;
  promptTemplateId: string;
  outputContract: WorkflowOutputContract;
  workspacePolicy: WorkflowWorkspacePolicy;
  retryPolicy: WorkflowRetryPolicy;
}

export interface WorkflowNodeDefinition {
  id: string;
  title: string;
  roleId: string;
  dependsOn: string[];
  condition: WorkflowCondition | null;
  fanOut: WorkflowFanOut | null;
  approvalGate: WorkflowApprovalGate | null;
  timeoutSeconds: number;
  maxAttempts: number;
}
```

Initial built-in role templates:

- Orchestrator;
- Spec writer/refiner;
- Implementer;
- Unit/integration test author;
- Code reviewer;
- Spec-compliance reviewer;
- Security reviewer;
- UI/native acceptance verifier;
- Fixer;
- Summarizer/handoff writer.

Templates are user-editable copies. Assembly does not hard-code one model provider into a role.

### 9.3 Deterministic workflow engine

Create one Rust `WorkflowEngine` that consumes the existing append-only orchestration ledger.

Core responsibilities:

- validate and version workflow definitions;
- materialize a run with immutable input snapshot/hash;
- schedule ready nodes in dependency order;
- enforce global/per-workflow/per-provider concurrency;
- allocate or select worktrees through the existing worktree authority;
- start an ACP session through `AgentRuntimeManager`;
- pass explicit role, model, effort, permission, cwd, input context, and output schema;
- collect typed node output and artifact references;
- evaluate deterministic gates;
- pause for approval/user input;
- retry only allowed failures;
- cancel descendants when policy requires;
- persist every state transition as an append-only event;
- rebuild the read model deterministically after restart.

Workflow states:

```ts
export type WorkflowRunState =
  | 'draft'
  | 'queued'
  | 'running'
  | 'waiting-approval'
  | 'waiting-input'
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
  | 'waiting-approval'
  | 'waiting-input'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'skipped';
```

### 9.4 Two orchestration modes

#### Deterministic coordinator — default

The engine schedules a predefined graph. Agents receive only their node task and explicit context. This is the safest and most reproducible mode for implementation/review/spec-compliance pipelines.

#### Agent-directed delegation — optional

A selected orchestrator agent receives a small Assembly MCP companion with allow-listed tools:

```text
workflow_list_roles
workflow_delegate_agent
workflow_get_agent_status
workflow_cancel_agent
workflow_submit_result
workflow_request_gate
```

The companion sends typed requests to the WorkflowEngine. It never spawns processes, creates worktrees, reads secrets, or chooses arbitrary executables itself.

Delegation request:

```ts
export interface AgentDelegationRequest {
  workflowRunId: string;
  parentNodeRunId: string;
  parentOwnedId: string;
  parentToolCallId: string;
  roleId: string;
  task: string;
  requestedWorkspace: string | null;
  providerOverride: string | null;
  configOverrides: Record<string, string | boolean>;
  depth: number;
}
```

The engine applies depth, concurrency, budget, workspace, provider, and permission policy before starting the child ACP session. This follows the useful CodeG broker pattern while preserving Assembly’s stronger workflow model.

### 9.5 Native provider subagents versus Assembly workflow agents

Provider-native subagents:

- are created by the provider inside a parent turn;
- are observed and rendered through ACP/provider metadata;
- may not have independent Assembly worktrees or configurable provider/model;
- remain children of the provider turn.

Assembly workflow agents:

- are created by WorkflowEngine;
- each have an Assembly `ownedId`, runtime capabilities, role, node, and optional worktree;
- may use different providers/models/permissions;
- have explicit retry/cancel/gate/output contracts.

The UI displays both in one agent tree but labels provenance:

```text
Provider subagent
Workflow agent
```

No provider-native child is silently promoted into a workflow node.

### 9.6 Worktree and file-collision policy

Workflow workspace policies:

```ts
export type WorkflowWorkspacePolicy =
  | { kind: 'read-only-current' }
  | { kind: 'shared-current'; fileAllowList: string[] }
  | { kind: 'dedicated-existing'; worktreeId: string }
  | { kind: 'dedicated-new'; branchTemplate: string }
  | { kind: 'none' };
```

Rules:

- the existing worktree service remains the only create/remove/safety authority;
- parallel implementers default to dedicated worktrees;
- parallel read-only reviewers may share a snapshot/current worktree;
- two write nodes cannot claim overlapping file leases in one worktree;
- worktree cleanup uses existing dirty/unmerged/locked checks and confirmation;
- a node finishing does not by itself prove that its worktree is integrated, pushed, or safe to
  remove, so the workflow may retain that still-active worktree for review;
- the moment the worktree's result is merged, cherry-picked, pushed to its PR/remote branch, or
  abandoned, the node owner removes it through the existing worktree authority and prunes it;
- dirty, locked, or unmerged worktrees are never deleted silently: the receipt records path,
  branch, `git status --short`, blocking reason, and the exact controller/agent that owns cleanup;
- workflow completion is not allowed to normalize an orphaned worktree. The final workflow and
  A12 receipts enumerate every created worktree as `removed` or `not removed` with its owner.

### 9.7 Output contracts and gates

Built-in output contracts:

```text
ImplementationReceipt
  changed files, summary, tests, known risks, commit/branch/worktree, artifacts.

ReviewReceipt
  severity, file/line, evidence, recommendation, confidence, blocking flag.

SpecComplianceReceipt
  requirement ID, status, evidence, gap, recommended action.

VerificationReceipt
  command, exit, duration, evidence artifacts, cleanup receipt.

PlanReceipt
  ordered steps, dependencies, risk, estimated parallel lanes.
```

Output is validated as structured data where supported. Markdown remains a display/overflow form, not the authoritative gate input.

Initial gates:

- implementation tests pass;
- code review has no unresolved blocking findings;
- spec-compliance checklist passes;
- security review passes for configured scope;
- native acceptance evidence exists when required;
- human approval for merge, destructive action, or remote mutation.

### 9.8 Budgets and failure policy

Workflow budgets include:

- maximum active agents;
- maximum child depth;
- maximum attempts per node;
- maximum wall time;
- optional token/cost budget when provider usage is available;
- maximum tool-terminal count;
- maximum worktree/disk allocation.

Budget exhaustion is a typed failure, not an agent prose message. The engine can pause for user approval to extend a budget.

---

## 10. Agent Control Center and workflow UI

### 10.1 Surfaces

Create one registered center destination **Agents** or **Workflows** with:

```text
WorkflowControlCenter.svelte
├── WorkflowRunList.svelte
├── WorkflowRunHeader.svelte
├── WorkflowGraph.svelte
├── WorkflowLaneBoard.svelte
├── AgentHierarchy.svelte
├── WorkflowTimeline.svelte
├── WorkflowNodeInspector.svelte
├── AgentRuntimeInspector.svelte
└── WorkflowTemplateEditor.svelte
```

Also provide a compact right Paneview panel `AgentActivityPane` showing active agents, waiting approvals, failures, and current workflow progress.

### 10.2 Control-center capabilities

The user can:

- start a workflow from a template;
- choose providers/models/effort/permissions per role before start;
- see the DAG/lane board and current loop position;
- inspect each agent’s conversation, tools, plan, task list, output, worktree, and usage;
- see parent/child relationships and provenance;
- open the agent conversation or worktree in the existing center/editor;
- pause/resume/cancel a workflow;
- cancel/retry/skip an eligible node;
- steer an active ACP agent when supported;
- answer structured input or permissions;
- approve a gate;
- compare reviewer findings;
- open artifacts, commits, PRs, test logs, and native evidence;
- save a run as a reusable workflow template.

### 10.3 Loop visibility

Every run exposes a deterministic loop phase:

```ts
export type WorkflowLoopPhase =
  | 'planning'
  | 'dispatching'
  | 'implementing'
  | 'testing'
  | 'reviewing'
  | 'checking-spec'
  | 'fixing'
  | 'awaiting-human'
  | 'certifying'
  | 'complete';
```

The phase is derived from node states/types, never guessed from assistant prose. The UI may show an agent-generated summary beside it, clearly marked as generated.

### 10.4 Session-list behavior for workflow agents

Workflow children do not flood the main left rail by default.

- top-level user sessions and pinned workflow agents may appear in Working;
- all workflow agents appear in the active workflow’s tree;
- a child can be pinned to Working explicitly;
- completed workflow children remain available through the Workflow run and Session Library;
- removing a workflow run record does not silently delete provider transcripts or worktrees.

---

## 11. Shell, navigation, Dockview, and Paneview

### 11.1 Left rail product model

The left rail becomes a narrow workspace/worktree/active-agent navigator inspired by the supplied Orca and ChatGPT references.

Default hierarchy:

```text
Working                    Paneview pane
  repository/workspace
    compact worktree row
      active/pinned agent rows

Done                       Paneview pane
  recently completed work

Settled                    Paneview pane, collapsed by default
  archived work references
```

Rows:

- target height 28–34px for normal compact rows;
- no permanently wide card chrome;
- leading repo/worktree/provider identity;
- one-line title;
- state dot plus word where needed;
- branch/task/PR only when useful;
- chevron expands inline detail;
- hover/focus popover shows basic facts without requiring expansion;
- actions appear on `focus-within` and hover, never hover only;
- active row is clear without a giant background card.

Hover/focus detail may include:

- exact project/worktree/cwd;
- provider/model/effort;
- current plan/task/agent;
- last activity;
- dirty/check/PR summary;
- ports/processes;
- Open, Terminal, Inspect, Mark done.

### 11.2 Separate Session Library

Remove the complete resumable-session archive from the main left column.

Create one canonical `SessionLibraryWorkspace` that can be registered as:

- a center Dockview destination by default;
- optionally a right Paneview panel through the same component/state.

It includes:

- provider, project, worktree, state, model, date, and text search;
- grouped history and resumable sessions;
- compact/comfortable density;
- preview/hover information;
- exact resume/open/fork/archive/delete actions;
- pagination/virtualization for hundreds of sessions;
- provider ACP `session/list` data merged with existing transcript scanners;
- no duplicate row for an already-owned native session.

Only one placement is mounted at a time; moving it reuses the same store/service.

### 11.3 Paneview contract

Use Dockview Paneview for primary left/right vertical sections.

Extend the existing `tauri-svelte-preview/src/lib/shell/layout/paneStack.ts` and
`layoutStorage.ts` authority with one generic side-pane registry. Do not create a second Paneview
factory, layout store, persistence key family, parking mechanism, or side-shell service. The
registry is a typed roster consumed by the existing `createPaneStack(...)` implementation:

```ts
export type SidePaneRegion = 'left' | 'right' | 'bottom';

export interface SidePaneRegistration {
  id: string;
  title: string;
  region: SidePaneRegion;
  component: Component;
  minimumSize: number;
  preferredSize: number;
  maximumSize: number | null;
  defaultExpanded: boolean;
  persistent: boolean;
  order: number;
}
```

Rules:

- Working/Done/Settled are Paneview panes, not only Collapsible blocks;
- right-side Files, Source Control, Worktrees, Problems, Resources, Agents, Context, Run Configurations, and other primary sections are pane registrations;
- pane resize/collapse/order persists by versioned layout;
- panel state survives park/move/minimize;
- no panel starts IO merely because its host is constructed;
- visibility activation remains imperative;
- all left/right tabs or sections added by later work must register through this roster;
- small detail accordions inside a pane are allowed; primary layout accordions are not.

### 11.4 Center roster additions

The center Dockview roster expands to:

```text
Session
Editor
Browser
Diff
Git Graph
Pull Requests
Markdown
Agents / Workflows
Resources
Usage
Session Library
```

Permanent/default visibility is configurable. Hidden destinations remain registered/parked rather than destroyed.

### 11.5 Floating action island

Retain and strengthen the master-plan `WorkbenchActionFab` concept:

- one global bottom-right island;
- context-aware actions for Session, Editor, Browser, Git/PR, Agents, Resources, Usage, and History;
- collision-safe fan/horizontal/bottom-sheet layout;
- no business logic inside the island;
- actions route to existing commands/services;
- browser floating/maximize, resource popup, usage popup, and agent status are reachable from this system or adjacent status-bar indicators without creating independent floating-button families.

---

## 12. Browser — docked, floating, maximized, collapsed

The original Work Package 8 remains the native security and lifecycle baseline. This section adds the clarified presentation and capture requirements.

### 12.1 Presentation model

```ts
export type BrowserPresentationMode =
  | 'docked'
  | 'floating'
  | 'maximized'
  | 'collapsed';

export interface BrowserFloatingBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
```

Behavior:

- **docked:** existing Browser center panel;
- **floating:** root-level movable/resizable browser window above the workbench, defaulting to the supplied medium-size composition and retaining the same native page;
- **maximized:** root-level overlay fills the available workbench below macOS window chrome;
- **collapsed:** native view hidden; only the global action island/Browser status remains.

Minimize from floating/maximized returns to the previous docked or collapsed state according to explicit user action. Maximize/restore preserves URL, history, forms, cookies, scroll, selected tab, viewport, devtools state, annotations, and profile.

Floating bounds are clamped to the current window and restored per owned workspace. Cold launch never starts maximized unexpectedly.

### 12.2 Toolbar contract

The visible toolbar includes capability-gated actions matching the screenshots:

```text
Back / Forward / Reload / Address
Import
Grab page element
Annotate page element
Draw on screenshot
Open browser devtools
Open in default browser
Profile / Viewport / Browser settings
Maximize / Restore
Minimize / Collapse
More
```

Import remains disabled until a separate secure cookie/import design exists, except for explicitly safe file/context imports. Tooltips and accessible names use plain English.

### 12.3 Three distinct feedback tools

#### Grab page element

- arm element picker;
- capture bounded DOM/accessibility metadata, selector, page URL/title, rect, and element screenshot when available;
- create a temporary context attachment;
- allow Copy or Add to conversation;
- no note required;
- no auto-send.

#### Annotate page element

- arm the same picker;
- show selected-element outline;
- open the annotation card near the element;
- require note plus Change/Question intent;
- queue immutable annotation with metadata and screenshot crop;
- edit/remove/copy/stage to conversation;
- no auto-send.

#### Draw on screenshot

- capture the visible browser viewport through a proven native WKWebView snapshot path;
- hide the live child view while the main webview displays the snapshot canvas;
- provide pen/highlighter/arrow/rectangle/text/undo/clear/crop controls;
- save a flattened annotated image plus optional note;
- store it in the existing conversation attachment vault;
- restore the same live page without reload;
- no arbitrary drawing script runs inside remote page content.

### 12.4 Native capture spike

Before the browser implementation claims Grab/Annotate/Draw, prove in rebuilt Tauri/macOS:

1. injected inspector can select same-origin and cross-origin top-level page elements only to the extent WKWebView permits;
2. element metadata is bounded and cannot expose cookies/storage/full HTML;
3. `WKWebView.takeSnapshot` or an equivalent narrowly wrapped native API captures the visible page at correct scale;
4. an element rect can be cropped correctly after scroll/zoom/device scale;
5. the snapshot overlay receives pointer/keyboard input above the child view;
6. restoring the child view does not reload or lose page state;
7. failure has an honest fallback: metadata-only annotation or user-selected whole-window screenshot, never a fabricated crop.

If the required native snapshot cannot be safely exposed through Tauri without broad AppKit/WebKit control, stop and write a focused native-bridge decision before implementing markup.

### 12.5 Conversation payload

```ts
export interface BrowserFeedbackAttachment {
  id: string;
  kind: 'grab' | 'annotation' | 'markup';
  workspaceId: string;
  tabId: string;
  generation: number;
  url: string;
  title: string;
  selector: string | null;
  accessibleName: string | null;
  textSnippet: string | null;
  note: string | null;
  intent: 'change' | 'question' | 'context';
  imageAttachmentId: string | null;
  createdAt: string;
  sourceHash: string;
}
```

Staging to conversation:

- captures exact `ownedId` and generation;
- adds structured text/context plus image attachment when available;
- merges with an existing nonempty draft only after preview;
- never sends automatically;
- retains the browser queue on failure/stale owner.

### 12.6 Browser profiles and viewport

Reuse the original profile-isolation policy. The overflow/profile menu includes:

```text
Default
New Profile…
Import Cookies          disabled until separately authorized
Viewport Size
Browser Settings…
```

Viewport selection changes actual child-view dimensions and is validated inside the page. Devtools is development-only until release policy explicitly allows it.

---

## 13. Resource Manager, workspace space, and Usage

### 13.1 Separate compact and full surfaces

Create five distinct product areas over shared deterministic services:

1. **Resource Manager popover** — compact live CPU/RSS/process tree and quick actions;
2. **Resources center workspace** — full process/port/LSP/runtime ownership and controls;
3. **Workspace Space center workspace** — disk treemap, worktree/build/cache breakdown, and safe cleanup;
4. **Usage popover** — current provider reset windows/percentages and account shortcuts;
5. **Usage center workspace** — historical token/session/activity/provider analytics.

The compact and full views use the same stores. Opening the full page must not start a second scan.

### 13.2 Resource Manager popover

Opened from a status-bar segment or floating action:

- total CPU and RSS;
- process tree grouped by repository/worktree/session/agent;
- active terminal/tool-terminal/language-server/Playwright ownership;
- expand/collapse groups;
- refresh;
- safe stop only for exact app-owned processes;
- “Review inactive workspaces”;
- “Space” summary and Scan/Review;
- action to open Resources or Workspace Space full page.

External processes are visible when useful but never stoppable.

### 13.3 Resources center workspace

Includes:

- complete app-owned process hierarchy;
- CPU/RSS sparkline/history from bounded samples;
- exact PID/PGID, owner, cwd/root, ports, state, age;
- tool-terminal and provider sidecar ownership;
- language server root/process/build-host status and logs;
- stop/restart/inspect with revalidation;
- provider adapter health/version/stderr;
- memory-pressure policy and recent evictions;
- links to the exact session, workflow agent, worktree, or terminal.

Preserve the master plan’s one-Roslyn-owner and no-process-name-kill rules.

### 13.4 Workspace Space full workspace

Model:

```ts
export interface WorkspaceDiskEntry {
  id: string;
  repositoryId: string;
  workspaceId: string;
  path: string;
  kind: 'worktree' | 'build-output' | 'dependency-cache' | 'agent-data' | 'other';
  bytes: number;
  reclaimableBytes: number;
  protection: DiskProtection;
  topLevelItems: WorkspaceDiskItem[];
}
```

UI:

- scanned/reclaimable/workspace/updated summary cards;
- treemap sized by bytes;
- selected workspace detail with top-level items;
- sortable/filterable virtualized list;
- active/main/dirty/unmerged/locked protection labels;
- select/clear/delete-selected workflow;
- every deletion routes through existing worktree/disk safety and explicit confirmation;
- disk scan is manual/visibility-gated and bounded to known roots.

No broad `du /`, Docker prune, repository-wide worktree prune, or unowned cache deletion.

### 13.5 Usage popover

Shows only authoritative current provider usage:

- provider/account;
- reset time/window;
- percent consumed/remaining according to the provider’s semantics;
- separate plan/Fast/Fable/other windows when exposed;
- unavailable/error reason when not exposed;
- refresh;
- open Usage details;
- manage accounts/auth through provider runtime.

Current quota data comes from ACP/provider-authored events or documented stable local APIs. Transcript volume is never presented as remaining quota.

### 13.6 Usage analytics workspace

Historical analytics derive from normalized runtime usage events and incremental transcript indexing.

```ts
export interface UsageEvent {
  id: string;
  provider: string;
  providerInstanceId: string;
  ownedId: string | null;
  workflowRunId: string | null;
  turnId: string | null;
  occurredAt: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  reasoningTokens: number | null;
  model: string | null;
  estimatedCost: number | null;
  estimateRateVersion: string | null;
}
```

Persistence:

- append normalized usage events under app support;
- maintain incremental cursor/index per provider transcript/runtime;
- maintain rebuildable daily/provider/model aggregate snapshots;
- source events remain append-only;
- no full rescan on every popup open;
- estimated cost is labelled **Estimated**, uses a versioned user-visible rate table, and remains unavailable when no applicable rate exists.

Full page includes:

- total tokens and estimated cost;
- active days, sessions, turns/events, agents spawned, work time, PRs created when deterministic receipts exist;
- daily intensity calendar;
- token mix;
- provider/model/workflow/project breakdowns;
- filter by date/provider/model/project/workflow;
- data-source health and last indexed cursor;
- export bounded CSV/JSON summary.

No secret prompts, file contents, cookies, auth tokens, or full provider raw events are stored in the usage index.

---

## 14. Reusable AI assistance throughout Assembly

### 14.1 One assistance recipe registry

Create one app-owned recipe registry over `AgentRuntimeManager` and deterministic facts:

```ts
export interface AssistanceRecipe<I, O> {
  id: string;
  title: string;
  surface: AssistanceSurface;
  inputSchema: JsonSchema;
  outputSchema: JsonSchema;
  buildContext(input: I): Promise<AssistanceContext>;
  providerPolicy: ProviderSelectionPolicy;
  mutationPolicy: 'none' | 'proposal' | 'confirmed';
  validateOutput(output: unknown): O;
}
```

Initial surfaces/recipes:

- PR title/body draft;
- PR code review;
- address review comments;
- fix failing checks;
- conflict resolution plan;
- commit-message draft;
- explain diff/file/problem;
- generate/review run configuration;
- summarize session/worktree/project/workflow;
- browser feedback implementation plan;
- form-field suggestions;
- pre-save advisory review;
- spec-compliance review;
- worktree cleanup/reorganization proposal;
- test-plan generation.

### 14.2 Deterministic validation versus AI advice

Every form/save surface distinguishes:

```text
Deterministic validation
  Required fields, types, paths, capabilities, stale facts, safety rules.
  May block save.

AI advisory review
  Completeness, wording, suspicious combination, missing context, likely mistake.
  Never silently blocks or changes data unless the user enables an explicit policy.
```

A save flow may offer:

```text
Review with AI
Apply selected suggestions
Save anyway
Cancel
```

The AI receives typed field values and bounded context, not raw component HTML or secrets. Suggestions are field-level patches with explanation and confidence. Applying a patch is explicit and undoable before save.

### 14.3 GitHub/PR integration

- Draft PR title/body from branch, diff summary, issue/task, commits, and template;
- review code with findings linked to file/line;
- address selected review thread;
- fix selected failed checks after deterministic log/artifact retrieval;
- prepare comments/review/merge but use existing two-step confirmation for remote mutation;
- never let hostile PR text add CLI args, tools, or instructions outside the delimited context.

### 14.4 Audit and provenance

Every recipe result records:

- recipe ID/version;
- provider/model/config used;
- input fact references and hash;
- output hash;
- accepted/rejected fields;
- proposed action;
- confirmation and deterministic execution result when applicable;
- redacted failure.

This extends the existing append-only orchestration audit and never records secrets or full image bytes.

---

## 15. Persistence, recovery, security, and trust boundaries

### 15.1 Sources of truth

```text
Provider conversation/context
  Provider/ACP session storage and native transcript.

Assembly normalized conversation journal
  Rebuildable projection/audit for UI continuity; not the provider context authority.

Owned session/workspace state
  Existing versioned local persistence keyed by ownedId.

Workflow runs
  Existing append-only orchestration ledger plus deterministic reducer.

Usage analytics
  Append-only normalized usage events plus rebuildable aggregate cache.
```

### 15.2 Recovery

On app restart:

1. reconcile owned sessions and surviving PTYs;
2. restore workspace snapshots without starting providers merely because rows exist;
3. reattach structured runtimes only for sessions that were active/running and whose provider supports resume/load;
4. restore terminal projections for surviving PTYs;
5. replay workflow ledger and mark indeterminate running nodes as recovering;
6. query provider session state before retrying work;
7. never rerun a possibly-completed consequential action without deterministic revalidation and user-visible outcome-unknown state.

### 15.3 Secrets

Never persist or expose through conversation/workflow/usage/browser telemetry:

- provider tokens/API keys;
- cookies/local storage/form values;
- full process environments;
- GitHub auth headers/tokens;
- keychain values;
- arbitrary provider stderr containing secrets;
- complete remote HTML;
- unredacted credential-bearing URLs.

### 15.4 Provider and sidecar trust

- pinned version/hash;
- executable path shown in settings;
- bounded stderr ring;
- no Tauri IPC capability;
- explicit workspace roots;
- process group cleanup;
- update is a separate reviewed action;
- custom provider manifests require explicit trust and cannot silently inherit existing credentials.

---

## 16. Exact work packages

### 16.0 Dispatch contract and cost-aware model manifest

Every implementation dispatch copies this complete contract into the agent prompt. A heading in
this document is not sufficient authorization.

1. Record base SHA, assigned owned paths, forbidden controller seams, dependencies, exact existing
   symbols, exact new symbols, ordered edits, focused tests, native acceptance, stop conditions,
   and the return-receipt template.
2. Use `fork_turns: "none"` and the explicit model/reasoning pair below. Never silently substitute
   Terra, an unrequested reasoning level, or an unspecified model. Luna Fast is preferred, never
   blocking: request `service_tier: "fast"` when the native spawn API exposes it; otherwise
   continue with explicit Luna Max/max/no-fork and do not claim Fast.
3. Feature agents edit only their leased files. They return exact integration receipts for
   `main.rs`, `tauriSource.ts`, public conversation/session/orchestration types, Settings, shell
   rosters, Tauri capabilities, package/Cargo manifests, and lockfiles. The controller applies
   each shared seam once after review.
4. No agent creates a second runtime, conversation store, orchestration ledger/reducer, Paneview
   factory/layout store, Dockview roster, settings authority, worktree authority, terminal
   registry, Browser child-view owner, resource owner, usage index, or confirmation executor.
5. No more than two build/test-heavy agents run concurrently; the default is one. Provider-native,
   Browser-native, and final Tauri proofs are serialized. A feature-agent packet may author focused
   tests but runs heavy commands only when the controller explicitly assigns a runner slot.
6. Every data lane repeats the hard query rule: filtering, joins, grouping, aggregation, sorting,
   and paging execute DB-side in one translated SQL query or database view; no materialize-then-
   shape, client evaluation, lazy loading, per-row follow-up, or N+1. Inspect generated SQL, add a
   view plus covering index when translation is insufficient, sweep sibling paths, and keep a
   roughly 20-row page to 1-3 total DB queries.
7. Whoever opens a Browser/native proof session stops that exact named daemon/process tree before
   returning. Whoever creates a worktree removes and prunes it immediately after merge,
   cherry-pick, push-to-PR, or abandonment; dirty/locked/unmerged trees are reported with exact
   path, branch, status, reason, and owner rather than deleted.

| Packet | Route | Release gate | Shared-seam owner |
| --- | --- | --- | --- |
| master WP0, WP12A | Luna Max / max / no fork, read-only or bounded preparation | explicit implementation authorization; current task/base | controller |
| A0-ACP, A0-Browser | Luna Max / max / no fork, evidence-only | R1 | controller |
| master WP1 contrast | SOL-medium / medium / no fork | R0 applies resolved extension policy | controller integrates |
| master WP1B identity | Luna Max / max / no fork | WP1 | controller integrates |
| master WP2 shell/shared contracts | SOL-medium / medium / no fork | WP1B | controller integrates |
| master WP2 file Dockview + Settings restoration | Luna Max / max / no fork | frozen WP2 shell contracts | controller integrates |
| A1 contracts | SOL-medium / medium / no fork | A0 evidence | controller |
| A2 runtime/ACP transport | SOL-medium / medium / no fork | R2 | controller integrates |
| A3-Codex, A3-Claude | Luna Max / max / no fork | A2 contract tests | controller integrates |
| A3-terminal | SOL-medium / medium / no fork | A2 contract tests | controller integrates |
| A4 conversation UI | Luna Max / max / no fork | A1 fixtures; native gate waits for A3 | controller integrates |
| A5 native handoff | SOL-medium / medium / no fork | A2+A3+A4 native proof | controller integrates |
| A6 WorkflowEngine | SOL-medium / medium / no fork | stable A2/A3 lifecycle | controller integrates |
| A7 Agent Control Center | Luna Max / max / no fork | A6 event/control API | controller integrates |
| A8 Paneview/session shell | Luna Max / max / no fork | A1 roster/snapshot contracts | controller integrates |
| A9 Browser product lane | Luna Max / max / no fork | A0-Browser+A1 | controller integrates; SOL owns native exception |
| A10 resources/space/usage | Luna Max / max / no fork | A1 resource identity | controller integrates |
| A10 Roslyn lifecycle | SOL-medium / medium / no fork | isolated lifecycle decision | controller integrates |
| A11 assistance recipes | Luna Max / max / no fork | deterministic A2/A4+A9/product adapters | controller integrates |
| A12 | controller only | R5 | controller |
| R0-R6 reviews | SOL-medium / medium / no fork, read-only | preceding milestone | no edits |

Each Luna lane stops and returns a bounded SOL decision packet when it encounters a genuinely
high-judgment runtime, security, process-ownership, persistence, schema, native feasibility, or
mutation-boundary choice. SOL does not absorb straightforward implementation merely because it
reviews the milestone.

### Work Package A0 — two evidence-only feasibility packets

**Sequential gate, parallel probes. No product wiring.** A0-ACP and A0-Browser may run after R1
as separate Luna Max agents with separate process/Browser ownership. Neither may edit current
product files, manifests, capabilities, routes, conversation/terminal/browser authorities, or
`main.rs`.

#### A0-ACP — adapter and packaging evidence

Current anchors are `agent_conversation/process.rs:8-92` (`ConversationProcess::spawn/stop`), the
dead spike in `provider.rs` (`run_provider`, `run_codex`, `normalize_codex`, `run_claude`,
`normalize_claude`), old public types in `protocol.rs:3-166`, and the intentionally unwired
`ensure_agent_conversation` at `mod.rs:283-293`. Cargo has no ACP SDK dependency. A0 must not
activate this spike or spawn a provider from the app.

Create only evidence under `docs/superpowers/evidence/tsk-808/acp-runtime-spike.md` and redacted
`acp-fixtures/{codex,claude}/`. In disposable external probes, record exact executable, version,
content hash, initialize/new/load/resume, text/image, model, effort/thought, mode/permissions,
assistant/reasoning/tool/file-change, approval/input, plan/task, child identity, cancel/close,
cold/warm time, RSS, process tree, stderr bounds, shutdown, and app-exit cleanup. Prove a structured
session creates no visible or hidden user PTY. Select a packaged Claude executable or bundled and
pinned Node 22 runtime; per-launch network install is forbidden.

Stop on unsupported image/config/load-resume/child identity, unpinned packaging, credential/root
overreach, leaked descendants, or inability to prove no PTY. Return evidence paths, versions/
hashes, capability matrix, unsupported features, packaging decision, process-cleanup receipt, and
`No product source changed`.

#### A0-Browser — native capture and overlay evidence

Current frontend anchors are `browserStore.svelte.ts` (`browser`, `activateBrowser`,
`setBrowserUrl`, `reloadBrowserFrame`, `captureBrowserState`, `restoreBrowserState`), the iframe at
`components/BrowserPanel.svelte:99-109`, `/next/+page.svelte:26-29,420,466,1035`, and
`sessionWorkspaces.ts:35-39,74-78`. No Rust WebView/WKWebView Browser bridge exists.

Create only `docs/superpowers/evidence/tsk-808/native-browser-spike.md`. In a uniquely named,
rebuilt-Tauri probe, prove one child-view identity across docked/floating/maximized/collapsed,
z-order and pointer/keyboard delivery, same/cross-origin inspector bounds, profile/cookie
isolation, no launch IO, native viewport snapshot scale/crop after scroll/zoom, exact
owner/generation/hash staging, and child cleanup. Capture only bounded selector/URL/title/rect data;
never cookies, full HTML, storage, tokens, or arbitrary script. An honest fallback is metadata-only
or user-selected whole-window capture, never a fabricated crop.

Stop and return U2 evidence if safe snapshot/overlay needs broad AppKit/WebKit control, z-order/
isolation fails, or the locked same-live-page behavior is impossible. The opener reports the named
Browser/Tauri process-tree cleanup. No `browser.rs`, inspector bridge, frontend model, capability,
or route is created during A0.

### Work Package A1 — freeze runtime, event, ownership, and config contracts

**Route/owner:** SOL-medium on a sequential controller-owned seam. Current anchors are
`ownedSessions.ts:10-68,85-237`, `sessionWorkspaces.ts:26-79,166-237`,
`conversationTypes.ts:1-148`, `conversationReducer.ts:8-181`,
`conversationStore.svelte.ts:23-236`, and Rust `agent_conversation/protocol.rs:3-166` plus
`mod.rs:34-215,283-380`.

Add `AgentExecutionOwner`, `AgentRuntimeState`, additive `OwnedAgentRuntimeFields`, revisioned
`AgentCapabilities`, provider manifests/config options/commands, canonical event/item/content/
request/approval/input types, writer-lease transitions, and `ToolTerminalIdentity` in the existing
public type authorities with matching Rust serde types. ACP-native frames do not escape adapters.
Migrate live PTYs to terminal owner and exited processes to stopped; structured is assigned only
after ACP init plus session/new. Version `SessionConversationWorkspace` for generation, owner,
draft, attachment IDs, config, child/parent scroll, sequence, and telemetry; preserve old unknowns
instead of inventing facts. Failed migration preserves PTY/native/transcript/draft/workspace/
attachments.

Add `scripts/agentRuntimeContracts.test.mjs` and extend protocol, reducer, isolation, workspace,
owned-session, and serde tests. Prove two `ownedId`s remain isolated; stale generation/sequence is
refused/repaired; unknown capability categories round-trip; migrations are lossless; and no test
starts a provider. Stop before A2 on any need for a second store/runtime or inability to express one
owner/generation/writer lease. Return exact public symbols, migration fixtures, tests/results, and
the controller commit SHA that freezes the seam.

### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport

**Route:** SOL-medium after R2. Refactor, do not coexist with, the current
`AgentConversationRegistry`, `ConversationSession`, `ProviderCommand`, `ensure`, `emit_payload`,
`send_command`, `snapshot`, and `close` in `agent_conversation/mod.rs`. `TerminalRegistry` remains
the sole user-PTY authority.

Own new `agent_conversation/manager.rs` (`AgentRuntimeManager`, `ManagedAgentSession`, writer lease,
generation/sequence, bounded event snapshot), `capabilities.rs` (manifest/capability/config
validation), `journal.rs` (bounded append/rebuild/repair), `providers/mod.rs` (`ProviderRegistry`),
`providers/acp.rs` (transport boundary), `providers/acp_client.rs` (initialize/session/prompt/
image/config/approval/input/cancel/close), and `providers/process.rs` (piped sidecar supervision,
bounded stderr, process-group cleanup). Tool terminals carry `TerminalKind` plus
`ToolTerminalIdentity` through the existing `TerminalRegistry`; they never set
`OwnedSession.ptySessionId` or appear as user PTYs.

Implement one manager per app, one current-generation lease per `ownedId`, full-state capability
replacement, canonical events, bounded snapshots/journal, stale-response rejection, close and
app-exit cleanup. Return controller receipts for `mod.rs`, `main.rs`, capabilities and manifests;
do not edit frontend, provider-specific mapping, terminal projection, handoff, Browser, or shared
A1 types.

Run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` in one
heavy slot. Tests cover initialization/new/load/resume, prompt/image, correlated approval/input,
config replacement, cancel/close, sequence repair, generation rejection, journal recovery, process
tree exit, no user PTY, and distinct tool-terminal identity. Native proof starts one pinned ACP
sidecar per structured `ownedId` with no PTY or leaked descendant. Stop on arbitrary credential/
root authority, duplicate runtime, cleanup failure, or stale-writer ambiguity. Return files,
symbols, registrations, dependency receipts, tests/results, native process evidence, and cleanup.

### Work Package A3 — provider adapters and capability mapping

**Parallel only after A2 contract tests pass.** A3-Codex and A3-Claude are Luna Max; A3-terminal is
SOL-medium. All consume A1/A2 public contracts, edit separate provider/parser files, and return
shared-seam receipts.

**A3-Codex:** own `providers/codex.rs`, redacted
`src-tauri/fixtures/agent_conversation/codex/*.jsonl`, and optional
`scripts/agentConversationCodex.test.mjs`. Map authoritative ACP text/image/model/effort/mode,
assistant/reasoning/tool/file, approval/input, plan/task, nested child, cancel/close/load/resume into
canonical events. Never activate the dead `provider.rs` or fall back to a PTY. Run focused Rust
fixture tests plus the public protocol test. Native proof shows ordered IDs, one manager writer, no
PTY, and cleanup.

**A3-Claude:** own `providers/claude.rs`, redacted
`src-tauri/fixtures/agent_conversation/claude/*.jsonl`, and optional
`scripts/agentConversationClaude.test.mjs`. Apply the same mapping/acceptance to A0's pinned Claude
packaging; no per-launch install or legacy `claude -p` path. Stop on unproven packaging, config,
image, child association, load/resume, or cleanup.

**A3-terminal:** replace whole-tail `agent_conversation/transcript.rs` with
`transcript/{mod,codex,claude}.rs` and add `terminal_projection.rs`. Cache canonical path plus file
identity, offset and partial final line; parse appended durable JSONL only; detect truncation,
rotation, archive, gaps and duplicates; perform bounded reconciliation; and emit canonical events
only from durable evidence. Add Rust fixture tests and
`scripts/agentConversationTerminalProjection.test.mjs`. Preserve `terminal.rs`/
`terminalService.ts` as the PTY authority. After event proof, return exact controller receipts to
remove `transcriptMirrors`, `refreshTranscript`, `startConversationTranscriptMirror`, and
`stopConversationTranscriptMirror` from `conversationService.ts:24-102`,
`ConversationSurface.svelte:64-67`, and `/next/+page.svelte:491-539`; do not remove polling first.

Stop any lane on invented capability/item data, stale owner/generation, duplicate writer/runtime,
or leaked reader/process/timer. Return mapping table, owned files/fixtures, public-seam receipts,
focused results, native evidence, and cleanup.

### Work Package A4 — conversation UI and screenshot input

**Route:** Luna Max over frozen A1 fixtures; native provider acceptance waits for all A3 lanes.
Extend the existing `ConversationSurface.svelte`, `ConversationMessage.svelte`, conversation store
helpers, and service adapters. Add `ConversationHeader.svelte`, `AgentConfigBar.svelte`,
`ConversationTimeline.svelte`, `TimelineItem.svelte`, typed item components for user/assistant/
reasoning/plan/tasks/command/file/tool/subagent/approval/input/error, `ConversationComposer.svelte`,
`AgentCommandMenu.svelte`, and `ConversationAgentTree.svelte`. A pure
`conversationCommandCatalog.ts` is allowed; a second store/runtime/provider service is not.

Ordered edits are screenshot paste/preview/send/restore/cleanup first; capability-driven model/
effort/mode/permissions; ordered typed timeline and safe Markdown/code; correlated approvals and
structured input; plan/task state; recursive read-only child tree; virtualized history/scroll
follow; then provider-advertised plus local command discovery. Slash commands are not a substitute
for controls, unsupported controls remain absent/disabled with reason, and screenshot paste never
auto-sends.

Extend existing protocol/store/isolation/workspace/paste tests and add
`conversationControls.test.mjs`, `conversationCommandCatalog.test.mjs`,
`conversationMessageSafety.test.mjs`, and `conversationTimeline.test.mjs`; run focused Node tests,
`pnpm check:svelte`, and `pnpm check` serially. Final rebuilt-Tauri proof shows image input,
capability controls, typed ordering, recursive child state, virtualization/scroll, commands, exact
owner isolation, and no hidden PTY. Stop on unbacked controls, flattening, cross-owner response,
unsafe Markdown/secret display, or automatic send. Return owned files, consumed contracts,
controller receipts, tests/results, native evidence, and cleanup.

### Work Package A5 — native CLI handoff

**Route:** SOL-medium only after integrated A2+A3+A4 native proof. Add
`agent_conversation/handoff.rs` for structured-to-terminal, terminal-to-structured, same-session/
fork, rollback, writer-lease/process-tree assertions, and exact history boundary/reconciliation;
add Rust tests and `scripts/agentConversationHandoff.test.mjs`. Extend the existing
`conversationService.ts` with typed calls through a controller receipt; do not create a second
handoff/provider service.

The controller wires actions into `ConversationSurface.svelte` and `/next/+page.svelte` while
preserving `terminal.rs`, `terminalService.ts`, `ownedSessions.ts`, workspace/draft/attachment
state, and generation. Structured-to-terminal opens the same native provider session only after
ACP writer release; terminal-to-structured loads/resumes only after TUI writer release. Same-
session and fork choices are distinct and labeled. Failure restores the prior owner/process and
preserves all state. The optional Codex shared-daemon optimization is a separate later probe.

Run focused handoff, owned-session, isolation, workspace, live-terminal, terminal-service, and
Rust agent-conversation tests serially. Native acceptance performs both directions, same/fork,
rollback, restart/recovery, exact native ID/history boundary, and proves one process-tree writer
with no duplicate sidecar/PTY. Stop on dual writer, stale generation, destructive failure, or
history mismatch. Return lease transitions, files, controller receipts, tests/results, native
process evidence, rollback evidence, and cleanup.

### Work Package A6 — WorkflowEngine core

**Route:** SOL-medium. Start only after A1 contracts and A2/A3 lifecycle APIs are stable. No UI
except fixtures and a test harness. The current anchors at `beebda6` are
`src-tauri/src/orchestration.rs:8-16` (`ORCHESTRATION_SCHEMA_VERSION`, store constants,
`OrchestrationEvent`), `:100-228` (`OrchestrationRun`, list/record entry points), `:230-365`
(normalized append-only JSONL read/write/reduce), and `:1096+` tests. `main.rs:1399-1413` wraps the
two commands and `:5173-5270` owns managed state/registration. `orchestrationView.ts` remains a
presentation adapter; its regex phase inference is never scheduling authority.

**Owned files and symbols:** implement `src-tauri/src/workflow.rs`; split only when size warrants
into `workflow/{types,reducer,policy}.rs`. Define `WorkflowDefinitionV1`,
`AgentRoleDefinition`, `WorkflowNodeDefinition`, `WorkflowRunState`, `WorkflowNodeState`,
`WorkflowLoopPhase`, `WorkflowRunRecord`, `WorkflowNodeRunRecord`, `WorkflowGateRecord`,
`WorkflowArtifactRef`, `WorkflowReducer`, `WorkflowScheduler`, `WorkflowPolicy`, `WorkflowEngine`,
`WorkflowCommand`, `WorkflowError`, injected `AgentRuntimePort`, `WorktreeLeasePort`, `Clock`, and
`IdGenerator`. Engine entry points are `create_run`, `start`, `pause`, `resume`, `cancel`,
`retry_node`, `skip_node`, `approve_gate`, `submit_result`, `dispatch_ready`,
`rebuild_from_events`, and `list_runs`; all are explicit and idempotent.

**Ordered edits:** validate definitions and cycles; freeze the input snapshot/hash; implement the
pure reducer; implement deterministic ready-node ordering; enforce global/workflow/provider
concurrency, depth, attempt, wall/token/tool-terminal/worktree budgets; map role policy to A2
capabilities; acquire/release existing worktree/file leases; dispatch through the single
`AgentRuntimeManager`; validate one of `ImplementationReceipt`, `ReviewReceipt`,
`SpecComplianceReceipt`, `VerificationReceipt`, or `PlanReceipt`; append every transition to the
existing ledger; rebuild exact state after restart. Markdown is display/overflow, never gate input.

**Controller receipts:** extend the existing `OrchestrationEvent` additively with serde-defaulted
workflow identity, node/parent, `ownedId`, attempt/depth, input hash, output contract/artifacts,
gate, lease, provider instance, provenance, sequence, and idempotency fields. Preserve old rows,
event kinds, JSONL path, CLI presets, and `OrchestrationRun`. Register one managed engine and the
commands `list_workflow_runs`, `create_workflow_run`, `start_workflow_run`,
`pause_workflow_run`, `resume_workflow_run`, `cancel_workflow_run`, `retry_workflow_node`,
`skip_workflow_node`, `approve_workflow_gate`, and `submit_workflow_result` through `main.rs` and
`tauriSource.ts`. The frontend cannot append transitions directly.

**Tests:** add Rust tests for DAG/cycle rejection, ready ordering, concurrency/provider/depth/
budget/timeouts, worktree/file leases, role mapping, retry/cancel/skip/gates, restart replay,
phase derivation, structured-output refusal, hostile prose, delegation authorization, and audit
order. Run serially:

```bash
cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml workflow -- --nocapture
cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml orchestration -- --nocapture
pnpm --dir tauri-svelte-preview test:orchestration-event
pnpm --dir tauri-svelte-preview test:workflow-contracts
```

Stop if a child bypasses A2, worktree/file leases, policy, or typed output; phase/gate decisions
depend on prose/regex; multi-writer ordering cannot be made deterministic in the same ledger; or a
second ledger/reducer/store/process path appears. Return exact files, symbols, ledger migration,
command receipts, tests/results, recovery evidence, process/worktree cleanup, and stop result.

### Work Package A7 — Agent Control Center and MCP delegation companion

**Route:** Luna Max after A6 event/control APIs pass. The lane consumes typed engine snapshots;
it does not read/reduce JSONL, infer state from `orchestrationView.ts`, or create another
conversation/workflow store.

**Owned files/symbols:** add `shell/workflows/workflowTypes.ts` with `WorkflowRunView`,
`WorkflowNodeView`, `WorkflowAgentView`, `WorkflowGateView`, `WorkflowArtifactView`, typed command
DTOs, and `provider-native|workflow` provenance; keep `workflowRunId` distinct from `ownedId`.
Add `workflowService.ts` for Tauri invokes/subscription only and `workflowStore.svelte.ts` as a
snapshot cache plus selection/filter state only. Add `WorkflowControlCenter.svelte`,
`WorkflowRunList.svelte`, `WorkflowRunHeader.svelte`, `WorkflowGraph.svelte`,
`WorkflowLaneBoard.svelte`, `AgentHierarchy.svelte`, `WorkflowTimeline.svelte`,
`WorkflowNodeInspector.svelte`, `AgentRuntimeInspector.svelte`, `WorkflowTemplateEditor.svelte`,
and `AgentActivityPane.svelte`.

If A6 approves the optional companion, add `src-tauri/src/workflow_mcp.rs` with only
`workflow_list_roles`, `workflow_delegate_agent`, `workflow_get_agent_status`,
`workflow_cancel_agent`, `workflow_submit_result`, and `workflow_request_gate`. Validate caller,
parent tool call, `ownedId`, generation, capability revision, role, depth, concurrency, budget,
workspace, provider, permission, and output contract before asking `WorkflowEngine`. It exposes no
arbitrary executable, path, Git, worktree, credential, secret, or raw process function.

**Controller receipts:** register one Agents/Workflows center panel in `centerDock.ts` and one
AgentActivity right pane through A8's registry; wire `main.rs`/`tauriSource.ts` commands once; open
existing child conversation/worktree surfaces by `ownedId`/lease ID. Do not edit shared rosters,
`ShellFrame.svelte`, `ShellSidebar.svelte`, or manifests in the Luna lane.

**Tests and native acceptance:** add `scripts/agentControlCenter.test.mjs` and
`scripts/workflowMcp.test.mjs`; prove snapshot-only state, provenance, typed phase, all control
actions, allow-list enforcement, hostile-output isolation, and absence of direct `invoke` outside
the service. In rebuilt Tauri, prove the run list, DAG/lanes/hierarchy/timeline/inspectors reflect
engine truth; pause/resume/cancel/retry/skip/approve/steer are real; provider-native and workflow
children are distinct; a child opens existing conversation/worktree surfaces; one child can be
pinned to Working; and MCP refusals/accepted actions have ledger receipts.

Stop if controls are no-op, identity/provenance is ambiguous, UI re-reduces raw events, MCP gains
unbounded authority, or a second store/ledger/runtime appears. Return owned files, service DTOs,
controller roster/command receipts, tests/results, native proof, cleanup, and stop result.

### Work Package A8 — Paneview shell and session navigation

**Route:** Luna Max after A1 freezes `ownedId`, Settled semantics, session snapshots, and roster
contracts. Extend `layout/paneStack.ts:1-274`, the sole `createPaneview` primitive, and
`layout/layoutStorage.ts:10-127`, the sole persistence authority. `ShellFrame.svelte:85-164`
continues to create one outer Gridview and one center Dockview. `ShellSidebar.svelte:67-313`
remains the only right-column Paneview caller. Do not introduce a second factory/store/key family.

**Owned files/symbols:** add pure `layout/sidePaneRegistry.ts` with `SidePaneRegion` and
`SidePaneRegistration`; it validates unique ID/region/order/size/persistence and maps to `PaneSpec`
without creating UI, storing state, or doing IO. Add `WorkingPane.svelte`, `DonePane.svelte`,
`SettledPane.svelte`, `WorktreeAgentRow.svelte`, and `SessionsPaneview.svelte`, which creates one
`PaneStack` and receives all data/actions as props. Add `sessionLibrary/
SessionLibraryWorkspace.svelte`, `sessionLibraryModel.ts`, `sessionLibraryService.ts`, and an
ephemeral `sessionLibraryStore.svelte.ts` for query/selection/page state only.

**Frozen contracts:** stable left IDs are `working`, `done`, `settled`; right registrations retain
Files/Source Control/Worktrees/Run/Context/Problems and add `agents` only when A7 exists. A1 adds an
explicit additive `settledAt: string|null` or reviewed `railSection` field to `OwnedSession`; old
records default safely and lose no PTY/native/transcript/workspace/attachment data. Never infer
Settled from age, title, or process state. Session Library deduplicates by `ownedId`, then
`(provider,nativeSessionId,canonicalCwd)`—never title—and derives current state from the existing
rail/runtime authorities rather than persisting a second archive.

**Ordered edits/controller receipts:** extend `PaneStackOptions` with an injected layout store,
retaining `storageKey` only as a compatibility adapter; migrate valid `viewPanesKey(id)` layouts
into one reviewed side-pane v1 map or retain that registry-generated family as the only keys;
preserve `ACTIVE_VIEW_KEY`, `SESSIONS_COLLAPSED_KEY`, and `RAIL_GROUPS_STORAGE_KEY` owners. Replace
the primary Working/Done Collapsibles and archive drawer in `SessionsColumn.svelte` with the left
Paneview host. Add `agents` and `session-library` to the existing center roster, bump
`CENTER_LAYOUT_KEY` v3 to v4 for the changed exact panel set, and leave outer grid v2 unchanged.
Move the one Session Library mount center-to-right through parking/reparenting without duplicate
IO or state. The controller alone changes `ownedSessions.ts`, `sessionRailStore`, `centerDock.ts`,
`sidebarViews.ts`, `ShellFrame.svelte`, `ShellSidebar.svelte`, and shared manifests.

**Tests:** add `sidePaneRegistry.test.mjs` and `sessionLibrary.test.mjs`; extend
`paneLayout.test.mjs`, `layout-storage.test.mjs`, `sidebar-views.test.mjs`,
`owned-sessions.test.mjs`, `session-strip.test.mjs`, and `session-groups.test.mjs`. Prove exact-set
migration/fallback, one Paneview factory, stable IDs, no construction IO, explicit Settled
migration, same-title/different-worktree identity, center/right one-mount movement, keyboard/focus,
and state preservation. Run focused scripts first and `pnpm --dir tauri-svelte-preview check` once
in the controller's serialized slot.

Native proof resizes/reorders/collapses/restores all left/right panes after restart; opens Agents
and Session Library; resumes exact same-title/different-worktree sessions; moves the one library
host without duplicate IO; pins one workflow child; and proves editor/browser/conversation/
workspace state survives. Stop on duplicate layout authority, implicit IO, invented Settled
semantics, title-based dedupe, double-mounted library, lost migration, or an unowned shared seam.
Return exact files, roster/key/migration receipts, tests/results, native proof, cleanup, and stop
result.

### Work Package A9 — Browser presentation and feedback

**Route:** Luna Max after a signed A0-Browser receipt and A1 identity contracts. Stop before dispatch
if A0 did not prove the rebuilt-native same-page child view, z-order/input, snapshot/inspector
boundary, profile isolation, and cleanup. The current anchors are
`browserStore.svelte.ts` (iframe-era state/snapshot helpers),
`components/BrowserPanel.svelte` (one `/next` singleton iframe), `/next/+page.svelte` (workspace
capture/restore and one Browser mount), `ShellOverlays.svelte` (one overlay root),
`ShellFrame.svelte`/`centerDock.ts` (one Browser center panel), `panelActivation.ts`/
`shellPanels.ts` (lazy activation), and `sessionWorkspaces.ts` (old optional URL snapshot). The
legacy `/` Browser panel stays frozen.

**Owned files/symbols:** add `shell/browser/browserTypes.ts`, `browserModel.ts`,
`browserBounds.ts`, `browserPresentation.ts`, `browserAnnotations.ts`, `browserBackend.ts`; migrate
`browserStore.svelte.ts` only as a compatibility facade preserving its snapshot exports. Add
`components/browser/BrowserTabs.svelte`, `BrowserToolbar.svelte`, `BrowserViewport.svelte`,
`BrowserFeedbackPanel.svelte`, `BrowserAnnotationToolbar.svelte`, `BrowserAnnotationCard.svelte`,
`BrowserOverlayHost.svelte`, and `BrowserExpandedOverlay.svelte`; make `BrowserPanel.svelte`
composition-only. Add Rust `src-tauri/src/browser.rs` and A0-approved
`browser_inspector.js`. `main.rs`, capabilities/config/manifests, shared shell/layout/route,
conversation, and Settings files are controller-owned receipts.

Use exact contracts `BrowserPresentationMode = 'docked'|'floating'|'maximized'|'collapsed'`,
`BrowserInteractionMode = 'browse'|'picking'|'annotating'|'drawing'`, viewport presets
`responsive|mobile-s|mobile-m|mobile-l|tablet|laptop|laptop-l|desktop|custom`,
`BrowserFloatingBounds`, and immutable `BrowserFeedbackAttachment` carrying workspace/tab/
generation/URL/title/selector/name/snippet/note/intent/image/source hash. `BrowserTabState` carries
identity, navigation/load, viewport, annotations, generation and error; workspace state carries
one ordered tab map, active ID, presentation/previous mode, floating bounds, interaction, pending
selection/markup, queue, opaque profile summary, and active generation. Enforce selector <=2 KB,
snippet <=500 characters, <=32 classes and <=100 annotations per tab.

`browserModel.ts` exports `activateBrowserWorkspace`, `deactivateBrowserWorkspace`,
`createBrowserTab`, `selectBrowserTab`, `closeBrowserTab`, `navigateActiveBrowserTab`,
`setBrowserViewport`, `setBrowserPresentationMode`, `expandBrowserFrom`,
`restoreBrowserToDock`, `minimizeBrowserToPrevious`, `collapseBrowserToControl`,
`beginBrowserElementPicker`, `acceptBrowserElementSelection`, `cancelBrowserAnnotation`,
`queueBrowserAnnotation`, `removeBrowserAnnotation`, `captureBrowserWorkspace`,
`formatBrowserFeedback`, and `stageBrowserFeedbackPreview`. One native child view exists per tab;
hide/measure/show the same view for overlays and movement, never create a second iframe/WebView.

Rust owns bounded commands `create_browser_tab`, `set_browser_tab_bounds`, `show_browser_tab`,
`hide_browser_workspace`, `navigate_browser_tab`, `reload_browser_tab`, `go_back_browser_tab`,
`go_forward_browser_tab`, `close_browser_tab`, release-gated `open_browser_tab_devtools`,
`open_browser_tab_external`, `clear_browser_workspace_data`, `arm_browser_element_picker`,
`cancel_browser_element_picker`, and only A0-approved viewport capture. Events carry workspace,
tab and monotonic generation; stale events are dropped. Reject non-HTTP/S, userinfo, file/data/
javascript/custom protocols. Profiles are opaque and workspace-isolated; import cookies stays
disabled absent a separate security decision.

Grab stages bounded metadata without a note or send; Annotate requires note plus Change/Question
and queues immutable metadata/crop; Draw uses a native visible-viewport snapshot and local markup,
then restores the same live view without reload. Feedback targets exact `ownedId`/generation through
the existing conversation draft and attachment vault with preview/merge; mismatch/draft conflict
retains the queue and never PTY-writes or auto-sends. No cookies, storage, form values, full HTML,
tokens or provider screenshots are captured.

The controller mounts exactly one Browser overlay and one global `WorkbenchActionFab` under the
existing `ShellOverlays`; it derives a `WorkbenchActionContext` from the current center/owner/
workspace/target/generation and routes existing services. The island owns layout/focus/
confirmation only, remains >=44 px and 16 px from safe edges, and uses fan/horizontal/bottom-sheet
collision fallbacks. Browser actions expand/restore the same page. It never duplicates the command
palette or business logic.

Add `browserModel.test.mjs`, `browserBounds.test.mjs`, `browserAnnotations.test.mjs`,
`browserFeedback.test.mjs`, `browserBackend.test.mjs`, and `actionSurfaceModel.test.mjs`; extend
URL normalization, session-workspace, activation and Tauri-config tests. Prove four-state
transitions, bounds/presets, no-reload identity, bounded immutable feedback, stale target/draft
conflict, unsafe URL refusal, profile/capability behavior, and no secret fields. Rebuilt-Tauri
acceptance covers three tabs, real navigation, same/cross-origin selection, profile A/B isolation,
all viewports, drag/resize/max/min/restore/collapse, Grab/Annotate/Draw, exact draft staging,
global actions across Session/Editor/Browser/Resources/History, accessibility, dialog z-order, and
complete named child-view/Browser/Tauri cleanup. Browser preview is never acceptance.

Stop on absent A0 proof, unsafe inspector/snapshot, z-order/input/scroll failure, broad capability,
unproven profile isolation, owner/generation ambiguity, second view/store/palette, guessed capture,
or unclean process tree. Return exact files, state/command/event contracts, controller receipts,
migrations/security notes, focused results, native artifact, `No SQL/database touched`, cleanup,
and stop result.

### Work Package A10 — Resources, space, and usage

Split after A1 resource identity into four Luna Max sublanes and one isolated SOL-medium Roslyn
lane. Compact/full surfaces share one service/store; opening a full page never starts a second
scan/index. Current anchors are `main.rs:4028-4044,4190-4250,4546-4557`,
`terminal.rs:23-164`, `lsp.rs:1448-1611,2145-2274`, the existing Playwright process service/store,
`shellPanels.ts`, `panelActivation.ts`, and the single `centerDock.ts` roster. No SQLite dependency
exists at `src-tauri/Cargo.toml:1-35`; A10 therefore creates a reviewed database explicitly rather
than pretending JSONL or IndexedDB is an analytics authority.

#### A10-resources — one bounded process snapshot

Own `core/src/scanners/resources.rs`, `src-tauri/src/resources.rs`,
`shell/resources/{resourceTypes,resourceBackend,resourceService,resourceViewModel}.ts`,
`resourceStore.svelte.ts`, and compact/full resource components. Parse exactly one bounded
`ps -axo pid=,ppid=,pgid=,%cpu=,rss=,etime=,user=,command=` and one
`lsof -nP -iTCP -sTCP:LISTEN -Fpcn`, then join by PID once in Rust. Never run per-PID `ps`/`lsof`
or hidden mount-time scans. Join only proven `TerminalRegistry`, A2 manager, LSP registry and
Playwright identities. `TerminalSessionInfo` currently omits `owned_id` despite the start request;
provider PID/PGID is private. A1/controller must expose those fields additively; until then mark the
process External, never guess from name/path.

Define `ProcessOwner` variants App, OwnedSession, LanguageServer, ProviderSidecar, Playwright and
External plus a single `ResourceSnapshot`. Controller receipts add `read_resource_snapshot`,
`read_resource_disk_scan`, `stop_owned_resource`, `restart_language_server_root`,
`set_active_source_root`, `apply_resource_memory_pressure`, and `read_language_server_log` once.
Stop/restart revalidates PID, PGID, root, owner and registry generation; external rows never expose
Stop. Compact and full UI use one coalesced explicit/visible refresh and identical owner IDs.

#### A10-space — bounded manual disk scan

Own `core/src/scanners/disk.rs` or an explicitly leased part of `resources.rs`,
`src-tauri/src/resources_disk.rs`, and `shell/resources/workspaceSpace*` types/service/store/view.
Scan only explicit repository/worktree/build/dependency/agent roots with bounded depth/entries; no
broad `du`, Docker prune, repository-wide prune, or unowned deletion. Each `WorkspaceDiskEntry`
carries stable repository/workspace ID, path, kind, bytes, reclaimable bytes, top-level items and
`DiskProtection = Active|Dirty|Unmerged|Locked|UserData|SafeCandidate|Unknown`. Only
SafeCandidate is reclaimable. Deletion routes the existing `remove_project_worktree`/archive safety
and confirmation after a fresh identity/protection check; primary, active, dirty, unmerged, locked,
unknown and user data are refused. Return before/after byte and cleanup receipts.

#### A10-usage-current — authoritative quota only

Own `src-tauri/src/usage_current.rs` plus the shared usage-card adapter. Define
`ProviderUsageReader` and `ProviderUsageState = Available|Unavailable|Error`. Consume only A1 ACP
provider-authored quota windows or a documented stable local API; current input/output token fields
are not quota. Include provider/account/instance, semantics, authoritative percentages/reset,
capture time, source/version, and unavailable reason. Never infer remaining quota from transcript
size, local tokens, turns or elapsed time; never expose credentials/private args/raw output.

#### A10-usage-history — SQLite, incremental cursors, and DB-side analytics

Own `src-tauri/src/{usage_db,usage_sources,usage_indexer,usage_history}.rs`, migration
`migrations/0001_usage_history.sql` (or the exact embedded equivalent), and
`shell/usage/{usageTypes,usageBackend,usageService,usageAnalytics}.ts`,
`usageStore.svelte.ts`, and Usage workspace components. Controller adds `rusqlite`, state,
commands, capabilities and `tauriSource.ts` wrappers once. Open
`app_data_dir()/usage-history.sqlite3` with test-only `MAC_COMMAND_BAR_USAGE_DB`; migrations are
transactional. Money is integer micros, never float.

Migration 0001 creates `usage_events` with primary `id`, provider/instance, nullable owner/workflow/
turn/project/workspace IDs, event time, nonnegative input/output/cache/reasoning tokens, model,
nullable estimated-cost micros plus rate version, source kind/event ID/key and inserted time; add
`UNIQUE(provider,provider_instance_id,source_kind,source_event_id)`. Create covering indexes for
time/provider/model, provider/model/time, owner/time, workflow/time and project/time. Create
`usage_source_cursors` keyed by provider/instance/source kind/opaque source key with file identity,
offset, size, mtime, last event and update time; `usage_rate_versions` with provider/model/effective
range and integer rates plus lookup index; and `usage_daily_rollups` keyed by
day/provider/model/project with token/cost/event/session/turn/workflow aggregates. Expose a
`usage_daily_provider_model` view. Store no raw path, prompt, content, provider JSON, cookie,
credential or auth data.

ACP rich usage is primary. Codex/Claude JSONL fallback reads only bytes after a durable cursor;
truncation/rotation resets safely. One transaction bulk `INSERT OR IGNORE`s events, updates cursors,
then rebuilds affected rollups with set-based `DELETE` plus `INSERT ... SELECT ... GROUP BY` or an
equivalent DB-side upsert. Cumulative counters become deltas only with proven semantics. App start,
runtime event, or explicit refresh coalesces the index; opening a popover never full-scans tails.

`read_usage_summary` is one prepared `SELECT` containing `COUNT`, `COUNT(DISTINCT ...)`, and
`SUM(...)` over `usage_events` with date/provider/model/project/workflow filters. The breakdown page
is one prepared statement with `filtered` and `grouped` CTEs, SQL `GROUP BY`, deterministic
`ORDER BY`, `LIMIT/OFFSET`, `COUNT(*) OVER()` and filtered window totals. Daily/calendar/export
queries read the view with SQL filters/sort/page. A roughly 20-row page uses 1-3 total statements
for summary, rows and calendar. Rust maps rows only; TypeScript formats labels only. Neither may
group, aggregate, filter, sort or page results. Capture `EXPLAIN QUERY PLAN` and redacted SQL under
`MCB_SQL_TRACE=1`; reject client evaluation, N+1, full-table scans that ignore available indexes,
or any materialize-then-shape sibling. Estimated cost exists only when a versioned rate matches and
is labeled Estimated; no match is null/Unavailable, never zero.

#### A10-Roslyn — isolated lifecycle consolidation

SOL-medium owns `lsp.rs` lifecycle/process/public inventory, `csharpLanguageClient.ts` frontend
pool removal, `shellPanels.ts` hidden-warm removal, and exact EditorPanel/CodeLens compatibility.
Add `SourceLspProcessInfo`, `set_active_source_root`, `source_lsp_processes`,
`stop_server_for_root`, `restart_server_for_root`, `apply_memory_pressure`,
`read_server_log`, and clamped `ResourcePolicy`. Native C# is the sole process owner; active root is
pinned, inactive saved rows start zero Roslyn, warning evicts oldest inactive, critical stops all
inactive. Shutdown is LSP shutdown/exit, bounded wait, group TERM, bounded wait, KILL only fallback,
reap and prove no BuildHost descendants. Preserve semantic CodeLens/Peek and record
`MCB_TIMING=1` baseline/final evidence; a populated lens/Peek remains under two seconds without tab,
source, session, or workspace snapshot calls.

Focused tests cover parser/join bounds, exact ownership/refusal, disk protection/totals, quota
Unavailable semantics, DB migration/cursors/rotation/dedupe/no-secret rows, rollup equivalence,
rates, generated query/index plans and single-statement endpoints, plus Roslyn lifecycle/CodeLens.
Rebuilt-Tauri acceptance proves compact/full single state, exact owned stop/external refusal,
manual protected disk cleanup, authoritative-or-unavailable quota, restart-safe incremental usage,
DB-side filters/paging/export, no duplicate tokens, Roslyn pressure/cleanup, and no second scan.
Stop on guessed ownership/quota, per-PID query, broad disk deletion, duplicate scan/store, unstable
usage IDs/deltas, cursor duplication, raw secret persistence, DB-side rule violation, unverified
query plan, Roslyn duplicate/hard-kill-only lifecycle, CodeLens/Peek regression, or unclean native
proof. Each sublane returns owned files, shared receipts, focused/native results, SQL trace or
`No database touched`, and process/worktree cleanup.

### Work Package A11 — Assistance recipes throughout product

**Route:** Luna Max after A2/A4 expose one correlated structured-output request and A9/product fact
and action adapters are deterministic. Current `conversationService.ts::sendStructuredMessage` is
free text and is insufficient; do not scrape timeline prose or create a second provider/runtime.
No database is touched.

Own `shell/assistance/assistanceTypes.ts` with stable `AssistanceRecipeId`s for PR draft/review,
review comments, checks, conflicts, commit, diff/file/problem explanation, run-config generate/
review, workspace summary, Browser feedback, form suggestion, save/spec review, worktree cleanup and
test plan; `AssistanceSurface`, `AssistanceTargetRef`, hashed `AssistanceFactRef`, bounded
`AssistanceContext`, allow-listed `AssistanceFieldPatch`, and expiring `AssistanceProposal`.
Own `assistanceSchemas.ts` with strict input/output validators; reject unknown/prototype-bearing/
oversized objects, shell fragments in typed fields and non-allow-listed patches. Own
`assistanceRecipeRegistry.ts` with one versioned registration per recipe and mutation policy
`none|proposal`; no initial recipe executes directly.

Own `assistanceContext.ts` (`captureAssistanceContext`, `hashAssistanceFacts`,
`isAssistanceContextCurrent`, `revalidateAssistanceTarget`) and capture existing stores
synchronously with no implicit IO. Treat repository/diff/diagnostic/Browser/task/model text as
delimited untrusted data; exclude secrets, environment, prompts, cookies, hidden DOM and unrestricted
files. Own `assistanceStore.svelte.ts` for request presentation state only and
`assistanceService.ts` (`requestAssistance`, `applySelectedAssistancePatches`,
`cancelAssistanceRequest`). The service verifies request/owner/generation/facts/output hashes,
expiry and expected-value hash, then delegates selected fields to the existing typed surface
adapter. It never invokes Tauri, Git, shell, worktree, GitHub, filesystem, Browser, save or
confirmation directly.

Own `assistanceAudit.ts` for redacted additive requested/proposed/dismissed/stale/apply-started/
applied/refused/failed/outcome-unknown events and components `AssistanceAction.svelte`,
`AssistanceProposal.svelte`, and one `AssistanceHost.svelte`. The proposal shows provenance,
confidence, select/deselect, Apply selected, Dismiss, Retry and Continue without AI. Provider
failure never disables deterministic Save. Consequential proposals go through the existing
preview/confirmation/revalidation/audit executor.

Return controller receipts for the A2/A4 structured-output symbol; additive nullable recipe/hash/
confirmation/result fields in the existing orchestration event; exactly one host in
`ShellOverlays.svelte`; contextual insertion in Conversation, Git/diff, Problems, Run Configuration,
Browser, Context and form/save surfaces; and discovery-only `shellCommands.ts` entries. The agent
must not edit these shared seams, `main.rs`, capabilities, Settings, rosters, manifests or locks.

Add `assistanceRecipes.test.mjs`, `assistanceContext.test.mjs`,
`assistanceService.test.mjs`, `assistanceAudit.test.mjs`, and `assistanceUiContract.test.mjs`.
Cover every registration/version, bounded/untrusted context, secret redaction, correlation, stale
owner/generation/facts, schema/patch refusal, expiry/cancel/retry, expected-value mismatch,
advisory nonblocking behavior, audit order/outcome-unknown and zero direct mutation. Native proof
drafts/reviews several surfaces, resists hostile text, invalidates a changed target, applies one
safe field through the existing executor, cancels a consequential proposal with zero external
change, survives restart with audit receipts, and keeps deterministic actions usable when the
provider fails.

Stop on missing correlated structured output, target/hash identity, typed executor, bounded
context, allow-listed fields, or required shared-seam edit; stop on a second runtime/store/action
authority or direct mutation. Return recipe versions, owned files, consumed services, integration
receipts, tests/results, native/hostile/stale/audit evidence, `No database touched`, `No direct
mutation path`, cleanup, limitations and stop result.

### Work Package A12 — controller integration and certification

**Owner:** controller only. Reject any A0-A11 receipt that lacks base SHA, owned files, exact symbol,
registration/capability/migration, focused result, native evidence where required, known limits,
and cleanup. Freeze dispatches; inventory `git status --short`; attribute every path; reject stale,
overlapping or unowned changes.

Integrate in this order: Rust public contracts; Tauri state/setup/commands; frontend adapters/types;
Settings migrations/sections; center/side/session/library/action rosters; dependency manifests and
locks once; focused lane gates; one serialized full frontend/Rust gate; rebuilt-Tauri scenarios one
at a time; security, relevance, SQL/data, cleanup and R6 reviews; bounded fixes and repeated failed
gates; proof ledger/PR/task disposition; immediate worktree removal after merge, push-to-PR or
abandonment.

For `main.rs`, every receipt names the module/import, `.manage` singleton, setup listener and owner,
exact handler, capability marker, app-exit cleanup, bounds/identity/generation/permission and test.
Prove one `AgentRuntimeManager`, WorkflowEngine/ledger, terminal registry, Browser child-view owner,
resource/process owner and usage index. `capabilities/default.json` remains scoped to main and adds
only exact command/plugin permissions; never blanket shell/filesystem/process/HTTP/global-shortcut/
IPC or remote URLs. Backend feature detection remains distinct from Tauri ACL.

For package/Cargo receipts, deduplicate exact package/crate/version/features, consuming symbol,
license/security, process/network/filesystem and size effects; run each package manager once; reject
hand-edited locks or unrelated churn. Settings extends the existing `settingsStore.svelte.ts`
storage key and `SettingsDialog.svelte`/`SettingsHost`; each field has safe old-backend default,
enum validation, `mergeWithDefaults` migration, capability truth, UI row, reset, runtime apply/
rollback, persistence test and native restart proof. A11 adds no automatic-assistance default.

Integrate center IDs through `centerDock.ts`/`ShellFrame.svelte`, side IDs through
`sidebarViews.ts`/`ShellSidebar.svelte`, icons through `ActivityBar.svelte`, activation through
`panelActivation.ts`/`shellPanels.ts`, commands through `shellCommands.ts`, and singleton overlays
through `ShellOverlays.svelte`. Every roster receipt supplies stable ID/title, region, order/size/
collapse, parking host, activation/loader, migration, command/icon/accessibility, and teardown.
Reject IDs missing from a companion roster or any second Dockview/Paneview/layout authority.

After focused gates, run serially:

```bash
cd tauri-svelte-preview
pnpm check:svelte
pnpm check
pnpm build
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri:build
```

The rebuilt app must pass section 19.7 plus cold upgrade, truthful old-backend controls, one Codex
and Claude structured session without PTY, one-writer handoff, workflow restart/retry/cancel,
Browser owner/annotation, resource/usage identity and generated-SQL proof, A11 cancel/apply/audit,
complete app-exit cleanup, both themes, narrow window, keyboard, VoiceOver, Reduced Motion and
reload/restart. Run R6 as explicit read-only SOL-medium; route fixes and repeat until no blocker.

The final receipt lists base/final SHA, all lane receipts, unowned-diff result, Rust registrations/
ACLs/capabilities, package/Cargo/settings/rosters/actions, focused/full/native results, security,
relevance, SOL review, SQL trace or no-DB results, process/Browser/worktree cleanup, proof-ledger
rows, task/PR states and limitations. TSK-808 closes only after its whole deliverable is actually
complete and verified; planning completion alone leaves it open.

---

## 17. File ownership and controller seams

### 17.1 Controller-owned shared seams

Preserve the master-plan controller list and add:

- `src-tauri/src/agent_conversation/mod.rs` registration/public contract;
- `src-tauri/src/orchestration.rs` event schema/public reducer seams;
- `src/lib/shell/conversation/conversationTypes.ts` public canonical types;
- `src/lib/shell/ownedSessions.ts` migration/public identity;
- `src/lib/shell/sessionWorkspaces.ts` snapshot version;
- center and side roster IDs;
- package/Cargo manifests and locks;
- Tauri capabilities and `main.rs` command registration.

Feature agents return exact receipts for these files; the controller applies them in serialized commits.

### 17.2 Parallel lane ownership after contract freeze

| Lane | Owned paths | Heavy runner |
| --- | --- | --- |
| A0-ACP | evidence directory only; disposable external probe | one named provider-probe slot |
| A0-Browser | evidence file only; disposable native probe | one named native-Browser slot |
| Runtime/ACP | `agent_conversation/{manager,capabilities,journal}.rs`, provider transport/process | focused Rust slot |
| Codex provider | `agent_conversation/providers/codex*`, fixtures/tests | focused Rust slot |
| Claude provider | `agent_conversation/providers/claude*`, fixtures/tests | focused Rust slot |
| Terminal projection | transcript parsers, `terminal_projection.rs`, fixtures/tests | focused Rust slot |
| Native handoff | `handoff.rs` and handoff tests after provider proof | focused Rust/native slot |
| Conversation UI | conversation components/store helpers/tests excluding shared type seam | Node-light |
| Workflow engine | new workflow/orchestration engine files and reducer tests | focused Rust slot |
| Agent UI | new agent/workflow Svelte modules and tests | Node-light |
| Paneview/session shell | side-pane registry, session row/library components/tests | Node-light |
| Browser | browser Rust/frontend modules and tests | focused Rust/native slot |
| Resources/space | resources/disk modules and UI | focused Rust slot |
| Usage current/history | provider quota adapter, SQLite/index/query modules and UI | focused Rust/SQL slot |
| Roslyn lifecycle | `lsp.rs`, C# client/panel lifecycle and evidence | isolated SOL Rust/native slot |
| AI recipes | recipe/context/proposal modules and tests | Node-light |
| A12 integration | all returned shared-seam receipts | controller; serialized full/native |

No more than two heavy runners. Native browser and provider integration proofs run one at a time.

---

## 18. Combined dependency graph and execution schedule

This is the single execution schedule for the amendment and the still-authoritative foundation in
the TSK-808 master plan. It does not permit ACP, conversation, Paneview, Browser, resource, usage,
or workflow work to bypass R0's application of the resolved Works now/Bounded adapter policy,
contrast foundation, Assembly identity migration, restored Settings authority, or frozen shell
contracts.

The controller owns shared seams, integration commits, heavy-runner assignment, proof-ledger
updates, and review/fix routing. At most six file-independent agents may work concurrently, at most
two heavy build/test runners may overlap, and the default is one. Provider integration proof,
native Browser proof, and final rebuilt-Tauri certification run one at a time.

```text
explicit implementation authorization; Luna Fast preferred and tier absence nonblocking
  -> master WP0 re-anchor/reconcile
  -> master WP12A API/adapter comparison
  -> R0 SOL-medium read-only review
  -> apply RESOLVED U1: release Works now/Bounded adapter; skip all other new rows; no pause
  -> master WP1 contrast/tokens/shared primitives
  -> master WP1B Assembly identity/compatibility migration
  -> master WP2 SOL shell contracts -> Luna file Dockview + Settings restoration
  -> R1 SOL-medium read-only review
  -> A0-ACP and A0-Browser feasibility packets
  -> U2 only if a locked requirement or native/security architecture changes
  -> A1 shared contract freeze
  -> R2 SOL-medium read-only review
  -> A2 AgentRuntimeManager/ACP transport
       -> A3-Codex ─┐
       -> A3-Claude ├─> A2+A3+A4 integration -> structured-session native proof -> A5
       -> A3-terminal┘
  A1 -> A4 fixture-driven conversation UI
  A1 -> A8 Paneview/session shell
  A0-Browser+A1 -> A9 Browser product lane
  A1 -> A10 resources/space/current-usage/history; isolated A10-Roslyn gate
  -> R3 SOL-medium read-only review
  A2+A3 stable runtime APIs -> A6 WorkflowEngine
  A6 stable events/control API -> A7 Agent Control Center
  A4 exact targeting + deterministic product services -> A11 assistance recipes
  -> R4 SOL-medium read-only review
  A5+A6+A7+A8+A9+A10+A11 -> controller cross-surface integration
  -> R5 SOL-medium read-only review
  -> A12 controller certification
  -> R6 SOL-medium final read-only review
  -> bounded fixes, rerun failed gates, PR/task disposition, immediate cleanup
```

Every `R0`-`R6` review is a separate native-agent dispatch with
`agent_type: "default"`, `fork_turns: "none"`, `model: "gpt-5.6-sol"`, and
`reasoning_effort: "medium"`. The prompt grants no edit, build, test, browser, worktree, Notion,
commit, or fix authority. A milestone does not release dependents while its review has a blocking
finding. Straightforward fixes return to Luna Max; unresolved design/process-ownership choices go
to a bounded SOL-medium decision or implementation packet. The same review is rerun after fixes.

### Milestone 0 — re-anchor and apply resolved extension/API policy

1. Execute master Work Package 0 from refreshed `origin/main`; reconcile live task/code/native
   evidence and re-anchor every named shared symbol.
2. Execute only master packet 12A's API-first real-Tauri comparison. Do not execute generic VSIX
   stages 18.3-18.5.
3. Run R0 over the singleton host/security boundary, comparison classifications and metrics,
   editor/Peek safety, repository evidence, and cleanup receipts.
4. Apply the recorded U1 decision: release only Works now and Bounded adapter, then continue to
   Milestone 1 without another user pause.
5. Preserve existing declarative assets, but skip new Declarative only, Elevated host required,
   and Rejected rows without blocking other work. A future exact third-party package/hash or
   elevated-host proposal is separate deferred scope, not a gate on this wave.

### Milestone 1 — contrast, identity, Settings, and shell foundation

Sequentially after R0 applies the resolved policy:

1. Master Work Package 1: contrast, legibility, focus, and shared semantic primitives.
2. Master Work Package 1B: public Assembly identity/artifacts while preserving every compatibility
   identity and existing user state.
3. Master Work Package 2: SOL-medium freezes the high-judgment shell roster, shared-action,
   quick-open, edge-minimization, and nested-Dockview contracts; Luna Max then executes the fully
   specified file-Dockview and existing-Settings restoration packets against those contracts.
4. Run R1 over contrast, focus, identity compatibility, Settings authority, shell/action ownership,
   Dockview nesting, accessibility, migrations, and controller-owned seams.

No amendment UI packet or later master-plan UI packet starts before R1 passes.

### Milestone 2 — ACP/Browser feasibility and shared contracts

1. Run A0-ACP and A0-Browser as separate bounded evidence packets. They may share no process,
   Browser, temporary directory, or cleanup ownership.
2. Trigger U2 only if a probe contradicts a locked top-priority behavior or requires a new native,
   packaging, trust, or security architecture. Successful probes create no ceremonial pause.
3. Execute A1. The controller freezes `ownedId`, writer lease, provider/config/capability,
   canonical event/item/request, tool-terminal, snapshot migration, resource identity, and
   center/side roster contracts, commits them, and opens exact file leases.
4. Run R2 over A0 evidence and A1 contracts, including capability truth, no-hidden-TUI proof,
   process/Browser cleanup, migration safety, Browser feasibility, and shared seams.

### Milestone 3 — A2 first, then the first parallel implementation wave

1. Execute A2 before any A3 lane. A2 implements the single Rust `AgentRuntimeManager`, ACP
   supervision/transport, canonical events, bounded recovery, stale-generation handling, and
   process cleanup. It owns one heavy slot when its focused Rust test runs.
2. Only after the A2 transport contract passes, fan out A3-Codex, A3-Claude, and A3-terminal.
   A3-terminal is mandatory; it owns incremental transcript projection, imported-session
   reconciliation, and native/tool-terminal identity fixtures.
3. A4 may develop over frozen A1 fixtures while A2/A3 run, but provider-native acceptance waits
   for all three A3 lanes and controller integration.
4. A8 may run after A1's roster/snapshot contract; it must extend the existing Paneview authority.
5. A9 HTML/state work may run after A0-Browser+A1. Native child-view work and proof use one heavy
   slot and preserve the master Browser safety contract.
6. A10 resources, space, current usage, and history may fan out after A1 resource identity. The
   A10-Roslyn sublane remains isolated and high-judgment.
7. Unaffected master lanes may run only when their own dependencies pass and their exact files do
   not overlap A2-A10 leases. Conflicting older conversation/Browser/resource packets are replaced,
   never run as parallel authorities.
8. Run R3 over A2, all three A3 lanes, A4 fixture/native parity, A8 migration, A9 isolation and
   cleanup, A10 ownership/query behavior, focused evidence, and shared-seam receipts.

### Milestone 4 — runtime integration, handoff, workflows, and assistance

1. The controller integrates A2, all A3 lanes, and A4 once, then proves one Codex and one Claude
   structured session with image input, advertised config, typed events, plans/tasks, subagents,
   cancel/close/load, and no user-visible PTY.
2. Execute A5 only after that proof. Prove structured/native round trip, same-session/fork behavior,
   rollback, one writer, exact history reconciliation, and process-tree cleanup.
3. Execute A6 after A2/A3 lifecycle APIs stabilize. It extends the existing orchestration ledger
   with deterministic scheduling, budgets, retries, gates, worktree/file leases, recovery, and ACP
   children; it may not create a second ledger.
4. Execute A7 only after A6 event/control APIs pass.
5. Finish A9/A10 integrations without exceeding the heavy-runner limit.
6. Execute A11 only after A4 exact targeting and its consumed Git, PR, Browser, form/save,
   Problems, run-configuration, confirmation, revalidation, and audit services are deterministic.
7. Run R4 over one-writer handoff, workflow determinism, delegation boundaries, worktree/file-lease
   enforcement, control-center truth, Browser/resource/usage joins, assistance-recipe safety,
   native evidence, and cleanup.

### Milestone 5 — cross-surface integration

After R4, the controller applies reviewed receipts once and proves:

- Browser grab, annotation, and marked screenshots target the exact ACP conversation;
- provider-native children and workflow agents retain distinct provenance;
- workflow agents appear in Agent Control Center, optional Working pins, and Session Library;
- resource/usage facts join exact agent/session/workflow/tool-terminal identities;
- Session Library unifies structured/imported history without becoming the active left rail;
- assistance slots reuse deterministic product services and existing mutation boundaries;
- every primary side region uses the single Paneview authority and every center destination uses
  the single Dockview roster;
- only Works now and Bounded adapter extension/API paths execute for new candidates; existing
  declarative assets remain preserved.

Run R5 over the complete cross-surface diff and proof ledger. Fix and rerun R5 before A12.

### Milestone 6 — A12 certification and closure

A12 is controller-owned:

1. apply remaining Rust registrations, frontend adapters, roster entries, Settings sections,
   capabilities, and dependency-lock receipts once;
2. run focused tests, serialized full gates, rebuilt-Tauri native acceptance, security/relevance
   review, generated-query proof where applicable, and complete process/Browser cleanup;
3. run R6 over the integrated diff, proof ledger, native evidence, security/data boundaries,
   cleanup, and task disposition;
4. route bounded fixes by the model manifest and repeat failed gates/R6 until no blocker remains;
5. open/merge a PR only under explicit authorization, verify each independent task's closure, and
   remove/prune the integration worktree immediately when merged, pushed to the PR, or abandoned.

---

## 19. Focused tests and acceptance

### 19.0 Exact controller command matrix

Future script names below are added to `tauri-svelte-preview/package.json` exactly once by A12
after their files exist. Until then, execute a new test with
`node --experimental-strip-types scripts/<file>.test.mjs`; do not claim an absent package script
passed. Run focused commands before full gates and do not overlap more than two heavy runners.

```bash
cd tauri-svelte-preview

# A1/A2/A3/A4/A5 conversation and runtime
pnpm test:agent-conversation-protocol
pnpm test:agent-conversation-store
pnpm test:conversation-session-isolation
node --experimental-strip-types scripts/conversationWorkspaceRestore.test.mjs
node --experimental-strip-types scripts/agentRuntimeContracts.test.mjs
pnpm test:paste-cleanup
node --experimental-strip-types scripts/agentConversationTerminalProjection.test.mjs
node --experimental-strip-types scripts/conversationControls.test.mjs
node --experimental-strip-types scripts/conversationCommandCatalog.test.mjs
node --experimental-strip-types scripts/conversationMessageSafety.test.mjs
node --experimental-strip-types scripts/conversationTimeline.test.mjs
node --experimental-strip-types scripts/agentConversationHandoff.test.mjs
pnpm test:live-terminals
pnpm test:next-terminal-service
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture

# A6/A7 workflow and control center
pnpm test:orchestration-event
node --experimental-strip-types scripts/agentControlCenter.test.mjs
node --experimental-strip-types scripts/workflowMcp.test.mjs
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml workflow -- --nocapture
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml orchestration -- --nocapture

# A8 shell/session layout
node --experimental-strip-types scripts/sidePaneRegistry.test.mjs
pnpm test:pane-layout
pnpm test:layout-storage
pnpm test:sidebar-views
pnpm test:owned-sessions
pnpm test:session-strip
pnpm test:session-groups
node --experimental-strip-types scripts/sessionLibrary.test.mjs

# A9 Browser/action surface
pnpm test:normalize-browser-url
node --experimental-strip-types scripts/browserModel.test.mjs
node --experimental-strip-types scripts/browserBounds.test.mjs
node --experimental-strip-types scripts/browserAnnotations.test.mjs
node --experimental-strip-types scripts/browserFeedback.test.mjs
node --experimental-strip-types scripts/browserBackend.test.mjs
node --experimental-strip-types scripts/actionSurfaceModel.test.mjs
pnpm test:session-workspaces
pnpm test:panel-activation
pnpm test:tauri-config
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml browser -- --nocapture

# A10 resource, disk, usage and Roslyn
RUST_TEST_THREADS=1 cargo test --manifest-path ../core/Cargo.toml resources -- --nocapture
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml resources -- --nocapture
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage -- --nocapture
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml lsp -- --nocapture
pnpm test:csharp-language-client
pnpm test:source-code-lens-keys
pnpm test:source-ui
pnpm test:tauri-source

# A11 assistance
node --experimental-strip-types scripts/assistanceRecipes.test.mjs
node --experimental-strip-types scripts/assistanceContext.test.mjs
node --experimental-strip-types scripts/assistanceService.test.mjs
node --experimental-strip-types scripts/assistanceAudit.test.mjs
node --experimental-strip-types scripts/assistanceUiContract.test.mjs

# Shared static gate after each integrated light batch
pnpm check:svelte
pnpm check
```

A10 usage tests run with a disposable `MAC_COMMAND_BAR_USAGE_DB`, enable `MCB_SQL_TRACE=1`, inspect
each generated statement and `EXPLAIN QUERY PLAN`, assert indexes used, assert one query per endpoint
and 1-3 total queries for a typical page, then remove only that test database. A10 Roslyn proof uses
`MCB_TIMING=1`. Browser/provider/native sessions use unique names and report their exact cleanup.

### 19.1 Runtime/ACP tests

Add focused scripts/Rust tests for:

- provider initialization and version/hash;
- capability mapping and unknown categories;
- config option full-state replacement;
- image input success and unsupported fallback;
- ordered assistant/reasoning/tool/file/plan/task events;
- permission and user-input correlation;
- cancel/close/recovery;
- session list/load/resume;
- subagent parent metadata;
- process exit and cleanup;
- no PTY for structured session;
- tool-terminal identity separate from user PTY;
- stale generation and sequence repair.

### 19.2 Conversation tests

- screenshot paste is first-priority and exact-owner safe;
- draft/attachments restore independently for two sessions;
- model/effort/mode controls are not slash commands;
- TUI-only commands absent;
- provider-advertised/local commands merge correctly;
- typed timeline ordering;
- safe Markdown and code copy;
- approval and structured input;
- plans/tasks;
- live and historical child tree;
- long-session virtualization and scroll-follow.

### 19.3 Workflow tests

- DAG validation/cycle rejection;
- ready-node scheduling;
- concurrency and provider limits;
- depth/budget/timeouts;
- worktree policy and file-lease conflict;
- role config mapping to provider capabilities;
- retry/cancel/skip/gate behavior;
- restart replay/recovery;
- deterministic phase derivation;
- agent-directed delegation authorization;
- hostile child output cannot become a control command;
- structured output validation;
- audit event order.

### 19.4 Paneview/navigation tests

- Working/Done/Settled are Paneview registrations;
- right primary panels are Paneview registrations;
- layout migration preserves existing visibility/sizes;
- compact rows and hover/focus details;
- Session Library separate from active left rail;
- moving Session Library reuses state;
- no construction-time IO;
- keyboard/VoiceOver and minimum width.

### 19.5 Browser tests

In addition to the original master-plan browser tests:

- docked/floating/maximized/collapsed transitions;
- floating-bounds clamp/restore;
- same native page identity/no reload;
- grab versus annotate versus draw;
- native snapshot/crop scale;
- markup undo/clear/save;
- exact owner/generation/hash staging;
- Settings/dialog z-order;
- profile/viewport isolation;
- app-exit child-view cleanup.

### 19.6 Resource/usage tests

- one bounded process snapshot and owner joins;
- external stop refusal;
- provider sidecar/tool-terminal ownership;
- disk treemap totals and protection;
- existing worktree safety for delete;
- current quota authority/unavailable reason;
- incremental usage indexing and duplicate-event rejection;
- aggregate rebuild equivalence;
- rate-table version and estimated-cost labels;
- no secret/raw prompt persistence.

### 19.7 Native end-to-end acceptance

One rebuilt-Tauri proof must show:

1. Create a new Codex structured session. No user PTY exists. Paste a screenshot, select model/effort/permissions, observe reasoning/tool/file/plan/task items, and open a child agent.
2. Repeat with Claude using its advertised options and a nested child transcript.
3. Open Codex in native CLI, continue the same conversation, return to structured mode, and prove one writer and exact history reconciliation.
4. Start a workflow with Orchestrator -> two Implementers in separate worktrees -> Test -> Code Review -> Spec Compliance -> Fix -> final review. See every node/agent, current loop phase, tool output, worktree, finding, gate, retry, and completion.
5. Cancel one child and retry one failed node without disturbing siblings.
6. Pin one workflow agent to Working; verify the rest remain in the workflow tree and Session Library.
7. Open the Browser from the bottom action island in floating mode, maximize it, grab an element, annotate another, draw on a screenshot, and stage all three into the exact active conversation without auto-send.
8. Open Resource Manager compact view, Resources full page, Workspace Space treemap, Usage compact view, and Usage analytics. Values share one underlying state and actions target exact owners.
9. Use AI to draft a PR, review a diff, suggest form fields, and run a pre-save advisory check. Cancel one proposal and confirm one disposable safe action; audit receipts remain after restart.
10. Working/Done/Settled and every right primary panel resize/collapse/reorder as Paneview panes and restore after restart.
11. Session Library searches and resumes an exact same-title/different-worktree session without cluttering the active rail.
12. Both themes, minimum/narrow window, keyboard, trackpad, VoiceOver, Reduced Motion, reload/restart, and complete child-process/webview/PTY/Roslyn cleanup pass.

---

## 20. Stop conditions and explicit user decision gates

A stop condition is not automatically a user decision. The controller first returns a bounded
evidence or repair packet for routing, ownership, dirty-seam, capability, test, cleanup, and
implementation failures. U0 and U1 are resolved and nonblocking. U2 is conditional evidence,
not an up-front selection or a reason to pause independent lanes.

### 20.1 Explicit user gates

- **U0 — resolved, nonblocking:** Luna Fast is preferred. Request and report
  `service_tier: "fast"` when available; if the native API has no tier field, immediately continue
  with explicit `gpt-5.6-luna`, `max`, `fork_turns: "none"` and make no Fast claim. Missing or
  unreported Fast tier never pauses an implementation lane. Model and reasoning-effort overrides
  remain the previously approved routing contract.
- **U1 — resolved, nonblocking extension/API selection:** release new candidates classified as
  `Works now` or `Bounded adapter`. Preserve existing declarative assets, but skip new
  `Declarative only`, `Elevated host required`, and `Rejected` rows without blocking the wave.
  The comparison and R0 are evidence/review gates, not a new user-selection pause. A future exact
  third-party package/hash or elevated-host proposal remains separate deferred scope.
- **U2 — A0 architecture exception:** ask only if ACP or Browser feasibility contradicts a locked
  top-priority behavior or requires a new native, packaging, trust, or security architecture. A
  successful A0 creates no ceremonial pause.

If live TSK-808 is already closed, continuation requires explicit user revival; do not reopen it
or keep implementing from historical plan text.

### 20.2 Controller stop and repair conditions

Stop the affected lane, preserve evidence, and do not release dependents when:

- the required Luna Max or SOL-medium model/effort route is unavailable, rejected, or downgraded;
- refreshed `origin/main`, current task state, or a protected merged checkpoint cannot be safely
  attributed;
- a packet needs an unlisted or controller-owned file, overlaps an active lease, or the controller
  cannot attribute its diff;
- more than two heavy build/test runners would overlap;
- A2 has not passed its contract tests before any A3 provider or A3-terminal lane starts;
- provider capability/config values would be guessed, hard-coded, or shown without effect;
- image input, load/resume, subagent parent identity, cancel/close, cleanup, or no-hidden-TUI
  behavior cannot be demonstrated;
- structured/native handoff cannot prove one writer, rollback, and exact history reconciliation;
- workflow scheduling/recovery would parse agent prose or bypass worktree, concurrency, budget,
  permission, output-schema, or file-lease policy;
- delegation would gain arbitrary process, path, Git, worktree, credential, executable, or IPC
  authority;
- Paneview/Dockview migration would create a second layout/store authority or lose persisted state;
- Browser clipping, profile/capability isolation, snapshot/overlay safety, exact ownership, or
  cleanup fails;
- resource/process ownership cannot be revalidated or provider quota would be guessed;
- assistance would silently mutate, block, or bypass preview/confirmation/revalidation/audit;
- CodeLens/Peek counts, editor identity, or tab-strip visibility changes without an attributable
  trace;
- SQL aggregation, grouping, filtering, joins, sorting, or paging would occur in memory, generated
  SQL cannot be inspected, or an N+1/load-then-loop path remains;
- focused, full, or native proof cannot be run, attributed, and cleaned up.

If one of these conditions requires U2, the review card contains exact alternatives, evidence,
security/resource consequences, and a recommended choice for the affected lane. Independent lanes
continue. Otherwise route the bounded repair under section 16's model manifest without
interrupting the user.

---

## 21. First implementation action

Do not begin with amendment A0 and do not revive the retired PTY-only Work Package 10A packet.

After implementation is explicitly authorized, with Fast preferred already recorded and tier
absence nonblocking:

1. refresh `origin/main`, verify TSK-808 is still open, record the exact base SHA, and create the
   controller-owned worktree only under
   `/Users/blackcolours/dev/work/worktrees/mac-command-bar/<task-slug>`;
2. execute master Work Package 0 only and preserve merged CodeLens/Roslyn, extension, DiffEditor,
   SCM, conversation, and native-proof boundaries;
3. execute master packet 12A's API-first real-Tauri comparison only;
4. run R0 with explicit `gpt-5.6-sol`, `medium`, `fork_turns: "none"`, and no edit authority;
5. apply the resolved U1 policy: release Works now/Bounded adapter, skip every other new row, and
   continue without another user pause;
6. execute master Work Package 1, then Work Package 1B, then Work Package 2;
7. run R1 with the same explicit SOL-medium read-only contract;
8. only after R1 passes, execute A0-ACP and A0-Browser, then continue through section 18.

The first broad UI work remains contrast/shared primitives, Assembly identity compatibility, and
the restored Settings/shell authority. Screenshot paste and structured conversation controls are
the first amendment-owned user-visible work after A0/A1/A2 contracts; slash-command expansion and
cosmetic transcript work are not substitutes.

---

## 22. Research source index

Primary sources reviewed for this amendment:

- ACP Rust SDK: `https://github.com/agentclientprotocol/rust-sdk`
- ACP session configuration options: `https://agentclientprotocol.com/rfds/session-config-options`
- Codex ACP adapter: `https://github.com/agentclientprotocol/codex-acp`
- Claude Agent ACP adapter: `https://github.com/agentclientprotocol/claude-agent-acp`
- T3 Code provider runtime: `https://github.com/pingdotgg/t3code/blob/main/packages/contracts/src/providerRuntime.ts`
- Jockey ACP client/orchestrator: `https://github.com/recailai/jockey`
- CodeG delegation broker: `https://github.com/xintaofei/codeg/tree/main/src-tauri/src/acp/delegation`
- Orca browser implementation: `https://github.com/stablyai/orca/blob/main/src/renderer/src/components/browser-pane/BrowserPane.tsx`
- Orca compact worktree/session card family: `https://github.com/stablyai/orca/tree/main/src/renderer/src/components/sidebar`

Repository source-of-truth files remain the paths named throughout the two prior plans and the current `main` implementation. Re-anchor before dispatch because line numbers and symbols may move.
