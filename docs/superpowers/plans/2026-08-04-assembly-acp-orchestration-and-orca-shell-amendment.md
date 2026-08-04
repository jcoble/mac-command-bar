# Assembly ACP Runtime, Workflow Orchestration, and Orca-Inspired Shell Amendment

**Date:** 2026-08-04  
**Status:** Superseding product and implementation amendment; implementation is not authorized by this document alone.  
**Repository baseline reviewed:** `jcoble/mac-command-bar` at `4e8192e0cdce791f53e93af39a6d2d2e2c322911`.  
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

Add `ToolTerminalRegistry` beside, not inside, the existing `TerminalRegistry` unless the existing registry can be safely extended with an explicit terminal kind.

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
- a workflow can complete while retaining worktrees for review;
- removal is never automatic merely because an agent finished.

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

Create a generic side-pane registry:

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

Create three distinct product areas over shared deterministic services:

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

### Work Package A0 — re-anchor, prove adapters, and settle packaging

**Sequential. No broad implementation before completion.**

1. Re-anchor the latest `main` symbols and current task/plan state.
2. Record the current Codex/Claude CLI and transcript versions/shapes.
3. Build a minimal Rust ACP client spike using the official SDK.
4. Start pinned Codex ACP and Claude ACP adapters in disposable sessions.
5. Record capabilities and prove:
   - new session;
   - image prompt;
   - model option;
   - effort/thought option;
   - mode/permission option;
   - assistant/thought stream;
   - command/file-change tool;
   - approval request;
   - plan/task update where supported;
   - subagent/nested transcript where supported;
   - cancel;
   - close;
   - load/resume.
6. Prove no user-visible PTY is created unless requested.
7. Choose Claude adapter packaging: packaged executable or bundled pinned Node 22 runtime.
8. Record cold start, warm start, RSS, child process tree, shutdown, and app-exit cleanup.
9. Render a capability matrix and stop if a top-priority requirement cannot be demonstrated.

**Outputs:**

- `docs/superpowers/evidence/tsk-808/acp-runtime-spike.md`;
- redacted protocol fixtures;
- exact adapter versions/hashes;
- packaging decision;
- no product wiring.

### Work Package A1 — freeze runtime, event, ownership, and config contracts

**Sequential controller-owned seam.**

- add the types in sections 5 and 6;
- migrate `OwnedSession` and conversation workspace snapshots;
- define provider manifest/capability/config types;
- define writer-lease transitions;
- define canonical event/item/request types;
- define tool-terminal identity;
- add pure reducer/migration tests;
- do not start a real provider yet.

### Work Package A2 — Rust `AgentRuntimeManager` and ACP transport

- refactor the existing `agent_conversation` registry;
- add process supervision and ACP client connection;
- add initialization, session lifecycle, prompt, image, cancellation, config, permission, user-input, and close;
- emit canonical events;
- retain bounded snapshots/journal;
- process cleanup and stale-generation tests;
- no frontend UI changes beyond adapter tests.

### Work Package A3 — provider adapters and capability mapping

**Parallel after A2 contract freeze:**

- A3-Codex: Codex ACP mapping and fixtures;
- A3-Claude: Claude ACP mapping and fixtures;
- A3-terminal: incremental transcript projection and imported-session reconciliation.

Each lane edits separate provider/parser files and returns shared-seam receipts.

### Work Package A4 — conversation UI and screenshot input

**Can run in parallel with A3 over deterministic fixtures.**

Order inside the lane:

1. screenshot paste/preview/send/restore/cleanup;
2. config bar and model/effort/mode options;
3. typed timeline and Markdown;
4. approvals/structured input;
5. plans/tasks;
6. live subagent tree;
7. virtualized history and scroll-follow;
8. provider/local command menu.

Native provider proof waits for A3 integration.

### Work Package A5 — native CLI handoff

