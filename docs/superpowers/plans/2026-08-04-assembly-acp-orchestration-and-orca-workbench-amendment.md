# Assembly ACP Runtime, Workflow Orchestration, and Orca-Inspired Workbench Amendment

**Date:** 2026-08-04  
**Status:** Product direction locked; implementation not started  
**Execution authority:** This is the newest plan for the areas named below. It amends and supersedes conflicting language in:

- `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`
- `docs/superpowers/plans/2026-08-02-tsk-809-810-conversation-workbench.md`

The earlier product-wave plan remains authoritative for all unaffected work packages, safety requirements, compatibility identities, testing discipline, and milestone review gates. This amendment replaces the architecture and execution details for:

- Work Package 2 and Work Package 3 session/navigation layout where this amendment specifies Paneview ownership and a separate session library;
- Work Package 8 browser presentation, capture, annotation, and conversation handoff;
- Work Package 9 resource and provider-usage presentation;
- Work Package 10A conversation runtime, controls, commands, attachments, tools, plans, tasks, and subagents;
- Work Package 11 integrated agent and workflow orchestration;
- the cost-aware dispatch manifest and lane dependency graph for those packages.

The older TSK-809/810 file remains historical discovery evidence only. Do not dispatch its PTY-only architecture.

---

## 1. Why this amendment exists

The current repository has a strong session/worktree/PTY foundation and a useful transcript-backed conversation projection, but the product target is larger than a transcript viewer:

1. A Codex-app/T3-style structured conversation for Claude Code and Codex with one visual language.
2. Real model, effort, permission, mode, and other runtime selectors rather than fake metadata buttons or slash-command proxies.
3. Rich live items: reasoning, plans, tasks, command execution, command output, file changes, diffs, MCP calls, approvals, user questions, context compaction, and errors.
4. Screenshot paste directly into the composer as a true image attachment. This is a release-blocking priority, not polish.
5. Live provider subagent visibility plus app-owned agents that can be started, paused, cancelled, retried, reassigned, and inspected.
6. Durable workflows in which one orchestrator coordinates implementers, code reviewers, spec-compliance reviewers, test/fix agents, or any user-defined roles, potentially across different providers and models.
7. An Orca-like embedded browser that can dock, float, fill the workbench, grab an element, annotate it, draw on a screenshot, and stage exact visual feedback into the active conversation.
8. Compact resource and provider-usage popovers that open into full workspaces.
9. A narrower, quieter left rail; hover summaries; accordion detail; a separate session library; and Paneview-backed left/right stacks.
10. AI assistance throughout pull requests, reviews, forms, save flows, validation, and other deterministic product surfaces without giving the model an unvalidated mutation path.

The current PTY-only design cannot provide authoritative live controls, approval requests, tool lifecycle, or provider-neutral workflows. The replacement is not “remove the terminal.” It is an explicit dual-mode architecture with one writer at a time.

---

## 2. Locked product decisions

### 2.1 ACP is the structured runtime

New Claude Code and Codex sessions started from Assembly use an ACP-compatible structured runtime by default.

- Codex uses a pinned `codex-acp` sidecar, which in turn owns Codex app-server.
- Claude Code uses a pinned `claude-agent-acp` sidecar, which uses the official Claude Agent SDK.
- Assembly is the ACP client.
- The existing Rust/Tauri process boundary owns adapter startup, restart, health, logging, cancellation, and shutdown.
- The Svelte frontend never speaks provider JSON-RPC directly.

ACP is a protocol connection, not a user-facing terminal. Starting an ACP session does **not** create an xterm tab or launch the Codex/Claude full-screen TUI. The adapter is a managed process and may run commands or expose terminal output as tool activity, but that tool output is not the same thing as a raw interactive provider CLI.

### 2.2 The terminal remains a first-class execution mode

The existing PTY registry, xterm views, scrollback, tombstones, reattachment, and login-shell behavior remain.

A provider conversation has exactly one execution owner:

```ts
export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';
```

- **Structured-owned:** prompts, approvals, settings, cancellation, tools, and events flow through ACP.
- **Terminal-owned:** the native interactive CLI is the only writer; Assembly projects its durable transcript and terminal facts into the structured view.
- **Transitioning:** neither side may accept a new prompt until handoff completes or rolls back.
- The same native provider session must never have a live ACP writer and live CLI writer at the same time.

The Conversation/Terminal toggle performs a backend ownership handoff. It is not merely a CSS visibility switch.

### 2.3 Configuration is UI, not slash commands

Model, reasoning effort, permission/sandbox mode, collaboration/plan mode, fast/service tier, and provider-defined booleans or selects are rendered as proper controls in the composer/header.

- Values and choices come from ACP session configuration options and provider capability updates.
- The UI never hard-codes a model list as authoritative.
- Requested values show pending state until the provider confirms them.
- Unknown categories appear under **More options** rather than being discarded.
- Unsupported mutation is disabled with a reason.
- `/model`, `/permissions`, and equivalent TUI-only picker commands do not appear as normal structured slash commands.

### 2.4 Slash commands are intentionally smaller than the TUI list

The structured command menu contains only:

1. commands advertised by the active ACP runtime that have meaningful structured behavior;
2. discovered skills or provider commands supported by that runtime;
3. Assembly-local commands such as opening Terminal, Conversation, Diff, Browser feedback, or a workflow.

Do not copy the complete Codex TUI command enum into Assembly. Commands that configure the TUI itself—theme, pets, keymap, raw terminal rendering, title line, terminal status line, TUI exit, and similar—are omitted. App-local equivalents belong in normal Assembly controls or Settings.

### 2.5 Screenshot paste is a hard gate

The following must work before conversation work is considered usable:

1. Paste one or more screenshots with `Cmd+V` into the composer.
2. See local previews immediately.
3. Remove or reorder attachments before send.
4. Send images as real ACP image content blocks in structured mode.
5. Preserve draft and previews on any failure.
6. Restore managed pending attachments after an app restart.
7. Use the same attachment pipeline for browser grabs and browser markup.

Terminal-owned fallback may reference managed paths in the native CLI prompt because the CLI is the writer. Structured mode must not reduce an image to a file-path sentence when the provider advertises image input.

### 2.6 Workflows are Assembly-owned

ACP normalizes communication with an agent. ACP does not by itself define the product’s workflow graph, retry policy, role catalog, gates, budget, or orchestration UI.

Assembly owns a deterministic workflow engine above providers. An orchestrator agent may propose or delegate work, but the engine owns:

- step state;
- dependencies;
- concurrency;
- retries;
- cancellation;
- approval gates;
- provider/model/effort selection;
- worktree assignment;
- artifact linkage;
- completion criteria;
- audit history.

### 2.7 Provider-native and Assembly-owned subagents are different

- **Provider-native subagents** are spawned internally by Codex or Claude. Assembly displays their live or persisted output and relationship. Control is limited to what the provider exposes.
- **Assembly workflow agents** are logical ACP sessions spawned by Assembly. Assembly can cancel, retry, continue, reassign, or inspect them because it owns their lifecycle.

The UI may present both in one tree, but their badges and available controls must make the ownership difference explicit.

### 2.8 Layout decisions

- Center destinations use Dockview.
- Vertical stacks in left and right regions use Dockview Paneview.
- Working, Done, and Settled are Paneview panes, not hand-built flex/collapsible regions.
- Every panel inside a left/right tab is a Paneview pane unless it is a transient menu, popover, dialog, tooltip, or full-workbench overlay.
- The session library is removed from the active-work rail and becomes its own destination/pane.
- The global bottom-right action island remains one app-owned overlay surface.

---

## 3. Repository baseline to preserve

The implementation extends these existing authorities rather than adding parallel copies:

### Stable app/session identity

- `OwnedSession.ownedId` remains the primary application identity.
- `nativeSessionId` remains provider metadata.
- `ptySessionId` remains terminal metadata.
- Per-session editor, browser, diff, conversation, and Dockview state remain keyed by `ownedId`.

### Existing terminal authority

Preserve:

