# Structured Agent Conversation Implementation Plan

> **Execution rule:** Implement one task at a time in the current protected checkout. Do not
> stage, commit, reset, stash, or rewrite unrelated dirty WIP. Do not create a worktree for this
> lane unless the user explicitly asks for one. Do not make live Claude or Codex model requests
> during automated verification.

**Goal:** Make Claude and Codex sessions open as structured, resumable conversations with a
centered timeline and floating composer, while keeping the existing raw terminal one click away,
preserving each session's independent Session/Editor/Browser/Diff workspace, and applying Houston
through Monaco's VS Code theme service.

**Architecture:** Keep `ownedId` as the stable application identity. Add one Rust
`AgentConversationRegistry` keyed by `ownedId`, with provider adapters for persistent Codex
app-server JSON-RPC and Claude bidirectional stream-json. Normalize provider output into one typed
Tauri event stream. Keep frontend state keyed by `ownedId`, mount the structured view inside the
existing Session Dockview panel, and reuse the existing PTY service only for raw fallback.

**Tech stack:** Svelte 5, TypeScript, Vite, Tauri 2, Rust/Tokio, Dockview, Monaco's installed
VS Code compatibility layer, `codex app-server`, and `claude -p` stream-json.

**Approved specification:**
`docs/superpowers/specs/2026-08-01-structured-agent-conversation-design.md`

---

## Locked constraints

1. Structured conversation is the default for owned Claude and Codex sessions. The existing
   `TerminalSurface` remains available as the raw fallback and is not deleted.
2. Provider output is never reconstructed by scraping terminal screen text. New user messages
   come from the composer; provider history is imported only through structured provider APIs.
3. Switching sessions does not restart Claude, Codex, Roslyn, Monaco, or the PTY. Background
   events continue routing to the store for their owning `ownedId`.
4. Do not put full conversation transcripts in `localStorage`. Persist small workspace metadata
   there; retain live transcript state in memory and restore durable history from the provider or
   Rust conversation snapshot.
5. Houston is registered before the one-time VS Code service initialization. Remove the forced
   `Default Dark Modern` setting instead of repainting Monaco after startup.

## File structure

### New Rust files

- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs` — registry, Tauri commands,
  lifecycle, event emission, and adapter selection.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs` — serialized request,
  connection, event, payload, approval, and error types.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs` — child-process ownership,
  line framing, stderr draining, cancellation, and shutdown.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/codex.rs` — Codex app-server adapter.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/claude.rs` — Claude stream-json adapter.

### New frontend files

- `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts` — normalized frontend
  contract matching Rust serialization.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts` — pure ordered event
  reducer and history deduplication.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts` — state keyed by
  `ownedId`; no backend calls.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts` — Tauri commands,
  global event listener, connection guards, and routing.
- `tauri-svelte-preview/src/lib/shell/components/ConversationPanel.svelte` — structured/raw mode
  shell and connection state.
- `tauri-svelte-preview/src/lib/shell/components/ConversationTimeline.svelte` — message, tool,
  approval, status, and error rendering.
- `tauri-svelte-preview/src/lib/shell/components/ConversationComposer.svelte` — draft, send, stop,
  and raw-terminal controls.
- `tauri-svelte-preview/src/lib/shell/themes/vscodeThemeAdapter.ts` — Houston-to-VS Code theme
  registration before Monaco service initialization.

### New focused tests

- `tauri-svelte-preview/scripts/agentConversationProtocol.test.mjs`
- `tauri-svelte-preview/scripts/agentConversationStore.test.mjs`
- `tauri-svelte-preview/scripts/agentConversationService.test.mjs`
- `tauri-svelte-preview/scripts/conversationSessionIsolation.test.mjs`
- `tauri-svelte-preview/scripts/houstonVscodeTheme.test.mjs`

### Existing files to modify

- `tauri-svelte-preview/src-tauri/src/main.rs`
- `tauri-svelte-preview/src-tauri/Cargo.toml` and `Cargo.lock` only if the implementation needs a
  missing Tokio feature or a narrowly justified protocol dependency.