- structured -> terminal;
- terminal -> structured;
- rollback on failure;
- same-session and fork flows;
- one-writer process/tree assertions;
- exact history reconciliation;
- optional Codex shared-daemon spike kept separate.

### Work Package A6 — WorkflowEngine core

- versioned workflow/role/node contracts;
- deterministic scheduler/reducer;
- existing orchestration-ledger integration;
- concurrency/budget/depth/retry/gate policy;
- worktree/lease integration through existing services;
- ACP child session dispatch;
- restart recovery;
- no UI except fixtures/CLI test harness.

### Work Package A7 — Agent Control Center and MCP delegation companion

- run list, graph/lane board, hierarchy, timeline, inspectors, template editor;
- compact AgentActivity Paneview;
- pause/resume/cancel/retry/skip/approve/steer;
- optional allow-listed MCP companion for agent-directed delegation;
- provider-native and workflow-agent provenance;
- no direct process/worktree mutation from MCP tools.

### Work Package A8 — Paneview shell and session navigation

- side-pane registry;
- Working/Done/Settled as left Paneview panes;
- right tool sections as Paneview registrations;
- compact worktree/agent rows and hover/focus details;
- Session Library separate center/right movable surface;
- persisted sizes/order/collapse;
- migrate old layout without losing workspace state.

### Work Package A9 — Browser presentation and feedback

- preserve original Work Package 8 native child-view safety;
- add floating/maximized modes;
- toolbar actions;
- element grab/annotation;
- native snapshot spike;
- screenshot markup;
- exact conversation staging;
- profile/viewport/settings;
- global floating action island integration.

### Work Package A10 — Resources, space, and usage

Parallel sublanes after shared data contracts:

- A10-resources: process ownership and compact/full resource UI;
- A10-space: disk scan/treemap/cleanup integration;
- A10-usage-current: authoritative provider quota popover;
- A10-usage-history: incremental normalized usage events and analytics workspace;
- A10-Roslyn: preserve the master plan’s separately reviewed high-judgment lifecycle consolidation.

### Work Package A11 — Assistance recipes throughout product

- typed recipe registry;
- PR and review recipes;
- Git/diff/Problems/run-config/browser/form/save recipes;
- reusable AI action slot/popover;
- field-level proposal UI;
- confirmation/revalidation/audit integration;
- hostile-context tests;
- no automatic consequential mutation.

### Work Package A12 — controller integration and certification

- register Rust commands/capabilities once;
- wire frontend adapters once;
- integrate center/side rosters once;
- add settings sections only for implemented capabilities;
- serialized build/test/native proof;
- security/relevance/SOL review;
- proof ledger and task disposition;
- PR and worktree cleanup.

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
| Codex provider | `agent_conversation/providers/codex*`, fixtures/tests | focused Rust slot |
| Claude provider | `agent_conversation/providers/claude*`, fixtures/tests | focused Rust slot |
| Terminal projection/handoff | transcript parsers, `terminal_projection.rs`, handoff tests | focused Rust slot |
| Conversation UI | conversation components/store helpers/tests excluding shared type seam | Node-light |
| Workflow engine | new workflow/orchestration engine files and reducer tests | focused Rust slot |
| Agent UI | new agent/workflow Svelte modules and tests | Node-light |
| Paneview/session shell | side-pane registry, session row/library components/tests | Node-light |
| Browser | browser Rust/frontend modules and tests | focused Rust/native slot |
| Resources/space | resources/disk modules and UI | focused Rust slot |
| Usage | usage index/analytics modules and UI | Rust-light/Node |
| AI recipes | recipe/context/proposal modules and tests | Node-light |

No more than two heavy runners. Native browser and provider integration proofs run one at a time.

---

## 18. Parallel execution schedule

### Milestone 0 — sequential decisions

1. A0 re-anchor and ACP/native-browser feasibility spikes.
2. User reviews capability/packaging matrix if a top-priority behavior differs from expectation.
3. A1 freezes shared contracts.
4. Controller commits the contracts and opens file leases.