- `src/lib/shell/terminalService.ts`
- `src/lib/liveConversationTerminals.ts`
- `src-tauri/src/terminal.rs`
- the one terminal-output listener;
- background output routing;
- backend scrollback;
- tombstone behavior;
- explicit close as the only PTY-kill path;
- login-shell environment loading;
- `COMMANDBAR_SESSION_ID` correlation.

### Existing conversation work

Reuse and refactor:

- `src/lib/shell/conversation/conversationTypes.ts`
- `conversationReducer.ts`
- `conversationStore.svelte.ts`
- `conversationService.ts`
- `components/ConversationSurface.svelte`
- `components/conversation/ConversationMessage.svelte`
- `src-tauri/src/agent_conversation/`
- attachment validation and managed storage;
- transcript discovery and child association;
- generation and sequence guards;
- per-session workspace capture/restore.

The current `provider.rs`/`process.rs` code is a useful protocol spike but is not the production runtime. Replace it with the provider-host architecture below; do not wire the dead path beside a running PTY.

### Existing orchestration authority

Extend the append-only orchestration/event abstractions rather than building an unrelated workflow database and a second event bus.

### Existing pane and center authorities

Extend:

- `ShellFrame.svelte`
- `centerDock.ts`
- the existing right-side Paneview/pane stack implementation;
- session workspace snapshots;
- command registry and panel activation.

---

## 4. Researched implementation references

These projects are implementation references, not dependencies copied wholesale.

### Official ACP stack

- `agentclientprotocol/rust-sdk`: Rust client/agent/proxy/conductor primitives.
- `agentclientprotocol/codex-acp`: Codex app-server adapter with models, reasoning effort, fast mode, approvals, sandbox, images, commands, file changes, plans, reviews, terminal output, reasoning, web search, token usage, and subagent metadata.
- `agentclientprotocol/claude-agent-acp`: Claude Agent SDK adapter with images, tool permissions, edit review, TODOs, nested subagent transcripts, terminals, slash commands, and MCP.

### Provider-neutral runtime design

- T3 Code’s provider runtime contracts separate sessions, turns, items, deltas, requests, tools, plans, tasks, and raw provider data.
- Jockey stores runtime-discovered models, modes, configuration options, and available commands separately, which is the correct control/catalog split.

### Workflow delegation

CodeG’s ACP delegation broker demonstrates a concrete cross-provider pattern:

- parent agent calls an MCP delegation tool;
- a broker validates depth and working directory;
- a fresh child ACP connection is created with provider-specific config defaults;
- parent tool-use ID and child conversation ID are linked;
- status/cancel are explicit;
- completion returns a structured tool result;
- the frontend can open the child session for full output.

Assembly adopts this pattern but supports both one-shot and persistent workflow agents and stores workflow state in its own append-only run model.

### Orca product surfaces

Orca’s source confirms the screenshots are backed by concrete components rather than static mockups:

- persistent browser/webview lifecycle;
- page-element grab and annotation;
- screenshot markup;
- compact resource manager and full workspace-space manager;
- compact provider-usage roster and full usage analytics;
- narrow worktree/session rows with hover identity and expandable detail.

Assembly reuses the interaction patterns, not Orca’s Electron-specific runtime.

---

## 5. Structured runtime architecture

### 5.1 Module layout

Refactor `src-tauri/src/agent_conversation` into:

```text
agent_conversation/
├── mod.rs
├── manager.rs
├── ownership.rs
├── capabilities.rs
├── config.rs
├── commands.rs
├── events.rs
├── reducer.rs
├── persistence.rs
├── attachments.rs
├── handoff.rs
│
├── acp/
│   ├── mod.rs
│   ├── client.rs
│   ├── connection.rs
│   ├── host.rs
│   ├── process.rs
│   ├── normalize.rs
│   └── permissions.rs
│
└── transcript/
    ├── mod.rs
    ├── watcher.rs
    ├── claude.rs
    ├── codex.rs
    └── children.rs
```

This remains one Rust-managed conversation subsystem.

### 5.2 Provider host registry

```rust
pub struct AgentProviderHostRegistry {
    hosts: Arc<Mutex<HashMap<ProviderInstanceId, ProviderHost>>>,
}

pub struct ProviderHost {
    pub provider_instance_id: ProviderInstanceId,
    pub provider: AgentProvider,
    pub process: AcpSidecarProcess,
    pub connection: AcpClientConnection,
    pub capabilities: AgentProviderCapabilities,
    pub state: ProviderHostState,
    pub generation: u64,
    pub sessions: HashSet<String>,
}
```

A host is keyed by configured provider instance, not by the visible tab. The preferred steady state is one healthy adapter process per configured local provider instance with multiple logical ACP sessions, because the adapters support session maps. The implementation spike must prove multi-session isolation for each pinned adapter. If a specific adapter version cannot safely multiplex, the same registry may use one host per active session behind the same interface; the frontend contract does not change.

Host responsibilities:

- spawn the pinned sidecar with a minimal environment;
- initialize ACP once;
- capture provider capabilities;
- route requests and notifications by ACP session ID;
- detect exit and reject in-flight operations;
- restart with bounded backoff;
- never auto-restart into a second writer for a terminal-owned session;
- retain a bounded redacted stderr ring;
- report version mismatch clearly;
- shut down child process groups on app exit.

### 5.3 Adapter packaging

#### Codex

- Pin an exact `codex-acp` version in an adapter lock manifest.
- Build or vendor architecture-specific standalone binaries for macOS arm64 and x64.
- Prefer the adapter’s bundled compatible Codex dependency for the first proof.
- Add a Settings override for a user-selected Codex binary only after version/capability probe.
- Sign and notarize the sidecar with the app.

#### Claude

- Pin an exact `claude-agent-acp` version and exact Claude Agent SDK dependency.
- Bundle an architecture-specific Node 22 runtime as a Tauri sidecar and ship the compiled adapter JS beside it.
- Launch only the bundled runtime by default, avoiding GUI-shell PATH assumptions.
- A developer-only override may use a local adapter after version and checksum display.
- Measure added app size and startup time; do not make the user install Node manually for the packaged app.

#### Version manifest

Create:

```text
src-tauri/agent-adapters.lock.json
```

with:

```ts
interface AgentAdapterLock {
  schemaVersion: 1;
  adapters: Array<{
    provider: 'codex' | 'claude';
    adapterVersion: string;
    protocolVersion: string;
    entrypoint: string;
    sha256: string;
    supportedArchitectures: string[];
  }>;
}
```

Startup verifies the selected binary/script before executing it.

### 5.4 Managed session

```rust
pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider_instance_id: ProviderInstanceId,
    pub provider: AgentProvider,
    pub native_session_id: Option<String>,
    pub acp_session_id: Option<String>,
    pub pty_session_id: Option<String>,
    pub owner: AgentExecutionOwner,
    pub generation: u64,
    pub active_turn_id: Option<String>,
    pub state: ManagedAgentState,
    pub capabilities: AgentSessionCapabilities,
    pub config: Vec<AgentConfigOption>,
    pub last_sequence: u64,
}
```

`ownedId` never changes during resume, adapter restart, CLI handoff, or provider-native ID discovery.

### 5.5 Tauri command boundary

Replace ambiguous old commands with narrow requests:

```text
ensure_agent_runtime_host
start_structured_agent_session
resume_structured_agent_session
send_structured_agent_prompt
steer_structured_agent_turn
interrupt_structured_agent_turn
set_structured_agent_config_option
respond_structured_agent_permission
respond_structured_agent_user_input
close_structured_agent_session
begin_agent_terminal_handoff
complete_agent_terminal_handoff
begin_agent_structured_handoff
complete_agent_structured_handoff
read_agent_conversation_snapshot
read_agent_session_capabilities
```

Every mutating request contains:

- `ownedId`;
- expected owner;
- expected generation;
- provider instance ID;
- provider request/turn/item ID where applicable.

A stale owner or generation fails before provider IO.

---

## 6. Canonical conversation protocol

### 6.1 Item model

Replace the text/tool/error-only timeline with ordered typed items:

```ts
export type AgentTimelineItem =
  | AgentUserMessageItem
  | AgentAssistantMessageItem
  | AgentReasoningItem
  | AgentPlanItem
  | AgentTaskItem
  | AgentCommandItem
  | AgentFileChangeItem
  | AgentMcpToolItem
  | AgentDynamicToolItem
  | AgentWebSearchItem
  | AgentImageItem
  | AgentSubagentItem
  | AgentApprovalItem
  | AgentUserInputItem
  | AgentContextCompactionItem
  | AgentReviewItem
  | AgentStatusItem
  | AgentErrorItem;
```

Common fields:

```ts
export interface AgentTimelineItemBase {
  id: string;
  ownedId: string;
  provider: AgentProvider;
  turnId: string | null;
  parentItemId: string | null;
  sequence: number;
  status: 'pending' | 'running' | 'waiting' | 'completed' | 'failed' | 'cancelled' | 'declined';
  startedAt: number | null;
  completedAt: number | null;
  providerRefs: {
    sessionId?: string;
    turnId?: string;
    itemId?: string;
    requestId?: string;
  };
  providerData?: unknown;
}
```

Provider data is retained for diagnostics and new features but is never rendered unsafely or used as the primary component discriminator.

### 6.2 Event model

```ts
export type AgentRuntimeEvent =
  | SessionStartedEvent
  | SessionConfiguredEvent
  | SessionStateChangedEvent
  | SessionExitedEvent
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
  | PlanUpdatedEvent
  | TaskUpdatedEvent
  | UsageUpdatedEvent
  | ChildrenUpdatedEvent
  | RuntimeWarningEvent
  | RuntimeErrorEvent;
```

Content channels:

```ts
export type AgentContentChannel =
  | 'assistant'
  | 'reasoning'
  | 'reasoning-summary'
  | 'plan'
  | 'command-output'
  | 'file-change-output';
```

The reducer keeps provider order. Text before a tool, the tool, and text after the tool must remain in that order.

### 6.3 Persistence

Use an app-owned append-only projection log for crash recovery without replacing provider history:

```text
~/Library/Application Support/MacCommandBar/conversations/<ownedId>/events.jsonl
~/Library/Application Support/MacCommandBar/conversations/<ownedId>/snapshot.json
```

The compatibility path retains the existing Application Support identity during the Assembly rebrand.

Rules:

- append normalized events after validation;
- write periodic atomic snapshots;
- compact only events already represented by a snapshot;
- cap raw provider payloads and redact secrets;
- on restart, restore the local projection immediately, then reconcile from ACP load-session or provider transcript;
- provider history remains the context authority;
- local canonical history remains the UI recovery authority.

### 6.4 Transcript projection

The existing transcript reader becomes `TerminalTranscriptProjection`.

It is used for:

- terminal-owned sessions;
- sessions started outside Assembly;
- old archived sessions;
- history reconciliation after returning from CLI;
- provider-native children that are no longer live;
- fallback when an adapter cannot load an old session.

Replace the 500ms four-megabyte reread with a Rust watcher/cursor:

```rust
pub struct TranscriptCursor {
    pub path: PathBuf,
    pub file_identity: FileIdentity,
    pub offset: u64,
    pub partial_line: Vec<u8>,
    pub generation: u64,
}
```

The watcher reads appended lines only, detects truncation/rotation, emits normalized items, and performs an occasional bounded reconciliation snapshot. It never scrapes terminal pixels.

---

## 7. Runtime capabilities, pickers, and commands

### 7.1 Capability model

```ts
export interface AgentSessionCapabilities {
  loadSession: boolean;
  images: boolean;
  embeddedContext: boolean;
  additionalDirectories: boolean;
  steering: boolean;
  interrupt: boolean;
  permissions: boolean;
  structuredUserInput: boolean;
  plans: boolean;
  tasks: boolean;
  subagents: boolean;
  nestedSubagentTranscripts: boolean;
  toolTerminals: boolean;
  terminalOutput: boolean;
  configOptions: boolean;
  availableCommands: boolean;
}
```

The UI renders from capability facts. It does not infer support from provider name.

### 7.2 Dynamic configuration options

```ts
export interface AgentConfigOption {
  id: string;
  category:
    | 'model'
    | 'reasoning-effort'
    | 'permission-mode'
    | 'interaction-mode'
    | 'collaboration-mode'
    | 'service-tier'
    | 'boolean'
    | 'select'
    | 'unknown';
  label: string;
  description: string | null;
  currentValue: string | boolean | null;
  choices: AgentConfigChoice[];
  mutable: boolean;
  source: 'acp' | 'provider-extension' | 'terminal-observed';
}
```

Picker placement:

- composer primary row: Model, Effort, Permission/Mode;
- optional primary control: Fast/service tier when available;
- plan/collaboration mode beside the primary controls when exposed;
- remaining options under More.

Changing model must recalculate supported effort choices and preserve the current effort only when the provider confirms it is valid.

### 7.3 Command catalog

```ts
export interface AgentCommandDescriptor {
  name: string;
  description: string;
  inputHint: string | null;
  source: 'provider' | 'skill' | 'assembly';
  action:
    | { kind: 'provider-command'; command: string }
    | { kind: 'insert-skill'; text: string }
    | { kind: 'assembly-command'; id: string };
  autoSubmit: false;
}
```

The command menu merges:

- ACP `available_commands_update`;
- discovered skills advertised by the runtime;
- Assembly commands.

Assembly commands initially include:

```text
/terminal
/conversation
/new-workflow
/open-workflow
/open-diff
/open-browser-feedback
/attach-file
```

Provider commands are inserted or invoked through the structured command path only when advertised. Selection never auto-submits.

Do not include provider TUI-local commands that only manipulate terminal UI.

---

## 8. Conversation experience

### 8.1 Component structure

```text
ConversationSurface.svelte
├── ConversationHeader.svelte
├── AgentConfigBar.svelte
├── AgentTree.svelte
├── ConversationTimeline.svelte
│   └── ConversationTimelineItem.svelte
│       ├── UserMessageItem.svelte
│       ├── AssistantMessageItem.svelte
│       ├── ReasoningItem.svelte
│       ├── PlanItem.svelte
│       ├── TaskItem.svelte
│       ├── CommandItem.svelte
│       ├── FileChangeItem.svelte
│       ├── ToolItem.svelte
│       ├── ApprovalItem.svelte
│       ├── UserInputItem.svelte
│       ├── SubagentItem.svelte
│       └── ErrorItem.svelte
├── ConversationComposer.svelte
├── AttachmentStrip.svelte
├── AgentCommandMenu.svelte
└── ConversationTerminalHandoff.svelte
```

`ConversationSurface` remains the one Session-tab composition root.

### 8.2 Markdown

Use the Work Package 10 sanitizer and renderer policy for conversation Markdown, with streaming-safe behavior and custom renderers for:

- code blocks;
- file links;
- tables;
- task lists;
- local image references;
- copy actions.

Do not render arbitrary HTML.

### 8.3 Tool and command rows

Default presentation is quiet and compact:

```text
✓ Read src/lib/foo.ts
● Running cargo test
✓ Modified 3 files  +42 −17
```

Expand reveals:

- exact bounded command;
- working directory;
- duration;
- exit status;
- stdout/stderr;
- file locations;
- provider details when useful.

Large output is virtualized or truncated with an explicit **Open full output** action. It is never placed into one giant DOM node.

### 8.4 Plans and tasks

Plans are first-class structured items.

```ts
export interface AgentPlan {
  id: string;
  explanation: string | null;
  steps: Array<{
    id: string;
    title: string;
    status: 'pending' | 'in-progress' | 'completed' | 'failed';
    agentNodeId: string | null;
  }>;
}
```

The UI shows plan progress in the conversation and mirrors it into the Agent Control Center when the plan belongs to a workflow.

### 8.5 Approvals and questions

Permission requests render inline with the affected item. Buttons come from the runtime’s supplied options rather than a fixed Approve/Deny assumption.

Structured user input supports:

- free text;
- single choice;
- multiple choice;
- confirmation;
- form fields when provider metadata supports them.