- `tauri-svelte-preview/src/lib/tauriSource.ts`
- `tauri-svelte-preview/src/routes/next/+page.svelte`
- `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts`
- `tauri-svelte-preview/src/lib/shell/browser/browserStore.svelte.ts`
- `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`
- `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte`
- `tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts`
- `tauri-svelte-preview/src/lib/shell/themes/themeRegistry.ts`
- `tauri-svelte-preview/src/lib/shell/editor/monacoWorkers.ts`
- `tauri-svelte-preview/package.json` and `pnpm-lock.yaml`
- Existing focused tests for workspace snapshots, the C# client, and the theme registry.

---

## Task 1: Freeze the normalized protocol with pure tests

**Files:**

- Create `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts`
- Create `tauri-svelte-preview/src/lib/shell/conversation/conversationReducer.ts`
- Create `tauri-svelte-preview/scripts/agentConversationProtocol.test.mjs`
- Modify `tauri-svelte-preview/package.json`

### Step 1: Add the failing protocol test

Test these exact rules:

- every event has `ownedId`, `provider`, `generation`, `sequence`, and `timestampMs`;
- events from an older generation are ignored;
- an already-applied sequence is ignored;
- assistant deltas append to the matching native item and completion seals it;
- imported history deduplicates by provider-native item ID;
- an error preserves all completed timeline entries.

Use deterministic fixtures rather than provider processes. Add
`test:agent-conversation-protocol` to `package.json`.

Run:

```bash
pnpm test:agent-conversation-protocol
```

Expected: FAIL because the types and reducer do not exist.

### Step 2: Implement the shared frontend contract

Use these public shapes:

```ts
export type AgentConversationProvider = 'codex' | 'claude';

export interface AgentConversationEvent {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  sequence: number;
  timestampMs: number;
  payload: AgentConversationPayload;
}

export type AgentConversationPayload =
  | { kind: 'connection'; state: ConversationConnectionState; nativeSessionId?: string }
  | { kind: 'userMessage'; itemId: string; text: string; completed: true }
  | { kind: 'assistantDelta'; itemId: string; delta: string }
  | { kind: 'assistantMessage'; itemId: string; text: string; completed: true }
  | { kind: 'tool'; itemId: string; name: string; state: ToolState; summary?: string }
  | { kind: 'approval'; requestId: string; state: ApprovalState; summary: string }
  | { kind: 'turn'; turnId: string; state: TurnState }
  | { kind: 'usage'; inputTokens?: number; outputTokens?: number }
  | { kind: 'error'; code: string; message: string; recoverable: boolean };
```

The reducer returns the previous object unchanged for stale events. For a gap in sequence, mark
the session `desynchronized` and retain history; do not guess missing deltas.

### Step 3: Prove the reducer

Run the focused test again.

Expected: PASS with deterministic sequence, generation, deduplication, and error-retention cases.

### Step 4: Check the task boundary

Confirm no terminal parsing, Tauri call, Svelte component, or `localStorage` access exists in the
new files.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 2: Add per-session conversation and complete workspace state

**Files:**

- Create `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts`
- Modify `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts`
- Modify `tauri-svelte-preview/src/lib/shell/browser/browserStore.svelte.ts`
- Modify `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`
- Modify `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte`
- Modify `tauri-svelte-preview/scripts/sessionWorkspaces.test.mjs`
- Create `tauri-svelte-preview/scripts/agentConversationStore.test.mjs`
- Create `tauri-svelte-preview/scripts/conversationSessionIsolation.test.mjs`

### Step 1: Add failing session-isolation tests

Create three `ownedId` fixtures and prove:

- each session has an independent transcript, draft, connection generation, and visible mode;
- editing session A's draft does not mutate B or C;
- hidden session events update only their owning store;
- outgoing workspace capture and incoming restore include browser state, diff context, active center
  panel, serialized center layout, conversation mode, draft, and raw-terminal visibility;
