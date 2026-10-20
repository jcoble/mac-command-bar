# Per-project session list: read-only recon

## Scope and labels

- **[Verified]** This report covers the current path in this worktree only. Each code claim below has a `file:line` receipt.
- **[Verified source comment]** A number written in a source comment is reported as a source-documented sample, not as a measurement re-run during this recon.
- **[Assumed/unverified]** No elapsed-time split is assumed. Where the code gives operation sizes but no timer, the report says so rather than ranking the costs.

## Executive finding

- **[Verified]** History creates a set of the expanded project's row keys, checks the frontend-held cache, and calls `refresh(keys)` only when those keys are not all held (`tauri-svelte-preview/src/lib/shell/panels/history/HistoryPanel.svelte:174-186`).
- **[Verified]** On that cache-miss path, `refresh(keys)` still invokes the provider list with no project argument, builds one library from all owned/current/provider records, and filters to `keys` only after the full build (`tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:120-128`). The page wires the provider callback to the unscoped Tauri list command (`tauri-svelte-preview/src/routes/next/+page.svelte:446-456`).
- **[Verified]** The Tauri command accepts no path or project id and directly runs the core scanner on a blocking task (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`). The command is registered in the app invoke handler (`tauri-svelte-preview/src-tauri/src/main.rs:6113-6121`).
- **[Verified]** The scanner returns an in-memory `Vec<AgentSessionRecord>` and the current Tauri call path has no store step: the command imports and calls `scan_sessions` directly (`tauri-svelte-preview/src-tauri/src/main.rs:18-19`, `tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `core/src/scanners/sessions.rs:387-507`).
- **[Verified]** The current code gives fixed read sizes and operation order, but no elapsed-time instrumentation or stage timings in this refresh path (`core/src/scanners/sessions.rs:387-507`, `tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`). Any claim that walking, reading, or parsing is the dominant wall-clock cost is therefore **[Assumed/unverified]**.

## 1. Full current path: frontend → Tauri → core

### Project expansion and frontend refresh

1. **[Verified]** The panel obtains the shared session-library host and derives its collapsed summary from the rail's owned and available records (`tauri-svelte-preview/src/lib/shell/panels/history/HistoryPanel.svelte:89-102`).
2. **[Verified]** `toggleProject(project)` derives `keys` from every row in the project's worktrees, checks which matching records are already held, releases a partial cache, and calls `host.service.refresh(keys)` when the set is not complete (`tauri-svelte-preview/src/lib/shell/panels/history/HistoryPanel.svelte:160-186`). The same expand also requests repository checkouts with `[project.path]` (`tauri-svelte-preview/src/lib/shell/panels/history/HistoryPanel.svelte:184-187`).
3. **[Verified]** The service's `refresh(keys?)` reads owned records, reads the optional current-provider source, awaits `listProviderSessions()` if present, calls `buildSessionLibrary` on the complete arrays, then applies the key filter. `keys` does not reach the provider callback (`tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:27-39`, `tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:120-128`).
4. **[Verified]** In the page's service wiring, `getOwnedSessions` reads `rail.owned` and `listProviderSessions` calls `listAgentSessionsFromTauri()` first, with the local bridge as fallback; `getAvailableSessions` is not supplied (`tauri-svelte-preview/src/routes/next/+page.svelte:444-456`). Thus `current` is empty for this service instance and the provider callback supplies the fresh unscoped snapshot (`tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:121-124`).

### Tauri transport and registration