The request is tied to exact owner, generation, turn, and request ID and expires on ownership change.

### 8.6 Scroll and performance

- preserve parent and child scroll independently;
- auto-follow only when within 80px of the bottom;
- show **Jump to latest** otherwise;
- virtualize long timelines by stable item row;
- batch high-frequency text deltas to one paint per animation frame;
- never replace the completed timeline when a refresh fails.

---

## 9. Screenshot and attachment pipeline — priority packet

### 9.1 Managed attachment model

```ts
export interface ConversationAttachment {
  id: string;
  ownedId: string;
  kind: 'image' | 'file' | 'browser-element' | 'browser-markup';
  name: string;
  mimeType: string;
  managedPath: string;
  byteLength: number;
  width: number | null;
  height: number | null;
  sha256: string;
  previewUrl: string | null;
  sourceMetadata: BrowserAttachmentMetadata | null;
}
```

### 9.2 Paste flow

1. Capture image clipboard files before text paste handling.
2. Validate aggregate count and bytes client-side for immediate feedback.
3. Send bytes to the existing Rust attachment boundary.
4. Decode enough image metadata in Rust to verify signature and dimensions.
5. Write with owner-only permissions under exact `ownedId`.
6. Return the managed record and create a blob preview.
7. Preserve the draft and all successful attachments if one file fails; show per-file error.

### 9.3 Structured send

When the active structured runtime advertises image input:

- read the validated managed file in Rust;
- construct ACP image content blocks with MIME and bounded encoded data or the SDK-supported image representation;
- include text and images in one prompt request;
- clear attachments only after the provider accepts the prompt;
- retain the attachment record in the canonical user-message item.

### 9.4 Terminal-owned send

For a terminal-owned native CLI:

- append canonical managed paths once using provider-supported prompt syntax;
- send bracketed paste and Return through the existing PTY service;
- do not pretend the PTY accepted image modality until the provider transcript records the turn;
- preserve attachments on write failure.

### 9.5 Browser attachments

Browser element grabs and markup use the same attachment vault. Source metadata contains only bounded page/element facts and never cookies, storage, tokens, or full HTML.

### 9.6 Required tests

- PNG/JPEG/GIF/WebP signature validation;
- malformed and decompression-bomb bounds;
- per-file, aggregate-byte, and count caps;
- owner/path/symlink containment;
- multi-image ordering;
- partial failure;
- restart preview restoration;
- structured image content blocks;
- terminal fallback path insertion exactly once;
- stale owner/generation rejection;
- cleanup on remove/session deletion;
- browser metadata redaction.

This packet precedes visual conversation polish.

---

## 10. Terminal/structured ownership handoff

### 10.1 State machine

```text
StructuredOwned
  -> PreparingTerminal
  -> TerminalOwned
  -> PreparingStructured
  -> StructuredOwned
```

Every transition has rollback.

### 10.2 Structured to terminal

1. Refuse while an unanswerable approval/user-input request is open, or require the user to resolve/cancel it.
2. Finish or interrupt the active turn.
3. Persist and reconcile the latest canonical state.
4. Close/detach the ACP logical session without deleting native history.
5. Launch the provider’s exact resume command in a new or existing owned PTY.
6. Verify the CLI resumed the expected native session ID from transcript evidence.
7. Set owner to terminal and reveal xterm.
8. On failure, close the attempted PTY and restore structured ownership.

### 10.3 Terminal to structured

1. Require the native CLI to be idle or ask to interrupt it.
2. Stop the provider TUI process while preserving terminal scrollback.
3. Reconcile transcript additions.
4. Resume/load the same provider-native session through ACP.
5. Verify native session ID, cwd/worktree, and provider.
6. Set owner to structured and return to the conversation timeline.
7. On failure, offer **Restart native CLI** and keep terminal ownership.

### 10.4 Fork behavior

Provide **Fork to Terminal** and **Fork to Conversation** when the provider supports native fork. A fork receives a new `ownedId` and native session ID and therefore can run concurrently with its parent.

### 10.5 Acceptance

For both Claude and Codex:

- exchange two turns structured;
- hand off to native CLI;
- add a turn;
- return structured;
- observe exact history once;
- keep one provider writer at every instant;
- preserve editor/browser/diff/layout state;
- fail a handoff deliberately and prove rollback.

---

## 11. Agent and subagent model

### 11.1 Unified node

```ts
export interface AgentNode {
  id: string;
  ownedId: string;
  workflowRunId: string | null;
  provider: AgentProvider;
  providerInstanceId: string;
  nativeSessionId: string | null;
  parentNodeId: string | null;
  parentItemId: string | null;
  roleId: string | null;
  label: string;
  ownership: 'provider-native' | 'assembly-workflow';
  lifecycle:
    | 'queued'
    | 'starting'
    | 'running'
    | 'waiting'
    | 'blocked'
    | 'completed'
    | 'failed'
    | 'cancelled';
  currentStepId: string | null;
  currentItemId: string | null;
  startedAt: number | null;
  updatedAt: number | null;
  completedAt: number | null;
  depth: number;
  capabilities: AgentNodeControlCapabilities;
}
```

### 11.2 Provider-native children

Normalize:

- Codex collaboration/subagent tool calls and thread metadata;
- Claude Agent/Task tool calls and nested transcript metadata;
- persisted transcript evidence when live events are unavailable.

Show:

- role/label;
- state;
- elapsed time;
- current tool/task when available;
- nested output;
- files/commands touched when provider supplies them.

Do not invent stop/retry controls for a provider-native child unless the provider exposes them.

### 11.3 Assembly workflow children

Assembly-owned child sessions support:

- open transcript;
- send follow-up;
- pause when provider supports it;
- cancel active turn;
- cancel session;
- retry step;
- continue with same session;
- replace provider/model before retry;
- reassign worktree;
- mark result accepted/rejected.

All actions are generation-guarded and logged.

---

## 12. Workflow orchestration engine

### 12.1 Principle

The workflow engine is deterministic infrastructure. The orchestrator agent is a participant, not the scheduler of record.

The engine decides when a step is eligible from persisted facts. An orchestrator agent may:

- decompose a goal;
- propose steps;
- choose among allowed roles/providers;
- request a retry;
- summarize results;
- identify follow-up work.

The engine validates every proposal against workflow definition, depth, concurrency, budget, worktree, and approval rules.

### 12.2 Workflow definition

```ts
export interface WorkflowDefinitionV1 {
  version: 1;
  id: string;
  name: string;
  description: string;
  orchestrator: WorkflowOrchestratorDefinition | null;
  roles: WorkflowRoleDefinition[];
  steps: WorkflowStepDefinition[];
  edges: WorkflowEdge[];
  defaults: WorkflowDefaults;
  limits: WorkflowLimits;
  completionPolicy: WorkflowCompletionPolicy;
}
```

```ts
export interface WorkflowRoleDefinition {
  id: string;
  name: string;
  description: string;
  providerInstanceId: string | null;
  modelOption: string | null;
  effortOption: string | null;
  permissionOption: string | null;
  interactionMode: string | null;
  systemInstructions: string;
  allowedTools: string[] | null;
  worktreePolicy: 'shared-read' | 'dedicated' | 'inherit' | 'none';
  persistentSession: boolean;
}
```

```ts
export interface WorkflowStepDefinition {
  id: string;
  title: string;
  description: string;
  roleId: string;
  kind:
    | 'plan'
    | 'implement'
    | 'review'
    | 'spec-compliance'
    | 'test'
    | 'fix'
    | 'verify'
    | 'summarize'
    | 'custom';
  promptTemplate: string;
  dependsOn: string[];
  inputs: WorkflowInputBinding[];
  outputs: WorkflowOutputDefinition[];
  gate: WorkflowGateDefinition | null;
  retry: WorkflowRetryPolicy;
  parallelGroup: string | null;
  continueSessionFromStepId: string | null;
}
```

### 12.3 Run model