- storage keys are scoped by `ownedId` and do not collide.

Run:

```bash
pnpm test:session-workspaces
pnpm test:agent-conversation-store
pnpm test:conversation-session-isolation
```

Expected: existing workspace test may pass, while the two new tests fail on missing state.

### Step 2: Implement the conversation store

Expose a store API that never calls the backend:

```ts
interface AgentConversationStore {
  get(ownedId: string): ConversationSessionState;
  ensure(ownedId: string, provider: AgentConversationProvider): ConversationSessionState;
  apply(event: AgentConversationEvent): ApplyEventResult;
  setDraft(ownedId: string, draft: string): void;
  setMode(ownedId: string, mode: 'structured' | 'raw'): void;
  markSending(ownedId: string, sending: boolean): void;
  remove(ownedId: string): void;
}
```

Keep transcript entries in memory. The persisted workspace stores only draft, mode, provider
cursor metadata, and layout/UI state.

### Step 3: Extend `SessionWorkspaceSnapshot`

Add backward-compatible optional fields, normalize older snapshots, and bump the storage version
only if needed:

```ts
conversation?: {
  mode: 'structured' | 'raw';
  draft: string;
  providerGeneration?: number;
  lastSequence?: number;
};
browser?: { url: string; canGoBack: boolean; canGoForward: boolean };
center?: { activePanelId: string; layout: unknown };
rawTerminalVisible?: boolean;
```

Do not serialize provider output or secret-bearing tool payloads.

### Step 4: Make browser and Dockview capture/restore explicit

Replace the one global browser URL assumption with `captureBrowserState()` and
`restoreBrowserState(snapshot)`. Change center Dockview creation to accept a session-specific key
or explicit `captureLayout()` / `restoreLayout()` calls. Do not create multiple active Dockview
instances just to persist different sessions.

### Step 5: Prove three-session isolation

Run the three focused commands from Step 1.

Expected: PASS, including migration of a snapshot that predates conversation/browser/center fields.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 3: Implement the Rust registry and provider-independent lifecycle

**Files:**

- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs`
- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/process.rs`
- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
- Modify `tauri-svelte-preview/src-tauri/src/main.rs`
- Modify `tauri-svelte-preview/src-tauri/Cargo.toml` and `Cargo.lock` only if required

### Step 1: Write Rust tests against a fake adapter

Test these lifecycle invariants inside the new module:

- `ensure` is idempotent for the same `ownedId`, provider, and live generation;
- changing provider closes the old child before installing the new one;
- reconnect increments generation and resets sequence to one;
- `send`, `stop`, and approval responses reject the wrong generation;
- closing session A does not close B;
- malformed child output becomes a recoverable error event;
- stderr is drained continuously and cannot fill the child pipe;
- registry drop terminates all child processes it owns.

Run:

```bash
cd tauri-svelte-preview/src-tauri
RUST_TEST_THREADS=1 cargo test agent_conversation -- --nocapture
```

Expected: FAIL because the module is not registered.

### Step 2: Define Rust wire types matching TypeScript

Use Serde tagged enums with frontend-compatible camelCase field names. The public command request
types are:

```rust
EnsureAgentConversationRequest { owned_id, provider, cwd, native_session_id }
SendAgentConversationMessageRequest { owned_id, generation, text }
RespondAgentConversationApprovalRequest { owned_id, generation, request_id, decision }
StopAgentConversationTurnRequest { owned_id, generation }
```

The connection result includes `ownedId`, `provider`, `generation`, `nativeSessionId`, and state.

### Step 3: Implement the registry boundary

Register one managed `AgentConversationRegistry`. Expose exactly these Tauri commands:

- `ensure_agent_conversation`
- `send_agent_conversation_message`
- `respond_agent_conversation_approval`
- `stop_agent_conversation_turn`
- `close_agent_conversation`
- `read_agent_conversation_snapshot`

