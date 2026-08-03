# Mac Command Bar Structured Agent Conversation Design

**Date:** 2026-08-01
**Status:** Approved by the user
**Scope:** Replace the default raw Claude/Codex terminal presentation with a Codex-style structured conversation while preserving the terminal, session identity, Dockview workspaces, Monaco/Roslyn behavior, and per-session isolation.

## 1. Locked product decisions

1. Structured conversation is the default Session surface for Claude and Codex.
2. Raw terminal remains available as a fallback for unsupported interactions, recovery, and direct CLI use.
3. Every owned session keeps an independent four-tab workspace: Session, Editor, Browser, and Diff. Switching sessions restores that session's content, selections, open files, browser state, and Dockview layout; none of those are shared between sessions.
4. The Session tab uses a centered message timeline and a floating composer at the bottom, following the interaction model of the Codex desktop app.
5. Existing editor, browser, diff, source-control, terminal, workspace snapshot, and owned-session abstractions are extended rather than replaced.
6. Houston remains the default application theme. Monaco must receive Houston through the VS Code theme service; the C# integration must not force Default Dark Modern.

## 2. Architecture

### 2.1 Stable app-owned identity

`ownedId` remains the primary identity for the visible session and its complete workspace. Provider-native identifiers are adapter metadata:

- Codex: thread ID plus active turn ID.
- Claude: session ID plus current streamed request.
- Terminal: existing PTY session ID used only by the raw-terminal fallback.

Changing or resuming a provider-native session updates metadata without replacing `ownedId` or losing the session's workspace.

### 2.2 Rust-managed conversation registry

Add one Rust-managed `AgentConversationRegistry`, keyed by `ownedId`. It owns provider processes, protocol connections, request cancellation, reconnection, and normalized event emission.

Provider adapters:

- **Codex adapter:** a persistent `codex app-server` connection using its structured thread, turn, item, approval, and error events.
- **Claude adapter:** a persistent `claude -p` process using bidirectional `stream-json` input/output. No terminal-screen parsing is used.
- **Raw terminal adapter:** the existing PTY/session service. It is opened explicitly or offered automatically when structured mode cannot represent a provider interaction.

The frontend talks through the existing Tauri IPC/event boundary. No second WebSocket bridge or second session router is added.

### 2.3 Normalized conversation protocol

Rust translates provider events into one frontend contract. The minimum event set is:

- session connected, resumed, interrupted, completed, or failed;
- user and assistant message deltas plus completed messages;
- reasoning/status summaries when the provider exposes them;
- tool started, updated, completed, or failed;
- approval requested, accepted, declined, or expired;
- usage and context information when available;
- provider error with a recoverable action.

Every event carries `ownedId`, a monotonically increasing sequence number, provider-native correlation IDs, and a timestamp. The frontend discards stale events from a prior connection generation.

## 3. Frontend components and state

### 3.1 Session conversation surface

Replace `TerminalSurface` as the default Session content with a focused `ConversationPanel` composed of:

- a virtualized or incrementally rendered message timeline;
- distinct user, assistant, tool, approval, status, and error items;
- a floating multiline composer with send, stop, attach/context, and raw-terminal actions;
- an honest connection/loading state that does not erase already-rendered messages;
- scroll-follow behavior that stops when the user scrolls upward and resumes on explicit action.

The raw terminal mounts inside the same Session tab as a fallback mode. Switching between structured and raw views does not start a second provider session.

### 3.2 Per-session workspace state

Extend the existing retained workspace keyed by `ownedId` to include:

- conversation timeline and draft composer text;
- provider connection/resume metadata;
- open editor files, active editor, selections, and dirty state;
- browser URL, history, and navigation state;
- selected diff/source-control context;
- serialized Dockview layout and active center tab;
- raw-terminal visibility and bound PTY ID.

The current global center-layout storage key becomes session-scoped. Selecting a rail session swaps the complete retained workspace rather than reconstructing individual tabs.

## 4. Data flow

### Sending a turn

