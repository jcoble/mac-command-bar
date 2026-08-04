# TSK-809 and TSK-810 Conversation Workbench Implementation Plan

> **Superseded again by the 2026-08-04 ACP/workflow amendment:** Product direction now requires
> structured ACP-owned sessions by default, explicit structured/raw single-writer handoff,
> provider-advertised model/effort/permission selectors, real image content blocks, rich tool/plan
> events, and app-owned workflow orchestration. Execute this work through
> `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-shell-amendment.md`, which also
> amends Work Packages 10A and 11 of the TSK-808 master plan. The PTY-only architecture below is
> retained solely as historical discovery and as the fallback/import path for terminal-owned
> sessions.

> **Superseded execution authority (2026-08-03):** This file remains discovery/reference
> evidence. Execute TSK-809 and TSK-810 only through Work Package 10A and the cost-aware dispatch
> manifest in
> `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`. Do not dispatch
> from this older file independently or create a second conversation runtime, store, composer,
> provider host, transcript pane, or child-agent registry.

> **Historical task breakdown only:** The checkboxes below are not an active dispatch queue.

**Goal:** Upgrade the existing transcript-backed Claude/Codex Session surface with a Codex-like composer, attachments, commands, truthful runtime controls, telemetry, and read-only child-agent transcripts.

**Architecture:** Keep the existing PTY as the only writer and expand the Rust JSONL reader into a snapshot API containing display messages, telemetry, and child descriptors. Keep all UI state keyed by `ownedId`; child inspection reads transcript files only and never launches or controls a process.

**Tech Stack:** Tauri 2, Rust/serde_json, Svelte 5 runes, TypeScript 6, existing dockview/session workspace store.

## Global Constraints

- Never launch a second Claude/Codex process for an existing owned session.
- Never scrape terminal pixels for transcript or runtime state.
- Do not remove or replace the Editor, Browser, Diff, or Session dock tabs.
- Missing provider metadata is shown as unknown, never guessed.
- Clipboard images are copied only into an app-managed per-session directory after Rust validates the session identifier and image payload.
- Child inspection is read-only and cannot start, resume, interrupt, approve, send to, or kill an agent.

---

### Task 1: Transcript snapshot and child discovery

**Files:**
- Modify: `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript.rs`
- Modify: `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
- Modify: `tauri-svelte-preview/src-tauri/src/main.rs`

**Interfaces:**
- Produces: `TranscriptSnapshot { messages, metadata, children }`
- Produces: `read_agent_conversation_transcript(provider, native_session_id, child_session_id)`
- Produces: `ConversationMetadata { model, effort, approval_policy, used_tokens, context_window }`
- Produces: `ChildAgentDescriptor { child_id, parent_id, provider, label, state, updated_at_ms }`

- [ ] Add failing Rust fixtures for Codex `session_meta`, `turn_context`, token-count events, and spawned-thread metadata.
- [ ] Add failing Rust fixtures for Claude parent messages, sidechain metadata, child lifecycle evidence, and child-only transcript reads.
- [ ] Replace the vector-only return with a serializable snapshot while retaining chronological text normalization.
- [ ] Restrict child lookup to validated transcript roots and exact safe session IDs.
- [ ] Register the updated Tauri command and run `cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation`.

### Task 2: Session-scoped composer state and services

**Files:**
- Modify: `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts`
- Modify: `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts`
- Modify: `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`
- Modify: `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts`
- Modify: `tauri-svelte-preview/scripts/agentConversationStore.test.mjs`
- Modify: `tauri-svelte-preview/scripts/conversationSessionIsolation.test.mjs`

**Interfaces:**
- Consumes: Rust `TranscriptSnapshot`.
- Produces: `ConversationComposerState`, `ConversationAttachment`, `ConversationMetadata`, and `ConversationChildAgent` keyed by `ownedId`.
- Produces: store mutations for draft, attachment, model, effort, approval, selected child, and scroll position.

- [ ] Extend the store tests to prove two owned sessions cannot share drafts, attachments, controls, selected children, or scroll positions.
- [ ] Add snapshot mapping that retains the last valid transcript when a refresh fails and ignores stale child responses.
- [ ] Persist lightweight composer selections and child selection through `SessionConversationWorkspace`.
- [ ] Add provider command catalogs and insertion helpers with no execution-on-selection behavior.
- [ ] Run `pnpm test:agent-conversation-store` and `pnpm test:conversation-session-isolation` from `tauri-svelte-preview`.

### Task 3: Safe clipboard attachments

**Files:**
- Create: `tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs`
- Modify: `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs`
- Modify: `tauri-svelte-preview/src-tauri/src/main.rs`
- Modify: `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`

**Interfaces:**
- Produces: `save_agent_conversation_attachment(owned_id, mime_type, bytes) -> SavedConversationAttachment`.
- Consumes: PNG, JPEG, GIF, and WebP clipboard payloads under an explicit size cap.

- [ ] Add Rust tests for safe IDs, supported MIME types, size limits, canonical app-managed paths, and unique filenames.
- [ ] Decode the frontend byte array in Rust, validate magic bytes, and write it under the application data directory.
- [ ] Return canonical path, MIME type, byte length, and attachment ID.
- [ ] Compose outgoing PTY text from the draft plus exact attachment paths while leaving the draft intact on failure.
- [ ] Run the focused Rust and TypeScript tests.

### Task 4: Codex-like transcript and composer UI

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationComposer.svelte`
- Create: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationMessage.svelte`
- Create: `tauri-svelte-preview/src/lib/shell/components/conversation/AgentTree.svelte`
- Modify: `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte`
- Modify: `tauri-svelte-preview/src/lib/shell/styles/next.css`

**Interfaces:**
- Consumes: session-scoped store state and service functions from Tasks 1–3.
- Produces: selectable transcript, screenshot previews, slash menu, runtime status controls, context meter, and child-agent tree.

- [ ] Render selectable user/assistant text with safe lightweight Markdown formatting for paragraphs, lists, inline code, and fenced code.
- [ ] Add clipboard-image capture, removable previews, slash command filtering, keyboard selection, and accessible labels.
- [ ] Remove both composer borders and use the existing Houston variables for Codex-like spacing, hierarchy, and restrained semantic color.
- [ ] Show metadata controls as pending until the authoritative transcript confirms a change; render unsupported controls read-only.
- [ ] Render child agents beneath the parent and open child transcript content without exposing write or process controls.

### Task 5: Compile and native acceptance

**Files:**
- Modify only files required by reproduced failures.

**Interfaces:**
- Verifies: TSK-809 and TSK-810 acceptance criteria in the real Tauri desktop target.

- [ ] Run `pnpm check`, the focused conversation tests, and focused Rust tests one at a time.
- [ ] Start `pnpm tauri:dev:next` and verify Claude and Codex parent transcripts, message submission, session switching, screenshot paste, copy selection, metadata, and child inspection.
- [ ] Verify Editor, Browser, Diff, and Session tabs remain present and Monaco/Roslyn still opens an existing C# file.
- [ ] Fix only reproduced failures and repeat the failing proof.
- [ ] Close TSK-809 and TSK-810 only after the native evidence passes and both task statuses read back as Done.