Emit one global event name, `agent_conversation_event`. Sequence assignment happens in the
registry after provider normalization, so adapters cannot create conflicting sequence numbers.

### Step 4: Implement safe child ownership

Reuse the process-management patterns already present in the terminal/LSP modules without coupling
conversation state to them. Validate `cwd` against the owning session/workspace boundary already
used by the app. Spawn without a shell. Drain stdout and stderr in independent tasks. Stop the
process group on explicit close and app exit.

### Step 5: Prove lifecycle behavior

Run the focused Rust test with `RUST_TEST_THREADS=1`.

Expected: PASS without launching real Claude or Codex.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 4: Implement the Codex app-server adapter

**Files:**

- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/codex.rs`
- Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
- Modify Rust tests inside the new module

### Step 1: Capture the installed app-server protocol locally

Generate the installed CLI's JSON schema into a temporary directory and inspect its exact method
and notification names:

```bash
codex app-server generate-json-schema --out "$(mktemp -d)"
```

Do not copy a stale protocol definition from documentation. Record the supported method mapping in
test fixtures or narrow Rust conversion functions, not in a second generic JSON-RPC framework.

### Step 2: Add failing JSON-RPC fixture tests

Cover initialize, thread start/resume, turn start, item delta/completion, tool activity, approval,
turn completion, interruption, and provider error. Assert native IDs survive normalization and
user messages enter the timeline as `userMessage` events.

### Step 3: Implement one persistent stdio connection

Launch `codex app-server --listen stdio://` once per owned Codex session unless the protocol
supports safely multiplexing ownership without state leakage. Perform its initialize handshake,
then start or resume the provider thread. Maintain request IDs and route notifications by native
thread/turn/item IDs.

Structured/raw view switching must not spawn or resume another app-server process.

### Step 4: Map stop, approval, resume, and history

- stop targets only the current native turn;
- approval responses carry the exact request ID and generation;
- resume imports provider history once and deduplicates it by native item ID;
- unsupported protocol/version returns a recoverable error offering raw terminal.

### Step 5: Run fake-process verification

Use a deterministic fixture process that emits app-server JSON-RPC. Do not make a live model call.

Run:

```bash
cd tauri-svelte-preview/src-tauri
RUST_TEST_THREADS=1 cargo test agent_conversation::codex -- --nocapture
```

Expected: PASS for streaming, history, stop, approval, and process cleanup.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 5: Implement the Claude stream-json adapter

**Files:**

- Create `tauri-svelte-preview/src-tauri/src/agent_conversation/claude.rs`
- Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
- Modify Rust tests inside the new module

### Step 1: Add failing JSONL fixture tests

Model Claude's installed CLI output as newline-delimited JSON fixtures. Cover system/init,
assistant text blocks, partial messages, tool use/result, permission requests when exposed,
result/completion, interruption, usage, malformed lines, and nonzero exit.

### Step 2: Launch Claude without a shell

Use the installed CLI's supported structured flags:

```text
claude -p --input-format stream-json --output-format stream-json
  --include-partial-messages --replay-user-messages
```

Add `--resume <session-id>` only when resuming known provider state. Keep stdin open for
bidirectional turns. Capture the native session ID from structured output.

### Step 3: Normalize messages and tool activity

Translate Claude content blocks into the same events used by Codex. Deduplicate replayed user
messages by native ID/content position. Never parse ANSI text or terminal screen rows.

### Step 4: Define honest unsupported behavior

If the installed Claude CLI does not expose a structured approval response required by an
interaction, emit a recoverable `approval`/`error` state with `Open raw terminal`; do not invent a
permission response or silently approve it.

### Step 5: Run fake-process verification

Run:

```bash
cd tauri-svelte-preview/src-tauri
RUST_TEST_THREADS=1 cargo test agent_conversation::claude -- --nocapture
```

Expected: PASS without a live Claude request or API charge.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 6: Wire the frontend service and structured Session panel

**Files:**