```ts
export interface WorkflowRun {
  id: string;
  definitionId: string;
  ownedId: string;
  title: string;
  goal: string;
  status:
    | 'draft'
    | 'running'
    | 'waiting'
    | 'blocked'
    | 'completed'
    | 'failed'
    | 'cancelled';
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
  steps: WorkflowStepRun[];
  agents: AgentNode[];
  artifacts: WorkflowArtifact[];
  decisions: WorkflowDecision[];
  budget: WorkflowBudgetState;
}
```

Step lifecycle:

```text
pending -> eligible -> queued -> starting -> running
        -> waiting-input | waiting-approval | review
        -> completed | failed | cancelled | skipped
```

### 12.4 Event log

Extend the existing orchestration JSONL schema with additive event kinds:

```text
workflow.created
workflow.started
workflow.state.changed
workflow.completed
workflow.cancelled
workflow.step.eligible
workflow.step.started
workflow.step.progress
workflow.step.output
workflow.step.completed
workflow.step.failed
workflow.step.retry.requested
workflow.step.retry.started
workflow.agent.spawned
workflow.agent.state.changed
workflow.agent.message.linked
workflow.artifact.published
workflow.gate.opened
workflow.gate.resolved
workflow.decision.recorded
workflow.budget.updated
```

Each event contains workflow run ID, step ID, agent node ID, `ownedId`, provider references, timestamp, and bounded payload. Replay must reconstruct the run without side effects.

### 12.5 Workflow broker and MCP delegation tools

Expose an Assembly-owned MCP server to orchestrator sessions with a minimal allow-listed tool surface:

```text
assembly_workflow_create_steps
assembly_workflow_spawn_agent
assembly_workflow_send_followup
assembly_workflow_get_status
assembly_workflow_publish_artifact
assembly_workflow_complete_step
assembly_workflow_fail_step
assembly_workflow_request_retry
assembly_workflow_cancel_agent
```

The parent agent never receives a raw process-spawn tool.

`assembly_workflow_spawn_agent` accepts:

```ts
export interface SpawnWorkflowAgentRequest {
  workflowRunId: string;
  stepId: string;
  roleId: string;
  task: string;
  inputArtifactIds: string[];
  requestedWorktreeId: string | null;
}
```

The broker:

1. verifies the requesting parent session owns the workflow;
2. validates role, step, depth, and concurrency;
3. resolves provider instance and supported config values;
4. verifies or creates the allowed worktree through the existing worktree authority;
5. starts a fresh ACP logical session or resumes the configured persistent role session;
6. links parent tool-use ID, workflow step, child `ownedId`, and ACP/native session IDs;
7. emits `workflow.agent.spawned`;
8. sends the bounded task and artifact references;
9. returns a running acknowledgement immediately or waits only when the definition explicitly selects one-shot behavior.

Status, cancellation, and results are scoped to the parent/workflow and cannot target an unrelated agent.

### 12.6 Default workflow templates

Ship templates as editable data, not hard-coded orchestration logic.

#### Implementation with independent review

```text
Planner
  -> Implementer
  -> [Code Review || Spec Compliance || Tests]
  -> Fixer, when any reviewer fails
  -> Verification
  -> Summary
```

#### PR readiness

```text
Inspect branch and PR facts
  -> Code Review
  -> Check failures
  -> Fix issues
  -> Re-run checks
  -> Draft PR title/body
  -> User approval gate
```

#### UI feedback loop

```text
Browser annotations
  -> UI implementer
  -> Browser verification
  -> Visual/spec reviewer
  -> Fix loop
  -> User review gate
```

#### Security/compliance review

```text
Threat-model reviewer || dependency reviewer || authorization reviewer
  -> findings merger
  -> prioritized remediation plan
  -> optional fix branches
```

### 12.7 Concurrency and budgets

```ts
export interface WorkflowLimits {
  maxDepth: number;
  maxActiveAgents: number;
  maxAgentsPerProvider: number;
  maxStepRetries: number;
  maxRuntimeMinutes: number;
  maxTurnsPerAgent: number;
  maxEstimatedTokens: number | null;
  stopOnBudgetExceeded: boolean;
}
```

The scheduler starts all eligible steps in a parallel group only while limits and worktree rules allow it.

Default product limits are conservative and visible. The user can override them per workflow before start.

### 12.8 Gates

Gate kinds:

```ts
export type WorkflowGateKind =
  | 'user-approval'
  | 'all-reviewers-pass'
  | 'tests-pass'
  | 'no-critical-findings'
  | 'artifact-present'
  | 'custom-deterministic';
```

A model cannot mark a deterministic gate satisfied by prose. Tests/checks/Git/PR facts come from existing services.

### 12.9 Agent Control Center

Create a full center Dockview destination:

```text
Agent Control Center
├── Workflow list
├── Active run graph
├── Step lanes
├── Agent tree
├── Live output inspector
├── Plan/task checklist
├── Artifact drawer
├── Decision/approval queue
├── Budget and elapsed metrics
└── Event timeline
```

Compact status appears in the global action island and session hover cards.

Controls:

- pause dispatching;
- resume dispatching;
- cancel run;
- cancel one Assembly-owned agent;
- retry failed step;
- skip optional step;
- approve/reject gate;
- open worktree/session/transcript/artifact;
- change provider/model/effort for a not-yet-started step or retry;
- send a follow-up to a persistent role session.

The UI always distinguishes **provider-native subagent** from **Assembly workflow agent**.

---

## 13. Orca-inspired browser workbench

This section replaces Work Package 8 where it conflicts. Keep its child-webview security, profile isolation, URL validation, and z-order spike requirements.

### 13.1 Presentation modes

```ts
export type BrowserPresentationMode =
  | 'docked'
  | 'floating'
  | 'expanded'
  | 'collapsed';
```

- **Docked:** normal Browser center destination.
- **Floating:** a resizable overlay above the workbench, preserving enough surrounding IDE context to compare code and page. It opens from the bottom-right island and remembers its last bounded size and position.
- **Expanded:** fills the workbench below macOS window chrome and overlays left/right/center regions.
- **Collapsed:** native child view hidden; state remains; bottom-right island remains.

Floating and expanded reuse the same native browser tab. No navigation reload and no second webview.

### 13.2 Browser toolbar

Match the proven Orca interaction set with Assembly semantics:

- profile selector;
- back/forward/reload;
- address bar;
- Import affordance, disabled with policy until cookie/import security is approved;
- **Grab page element**;
- **Annotate page element**;
- **Draw on screenshot**;
- development-only DevTools;
- open in default browser;
- overflow menu;
- floating/expanded toggle;
- collapse/minimize.

Overflow includes:

```text
Default profile
New Profile…
Import Cookies   (disabled until security contract)
Viewport Size
Browser Settings…
Clear Workspace Browser Data…
```

### 13.3 Grab page element

Grab is a fast context action:

1. Arm the inspector in the active page.
2. Highlight hovered element.
3. Click captures bounded selector, accessibility facts, text snippet, element/page rectangles, page URL/title, viewport, and a clipped screenshot.
4. Show a confirmation sheet.
5. User chooses Copy, Attach to conversation, or Convert to annotation.
6. Nothing auto-sends.

### 13.4 Annotate page element

Annotation captures the same element facts and opens a card anchored near the element:

- element identity;
- selector;
- note;
- Change/Question intent;
- Add/Cancel;
- `Cmd+Enter` to add.

Queued annotations are immutable records scoped to browser workspace and tab.

### 13.5 Draw on screenshot

1. Capture the current visible browser viewport through the native browser bridge.
2. Open an Assembly-owned markup overlay.
3. Support pen, arrow, rectangle, text, undo, clear, and crop.
4. Save the final image into the conversation attachment vault as `browser-markup`.
5. Preserve original page URL, viewport, timestamp, and optional note.
6. Copy or attach; never auto-send.

### 13.6 Feedback queue

The Browser Feedback panel shows:

```text
N annotations ready.
Select another element, draw on the page, copy all feedback, or attach it to a conversation.
```

Actions:

- edit note;
- change intent;
- remove;
- copy one/all as Markdown;
- attach one/all to the active conversation;
- ask an agent;
- start the UI feedback workflow template.

