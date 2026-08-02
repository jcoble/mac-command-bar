# Conversation Workbench Design

**Tasks:** TSK-809 and TSK-810

**Goal:** Bring the existing transcript-backed Claude and Codex conversation surface close to the Codex desktop experience while preserving MacCommandBar's session, workspace, tab, and PTY ownership model.

## Product boundary

The existing Claude or Codex process remains authoritative. Conversation rendering reads its durable transcript and message submission writes to its existing PTY. The feature must never start a second provider process, attach another app-server or stream-json client, or infer state by scraping terminal pixels.

TSK-809 delivers the conversation composer, attachments, commands, session controls, telemetry, and visual treatment. TSK-810 adds read-only discovery and inspection of child agents beneath the same parent session.

## Visual direction

Use the Codex desktop app as the interaction and spacing reference while retaining MacCommandBar's Houston-derived color tokens. The conversation should feel quiet and readable rather than panel-heavy:

- Center the transcript in a readable column with generous vertical separation between turns.
- Render assistant text as the primary reading surface and user messages as visually distinct compact turns.
- Use restrained semantic color only for errors, approvals, active work, and selected agents.
- Format prose, code, lists, tool activity, and approval requests with distinct hierarchy and consistent line height.
- Remove both the textarea focus rectangle and the enclosing rounded composer box. Keep an obvious insertion point, lightweight controls, and a floating bottom position without a container outline.
- Preserve the existing Editor, Browser, Diff, and Session dock tabs and all per-session layouts.

## TSK-809 architecture

### Frontend session state

Extend the conversation state keyed by `ownedId`. Each session owns its draft, attachment previews, selected model, reasoning effort, approval mode, command-menu state, and transcript scroll position. Switching sessions must never reuse these values from another owned session.

### Transcript and telemetry

Extend Rust transcript parsing to return a snapshot rather than only flattened messages. The snapshot contains chronological display items plus provider metadata available in the authoritative JSONL: model, reasoning effort, approval policy, token usage/context window, and turn state. Missing metadata is represented as unknown; the UI must not invent a value.

### Message submission and attachments

Plain text continues through the existing PTY submission path. Clipboard image data is handed to a Tauri command that validates the session identifier, writes the image into an app-managed per-session attachment directory, and returns the canonical path and media metadata. The outgoing prompt references those canonical paths so the existing agent can read them. Removing a preview before submission removes it from the draft; cleanup must never delete user-owned source files.

### Commands, skills, and controls

The slash menu is a discoverable frontend catalog filtered by provider. Selecting an entry inserts or applies the exact command; it never runs during menu navigation. Skills are discovered from the provider's existing skill locations and inserted using the provider's supported syntax.

Model, effort, and approval controls use the existing PTY only when the running provider supports an authoritative command. If a provider/session exposes metadata but no safe live mutation mechanism, the control is read-only and explains why. No UI selection may claim success until the authoritative transcript/runtime confirms the changed value.

### Copy and formatting

Transcript content uses normal selectable DOM text and standard macOS copy behavior. Rich transcript rendering supports paragraphs, lists, fenced code, inline code, and links without enabling arbitrary HTML. Tool and approval items remain compact disclosure rows so transcript narration stays dominant.

## TSK-810 architecture

### Child discovery

Rust scans only transcript locations already associated with the active parent session. Claude children are identified from sidechain/subagent metadata and Codex children from spawned-by/thread metadata. Each child record contains a stable child ID, parent ID, provider, label/role when available, lifecycle state derived from transcript evidence, timestamps, and a transcript locator.

### Child transcript inspection

Reading a child uses the same normalized transcript snapshot contract as a parent. Inspection is read-only: no start, resume, send, interrupt, approval, or kill command is available for child views.

### UI placement

The top-level session rail remains unchanged. A collapsible agent tree appears within the active Session surface. Selecting a child opens a session-owned dock tab, and parent/child selections and scroll positions persist within the owning workspace. Child status uses text plus restrained color so state is not conveyed by color alone.

## Error handling

- A missing or rotating transcript shows a non-destructive unavailable/loading state and retries without clearing the last valid snapshot.
- Unsupported metadata remains unknown rather than falling back to a guessed default.
- Attachment-write failures leave the draft intact and show a local error beside the affected preview.
- A stale child transcript remains inspectable and is marked unavailable/completed based on the last authoritative evidence.
- Session switching cancels or ignores stale asynchronous responses using the owned session and request generation.

## Verification

- Rust fixture tests cover Claude parent/sidechain formats, Codex parent/spawned-thread formats, telemetry extraction, path validation, and attachment persistence.
- Frontend tests cover per-session draft/control/attachment isolation, command selection, context display, child-tree mapping, and stale-response rejection.
- TypeScript and Rust compile checks must pass without introducing warnings in the conversation modules.
- Native Tauri verification covers Claude and Codex message submission, screenshot paste, selection/copy, session switching, live metadata display, and opening child transcripts without starting a process.
- Existing Monaco/Roslyn loading and Editor, Browser, Diff, and Session tabs receive a focused smoke check because the work shares the center dock.

## Stop conditions

Do not close TSK-809 until the controls affect or accurately report the real running session, pasted screenshots reach the agent, and the native composer matches the approved unboxed Codex-like treatment.

Do not close TSK-810 until both providers have real-format fixture coverage and native inspection proves that opening a child transcript does not create or control a process.