1. The composer submits text for the active `ownedId`.
2. The frontend service sends one typed Tauri command with `ownedId`, expected connection generation, and message content.
3. Rust validates ownership and routes the turn to the active provider adapter.
4. The adapter emits normalized incremental events.
5. The conversation store applies only the next valid sequence for that session and paints incremental updates without replacing completed history.

### Switching sessions

1. Persist the outgoing workspace snapshot, including draft and Dockview layout.
2. Activate the incoming `ownedId` and restore its retained workspace synchronously.
3. Keep both provider processes alive; no resume or Roslyn restart occurs solely because the visible session changed.
4. Route later provider events to their owning stores even when those sessions are hidden.

### Resume and history

Provider-native history is imported into the normalized model once and deduplicated by native IDs. Reconnect resumes from the last acknowledged provider position where supported. A process failure preserves rendered history and offers Retry or Open raw terminal; it never clears the timeline.

## 5. Houston theme integration

Register Houston as a VS Code-compatible theme contribution through `@codingame/monaco-vscode-theme-service-override` before Monaco's global service initialization. Set `workbench.colorTheme` to Houston and remove the `Default Dark Modern` override from the C# client configuration.

The application theme registry remains the source of Houston colors. An adapter produces VS Code workbench colors, semantic token colors, and TextMate token colors from that registry so the shell, Monaco, Peek, CodeLens, and future declarative theme extensions remain synchronized.

## 6. Failure and safety behavior

- Structured connection failure affects only the owning session.
- Malformed or out-of-order events are logged and ignored without clearing valid history.
- Stop targets the active provider turn, not the entire owned workspace.
- Approval responses include the exact provider request ID and are rejected after expiry or generation change.
- Provider output, tool text, repository text, and remote content are rendered as untrusted data.
- No Claude/Codex process is duplicated during view switching or structured/raw-mode switching.
- Raw terminal access remains explicit and continues using the existing owned PTY boundary.

## 7. Delivery slices

1. **Conversation shell:** normalized types, per-session store, timeline, composer, retained workspace, and a deterministic mock adapter.
2. **Codex adapter:** app-server lifecycle, streaming turns, stop, resume, tools, approvals, and errors.
3. **Claude adapter:** bidirectional stream-json lifecycle with equivalent normalized behavior.
4. **Theme and recovery:** Houston VS Code theme registration, raw-terminal fallback, reconnect, persistence, and native verification.

Each slice must leave the working terminal and Roslyn implementation intact.

## 8. Verification

Focused tests prove event ordering, connection generations, session isolation, draft retention, layout-key isolation, history deduplication, stop targeting, approval expiry, and structured/raw fallback.

Native Tauri acceptance uses at least three owned sessions across two workspaces:

1. Run Claude and Codex turns concurrently and switch repeatedly while both continue streaming.
2. Confirm each session restores its own Session, Editor, Browser, Diff, and dragged Dockview layout.
3. Stop one turn and prove no other provider process, PTY, or Roslyn workspace is affected.
4. Restart the app and prove history, draft, tabs, browser state, diff selection, and layouts restore to the correct `ownedId`.
5. Verify Houston colors in Monaco, CodeLens, Peek, diagnostics, and the shell with no Default Dark Modern repaint.

Browser preview may verify layout and deterministic mock events. Claims about provider streaming, PTY fallback, process cleanup, session switching, or Roslyn require the rebuilt native Tauri app.

## 9. Explicit non-goals

- Replacing Dockview, Monaco, Roslyn, the PTY registry, or the owned-session model.
- Parsing terminal screen output into messages.
- Building a general extension marketplace or arbitrary VSIX execution as part of this feature.
- Rebuilding Git graph, source control, or diff functionality inside the conversation lane.
- Shipping bundled Roslyn provisioning; that remains a separate packaging requirement.

## 10. Completion definition

Claude and Codex open by default as structured, resumable conversations with reliable streaming, tools, approvals, and stop behavior. Raw terminal remains one click away. Switching a session restores that session's complete independent four-tab workspace, and Houston remains stable across the shell and Monaco. The existing Roslyn CodeLens and workspace isolation continue working in the rebuilt native app.
