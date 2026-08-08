# TSK-808 Resources and Usage Rework Receipt

Status: complete — lane checks green; unrelated native bridge test remains environment-blocked

## Scope

- Resources data and owned-process attribution
- Usage ingestion, indexing, and quota presentation data
- Resources, Space, Usage, and Stats & Usage surfaces
- Owned Rust, Svelte, TypeScript, and tests only

## Evidence log

- 2026-08-08: Receipt created before repository changes. (verified)
- 2026-08-08: Local sources are present at `~/.claude/projects/<project-slug>/*.jsonl` (1,553 files found) and `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (388 files found). The first source records assistant message usage fields; the second records `event_msg`/`token_count` entries with `last_token_usage` and, when available, provider rate-limit windows. (verified)
- 2026-08-08: Current resource plumbing joins only exact root PIDs, which leaves descendants and every unrelated system process in the snapshot as `external`; current history refresh returns `0` without indexing a source. (verified)
- 2026-08-08: Resource scanner now traverses descendants from app-owned terminal/agent roots, drops unowned/system rows, and carries project/workspace/session labels through the native snapshot. Fixture coverage proves a sibling daemon is absent while all owned descendants remain. (verified)
- 2026-08-08: Usage source parsing now recognizes the real local transcript layouts and normalizes explicit input/output/cache/reasoning fields into SQLite with opaque source cursors; fixture parser and incremental-index tests pass. (verified)
- 2026-08-08: Incremental tails now recover the source session header and keep source-scoped event IDs stable across appended chunks; the Codex fixture proves two chunks remain one session with two non-duplicate events. (verified)
- 2026-08-08: Focused native tests pass: `cargo test ... usage` = 12 passed, 2 ignored; core resource scanner tests = 3 passed. The first combined filter attempt used an unsupported multi-filter argument and was corrected to separate `usage` and `resource` runs. (verified)
- 2026-08-08: An earlier `pnpm check:svelte` run surfaced the shared browser font-floor backlog; a final rerun now exits 0 with 0 errors and 0 warnings for the owned `/next` shell files. The tool still reports 16 old-shell errors outside this lane's owned set. (verified)
- 2026-08-08: Real local-source smoke test indexed 3 files before the first non-empty source and produced 5 events, input 15, output 751, cache-read 176256, cache-write 31290, reasoning 0. This proves non-zero data from the machine's local transcript stores; the full refresh remains cursor-driven and can continue across all discovered files. (verified)
- 2026-08-08: Live local quota smoke read the newest available provider rate-limit record as account `plus`, with a 5-hour window at 3% consumed and reset epoch `1786774261`; no secondary window was present in the newest record, so the UI renders the available window and does not invent a weekly zero. (verified)
- 2026-08-08: No Tauri command names were added or renamed. Existing resource and usage registrations in `main.rs` remain the capability surface; only `refresh_usage_history` changed from a no-op implementation to real ingestion. (verified)
- 2026-08-08: Full requested native command ran 239 passed, 3 ignored, 1 failed. The only failure is the pre-existing `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint`, which cannot bind its native bridge in this sandbox (`Operation not permitted`); resource/usage tests all pass, including the incremental tail-context fixture. (partial / environment blocker)
- 2026-08-08: `pnpm check` passed, `pnpm build` passed, and the owned resource/usage/space view-model scripts each passed. (verified)
- 2026-08-08: Final `pnpm build` passed after the last resources/usage surface changes; `git diff --check` is clean. (verified)

## Controller review addendum (2026-08-08)

- Boundary disclosure the lane omitted: `agent_conversation/manager.rs` (+32) adds a read-only
  `resource_roots()` snapshot (pid, owned id, provider, cwd) and `providers/acp.rs` (+9) exposes
  `process_id()`. In scope — the spec directs resources to the registries' root pids. Verified.
- Controller fix: `WorkspaceSpaceWorkspace.svelte` `selectAll()` mapped over `selectedEntries`
  (already-selected, so a no-op from empty); now maps `selectableEntries`. Found by checkpoint review.
- Controller re-ran verification: usage 12 passed, core resources scanners 3 passed, resource/usage/space
  view-model scripts passed.
