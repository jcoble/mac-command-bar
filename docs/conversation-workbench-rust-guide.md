# Conversation Workbench: Rust Guide for .NET Developers

## What owns the conversation

The Claude or Codex process that is already running in the session PTY remains the only agent process. The structured Session view reads that process's JSONL transcript and sends user input to its existing PTY. It never starts a second provider process.

In .NET terms, the PTY is the authoritative hosted service. The Svelte conversation view is a projection over its durable event log, not another service instance.

## Tauri commands are the controller boundary

`src-tauri/src/agent_conversation/mod.rs` exposes narrow Tauri commands. They play roughly the same role as ASP.NET controller actions:

- `read_agent_conversation_transcript` returns a normalized snapshot for one validated parent or child session.
- `save_agent_conversation_attachment` validates and stores one pasted image in the app's managed data directory.
- Existing connection commands remain available for the older structured-provider implementation, but the current Session view does not resume a second provider through them.

Svelte calls these commands with Tauri `invoke`. Rust validates identifiers and filesystem boundaries before touching disk.

## Transcript parsing

`src-tauri/src/agent_conversation/transcript.rs` is a read-only adapter over the providers' JSONL formats. It returns:

- chronological user and assistant messages;
- model, effort, approval policy, and token/context metadata when the transcript records them;
- child-agent descriptors and lifecycle state;
- a child transcript when a validated child ID is selected.

The parser uses `Option<T>` for data that providers may omit. This is the Rust equivalent of nullable reference/value types: the UI shows `unknown` instead of inventing a default.

Transcript paths are cached after discovery, so the 500 ms refresh reads the known file instead of recursively scanning the provider's session tree every time.

## Ownership and borrowing in the attachment path

`src-tauri/src/agent_conversation/attachments.rs` accepts borrowed inputs (`&str` and `&[u8]`). Rust does not copy those arguments merely to validate them. It creates owned `String` values only for data returned to the frontend.

The command accepts PNG, JPEG, GIF, and WebP, enforces a 20 MB limit, validates each file signature, rejects unsafe session-folder names, writes under the Tauri application-data directory, and verifies the canonical path remains inside that directory.

This is similar to an ASP.NET upload endpoint that validates the route ID, content type, file header, size, and final canonical storage path before returning a DTO.

## Frontend state and safety

Conversation state is keyed by `ownedId`. Drafts, pasted-image previews, selected child agents, metadata, and scroll positions cannot leak when switching between sessions or workspaces.

Pasted images are copied into app-managed storage. Sending a message appends their canonical paths to the prompt and uses the existing PTY paste-plus-Enter sequence. Removing a preview removes it from the draft but never deletes user-owned source files.

## Session controls

Model, reasoning effort, approval, skills, and agent pickers are owned by the running provider. Clicking one of those controls sends the provider's authoritative slash command to the existing PTY and temporarily shows the raw terminal picker. Returning to Conversation mode shows the transcript again, and the displayed metadata changes only after the provider records the new value.

This avoids two failure modes: a decorative dropdown that lies about runtime state, and a second app-server process competing with the existing terminal agent.

## Verification

The focused Rust tests cover transcript normalization, Claude sidechain isolation, Codex child activity, unsafe IDs, supported image signatures, spoofed/unsupported images, the size limit, and session registry behavior. The `/next` Svelte gate, TypeScript check, production frontend build, and Rust check all pass without new conversation-module warnings.

The native development binary rebuilds and runs, but the final manual acceptance step remains: verify one real Claude or Codex session can paste a screenshot, open the model/approval picker, return to Conversation mode, and open an existing child transcript without launching another process.