5. **[Verified]** `listAgentSessionsFromTauri()` returns `null` outside Tauri and otherwise invokes the command name `list_agent_sessions` with no arguments, typed as `AgentSession[]` (`tauri-svelte-preview/src/lib/tauriSource.ts:1350-1357`).
6. **[Verified]** The native command is `async fn list_agent_sessions() -> Result<Vec<AgentSessionRecord>, String>`. It runs `scan_sessions` through `spawn_blocking` and only maps task-join failure to a string; there is no project input or database call (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`).
7. **[Verified]** The command is included in `tauri::generate_handler!` alongside the other invoke commands (`tauri-svelte-preview/src-tauri/src/main.rs:6113-6121`).
8. **[Verified]** The Tauri crate imports `scan_sessions` and `AgentSessionRecord` from `mcb_core` (`tauri-svelte-preview/src-tauri/src/main.rs:18-19`); its manifest points at the standalone crate with a direct path dependency (`tauri-svelte-preview/src-tauri/Cargo.toml:11-19`, `core/Cargo.toml:1-4`).

### Core call and alternate bridge

9. **[Verified]** The Tauri path calls `core::scanners::sessions::scan_sessions` directly. The core dispatcher also maps the separate protocol action `"scan.sessions"` to `scan_sessions_action`, which calls the same function and wraps `{sessions, count}`; that dispatcher path is not used by the Tauri command (`core/src/dispatcher.rs:9-16`, `core/src/dispatcher.rs:128-134`).
10. **[Verified]** If the page is not running under Tauri, or the Tauri result is unavailable, it falls back to the local source bridge action `agent-sessions` (`tauri-svelte-preview/src/routes/next/+page.svelte:449-454`, `tauri-svelte-preview/src/lib/tauriSource.ts:1511-1513`). The page's initial rail scan uses the same Tauri command first and stores its all-session result in the available rail (`tauri-svelte-preview/src/routes/next/+page.svelte:691-710`).

## 2. Scanner enumeration, storage, and what “rebuild” re-reads

### Source enumeration

- **[Verified]** `scan_sessions()` starts from `HOME` and returns an empty vector if it is unavailable (`core/src/scanners/sessions.rs:387-390`).
- **[Verified]** Codex session files are recursively enumerated under `$HOME/.codex/sessions`; each candidate gets a bounded metadata-head probe to classify Codex-spawned subagents (`core/src/scanners/sessions.rs:400-408`, `core/src/scanners/sessions.rs:877-901`). Archived files under `$HOME/.codex/archived_sessions` are also recursively enumerated for subagent markers (`core/src/scanners/sessions.rs:411-420`).
- **[Verified]** The Codex index at `$HOME/.codex/session_index.jsonl` is read as a complete string and parsed; rows for subagents and rows with no matching rollout file are removed before rollout selection (`core/src/scanners/sessions.rs:422-428`).
- **[Verified]** Codex rollout files are sorted by modification time and the newest `CODEX_SESSION_FILE_LIMIT` files are selected, with an older file additionally selected when its filename carries an indexed thread id (`core/src/scanners/sessions.rs:430-440`, `core/src/scanners/sessions.rs:323-341`). The limit constant is 512 (`core/src/scanners/sessions.rs:17-27`).
- **[Verified]** cmux hook-session files are enumerated only from the immediate `$HOME/.cmuxterm` directory, selecting regular files ending in `-hook-sessions.json` (`core/src/scanners/sessions.rs:473-477`, `core/src/scanners/sessions.rs:1998-2024`).
- **[Verified]** Claude transcripts are recursively enumerated under `$HOME/.claude/projects`, filtered to remove subagent and agent-launched files, sorted by modification time, and limited to the newest 512 files (`core/src/scanners/sessions.rs:480-500`, `core/src/scanners/sessions.rs:9-16`).
- **[Verified]** After the source-specific reads, records are merged, empty/non-speaking records are removed, sorted by last activity, capped at `CODEX_SESSION_FILE_LIMIT + CLAUDE_SESSION_FILE_LIMIT + CMUX_SESSION_RESULT_HEADROOM` (1280), and enriched with derived metadata (`core/src/scanners/sessions.rs:503-507`, `core/src/scanners/sessions.rs:31-43`).

### Bounded reads and parsing

- **[Verified]** Codex uses a 256 KiB head and 256 KiB tail window (`CODEX_SESSION_HEAD_BYTES` and `CODEX_SESSION_TAIL_BYTES`) (`core/src/scanners/sessions.rs:22-30`). Each selected rollout is read with the combined helper and parsed as Codex JSONL (`core/src/scanners/sessions.rs:436-442`). The tail is then read again to replace the latest spoken turns when the tail contains any (`core/src/scanners/sessions.rs:444-467`).
- **[Verified]** The combined helper reads the whole file when its actual length is at most 512 KiB; otherwise it reads one 256 KiB head and one 256 KiB tail and joins them (`core/src/scanners/sessions.rs:2053-2077`). Therefore “256 KiB head + tail per session file” is the upper bounded Codex window for selected files, not an unconditional whole-file read of every file.
- **[Verified]** Claude uses a 64 KiB head probe to reject agent-launched transcripts, then reads a 256 KiB tail for each retained file and parses that bounded text as Claude JSONL (`core/src/scanners/sessions.rs:1388-1408`, `core/src/scanners/sessions.rs:488-500`, `core/src/scanners/sessions.rs:9-16`).
- **[Verified]** cmux retained files are read with `fs::read_to_string`, so that source is not windowed by the session scanner (`core/src/scanners/sessions.rs:473-477`).
- **[Verified]** Project-root derivation is a post-parse enrichment step. For each distinct `project_path`, a small map avoids repeating the lookup; `git_project_root` walks ancestors, checks `.git` directories, and reads `.git` files for worktrees (`core/src/scanners/sessions.rs:277-301`, `core/src/scanners/sessions.rs:255-275`).

### What is stored where

- **[Verified]** The scanner's output type is `AgentSessionRecord`, serialized with camel-case field names; it contains provider/id/title, project path/root, activity/model, resume commands, optional log and hints, bounded message/turn data, and other scanner-derived fields (`core/src/scanners/sessions.rs:45-121`). The Tauri command returns this vector directly; it does not write it to SQLite (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`).
- **[Verified]** The frontend keeps the provider/owned result in an in-memory `Map` named `held`; `refresh` inserts selected records and `release` removes keys (`tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:115-133`). This is the cache involved in the project expand path, not the SQLite store.
- **[Verified]** The durable SQLite store declares schema version 6 (`core/src/session_store.rs:1-8`). Its initial schema has a `sessions` table keyed by `owned_id` and an `events` table keyed by `(owned_id, seq)` (`core/src/session_store.rs:204-242`). The `events.item_id` column is a generated virtual column extracted from event JSON, with an index on `(owned_id, kind, item_id, seq)` (`core/src/session_store.rs:24-29`, `core/src/session_store.rs:965-982`).
- **[Verified]** A durable session row stores `owned_id`, native/provider/model/effort, cwd/worktree/branch, title/title source/project, state/suspended flags, timestamps, and `extra_json` (`core/src/session_store.rs:107-128`). The app opens this store at its app-data `sessions.db` path and validates the manager's store-backed session list during Tauri setup (`tauri-svelte-preview/src-tauri/src/main.rs:5974-5995`).

