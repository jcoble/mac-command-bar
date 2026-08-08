# TSK-808 Work Package A10 — Resources, Space, and Usage Receipt

Status: implementation in progress.

This receipt is append-only during the A10 lane. Claims are labelled **verified** or **assumed**. No commit will be created.

## Scope and constraints

- Work Package A10 only: resources, space, and usage.
- Do not edit `centerDock.ts`, `layoutStorage.ts`, `sidebarViews.ts`, `ShellFrame.svelte`, or `ShellSidebar.svelte`.
- Re-verify all anchors against the current tree before editing.
- Test-first; run one build/test process at a time.

## Initial receipt

- **Verified:** receipt file created before repository edits.
- **Verified:** the current branch contains the later handoff, assistance, and browser commits named by the request. `+page.svelte`, `main.rs`, the conversation store/service, and `ownedSessions` were re-opened before editing; their newer handoff wiring is retained.
- **Verified:** the forbidden layout and shell frame/sidebar files are unchanged.
- **Assumed:** the raw dispatch log is the available authority. It is a 31,066-line discovery transcript and does not contain the promised standalone packet file or a separately searchable erratum block; the A10 contract below is therefore reconstructed from the raw log's plan excerpts and the current tree, with this evidence limitation called out rather than invented away.
- **Verified adaptation:** the current `TerminalSessionInfo` still lacked the requested additive ownership field, so the resource lane exposes the existing terminal request ownership rather than duplicating the newer conversation handoff wiring. The current provider conversation token fields are not treated as quota data.

## Test-first failures

- **Verified failing test:** `RUST_TEST_THREADS=1 cargo test --manifest-path ../core/Cargo.toml resources -- --nocapture` failed before implementation because the new resource and disk parser symbols did not exist. The compiler reported missing `parse_ps_snapshot`, `join_resource_identities`, `ResourceProcess`, `ProcessOwner`, `DiskScanRoot`, and `scan_disk_roots`; this is the expected red starting point.
- **Verified failing tests:** the four new TypeScript contracts each failed before implementation with `ERR_MODULE_NOT_FOUND` for `resourceViewModel.ts`, `workspaceSpaceViewModel.ts`, `usageCurrent.ts`, and `usageAnalytics.ts`.
- **Verified green:** the focused core resource parser/join and disk protection tests now pass (2 resource tests; 2 disk tests). The scan uses one bounded `ps` read and one structured listener read, joins by PID once, and defaults unknown processes to External.
- **Verified green:** `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage -- --nocapture` passes 4 focused tests for provider quota unavailability, SQLite dedupe/rollups and DB-side paging SQL, and cursor reset after truncation.
- **Assumed implementation constraint:** the checkout has no SQLite Rust crate and dependency/network resolution is unavailable. The usage history keeps SQLite and DB-side SQL, but invokes the installed local `sqlite3` binary through `src-tauri/src/usage_db.rs`; `MCB_SQLITE_BIN` can point tests at an explicit binary. No provider payload, credentials, prompts, or raw paths are stored.
- **Verified green:** the four new frontend contracts pass: resource ownership/stop rules, bounded space protection labels, provider quota unavailability, and SQL-shaped usage breakdown paging.
- **Verified green:** `pnpm check:svelte` reports 0 errors and 0 warnings in files owned by `/next`; it also reports the expected 16 pre-existing errors outside that shell.
- **Verified green:** `pnpm test:tauri-source` passes.
- **Verified failure outside A10:** `pnpm test:source-code-lens-keys` fails in the existing CodeLens contract expecting `if (!nativeCsharpLanguageClient && onReferenceCountLookup)`; no A10 file touched that implementation.
- **Verified failure outside A10:** `pnpm test:source-ui` fails in the existing browser style contract (`Missing style block for .source-browser-stack`); no A10 file touched that component.
- **Verified green:** `pnpm test:csharp-language-client` passes.
- **Accepted environment failure:** the LSP suite ran 57 tests successfully and the single known sandbox failure is exactly `Could not bind native C# bridge: Operation not permitted (os error 1)` in `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint`.
- **Verified green:** `MCB_SQL_TRACE=1 RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage -- --nocapture` passes all 5 usage tests. The trace tail includes one `SELECT` summary, one SQL view daily query, a grouped/windowed breakdown contract, and versioned-rate lookup; no frontend-side grouping or paging is used.
- **Verified green:** `pnpm check` exits 0.
- **Verified green:** final `pnpm check:svelte` exits 0 with 0 `/next` errors and 0 `/next` warnings; the output still identifies 16 old-shell errors outside this gate.
- **Verified green:** final `pnpm build` exits 0. The build output contains existing accessibility/unused-selector warnings from old-shell files but no build error.
- **Verified green:** the five prescribed assistance contract tests pass from the preview package (`assistanceRecipes`, `assistanceContext`, `assistanceService`, `assistanceAudit`, and `assistanceUiContract`).