- Create `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`
- Create `tauri-svelte-preview/src/lib/shell/components/ConversationPanel.svelte`
- Create `tauri-svelte-preview/src/lib/shell/components/ConversationTimeline.svelte`
- Create `tauri-svelte-preview/src/lib/shell/components/ConversationComposer.svelte`
- Modify `tauri-svelte-preview/src/lib/tauriSource.ts`
- Modify `tauri-svelte-preview/src/routes/next/+page.svelte`
- Create `tauri-svelte-preview/scripts/agentConversationService.test.mjs`
- Modify `tauri-svelte-preview/package.json`

### Step 1: Add failing service tests with a fake Tauri boundary

Prove:

- the event listener is installed exactly once;
- events route by `ownedId`, even when another session is visible;
- `ensure` is guarded against duplicate in-flight calls;
- send clears only the submitted session's draft after acceptance;
- stop and approval include the expected generation;
- stale backend responses cannot replace a newer connection;
- component unmount does not close the provider process.

### Step 2: Add typed frontend command wrappers

Extend `tauriSource.ts` with typed wrappers for the six Rust commands. Keep raw `invoke` calls out
of Svelte components.

### Step 3: Implement the conversation service

The service owns all backend calls and one global listener. It updates the store, exposes
`ensure`, `send`, `stop`, `approve`, `decline`, `retry`, and `close`, and prevents duplicate
connections during Svelte remounts.

### Step 4: Build the Codex-style Session surface

`ConversationPanel` receives `ownedId`, provider, cwd, and native session metadata. It renders:

- a centered, readable timeline;
- distinct user and assistant messages;
- compact tool/status/approval rows;
- persistent rendered history during reconnect;
- a floating multiline composer at the bottom;
- Send while idle, Stop while running, Retry on recoverable failure;
- `Open raw terminal` without starting a second provider process.

Use accessible labels and keyboard behavior. Enter sends; Shift+Enter inserts a newline. Do not
copy the Codex app's branding or terminal typography.

### Step 5: Replace only the default Session content

In `/next/+page.svelte`, render `ConversationPanel` for Claude/Codex owned sessions and keep
`TerminalSurface` mounted/available for raw mode. Non-Claude/Codex sessions continue using their
existing terminal behavior.

### Step 6: Prove frontend behavior

Run:

```bash
pnpm test:agent-conversation-service
pnpm test:agent-conversation-store
pnpm test:conversation-session-isolation
pnpm check
```

Expected: PASS. The fake backend must demonstrate simultaneous streaming into three independent
sessions while the visible session changes.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 7: Apply Houston through the VS Code theme service

**Files:**

- Create `tauri-svelte-preview/src/lib/shell/themes/vscodeThemeAdapter.ts`
- Modify `tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts`
- Modify `tauri-svelte-preview/src/lib/shell/themes/themeRegistry.ts`
- Modify `tauri-svelte-preview/src/lib/shell/editor/monacoWorkers.ts`
- Modify `tauri-svelte-preview/package.json` and `pnpm-lock.yaml`
- Modify `tauri-svelte-preview/scripts/csharpLanguageClient.test.mjs`
- Modify `tauri-svelte-preview/scripts/themeRegistry.test.mjs`
- Create `tauri-svelte-preview/scripts/houstonVscodeTheme.test.mjs`

### Step 1: Add failing theme tests

Assert:

- `Default Dark Modern` is absent from the C# configuration;
- Houston is registered before `apiReady` resolves;
- one source of theme tokens feeds shell, workbench colors, TextMate rules, and semantic tokens;
- repeated editor mounts do not initialize the VS Code service or theme override twice.

### Step 2: Add the aligned theme service override

Add `@codingame/monaco-vscode-theme-service-override` at exact version `25.1.2`, matching every
installed `@codingame/monaco-vscode-*` package. Do not mix compatibility-layer versions.

### Step 3: Register Houston before Monaco initialization

