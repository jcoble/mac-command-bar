# TSK-808 Phase 1 baseline receipt

Captured from `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview` on 2026-08-09, before the Task 1 implementation change.

## Task 0 baselines (verified)

1. Command: `RUST_TEST_THREADS=1 MSBUILDDISABLENODEREUSE=1 cargo test --manifest-path src-tauri/Cargo.toml`
   - Result: exit 101.
   - Test counts: 250 passed, 2 failed, 3 ignored, 0 measured.
   - Failed tests:
     - `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint` — `Could not bind native C# bridge: Operation not permitted (os error 1)` at `src/lsp.rs:6171`.
     - `lsp::tests::typescript_language_server_smoke_reads_intelligence_actions` — expected TypeScript diagnostics were absent at `src/lsp.rs:5980`.

2. Command: `pnpm check:svelte`
   - Result: exit 0.
   - Owned `/next` shell: 0 errors, 0 warnings.
   - Elsewhere in the project: 16 errors, reported by the check wrapper as the old shell backlog. The underlying `svelte-check` error files were:
     - `src/lib/components/panels/ActivityFilesPanel.svelte` (1)
     - `src/lib/components/panels/ActivityGitPanel.svelte` (1)
     - `src/lib/components/panels/EditorPanel.svelte` (1)
     - `src/lib/components/panels/GitInsightsPanel.svelte` (1)
     - `src/routes/+page.svelte` (12)

3. Command: `for f in scripts/*.test.mjs; do node --experimental-strip-types "$f" >/dev/null 2>&1 || echo "FAIL: $f"; done`
   - Result: exit 0 for the loop; failing scripts reported:
     - `FAIL: scripts/sourceUi.test.mjs`
     - `FAIL: scripts/workspaceSnapshotPlan.test.mjs`

## Task 1 command enumeration (verified)

The fifteen frontend calls in `src/lib/shell/browser/browserBackend.ts` and their matching Rust commands in `src-tauri/src/browser.rs` are:

| Frontend method and call | Rust command input type |
| --- | --- |
| `create_browser_tab` (`browserBackend.ts:221-222`) | `BrowserTabInput` (`browser.rs:1202-1207`) |
| `set_browser_tab_bounds` (`browserBackend.ts:225-226`) | `BrowserBoundsInput` (`browser.rs:1215-1219`) |
| `set_browser_tab_viewport` (`browserBackend.ts:229-230`) | `BrowserViewportInput` (`browser.rs:1230-1234`) |
| `show_browser_tab` (`browserBackend.ts:233-234`) | `BrowserTarget` (`browser.rs:1238-1242`) |
| `hide_browser_workspace` (`browserBackend.ts:237-238`) | `BrowserWorkspaceTarget` (`browser.rs:1246-1250`) |
| `navigate_browser_tab` (`browserBackend.ts:241-242`) | `BrowserNavigationInput` (`browser.rs:1254-1258`) |
| `reload_browser_tab` (`browserBackend.ts:245-246`) | `BrowserTarget` (`browser.rs:1262-1266`) |
| `go_back_browser_tab` (`browserBackend.ts:249-250`) | `BrowserTarget` (`browser.rs:1270-1274`) |
| `go_forward_browser_tab` (`browserBackend.ts:253-254`) | `BrowserTarget` (`browser.rs:1278-1282`) |
| `close_browser_tab` (`browserBackend.ts:257-258`) | `BrowserTarget` (`browser.rs:1286-1290`) |
| `arm_browser_element_picker` (`browserBackend.ts:261-262`) | `BrowserPickerInput` (`browser.rs:1302-1306`) |
| `cancel_browser_element_picker` (`browserBackend.ts:265-266`) | `BrowserTarget` (`browser.rs:1310-1314`) |
| `capture_browser_viewport` (`browserBackend.ts:269-270`) | `BrowserTarget` (`browser.rs:1318-1322`) |
| `open_browser_tab_devtools` (`browserBackend.ts:273-274`) | `BrowserTarget` (`browser.rs:1326-1330`) |
| `open_browser_tab_external` (`browserBackend.ts:277-278`) | `BrowserTarget` (`browser.rs:1334-1338`) |

`clear_browser_workspace_data` is a Rust command with `BrowserWorkspaceTarget` at `browser.rs:1294-1300`; it has no frontend method and was not added or changed.

## TDD evidence (verified)

- Before the implementation fix, `node --experimental-strip-types scripts/browserBackend.test.mjs` exited 1 and listed all fifteen commands as passing their payload unwrapped.
- After the implementation fix, the same command exited 0 and printed `browserBackend: all tests passed`.

Native repro required: run the packaged/dev native app, open Stats & Usage, record the now-specific error text. The follow-up fix is driven by that text.

## Final sweep (Task 7, controller-run)

- `RUST_TEST_THREADS=1 cargo test`: 268 passed, 0 failed, 3 ignored (the flaky native C# lsp test passed this run; it failed intermittently during the wave).
- Gates: `pnpm check` OK; `pnpm check:svelte` 0 errors in /next-owned files (16 old-shell legacy errors unchanged); `pnpm build` OK.
- Script sweep: 2 failures, identical to the Task 0 baseline (sourceUi.test.mjs, workspaceSnapshotPlan.test.mjs — old-shell pins, die with TSK-811).

## Commits (phase 1)

858d9ed browser input wrapping · 207a6db+50f27c9 usage error surfacing · 8e5b336+7c77f9a+90010a1 ACP transport actor · 50ca299+4cad892 manager integration · b91e29d ownership split · 776ac66 structured-only app sessions + inspector · 8a883ea whole-branch fix wave (idempotent activation, one-shot busy guard, terminal events before cleanup, nativeSessionId persistence) · 81c84c2 atomic failed-prompt cleanup · 3ee53fe one-shot isolation for agent-minted turn ids.

Two review scenarios are parked as accepted ACP v1 limits (see .superpowers ledger): both require an agent still streaming updates after its end-of-turn response while a new turn is already active — indistinguishable from legitimate updates at the protocol level.

## Native acceptance (user)

1. `pnpm tauri:dev:next`. 2. New Codex session → structured immediately, streams live, no mode toggle, Inspector shows events. 3. Mid-turn approval answerable; Stop cancels (turn shows interrupted). 4. Externally-started CLI session keeps its terminal + projection. 5. Browser tab create/navigate/screenshot works. 6. Stats & Usage → history loads, or the error names the sqlite binary + db path (that text drives the follow-up fix).
