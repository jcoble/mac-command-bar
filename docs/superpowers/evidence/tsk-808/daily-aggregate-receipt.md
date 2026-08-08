# TSK-808 daily aggregate receipt

Target: make the Stats & Usage heatmap one cell per calendar day by reading SQL-aggregated daily token totals.

Status: complete; no commit created.

## Work log

- Receipt created before repository inspection (verified).
- Existing provider-summary pattern and commit `7044f08` inspected before implementation (verified).
- Daily rollup shape confirmed: it is keyed by `(day, provider, model, project_id)`, so the new read regroups the rollup table by `day` (verified).
- Failing Rust fixture captured before implementation: `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml usage` failed because `read_usage_daily_totals` was not yet defined at `src/usage_db.rs:571` (verified).
- Daily totals SQL, Rust row/command path, capability pin, frontend wrapper/store path, day-keyed heatmap, and view-model assertions implemented (verified by diff inspection; gates pending).
- Rust usage gate after implementation: `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml usage` passed, 16 passed and 2 ignored (verified).
- Frontend view-model gate: `node --experimental-strip-types scripts/usageViewModel.test.mjs` returned `usage view-model tests passed` (verified).
- Svelte gate: `pnpm check:svelte` reported `Files the /next shell owns: 0 error(s), 0 warning(s)`; 16 errors remain outside this gate in the old shell (verified).
- Production frontend gate: `pnpm build` completed successfully (`✓ built`, exit 0); existing accessibility, unused-selector, dynamic-import, and chunk-size warnings were emitted (verified).

## Final file claims

| Claim | Status | File:line evidence |
|---|---|---|
| Daily totals row and one SQL `GROUP BY day` read sum all stored token columns with the daily filter and paging shape | verified | `tauri-svelte-preview/src-tauri/src/usage_db.rs:143-155`, `:365-381`, `:454-458` |
| Two providers on one day produce one summed row | verified | `tauri-svelte-preview/src-tauri/src/usage_db.rs:593-623` |
| Tauri command is exposed and capability pin is updated | verified | `tauri-svelte-preview/src-tauri/src/usage_history.rs:48-51`, `tauri-svelte-preview/src-tauri/src/main.rs:1242-1279`, `:5421-5427`, `:8277-8282` |
| Typed Tauri/backend/service/store path keeps `daily` and adds `dailyTotals` | verified | `tauri-svelte-preview/src/lib/tauriSource.ts:98-104`, `:601-611`; `tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts:10-29`, `:69-78` |
| Heatmap uses fetched daily totals, normalizes against their max, and keys one cell by day | verified | `tauri-svelte-preview/src/lib/shell/usage/UsageWorkspace.svelte:18`, `:28` |
| View-model contract covers the daily-total path and rejects the old per-rollup heatmap iteration | verified | `tauri-svelte-preview/scripts/usageViewModel.test.mjs:12-23` |
| Worktree remains uncommitted and changes stay within the requested files plus this receipt | verified | Final `git status --short --branch` and `git diff --check` |