## Final packet application and safety tightening

- **Verified:** the resource lane still performs exactly one bounded `ps` inventory and one structured `lsof` listener inventory, joins by PID once in Rust, exposes App/OwnedSession/LanguageServer/ProviderSidecar/Playwright/External owners, and refuses an unknown or external owner. The newer handoff, assistance, browser, conversation, and layout work remains untouched.
- **Verified:** `TerminalSessionInfo.owned_id` is additive and comes from the existing terminal start request. No provider process is guessed from a command name, path, or token field; identities not proven by a registry remain External.
- **Verified:** disk scans accept explicit roots only, clamp depth to 8 and entries to 10,000, reject `/` and the home folder, use a deterministic stable ID, and report bounded totals plus top-level items. `SafeCandidate` is the only reclaimable label; cleanup rechecks the stable ID and byte count, routes worktrees through `remove_project_worktree_sync(..., false)`, refuses symlinks and protected rows, and now measures post-action bytes for the receipt.
- **Verified:** resource stop checks the generation before and after its fresh snapshot, then rechecks PID, PGID, owner ID, root, and owner before sending SIGTERM to the proven process itself. Group identity is recorded but an unowned sibling is not signalled.
- **Verified:** current usage is provider-authored-or-unavailable only. Token counters never become quota windows; the state records provider/account/instance, semantics, reset, capture time, source/version, and an honest unavailable reason.
- **Verified:** usage history uses the embedded SQLite migration at `app_data_dir()/usage-history.sqlite3` (or test-only `MAC_COMMAND_BAR_USAGE_DB`), integer micros, unique provider/instance/source/event IDs, indexes, transactional cursor/event/rollup updates, truncation/rotation reset, opaque source keys, versioned rate lookup, and SQL-only summary/breakdown/daily filters, grouping, sorting, and paging. Daily rollups include event, session, turn, workflow, token, and estimated-cost counts. SQL trace redacts string literals; the query-plan test proves the provider/model covering index is used.
- **Verified:** compact resource/usage cards and their full workspaces share one service/store and refresh locks; mounting the components does not start a scan or tail a source. Full native center registration and rebuilt-native proof remain deferred under the explicit native-app stop condition.

## Final verification receipts (serial, 2026-08-08)

- **Verified green:** `RUST_TEST_THREADS=1 cargo test --manifest-path ../core/Cargo.toml resources -- --nocapture` from `tauri-svelte-preview` — 2 passed, 0 failed; the other filtered targets reported 0 tests.
- **Verified green:** `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml resources -- --nocapture` — 0 tests, 0 failed, 237 filtered out; Rust compilation completed successfully.
- **Verified green:** `MCB_SQL_TRACE=1 RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage -- --nocapture` — 8 passed, 0 failed, 229 filtered out. The trace includes transactional migration, `INSERT OR IGNORE`, set-based rollup rebuild, summary aggregation, filtered/windowed breakdown, daily view paging, versioned-rate lookup, `EXPLAIN QUERY PLAN`, and redacted literals.
- **Accepted environment failure:** `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml lsp -- --nocapture` ran 58 tests: 57 passed and the one documented sandbox failure was exactly `Could not bind native C# bridge: Operation not permitted (os error 1)` in `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint`.
- **Verified green:** `pnpm test:csharp-language-client` — contract tests passed.
- **Verified failure outside A10:** `pnpm test:source-code-lens-keys` exits 1 because the existing contract still expects `/if (!nativeCsharpLanguageClient && onReferenceCountLookup)[\s\S]*?registerSourceCodeLensReferenceCommand/`; no A10 file changes that implementation.
- **Verified failure outside A10:** `pnpm test:source-ui` exits 1 with `Missing style block for .source-browser-stack`; no A10 file changes that component.
- **Verified green:** `pnpm test:tauri-source` exits 0.
- **Verified green:** the four A10 frontend contracts pass serially: `resourceViewModel`, `workspaceSpace`, `usageCurrent`, and `usageAnalytics`.
- **Verified green:** all five prescribed assistance contracts pass serially: `assistanceRecipes`, `assistanceContext`, `assistanceService`, `assistanceAudit`, and `assistanceUiContract`.
- **Verified green:** final `pnpm check:svelte` — `/next` owns 0 errors and 0 warnings; the output still reports 16 old-shell errors outside this gate.
- **Verified green:** final `pnpm check` exits 0.
- **Verified green:** final `pnpm build` exits 0; tail is `✓ built in 12.08s`, `Wrote site to "build"`, `✔ done`. Existing old-shell accessibility, unused-selector, chunk-size, dynamic-import, and plugin-timing warnings remain outside this A10 gate.
- **Verified green:** final `git diff --check` exits 0. The five forbidden layout/shell files remain unchanged.