### Concrete meaning of “rebuild” today

- **[Verified]** For a provider cache miss, “rebuild” means: walk the source trees; probe candidate heads; read the bounded Codex/Claude windows or full cmux hook files; parse each source format; merge records; filter, sort, truncate, derive git/project metadata; then build the frontend library and finally filter it to the requested keys (`core/src/scanners/sessions.rs:387-507`, `tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:120-128`).
- **[Verified]** It does not mean rebuilding a SQLite snapshot: the provider scan has no `SessionStore` call, and the command returns its vector directly (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `core/src/scanners/sessions.rs:387-507`).

## 3. What a per-project command would receive and return at the current frontend boundary

### Input already available at expansion time

- **[Verified]** The project group has `key`, `name`, `path`, count/activity, and worktrees. The view model seeds/creates the group path from the repository root, and the group key is `project:${path}` (`tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:296-321`, `tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:324-379`).
- **[Verified]** The expansion handler therefore has the exact filesystem path in `project.path`; it passes that path to the checkout command as `[project.path]`, while its current session refresh passes only the row-key set (`tauri-svelte-preview/src/lib/shell/panels/history/HistoryPanel.svelte:174-187`). There is no separate numeric/project-id input in this view-model path (`tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:324-379`).
- **[Verified]** The current native command has no input parameter and the current frontend wrapper sends no arguments (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `tauri-svelte-preview/src/lib/tauriSource.ts:1350-1357`).

