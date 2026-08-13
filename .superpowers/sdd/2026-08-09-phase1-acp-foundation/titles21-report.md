# Lane titles21 — real session titles

## Result

Verified complete in `/Users/blackcolours/dev/work/worktrees/mac-command-bar/titles21` on branch `lane/titles21`. No files were staged or committed.

## Title write and refresh path

- First-send title assignment: `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:769-797` checks that the stored title is empty and that SQLite has no earlier user message, then sets the title before the existing turn and user-message events are persisted and dispatched.
- Title normalization: `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1968-1978` treats null, empty, and whitespace-only titles as empty; it trims the first line and takes at most 64 Unicode scalar values without an ellipsis.
- Existing persistence seam: the existing event recorder persists session metadata before dispatch; no new event kind was added.
- Live rail refresh: `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:400-421` consumes the existing user-message event and updates only an empty in-memory rail title. A non-empty user title is never replaced.
- Label seam: `tauri-svelte-preview/src/lib/shell/sessionStrip.ts:46-58` prefers the stored title and retains the existing eight-character ID fallback. The mirrored frontend normalizer keeps the live projection aligned with the native rule.
- `WorktreeAgentRow` required no change: it already derives `label` through `sessionLabel` at `tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte:77`, renders it at line 443, and passes the same full value into the hover-card view at lines 247-250.
- Full hover title: `tauri-svelte-preview/src/lib/shell/components/SessionHoverCard.svelte:47-49,126-132` renders the complete title and now wraps instead of visually truncating it.

## Startup backfill SQL

`core/src/session_store.rs:345-364` executes one SQL statement per untitled session and returns only the earliest matching event payload:

```sql
SELECT payload
FROM events
WHERE owned_id = ?1
  AND json_extract(payload, '$.payload.kind') = 'userMessage'
  AND seq = (
      SELECT MIN(seq)
      FROM events
      WHERE owned_id = ?1
        AND json_extract(payload, '$.payload.kind') = 'userMessage'
  )
```

Recovery applies that result only to empty titles at `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1853-1871`, then uses the existing session persistence write. No journal rows are loaded, grouped, filtered, or sorted in Rust.

## Tests and verification

Named Rust tests:

- `first_prompt_titles_once_and_explicit_metadata_wins` — `manager.rs:3992-4088`
- `prompt_title_is_char_boundary_safe_and_capped` — `manager.rs:4090-4095`
- `startup_recovery_backfills_untitled_session_from_first_user_message` — `manager.rs:4330-4393`

Final Rust command:

```text
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 265 filtered out; finished in 21.05s
```

Frontend commands:

```text
pnpm run test:session-strip
sessionStrip: all assertions passed

pnpm run test:conversation-send-recovery
conversationSendRecovery.test.ts passed

pnpm run check:svelte
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

Additional final checks:

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
```

Both exited successfully with no output.

## Cleanup

Browser cleanup: not started — no browser, browser automation, or browser daemon was opened.

Vite cleanup: not started — no Vite server was launched and ports 5181, 5182, 5183, and 5177 were untouched.

Worktree cleanup: not removed — `/Users/blackcolours/dev/work/worktrees/mac-command-bar/titles21`, branch `lane/titles21`, contains the requested uncommitted deliverable and remains the user-assigned active worktree.

Pre-existing state preserved: untracked `tauri-svelte-preview/node_modules` was present before implementation and was not modified or removed.