Create a VS Code-compatible Houston theme from the existing registry and include the theme service
override in the one-time service initialization. Set `workbench.colorTheme` to Houston. Remove the
forced `Default Dark Modern` configuration.

### Step 4: Prove stable theme behavior

Run:

```bash
pnpm test:csharp-language-client
pnpm test:theme-registry
pnpm test:houston-vscode-theme
pnpm check
```

Expected: PASS with no post-start theme repaint and no duplicate worker/service initialization.

Do not commit unless the user explicitly authorizes a commit.

---

## Task 8: Integrate, build, and prove the native behavior

**Files:**

- Modify only defects discovered in files already owned by Tasks 1–7
- Add focused regression assertions only for reproduced defects
- Update the approved specification if implementation changes an external contract

### Step 1: Run all focused tests serially

Do not run heavy builds in parallel.

```bash
cd tauri-svelte-preview
pnpm test:agent-conversation-protocol
pnpm test:agent-conversation-store
pnpm test:agent-conversation-service
pnpm test:conversation-session-isolation
pnpm test:session-workspaces
pnpm test:csharp-language-client
pnpm test:theme-registry
pnpm test:houston-vscode-theme
pnpm check
```

### Step 2: Run Rust tests serially

```bash
cd tauri-svelte-preview/src-tauri
RUST_TEST_THREADS=1 cargo test agent_conversation -- --nocapture
```

### Step 3: Build frontend and Rust serially

```bash
cd tauri-svelte-preview
pnpm build
cd src-tauri
cargo build
```

Do not launch another heavy build while either command is active.

### Step 4: Prove browser-preview layout with fake providers

Use a uniquely named browser session. Verify centered timeline, floating composer, keyboard send,
stop state, raw fallback toggle, hidden-session updates, draft retention, browser restoration,
diff restoration, and session-specific Dockview layouts. Close the exact browser session and all
of its helpers immediately after proof.

Browser preview is not evidence for native provider/process behavior.

### Step 5: Prove rebuilt Tauri behavior

Start the rebuilt native app. Use deterministic fake-provider mode first, then—only with explicit
user authorization for live provider calls—verify one Codex and one Claude turn. Across at least
three owned sessions and two workspaces, prove:

- user composer messages and assistant output appear as structured entries;
- session switching does not duplicate processes or clear content;
- each Session/Editor/Browser/Diff workspace and dragged layout restores independently;
- stop targets one turn, while raw terminal remains available;
- Houston is stable in Monaco, CodeLens, Peek, diagnostics, and shell;
- app exit terminates task-owned provider processes while preserving provider-backed resume state.

### Step 6: Report exact completion state

Report:

- verified behavior in the rebuilt native app;
- tests and builds run with their results;
- WIP items, especially provider approval limitations or packaging paths;
- all files changed;
- browser/process cleanup status;
- a short .NET-to-Rust explanation: registry as a singleton service, adapter as an interface-like
  boundary, Tokio tasks as async background workers, channels as typed event queues, and Serde as
  the JSON contract mapper.

Do not label the feature complete if only fake providers or browser preview were proved. Do not
commit or push unless the user explicitly asks.

---

## Acceptance checklist

- [ ] Claude and Codex default to the structured Session surface.
- [ ] Composer messages appear as user messages without terminal scraping.
- [ ] Raw terminal is one click away and does not duplicate the provider process.
- [ ] Three sessions keep independent conversation, draft, Editor, Browser, Diff, and Dockview
      layout state while background streaming continues.
- [ ] Codex app-server and Claude stream-json both support resume, stop, tool state, errors, and
      the approvals their structured protocols expose.
- [ ] Stale generation and sequence events cannot erase or replace newer conversation state.
- [ ] Houston is registered once through the VS Code theme service; `Default Dark Modern` is gone.
- [ ] Existing Roslyn CodeLens behavior still works in the rebuilt native app.
- [ ] All automated tests avoid live model calls.
- [ ] Native acceptance distinguishes verified behavior from remaining WIP and confirms child
      process cleanup.