## Changed-file inventory (line anchors)

Every file changed by this lane is listed below with the current line anchors for its implementation or test surface.

- `core/src/scanners/mod.rs:1-5` — disk and resource scanner modules.
- `core/src/scanners/disk.rs:8-221` — disk kinds/protection, bounded scan, stable IDs, cleanup policy, tests.
- `core/src/scanners/resources.rs:16-352` — owner model, bounded `ps`/`lsof` parsing and join, action validation, tests.
- `tauri-svelte-preview/src-tauri/src/main.rs:26-35,263,1241-1281,3143-3149,5322-5323,5399-5412` — modules, capabilities, managed state, worktree safety seam, command registrations.
- `tauri-svelte-preview/src-tauri/src/terminal.rs:50-70,185-201` — additive terminal kind/owned identity projection.
- `tauri-svelte-preview/src-tauri/src/resources.rs:17-404` — registry, resource snapshot/disk/cleanup/stop commands, explicit-root validation, deferred lifecycle responses.
- `tauri-svelte-preview/src-tauri/src/usage_current.rs:5-124` — provider usage state/reader, unavailable normalization, test.
- `tauri-svelte-preview/src-tauri/src/usage_db.rs:11-487` — normalized event/cursor/rate/rollup types, SQLite transactions and SQL endpoints, redacted trace, plan/rate/dedupe tests.
- `tauri-svelte-preview/src-tauri/src/usage_history.rs:6-58` — app-data/test database path and Tauri history endpoints.
- `tauri-svelte-preview/src-tauri/src/usage_indexer.rs:6-35` — event and incremental JSONL ingestion.
- `tauri-svelte-preview/src-tauri/src/usage_sources.rs:8-149` — tail-only reads, stable file identity, opaque source keys, rotation tests.
- `tauri-svelte-preview/migrations/0001_usage_history.sql:1-82` — usage events/cursors/rates/daily rollup/view schema and indexes.
- `tauri-svelte-preview/src/lib/tauriSource.ts:69-100,479-557` — terminal/resource/usage types and Tauri wrappers.
- `tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte:20-21,158-183` — one mounted resource popover and one mounted usage popover.
- `tauri-svelte-preview/src/lib/shell/resources/ResourcePopover.svelte:2-75` — compact resource card and shared full resource/space views.
- `tauri-svelte-preview/src/lib/shell/resources/ResourcesWorkspace.svelte:2-36` — full process/owner/port view.
- `tauri-svelte-preview/src/lib/shell/resources/WorkspaceSpaceWorkspace.svelte:2-88` — explicit-root bounded scan, protection labels, confirmation, cleanup receipt.
- `tauri-svelte-preview/src/lib/shell/resources/resourceBackend.ts:1-59` — backend adapters.
- `tauri-svelte-preview/src/lib/shell/resources/resourceService.ts:1-54` — coalesced resource service and lifecycle adapters.
- `tauri-svelte-preview/src/lib/shell/resources/resourceStore.svelte.ts:1-57` — shared resource/disk state and refresh locks.
- `tauri-svelte-preview/src/lib/shell/resources/resourceTypes.ts:1-119` — resource/disk/cleanup contracts.
- `tauri-svelte-preview/src/lib/shell/resources/resourceViewModel.ts:1-31` — owner labels, stop policy, byte formatting.
- `tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceService.ts:1-8` — bounded space service.
- `tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceStore.svelte.ts:1-24` — shared space scan state.
- `tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceTypes.ts:1` — space type exports.
- `tauri-svelte-preview/src/lib/shell/resources/workspaceSpaceViewModel.ts:1-19` — protection/reclaimable view rules.
- `tauri-svelte-preview/src/lib/shell/usage/UsagePopover.svelte:2-19` — compact authoritative usage card.
- `tauri-svelte-preview/src/lib/shell/usage/UsageWorkspace.svelte:2-15` — historical summary/breakdown workspace.
- `tauri-svelte-preview/src/lib/shell/usage/usageAnalytics.ts:1-20` — summary and SQL-shaped breakdown model.
- `tauri-svelte-preview/src/lib/shell/usage/usageBackend.ts:1-32` — usage backend adapters.
- `tauri-svelte-preview/src/lib/shell/usage/usageCurrent.ts:1-21` — current usage normalization and window labels.
- `tauri-svelte-preview/src/lib/shell/usage/usageService.ts:1-35` — coalesced current usage and history service.
- `tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts:1-59` — shared current/history usage state.
- `tauri-svelte-preview/src/lib/shell/usage/usageTypes.ts:1-74` — current and historical usage contracts.
- `tauri-svelte-preview/scripts/resourceViewModel.test.mjs:1-7`, `workspaceSpace.test.mjs:1-7`, `usageCurrent.test.mjs:1-8`, `usageAnalytics.test.mjs:1-8` — failing-first then green frontend contracts.