Attaching stages both bounded Markdown facts and screenshot attachments into the exact active `ownedId` draft. A nonempty draft requires a merge preview.

### 13.7 Browser implementation boundary

Use managed Tauri child webviews with no Tauri capabilities. The main webview owns all browser chrome and overlays.

Mandatory native spike proves:

- child view clips correctly in docked, floating, and expanded modes;
- main-webview tooltips/cards/dialogs receive input above it;
- bounds track Dockview, resizing, scale factor, and overlay movement;
- child view hides before teleport and reappears only after final bounds;
- Settings and confirmation dialogs hide/lower the child view;
- one tab survives all presentation transitions without reload.

If WKWebView z-order cannot satisfy the interaction, stop before broad implementation and return a native-container decision packet.

### 13.8 Browser-to-agent context

Browser page text is untrusted context. The agent receives:

- exact user note;
- bounded element facts;
- page URL/title;
- screenshot attachment;
- explicit statement that page content is data, not instruction.

The browser never sends cookies, local storage, tokens, full HTML, or arbitrary evaluated page data.

---

## 14. Resource manager and workspace-space manager

This replaces the presentation portion of Work Package 9 while preserving its deterministic process ownership and Roslyn lifecycle rules.

### 14.1 Two related surfaces

#### Compact resource popover

Opened from the status bar:

- total CPU;
- total RSS;
- app process group;
- repository groups;
- worktree groups;
- session/terminal/browser/LSP rows;
- tiny history sparklines;
- refresh;
- owned-only stop action with confirmation;
- **Review inactive workspaces**;
- compact Space section with Scan/Review.

#### Full Resource Manager center destination

A Dockview center panel with:

- process ownership tree;
- CPU/RSS history;
- ports;
- terminal/session owner;
- browser tabs;
- LSP roots and PIDs;
- Playwright ownership;
- protected external processes;
- exact stop/restart controls only for registry-owned processes;
- log/output links;
- memory-pressure policy.

### 14.2 Workspace Space full view

A separate tab inside Resource Manager or a dedicated center destination contains:

- scanned bytes;
- reclaimable bytes;
- workspace count;
- last update;
- treemap by worktree;
- selected worktree item breakdown;
- searchable/sortable table;
- active agents, terminals, dirty buffers, browser tabs, changed files, PR/check facts;
- safe multi-select;
- delete/archive preview using the existing worktree-safety authority.

Never delete the primary checkout, active worktree, dirty buffer, or protected worktree through a space calculation alone.

### 14.3 Data collection

Keep the earlier one-shot/bounded Rust snapshot design:

- one `ps` snapshot;
- one `lsof` snapshot;
- registry and parent-process joins;
- one bounded disk scan per explicit refresh;
- no per-row process commands;
- no process-name-only ownership;
- no frontend-held live PID authority.

### 14.4 Pane behavior

The compact resource UI is a popover. The full Resource Manager is a center Dockview panel. Internal vertical sections use Paneview so users can resize process tree, details, and space analysis.

---

## 15. Provider usage and analytics

Provider usage is not the same surface as CPU/RSS resources.

### 15.1 Compact usage popover

Opened from the status bar and inspired by the supplied Orca evidence:

- Detailed/Compact density selector;
- one row per configured provider;
- provider icon/name/account/plan;
- reset countdown;
- each authoritative quota window and percentage;
- refresh;
- sign-in/manage-account action when supported;
- **Usage details & history**.

Missing data says unavailable and names the source/reason. Do not infer plan limits from token transcripts.

### 15.2 Full Usage Analytics center panel

Tabs:

```text
Overview
Claude
Codex
OpenCode
Future providers
```

Overview may include:

- total agents spawned;
- time agents worked;
- PRs created;
- tracking-since date;
- total observed tokens by type;
- estimated cost with explicit pricing snapshot/source/date;
- active days;
- cache share;
- daily intensity;
- provider mix;
- session/turn/event counts;
- per-provider cards.

Provider detail shows only facts available from its authenticated API, local transcript scanner, or adapter metadata and labels the source.

### 15.3 Persistence

Use append-only local usage samples with schema version and source, then derive analytics. Never persist auth tokens or raw prompts.

### 15.4 Refresh policy

- explicit refresh;
- provider event updates;
- no aggressive polling;
- countdown clocks are local presentation timers;
- rate-limit fetches are provider-specific and bounded.

---

## 16. Left rail, session library, and Paneview shell

### 16.1 Left rail purpose

The left rail is for active work, not every session on the machine.

It contains:

- product/navigation strip;
- project/workspace groups;
- active worktree/session rows;
- Working, Done, Settled Paneview panes;
- New session/workflow actions.

Target expanded width is compact and resizable, approximately 230–280 CSS pixels. Rows use one-line primary identity and no permanently expanded card chrome.

### 16.2 Compact session row

Default row shows:

- provider/status icon;
- concise title;
- model or role in muted text when known;
- state dot/word;
- branch/worktree badge only when valuable;
- attention count.

Hover/focus opens a bounded card with:

- full title and first prompt;
- provider/model/effort/mode;
- project/worktree/branch;
- current task/plan step;
- last activity;
- PR/check hint;
- terminal/structured owner;
- quick actions.

Hover content is supplementary. All actions remain keyboard reachable through the row menu.

### 16.3 Accordion detail

Clicking the disclosure opens an inline detail section without widening the rail. It contains deterministic facts and actions. Only one or a small bounded number of rows may remain expanded per Paneview pane.

### 16.4 Separate Session Library

Move the existing **Find a session** scanner/discovery UI out of `SessionsColumn`.

Create a separate `Session Library` destination that can be opened as:

- a right tool Paneview tab for quick discovery; and
- a full center Dockview panel for large history/search.

It contains:

- provider filters;
- project/repository/worktree grouping;
- model/date/status filters;
- search;
- resumability facts;
- transcript preview;
- Resume structured;
- Resume in terminal;
- Fork;
- Archive/delete only through provider-supported confirmed operations.

The left rail never renders hundreds of resumable sessions.

### 16.5 Paneview requirements

Create one typed side-pane registry:

```ts
export type SidePaneRegion = 'left' | 'right';

export interface SidePaneRegistration {
  id: string;
  title: string;
  region: SidePaneRegion;
  order: number;
  minimumSize: number;
  defaultSize: number;
  collapsible: boolean;
  content: Snippet;
}
```

Use Dockview Paneview for:

- Working;
- Done;
- Settled;
- Session Library quick view;
- Explorer sections;
- Source Control sections;
- Worktrees;
- Problems when right-docked;
- Resources quick view;
- Agent Control quick view;
- other future left/right tab internals.

Do not nest a different accordion/resizer implementation inside each tool.

Persist Paneview layouts per `ownedId` where content is session-specific and globally where content is truly global. Migrate current pane-stack/collapse state once.

### 16.6 Center panels

Expand center roster with:

```text
Session
Editor
Browser
Diff
Git Graph
Pull Requests
Markdown
Agent Control Center
Resource Manager
Usage Analytics
Session Library
```

Panels are lazy-activated and parked without destroying their state.

---

## 17. Global bottom-right action island

Preserve the product-wave global FAB requirement and extend it.

Contexts:

```ts
export type WorkbenchActionContextKind =
  | 'session'
  | 'editor'
  | 'browser'
  | 'git'
  | 'pull-request'
  | 'resources'
  | 'usage'
  | 'workflow'
  | 'history';
```

Examples:

- Session: Conversation/Terminal, attach screenshot, inspect agents, start workflow.
- Editor: quick open, format, ask about selection, create task, source control.
- Browser: float/expand/restore, grab, annotate, draw, send feedback, start UI workflow.
- Pull request: draft text, review, address review, fix checks.
- Resources: refresh, inspect, owned-process stop preview, workspace-space review.
- Workflow: open control center, pause dispatch, approval queue.

The action island routes to existing services. It owns no business logic.

---

## 18. AI throughout the workbench

### 18.1 Shared assistance contract

Create an app-wide typed recipe system rather than one-off prompt strings in every component:

```ts
export interface AiAssistRecipe<Input, Proposal> {
  id: string;
  title: string;
  providerPolicy: 'active-session' | 'configured-role' | 'user-choice';
  context(input: Input): AgentFactBundle;
  prompt(input: Input, facts: AgentFactBundle): string;
  parse(result: AgentResult): Proposal;
  validate(proposal: Proposal, facts: AgentFactBundle): ValidationResult;
  preview(proposal: Proposal): AiProposalPreview;
  apply(proposal: Proposal): Promise<AiApplyResult>;
}
```

All recipes:

- receive bounded deterministic facts;
- label untrusted text;
- produce a proposal;
- show a preview;
- revalidate before apply;
- never auto-apply a consequential mutation.

### 18.2 Pull requests

Recipes:

- draft title/body;
- review code;
- summarize checks;
- fix failed checks;
- address review comments;
- detect unresolved review threads;
- prepare merge summary;
- propose release notes.

Drafting fills the existing form as a proposal. Publishing remains the typed GitHub confirmation flow.

### 18.3 Forms and editors

Reusable field actions:

```text
Fill with AI
Improve wording
Explain this field
Check for contradictions
Validate against repository facts
```

The model receives the field schema and current deterministic facts, not the entire application state.

### 18.4 Save guard pipeline

```text
User presses Save
  -> deterministic validation
  -> optional AI advisory validation
  -> proposal/warnings
  -> user chooses Save, Edit, or Apply suggestion
  -> deterministic validation runs again
  -> typed save
```

Rules:

- deterministic errors may block save;
- AI warnings are advisory unless the user has explicitly configured a recipe as required;
- AI cannot silently alter values;
- all generated changes are visible as a diff;
- save revalidates owner/generation/version.

### 18.5 Editor and source control

Recipes:

- explain selection;
- propose refactor;
- generate tests;
- inspect a diff;
- draft commit message;
- review before commit;
- detect likely accidental files;
- summarize branch state.

They stage prompts/proposals in the active structured conversation or a configured workflow role and never bypass Git/file safety boundaries.

### 18.6 Browser

Recipes consume exact annotations/screenshots and can:

- explain an element;
- propose a UI fix;
- create an implementation task;
- start the UI feedback workflow;
- verify whether a later screenshot addressed annotations.

### 18.7 Audit

Record recipe ID, fact hashes, provider/model, proposal hash, user decision, apply result, and links to resulting PR/commit/workflow. Do not record secrets or full prompts when they contain sensitive context.

---

## 19. Security and trust boundaries

1. Remote/browser/PR/review text is untrusted data and is delimited in prompts.
2. ACP adapter processes receive only needed environment variables.
3. Provider/API credentials stay in provider-supported auth stores; they are not copied into Assembly state.
4. Child browser webviews receive no Tauri capabilities.
5. Workflow agents cannot spawn arbitrary processes; they request typed broker operations.
6. Workflow worktree creation/removal uses the existing worktree authority.
7. Provider configuration values are validated against advertised options.
8. All provider requests are scoped by owner and generation.
9. All consequential AI-assisted mutations use preview and deterministic revalidation.
10. Raw provider payloads, terminal output, page content, and Markdown are bounded and sanitized before display.
11. No provider-native subagent is presented as controllable unless control is proven.
12. No cross-provider/session ID is trusted without stored parent linkage.

---

## 20. Migration from current main

### Phase 0 — freeze contracts

Sequential controller work:

1. Add `AgentExecutionOwner` and migrate existing owned sessions:
   - live PTY -> terminal;
   - no PTY and resumable provider session -> stopped until selected;
   - plain shell -> terminal;
   - no old record becomes structured automatically.
2. Freeze canonical event/item/config/capability types in Rust and TypeScript.
3. Freeze side Paneview registry and center-panel roster additions.
4. Freeze workflow definition/run/event schemas.
5. Add adapter packaging/version manifest decisions.
6. Add feature flags:

```text
structuredAgentRuntime
agentTerminalHandoff
workflowOrchestration
nativeBrowserWorkbench
resourceManagerV2
usageAnalytics
paneviewSideRegions
aiAssistRecipes
```

### Phase 1 — structured runtime foundation

- package sidecars;
- implement ACP host registry;
- implement one Codex and one Claude smoke session;
- capture configuration options and capabilities;
- normalize text, tool, permission, plan, task, usage, and child events;
- persist canonical events;
- no broad UI yet.

### Phase 2 — conversation and attachment foundation

- implement image paste first;
- build typed timeline using fixture events;
- implement config controls;
- implement approval/user input;
- implement dynamic command menu;
- implement live/persisted child tree;
- retain terminal projection fallback.

### Phase 3 — ownership handoff

- structured-to-terminal;
- terminal-to-structured;
- rollback;
- fork;
- restart/recovery.

### Phase 4 — workflow engine

- workflow schema/store/reducer;
- scheduler;
- role/provider config resolution;
- Assembly MCP delegation broker;
- one-shot and persistent children;
- Agent Control Center;
- default templates.

### Phase 5 — shell and Orca-inspired surfaces

- side Paneview migration;
- narrow rail/hover cards;
- Session Library;
- browser native spike and presentation modes;
- element grab/annotation/markup;
- compact/full resources;
- compact/full usage.

### Phase 6 — AI recipes and cross-surface integration

- PR recipes;
- form/save recipes;
- browser workflow;
- editor/Git recipes;
- audit receipts.

### Phase 7 — certification and cleanup

- remove old 500ms mirror from structured-owned path;
- remove dead provider spike after parity;
- remove hard-coded conversation commands and metadata controls;
- migrate old state once;
- update docs and task statuses only after native evidence.

---

## 21. Parallel execution plan

The following preserves the earlier at-most-two-heavy-runner contract.

### Wave A — sequential shared contracts

| Packet | Owner | Output |
| --- | --- | --- |
| A0 repository re-anchor | Controller/Luna read-only | current symbols, branches, dirty-state receipt |
| A1 runtime/ownership contract | SOL decision then controller | Rust/TS types, migration, feature flags |
| A2 Paneview/center roster contract | SOL decision then controller | side registry, persistence version, center IDs |
| A3 workflow schema contract | SOL decision then controller | definition/run/event/broker interfaces |
| A4 adapter packaging proof | Luna implementation, SOL review | signed development sidecars, size/startup report |

No broad lane starts before A1–A3 are committed.

### Wave B — parallel foundations

| Lane | Scope | Route | Heavy slot |
| --- | --- | --- | --- |
| B1 ACP host/runtime | Rust ACP host, process supervisor, capabilities | Luna after A1/A4 | Rust slot |
| B2 Conversation fixtures/UI | typed reducer/components over recorded fixtures | Luna after A1 | Node slot |
| B3 Attachments | structured image blocks, restore/cleanup | Luna after A1 | focused Rust then Node |
| B4 Workflow reducer/scheduler | pure schema/reducer/scheduler, no providers | Luna after A3 | Node/Rust focused |
| B5 Side Paneview/navigation | rail, Paneview, Session Library shells | Luna after A2 | Node |
| B6 Browser presentation model | pure dock/float/expand/collapse/annotation state | Luna after A2 | Node |
| B7 Resource/usage view models | compact/full pure models and fixtures | Luna after A2 | Node |
| B8 AI recipe contracts | pure recipe/fact/proposal validation | Luna after A3 | Node |

B1 and a second Rust-heavy lane never run heavy tests simultaneously unless the controller grants the second slot.

### Milestone M1

SOL-medium read-only review of B1–B8:

- no duplicate runtime/store/router;
- owner/generation safety;
- event completeness;
- image attachment correctness;
- Paneview persistence;
- workflow determinism;
- security boundaries.

### Wave C — provider and workflow integration

| Lane | Scope | Dependency |
| --- | --- | --- |
| C1 Codex normalization | B1/B2 | ACP Codex fixture/live proof |
| C2 Claude normalization | B1/B2 | ACP Claude fixture/live proof |
| C3 runtime controls/commands | C1/C2 | dynamic config/commands |
| C4 tool/plan/task/subagent UI | C1/C2/B2 | typed timeline |
| C5 terminal projection watcher | A1/B2 | incremental JSONL projection |
| C6 workflow broker | B1/B4 | MCP delegation and child sessions |
| C7 Agent Control Center | B4/B5 | workflow UI |
| C8 ownership handoff | C1/C2/C5 | structured/terminal lease |