### Milestone 1 — first parallel wave

Run up to six lanes, with at most two heavy runners:

| Slot | Packet | Dependencies |
| --- | --- | --- |
| 1 | A3 Codex provider | A2 transport contract |
| 2 | A3 Claude provider | A2 transport contract |
| 3 | A4 conversation UI over fixtures | A1 event/config contract |
| 4 | A8 Paneview/session shell | A1 owned/session/roster contract |
| 5 | A9 browser HTML/state plus native spike | browser roster and attachment contract |
| 6 | A10 resources/space data model/UI | resource identity contract |

The two provider focused Rust lanes share the heavy slots first. Browser native proof waits until one provider releases a slot. Node-only lanes continue.

### Milestone 2 — runtime integration and product capabilities

1. Integrate A2/A3/A4 and run one Codex plus one Claude structured session proof.
2. Run A5 handoff after structured sessions pass.
3. Dispatch A6 WorkflowEngine while A9 browser product work and A10 usage work continue.
4. Dispatch A7 Agent Control Center after workflow events and runtime APIs stabilize.
5. Dispatch A11 assistance recipes after conversation targeting and deterministic context services pass.
6. Roslyn high-judgment consolidation remains isolated under the master plan’s review gate.

### Milestone 3 — cross-surface integration

- browser annotations and marked screenshots attach to exact ACP conversations;
- workflow agents appear in control center and optional pinned left rail;
- resource process tree links to agent/session/workflow/tool terminal;
- usage links to provider/model/project/workflow;
- Session Library merges ACP and scanner history;
- AI recipe slots are added to PR/Git/browser/forms/save/Problems/run-config surfaces;
- all side regions use Paneview and all center destinations use the one Dockview roster.

### Milestone 4 — one certification cycle

Controller applies shared receipts, runs focused tests, serialized full gates, security and relevance review, native evidence, proof ledger, PR, and cleanup.

---

## 19. Focused tests and acceptance

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

## 20. Stop conditions

Stop and return a decision packet rather than improvising when:

- Codex or Claude ACP cannot demonstrate first-class image input;
- required model/effort/mode controls cannot be discovered and updated authoritatively;
- the adapter requires an unpinned network-installed runtime at every launch;
- a structured session creates or requires a hidden native TUI as its ordinary transport;
- one-writer handoff cannot be proven;
- a provider-native session cannot be safely loaded/resumed from the intended source;
- subagent parent identity cannot be proven;
- workflow scheduling would depend on parsing agent prose;
- workflow child creation would bypass worktree/concurrency/permission policy;
- an LLM delegation tool would gain arbitrary process, path, Git, or worktree access;
- Paneview cannot preserve existing panel state/layout without replacing the shell architecture;
- native WKWebView snapshot/annotation controls cannot render above the child view safely;
- resource ownership cannot be revalidated;
- provider quota data is unavailable and the UI would need to guess;
- AI save/form assistance would silently mutate or block without explicit policy;
- more than two heavy runners overlap;
- a controller-owned seam is dirty or concurrently leased;
- native proof cannot be run and cleaned up.

---

## 21. First implementation action

Do not begin the broad feature wave from the old Work Package 10A PTY-only packet.

After explicit implementation authorization:

1. refresh `origin/main` and re-anchor current symbols;
2. create the controller-owned TSK-808 integration worktree under the existing mandated worktree root;
3. execute **Work Package A0 only**;
4. produce the Codex/Claude ACP capability and packaging report, including image input, config options, tools, plans/tasks, subagents, resource cost, and proof that a structured session creates no user PTY;
5. execute the native Browser snapshot/overlay spike required by section 12.4;
6. stop for a decision only if one of the locked top-priority requirements fails or requires a new native/security architecture;
7. otherwise freeze A1 contracts and then begin the parallel schedule in section 18.

The first broad user-visible implementation after those contracts is screenshot paste plus structured conversation controls, not slash-command expansion or cosmetic transcript work.

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