### Output shape currently consumed

- **[Verified]** The native wire result is `AgentSession[]` on the frontend, corresponding to `Vec<AgentSessionRecord>` with camel-case serialization (`tauri-svelte-preview/src/lib/tauriSource.ts:272-330`, `core/src/scanners/sessions.rs:45-121`, `tauri-svelte-preview/src/lib/tauriSource.ts:1350-1357`). Its exact current shape is:

  ```text
  AgentSession {
    provider: string
    id: string
    title: string
    description?: string | null
    model: string | null
    projectPath: string | null
    lastActivity: string | null
    resumeCommands: string[]
    logPath?: string | null
    branchHint?: string | null
    taskId?: string | null
    pullRequestHint?: string | null
    sourceLabel?: string | null
    messageCount?: number | null
    latestTurnPreview?: string | null
    latestTurns?: AgentSessionTurn[]
    projectRoot?: string | null
  }

  AgentSessionTurn { speaker: 'user' | 'agent'; text: string }
  ```

- **[Verified]** The session-library service maps each `AgentSession` into a `SessionLibraryRecord` with the provider/native identity, canonical cwd, title/description, project path/root, state/activity/message/log fields, turns, and the original `available` session (`tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryModel.ts:31-75`, `tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryModel.ts:214-241`).
- **[Verified]** The History load outcome exposes those mapped records as `records: readonly SessionLibraryRecord[]`, plus checkout data, state, and missing keys (`tauri-svelte-preview/src/lib/shell/history/sessionHistoryLoad.ts:6-15`, `tauri-svelte-preview/src/lib/shell/history/sessionHistoryLoad.ts:35-42`).

### Existing registration surface

- **[Verified]** A native per-project sibling command would be registered in the same `src-tauri/src/main.rs` command section as `list_agent_sessions` and added to the same `tauri::generate_handler!` list (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `tauri-svelte-preview/src-tauri/src/main.rs:6113-6121`). The frontend command wrapper surface is `tauri-svelte-preview/src/lib/tauriSource.ts` next to `listAgentSessionsFromTauri` (`tauri-svelte-preview/src/lib/tauriSource.ts:1350-1357`).

## 4. Does the store support one-project upsert or only a whole snapshot?

- **[Verified]** The durable store supports an individual row upsert: `SessionStore::upsert_session(&SessionRow)` locks the connection and calls `upsert_session_on` (`core/src/session_store.rs:403-407`). The SQL inserts one row and updates that same row on `ON CONFLICT(owned_id)` (`core/src/session_store.rs:1038-1084`).
- **[Verified]** The runtime manager uses that per-row method when persisting one managed session (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:3087-3094`). It also has a transaction form that upserts one session and optionally appends one event (`core/src/session_store.rs:409-451`).
- **[Verified]** The store's list method is an all-session query ordered by activity; it has no project/cwd filter (`core/src/session_store.rs:469-485`). The manager's list method delegates to that all-row list and overlays live runtime state (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2016-2045`).
- **[Verified]** These store methods concern app-owned ACP sessions keyed by `owned_id`, not the provider filesystem scan. The scanner command directly returns `scan_sessions()`; no store operation appears in that call path (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `core/src/scanners/sessions.rs:387-507`).
- **[Verified]** Therefore the current backend has neither a project-scoped provider-scan upsert nor a database “whole provider snapshot rebuild.” It has per-owned-session durable upsert, while provider history is rebuilt as an ephemeral vector and cached in the frontend (`core/src/session_store.rs:403-407`, `core/src/scanners/sessions.rs:387-507`, `tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryService.ts:115-128`).

## 5. Refresh cost: verified work versus unknown wall-clock share