C1 and C2 may proceed in parallel after B1. C6 may use recorded mock providers before C1/C2 live integration.

### Milestone M2

Native proof of:

- one Claude and one Codex structured session concurrently;
- screenshots to both;
- config pickers;
- tools/approvals/plans/tasks;
- provider-native children;
- one Assembly workflow with two parallel child agents;
- no duplicate provider writer;
- process cleanup.

### Wave D — product surfaces

| Lane | Scope | Dependency |
| --- | --- | --- |
| D1 native browser spike/runtime | B6 | child-webview proof |
| D2 browser chrome/annotations/markup | D1/B3/B5 | browser UX |
| D3 resource backend | B7 | deterministic resource snapshot |
| D4 resource UI/space manager | D3/B5 | compact/full views |
| D5 provider usage collectors | B7/C1/C2 | quota/usage facts |
| D6 usage UI/analytics | D5/B5 | compact/full views |
| D7 rail hover/detail polish | B5/C4/C7 | final session facts |
| D8 AI recipes | B8/C6/D2 | PR/forms/save/browser integrations |

### Milestone M3

SOL-medium review plus real-Tauri interaction recording for all supplied Orca-inspired states.

### Wave E — final integration

Controller-only shared seams, migration, package manifest, capabilities, route registration, and serialized full gates.

---

## 22. Focused test matrix

### ACP host

- initialize and capability negotiation;
- multi-session isolation;
- adapter exit/restart;
- request cancellation;
- stale generation;
- redacted logs;
- sidecar checksum/version;
- app-exit cleanup.

### Conversation

- ordered mixed items;
- text delta batching;
- model/effort/mode changes;
- unsupported/mismatch rollback;
- dynamic commands;
- approvals and structured input;
- plans/tasks;
- long timeline virtualization;
- parent/child scroll;
- restart reconciliation.

### Attachments

Use the complete matrix from section 9.

### Subagents

- Codex provider-native metadata;
- Claude nested transcript metadata;
- unrelated same-title exclusion;
- parent-link authorization;
- provider-native read-only controls;
- Assembly-owned child control;
- depth/cycle/cap bounds.

### Workflow

- DAG eligibility;
- parallel groups;
- depth/concurrency/budget;
- retries;
- cancellation races;
- user gates;
- deterministic gates;
- one-shot/persistent agents;
- provider/model override validation;
- parent teardown;
- replay;
- artifact linkage;
- hostile child output.

### Browser

- presentation transitions;
- same-tab/no-reload;
- z-order/input;
- profiles/workspace isolation;
- grab bounds;
- annotation queue;
- markup tools;
- exact conversation owner;
- draft conflict;
- URL/security rules.

### Resources and usage

- process ownership;
- external protection;
- one-shot join;
- worktree-space safety;
- compact/full state parity;
- provider quota source labeling;
- analytics aggregation;
- no secret persistence.

### Paneview shell

- layout migration;
- left/right pane persistence;
- session-specific/global scoping;
- parking without destroy;
- keyboard resizing;
- narrow widths;
- Session Library separation.

### AI recipes

- untrusted context delimiting;
- stale fact rejection;
- proposal validation;
- preview-before-apply;
- save revalidation;
- no auto-publish/mutation;
- audit redaction.

---

## 23. Native acceptance scenarios

### Scenario 1 — structured conversation basics

1. Start a new Codex structured session and a new Claude structured session in different worktrees.
2. Select model, effort, and permission mode from real provider options.
3. Paste two screenshots into each composer.
4. Remove one and send the other.
5. Observe assistant text, reasoning summary, tool calls, command output, file changes, and task/plan state.
6. Resolve an approval inline.
7. Switch sessions repeatedly without losing drafts, attachments, controls, scroll, tabs, browser, or diff.

### Scenario 2 — raw CLI handoff

Run the handoff roundtrip in section 10 for both providers and record process trees proving one writer.

### Scenario 3 — provider-native subagents

Trigger a Codex and Claude task that use provider-native children. Open every child output, verify nesting/state, and confirm unsupported controls are absent.

### Scenario 4 — Assembly workflow

Run:

```text
Planner
  -> Implementer
  -> Code Review || Spec Compliance || Tests
  -> Fixer
  -> Verification
```

Use at least two providers across roles. Prove parallel starts, role-specific models/effort, persistent output, retry, cancellation, gate behavior, artifacts, and full replay after restart.

### Scenario 5 — browser feedback

1. Open the browser docked.
2. Open the floating overlay from the action island.
3. Expand to full workbench.
4. Grab one element.
5. Annotate a second element.
6. Draw on a viewport screenshot.
7. Stage all three into the correct conversation without auto-send.
8. Start the UI feedback workflow.
9. Restore to floating, docked, and collapsed without page reload.

### Scenario 6 — resources and usage

1. Open compact resource popover.
2. Navigate to an exact session.
3. Open full Resource Manager.
4. Scan workspace space, inspect treemap/table, and cancel a destructive selection.
5. Open compact usage popover.
6. Open full analytics and switch provider tabs.
7. Verify source labels, reset countdowns, and no provider-token leakage.

### Scenario 7 — shell navigation

1. Resize left rail to minimum and comfortable widths.
2. Use hover/focus detail and inline accordion.
3. Resize Working/Done/Settled Paneview panes.
4. Open Session Library separately.
5. Restore after restart.
6. Verify every right/left tab’s internal panels use Paneview and retain state.

### Scenario 8 — AI throughout

1. Draft PR title/body; cancel.
2. Run an AI code review; stage findings only.
3. Fill a form field with a proposal; accept.
4. Trigger save guard; reject its suggestion and save unchanged.
5. Apply a second suggestion, revalidate, and save.
6. Confirm all consequential actions have deterministic receipts.

---

## 24. Stop conditions

Stop before implementation or integration when:

- the selected adapter cannot prove a capability required by the task;
- a provider silently changes config behavior across the pinned version;
- a structured session would require a concurrent raw CLI writer;
- a child cannot be linked to its parent by authoritative metadata;
- an Assembly workflow action would require arbitrary shell execution;
- a browser child view cannot remain below Assembly overlays;
- Paneview migration would destroy session state;
- resource ownership depends only on process names;
- usage data has no trustworthy source;
- AI assistance would mutate without preview/revalidation;
- a shared seam contains unmerged WIP;
- the requested model/effort override for an implementation lane is rejected or downgraded.

Return a narrowly scoped decision packet instead of guessing.

---

## 25. Completion definition

This amendment is complete only when:

1. New Claude and Codex sessions are structured-first ACP sessions.
2. ACP sessions do not create unnecessary xterm/provider-TUI instances.
3. Raw CLI remains available through a proven single-writer handoff.
4. Model, effort, permissions, and provider options are real pickers.
5. TUI-only commands are absent from the structured command menu.
6. Screenshot paste works reliably as real image input for both providers.
7. Tools, command output, file changes, plans, tasks, approvals, questions, and subagents render as typed items.
8. Provider-native children are inspectable; Assembly workflow children are controllable.
9. A durable cross-provider workflow can orchestrate role-based agents with parallel steps, gates, retries, budgets, and audit history.
10. The Agent Control Center shows where every Assembly-owned agent is in the workflow and allows valid control actions.
11. Browser docked/floating/expanded/collapsed modes use one live page and support grab, annotation, markup, and exact conversation staging.
12. Resource and usage compact popovers open into full Dockview workspaces.
13. The left rail is compact, uses hover/accordion details, and does not contain the global session library.
14. Working/Done/Settled and all left/right tab internals use Paneview.
15. AI assistance is reusable across PRs, forms, saves, Git, editor, and browser while remaining proposal-first and deterministically guarded.
16. Existing terminal, worktree, Monaco/Roslyn, Git, diff, browser/session workspace, and compatibility-state contracts remain intact.