## Completion checklist

- **Verified:** bounded resource inventory, owner identity/refusal, explicit disk roots/bounds/protection, before/after cleanup receipts, authoritative-or-unavailable current quota, SQLite migration/cursors/rotation/dedupe/rollups/rates, DB-side filters/grouping/aggregation/sort/page, redacted SQL tracing and query-plan evidence, shared compact/full stores, and required serial verification commands.
- **Deferred by the packet stop condition:** rebuilt native application proof, native resource/usage interaction and visual acceptance, native center-surface registration, and the isolated Roslyn lifecycle implementation. These are not claimed as complete.
- **Assumed:** the standalone dispatch packet and separate erratum file promised by the raw transcript are absent from this checkout; the receipt follows the raw log's corrected anchors plus the current-tree recheck and records that evidence limit explicitly.
- **Verified:** no commit was created; all changes remain uncommitted in the requested worktree.

## Completion-gate reruns

- **Verified green (fresh):** `pnpm check:svelte` exited 0. Tail: `Files the /next shell owns: 0 error(s), 0 warning(s).` The same output records 16 old-shell errors outside this gate.
- **Verified green (fresh):** `pnpm build` exited 0. Tail: `✓ built in 11.96s`, `Wrote site to "build"`, and `✔ done`. Existing accessibility, unused-selector, chunk-size, and dynamic-import warnings remain outside the A10 gate.
- **Verified green (fresh):** `RUST_TEST_THREADS=1 cargo test --manifest-path ../core/Cargo.toml resources -- --nocapture` — 2 passed, 0 failed.
- **Verified green (fresh):** `MCB_SQL_TRACE=1 RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage -- --nocapture` — 5 passed, 0 failed, 229 filtered out. The trace shows migration setup, transactional `INSERT OR IGNORE`, rollup rebuild SQL, one summary `SELECT`, daily-view paging with `LIMIT`/`OFFSET`, and versioned-rate lookup.

## Final closure after last code change

- **Verified green (latest):** the focused usage suite is now 8 passed, 0 failed, 229 filtered out after adding opaque source-key, stable file/entry identity, session/turn/workflow rollup, and covering-index plan assertions.
- **Verified green (latest):** `pnpm check:svelte` reports 0 `/next` errors and 0 warnings; the same gate reports 16 old-shell errors outside `/next`.
- **Verified green (latest):** `pnpm check` exits 0.
- **Verified green (latest):** `pnpm build` exits 0; tail is `✓ built in 11.76s`, `Wrote site to "build"`, `✔ done`.
- **Verified green (latest):** `git diff --check` exits 0 and no forbidden layout/shell file is modified.
- **Verified:** the receipt itself is part of the changed-file inventory at `docs/superpowers/evidence/tsk-808/a10-resources-receipt.md:1-132` and was created before all implementation edits.
- **Verified:** no commit was created. The requested worktree is intentionally left with uncommitted A10 changes for controller integration.
- **Correction (verified):** the receipt is tracked at `docs/superpowers/evidence/tsk-808/a10-resources-receipt.md:1`; the earlier range estimate was only a line-count note and does not change the evidence above.
