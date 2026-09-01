# Per-project session list — expand one project without scanning the world

2026-08-20. Owner requirement: expanding a project in the History panel must
read only that project's sessions. Today a cache miss runs the full unscoped
scan (every Codex rollout, every Claude project, cmux) and filters at the end.

## Recon receipts (from recon-project-list.md, all verified)

- Cache-miss path: `HistoryPanel.svelte:174-186` → `sessionLibraryService.ts:120-128`
  (`refresh(keys)` — keys never reach the provider) → `+page.svelte:446-456` →
  `tauriSource.ts:1350-1357` (`list_agent_sessions`, no args) →
  `src-tauri/src/main.rs:1750-1755` → `core/src/scanners/sessions.rs:387-507`.
- `project.path` (repo root; group key is `project:${path}`) is in hand at
  expand time (`HistoryPanel.svelte:174-187`, `sessionHistoryViewModel.ts:296-321`).
- Claude: project path decodes from the DIRECTORY name
  (`sessions.rs:480-500`, `decode_claude_project_dir`) — filterable with no file IO.
- Codex: cwd lives only inside the file (`session_meta` payload,
  `sessions.rs:639-656`); index rows carry no folder (`sessions.rs:362-371`).
  Every candidate already gets a 64 KiB head probe for subagent classification
  (`sessions.rs:877-901`) — median head content 27 KiB (`sessions.rs:28-30`).
- No SQLite on this path; result is an ephemeral `Vec<AgentSessionRecord>`
  cached frontend-side in `held` (`sessionLibraryService.ts:115-133`).

## Contract

### Core (`core/src/scanners/sessions.rs`)

New public function, same record type, no change to `scan_sessions()`:

```rust
pub fn scan_sessions_for_project(project_root: &str) -> Vec<AgentSessionRecord>
```

Narrowing rules — reuse the existing helpers, do not fork the pipeline:

1. Claude: `read_dir` the projects dir, decode each directory name, keep a
   directory when its decoded path's `git_project_root` (or the decoded path
   itself) equals `project_root`. Only matching dirs are enumerated/read.
2. Codex: enumerate candidates as today; extend the existing 64 KiB head probe
   to also extract `cwd` (one read, one helper — e.g.
   `codex_head_meta(path) -> (is_subagent, Option<cwd>)`). Drop a file before
   its 256 KiB window reads and parse when `git_project_root(cwd)` (memoized
   map, as `sessions.rs:277-301` does) does not equal `project_root`. A head
   with no readable cwd: keep the file (parse decides), do not guess.
3. cmux: read as today (tiny), filter parsed records by resolved root.
4. Then the existing merge/filter/sort/cap/enrich steps, and a final guard
   filter to `project_root`.

### Tauri (`tauri-svelte-preview/src-tauri/src/main.rs`)

`list_agent_sessions_for_project(project_path: String)` next to :1750, same
`spawn_blocking` shape, registered in the `generate_handler!` list
(:6113-6121). Returns `Vec<AgentSessionRecord>`.

### Frontend

- `tauriSource.ts`: `listAgentSessionsForProjectFromTauri(path: string)` next
  to :1350, invoking the new command.
- `sessionLibraryService.ts`: `refresh(keys?, scope?: { projectPath: string })`;
  the provider callback type gains the optional arg
  `listProviderSessions?(projectPath?: string)`. No other behaviour change —
  `held` insert and key filter stay as they are.
- `+page.svelte:446-456`: when the callback receives a path, call the scoped
  Tauri wrapper; unscoped otherwise. The initial rail scan (:691-710) stays
  unscoped and untouched.
- `HistoryPanel.svelte:174-186`: pass `{ projectPath: project.path }` to
  `refresh`.

## Test-first

Write these failing Rust tests first (temp-dir fixtures, as existing scanner
tests do): `scan_for_project_returns_only_matching_records`,
`claude_project_dirs_filter_to_requested_root`,
`codex_head_cwd_filters_rollouts_before_window_reads` (assert the non-matching
rollout's windows are never read — fixture with a poisoned/oversized body works).

## Verification, verbatim

- `cd core && cargo test scan_for_project` — and the full `cargo test` green.
- Controller runs frontend gates after merge in the main worktree:
  `npx tsc --noEmit` + `pnpm run check:svelte` ending
  "Files the /next shell owns: 0 error(s)".

## Done means

- Expanding a project on a cache miss invokes the scoped command only (log or
  breakpoint receipt), and the unscoped scan still serves the initial rail.
- All three named tests pass; no existing test broken.
- No new abstractions, no config flags, no store/SQLite involvement.

Expected diff: ~250-350 lines including tests, files named above only.
KISS/YAGNI: smallest diff that makes the named tests pass; a second design
with fewer moving parts wins; no code for hypothetical futures; needing ~2x
the estimate or a new file means STOP and report back instead.