| Stage | Verified operation | Timing status |
|---|---|---|
| Directory enumeration | Recursive `read_dir` walks for Codex and Claude `.jsonl` trees; immediate `read_dir` for cmux hook files (`core/src/scanners/sessions.rs:1980-2024`, `core/src/scanners/sessions.rs:402-481`). | **[Verified operation; wall-clock unmeasured]** |
| Candidate probes | Codex thread marker: 64 KiB head; Claude agent-launched probe: 64 KiB head (`core/src/scanners/sessions.rs:26-30`, `core/src/scanners/sessions.rs:877-881`, `core/src/scanners/sessions.rs:1393-1402`). | **[Verified fixed bounds; wall-clock unmeasured]** |
| Codex transcript reads | Selected files get a 256 KiB head plus 256 KiB tail, then a separate 256 KiB tail read for spoken-turn replacement (`core/src/scanners/sessions.rs:436-467`). Files no larger than the combined 512 KiB are read once as a whole by the helper (`core/src/scanners/sessions.rs:2061-2077`). | **[Verified fixed bounds; wall-clock unmeasured]** |
| Claude transcript reads | Retained files get a 256 KiB tail (`core/src/scanners/sessions.rs:9-16`, `core/src/scanners/sessions.rs:488-500`, `core/src/scanners/sessions.rs:2026-2043`). | **[Verified fixed bound; wall-clock unmeasured]** |
| cmux reads | Each retained hook-session JSON file is read with `fs::read_to_string` (`core/src/scanners/sessions.rs:473-477`). | **[Verified full-file operation; wall-clock unmeasured]** |
| Parsing and reduction | Codex, cmux, and Claude parsers run after the reads; records are merged, filtered, sorted, truncated, and enriched (`core/src/scanners/sessions.rs:442-507`). | **[Verified operations; wall-clock unmeasured]** |
| Project metadata | Each distinct `project_path` is mapped once to a git root by walking ancestors and reading `.git` markers (`core/src/scanners/sessions.rs:277-301`, `core/src/scanners/sessions.rs:255-275`). | **[Verified operation; wall-clock unmeasured]** |
| SQLite writes | The refresh command calls `scan_sessions` directly and never opens or writes `SessionStore`; this path performs **zero verified SQLite writes** (`tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`, `core/src/scanners/sessions.rs:387-507`). | **[Verified zero on this path]** |

### Numbers documented in source comments (not re-measured here)

- **[Verified source comment]** The Codex 256 KiB head reached the first typed prompt in 59 of 67 top-level sessions in the source author's sample (`core/src/scanners/sessions.rs:22-27`).
- **[Verified source comment]** The 64 KiB Codex metadata probe had a 27 KiB median and 44 KiB largest sample across 687 rollout files (`core/src/scanners/sessions.rs:28-30`).
- **[Verified source comment]** The Claude 64 KiB probe reached its entrypoint in 1050 of 1091 transcripts in the source author's sample (`core/src/scanners/sessions.rs:12-16`).
- **[Verified source comment]** The source comments record 266 of 342 index rows without rollout files and 47 of 179 Codex sessions with no spoken turn in the tail (`core/src/scanners/sessions.rs:362-371`, `core/src/scanners/sessions.rs:444-452`).

### Timing conclusion

- **[Verified]** Refresh work includes directory walks, bounded/full reads, parsing/reduction, and git-root metadata; SQLite writes are not part of this path (`core/src/scanners/sessions.rs:387-507`, `tauri-svelte-preview/src-tauri/src/main.rs:1750-1755`).
- **[Assumed/unverified]** The relative percentage spent in walking versus reads versus parsing is not established by the current source or by this recon. A timing claim about the dominant stage would require an instrumented run; none is asserted here.

## Verification boundary

- **[Verified command receipt]** The worktree was clean before this report was created (`git status --short` returned no lines).
- **[Verified command receipt]** No required command failed. No Cargo build or test was run because this recon only needed source inspection and the request explicitly said a build should not be necessary.
