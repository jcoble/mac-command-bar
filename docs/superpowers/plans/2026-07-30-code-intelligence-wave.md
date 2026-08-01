# Code intelligence wave — TSK-799 (2026-07-30)

Branch (integrator creates it; lanes NEVER run git): `tsk-799-code-intelligence`.
Root-cause evidence: `.superpowers/sdd/2026-07-30-code-intelligence-root-cause/findings.md`.
Notion task: TSK-799. Absorbs TSK-789 (backend cancellation remainder) and TSK-784 (status chip).

## The decisions this wave implements (user, 2026-07-30)

1. **VS Code-style lens.** The reference lens paints the moment a file opens — placeholder
   first — and the real SEMANTIC number (from the language server) fills in progressively,
   working its way down the document as each symbol's answer arrives. The name-based text
   count is retired from the native lens. It survives only where it is honest: project text
   search, and the browser preview (which has no language server at all).
2. **Rust and Svelte get lenses too.** Anchors come from LSP document symbols (rust-analyzer
   and svelte-language-server are already configured, `main.rs:2856-2890`), with the existing
   regex extractor as fallback for TS/JS/C# when the server has no answer yet.
3. **No more guessing about time.** Timing instrumentation lands behind a dev flag so the next
   desktop run produces numbers, not theories.

Deviation from the original task scope, recorded: the persistent rust-side index is DEFERRED —
after decision 1 the text counter no longer sits on the lens hot path (it serves only search),
so a fresh walk per user-initiated search is acceptable for now.

## Standing rules (every lane pastes these into its own head)

- NO git commands of any kind. You edit files in the primary checkout only. The integrator
  commits per-lane by path.
- FROZEN: `tauri-svelte-preview/src/routes/+page.svelte` (the OLD shell — carries the user's
  own uncommitted work; never open it for writing, never stage, never revert). `.vscode/` is
  untracked — leave it alone.
- Own ONLY your listed files. Anything needed in another lane's file goes into your
  integration note instead: `.superpowers/sdd/2026-07-30-code-intelligence-root-cause/_lane-<X>-INTEGRATION.md`
  (verbatim edits, exact file:line).
- Plain English in every user-visible string, error, and comment. No invented jargon. No
  co-author trailers, no Claude/Anthropic mention anywhere (you don't commit anyway).
- Test-first where a test can exist: write the failing test, watch it fail, then fix.
- Builds: only Lane A compiles rust, sequentially. Frontend lanes run node tests only —
  find the exact script names in `tauri-svelte-preview/package.json` before running anything;
  report the exact commands you ran and their output tails.
- UI rules: AlertDialog (never `window.confirm` — it silently no-ops in the Tauri webview),
  text sizes ≥ `text-[12px]`, colors via `--color-*` tokens only.
- Capability gating: the frontend may only depend on a NEW backend command when its name is
  in `read_backend_capabilities()` — Tauri silently drops unknown payload keys on old builds.

## Pinned backend contract (Lane A makes it true; B and F code against it)

1. **Existing LSP command names and payload shapes do not change**:
   `find_source_lsp_references`, `find_source_lsp_definitions`, semantic tokens, inlay hints,
   diagnostics, hover, etc. (wrappers at `main.rs:857-1020`). They become concurrent-safe.
2. **Document symbols command**: `find_source_lsp_document_symbols(root, language, path)` →
   `Array<{ name: string; kind: string; line: number; character: number }>` (0-based line,
   flattened — children inlined). If an equivalent command already exists under another name,
   Lane A adds this exact name as a thin wrapper anyway. Capability: `lspDocumentSymbols`.
3. **Status**: `read_source_lsp_status` (exists, `main.rs:663-675`) gains additive fields:
   `state: 'not-running' | 'starting' | 'indexing' | 'ready' | 'disabled'` and
   `detail: string | null` (plain-English sentence, e.g. "Loading the EdiPlatform solution").
   Frontend detects support by field presence, not by capability.
4. **Status push event**: Tauri event `source-lsp-status-changed` with payload
   `{ root: string; language: string; state: <same union>; detail: string | null }`, emitted
   on every state transition. No polling anywhere.
5. **Capabilities added**: `lspDocumentSymbols`, `lspStatusEvents`.

## Lane A — backend: unjam the language-server pipe (rust)

Owns: `tauri-svelte-preview/src-tauri/src/lsp.rs`, `tauri-svelte-preview/src-tauri/src/main.rs`,
`core/src/reference_counts.rs`, `core/src/lib.rs` (if needed), rust tests.

1. **Concurrent requests.** Today every request holds the session's `Arc<Mutex<...>>` for its
   whole round trip (`lsp.rs:357`, `:1543-1544`) — one at a time, up to 6s each. Rebuild as a
   multiplexed client: one writer (short lock only to write the framed message), one reader
   task that routes responses by JSON-RPC id to per-request channels. Multiple requests
   in flight on one server at once. Keep the initialize handshake as is (`lsp.rs:1506-1533`).
2. **Cancel what we abandon.** When a request's rust-side wait times out
   (`LSP_REQUEST_TIMEOUT` 6s, diagnostics 1.2s — `lsp.rs:15-16`), send `$/cancelRequest` with
   that id so the server stops working on it. This is the remaining scope of TSK-789.
3. **Status truth.** Extend `read_source_lsp_status` per contract #3. Derive `indexing` from
   LSP `$/progress` / `window/workDoneProgress` notifications when the server sends them
   (csharp-ls does); otherwise a conservative heuristic (starting until initialized; ready
   after). Emit the contract #4 event on transitions.
4. **Stop discarding the server's own voice.** Replace `Stdio::null()` for stderr
   (`lsp.rs:1471`) with a capped in-memory ring buffer (last ~200 lines) exposed via a new
   command `read_source_lsp_log(root, language)`; capability `lspLog` (nice-to-have, low
   priority — skip if it fights the mutex rework).
5. **Counter hygiene.** In `main.rs:1992`, stop reusing `MAX_PREVIEW_BYTES` as the counting
   pass's skip ceiling: new constant `MAX_REFERENCE_SCAN_BYTES = 4 * 1024 * 1024` (4MB). A
   660KB source file must count, not poison the pass (`reference_counts.rs:170-177`).
   The counter now serves search + browser preview only — do not remove it.
6. **Timing behind a flag.** When env `MCB_TIMING=1`, `eprintln!` one line per LSP request and
   per counting pass: command, duration ms, outcome. Plain English.
7. Capabilities: add `lspDocumentSymbols`, `lspStatusEvents` (and `lspLog` if built) to
   `read_backend_capabilities`.

Tests (write first): multiplexing — two concurrent requests to a fake stdio server script
resolve independently (follow existing lsp test patterns in the file); cancel message written
on timeout; status transitions; a 660KB file no longer sets `skipped_files` (existing test
style at `reference_counts.rs:321-365`).
Verify: `MSBUILDDISABLENODEREUSE=1 cargo test` in `tauri-svelte-preview/src-tauri` AND `cargo test` in `core/`; `cargo check` clean.
Done means: all listed behaviors implemented + tested, contract exactly as pinned,
integration note written (include: exact status payload, event name, capability names).

## Lane B — editor intelligence: the lens itself (frontend)

Owns: `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte`,
`tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts`,
`tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts`,
`tauri-svelte-preview/src/lib/shell/editor/sourceCodeLensKeys.ts`,
`tauri-svelte-preview/src/lib/sourceData.ts` (symbol-extraction section only),
their test files. Do NOT touch `EditorPanel.svelte` (Lane F) or `panelActivation.ts` (Lane E).

1. **Paint immediately.** `provideCodeLenses` (`MonacoSourceEditor.svelte:630-651`) returns
   every lens WITH a command attached from the start — title `"… references"` placeholder —
   so the row is visible the instant symbols are known. Register an `onDidChange` emitter on
   the provider; fire it whenever a count lands so Monaco re-renders that lens with the real
   number. Never invent a number: placeholder until the true count arrives.
2. **Semantic counts, progressive, top-down.** On native, a per-file scheduler asks the
   language server for each symbol's references (`find_source_lsp_references`, count the
   result; exclude the declaration itself if included) — max 4 in flight, document order
   starting from the viewport, remaining symbols streamed as answers return. Symbols leave
   the queue when the file closes or the user switches files (that IS the cancellation — the
   in-flight ≤4 finish harmlessly; Lane A's backend stops server-side runaways). While the
   server's status is not ready, keep lenses on placeholder — Lane F shows why.
3. **Browser stays as it is.** When no language server exists (`isNativeTauriRuntime()` false
   or lookup returns null), fall back to the existing text-count path unchanged — the browser
   preview keeps working exactly as today. The `atLeast`/"at least" wording applies only to
   that fallback.
4. **One cache, honest invalidation.** Collapse the two 30s caches
   (`referenceCountBatcher.ts:58-82` and `MonacoSourceEditor.svelte:346-349`) into one, keyed
   `projectRoot → file → symbol`, invalidated on model content change (debounced ~2s after
   the last keystroke re-counts only the edited file's symbols) and on file save. Keyed per
   project root so a workspace switch SWAPS the cache instead of wiping it — rewrite
   `setProjectRoot` (`sourceIntelligence.ts:557-566`) to keep a `Map<root, …>`.
5. **Anchors for every language.** When capability `lspDocumentSymbols` is present, lens
   anchors come from `find_source_lsp_document_symbols` (contract #2) — this switches on Rust
   and Svelte lenses. Fallback: the existing regex `extractSourceSymbols`
   (`sourceData.ts:1791-1803`) for TS/JS/C# when the command is absent (old build, browser)
   or returns nothing yet. Keep the 120-symbol cap.
6. **Delete the dead fallback.** `nativeReferences`/`setRecords`
   (`sourceIntelligence.ts:159, 313-319, 574`) have zero call sites repo-wide — remove them
   and rewrite the decision comment at `sourceIntelligence.ts:14-22` to record today's ruling
   (semantic lens, text search only for search).
7. **Timing behind a flag.** Reuse the dev-counter pattern (`devInvokeCounter.svelte.ts`): when
   the dev flag is on, `console.info` one line per lens lifecycle step (symbols known → first
   paint → each count landed), plain English, with ms.

Tests (write first): scheduler drains top-down with max 4 in flight; placeholder → number
transition fires the change emitter; cache survives root switch A→B→A; edit-debounce
invalidates only the edited file; browser fallback path unchanged (existing tests keep
passing). Verify: the repo's node test scripts for these files (find exact names in
`package.json`) + the svelte-check gate script used by prior waves.
Done means: lens visible immediately on file open, numbers stream in, old text-count lens
code path unreachable on native, integration note written (any `+page.svelte`/EditorPanel
wiring the integrator must do — you do not touch those files).

## Lane E — workspace switches stop demolishing state (frontend)

Owns: `tauri-svelte-preview/src/lib/shell/panelActivation.ts`,
`tauri-svelte-preview/src/routes/next/+page.svelte` (restore region ~lines 300-450 ONLY),
`tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts` (if present — verify name),
their test files. Do NOT touch `sourceIntelligence.ts` (Lane B owns it — its cache re-keying
already makes `setProjectRoot` non-destructive).

1. **Restore without re-reading the world.** `restoreWorkspace` (`+page.svelte:379-414`)
   closes every tab and re-reads every file from disk through `requestOpenFile`. Keep a
   per-workspace map of already-loaded editor models/tabs; on switch, detach and reattach
   instead of destroy and re-read. Cap retained workspaces (e.g. 3, least-recently-used
   evicted) so memory stays bounded. A file re-reads only when its tab was evicted.
2. **Panel reload only when the panel's inputs changed.** `sessionPicked`
   (`panelActivation.ts:220-228`) re-runs `loadPanel` for every panel ever shown. Make each
   loader receive the new root and decide cheaply (the services are already idempotent on
   same-input — worktrees pane proves the pattern); do not blanket-reload.
3. Preserve the diff-follows-session behavior fixed in PR #12 (`+page.svelte` restore guard —
   `diffPathFor` / `showStoredDiff`) — it must still pass its existing tests.

Tests (write first): A→B→A keeps A's tabs without a second disk read (spy on the read
wrapper); eviction works at the cap; existing sessionWorkspaces/diff tests still green.
Verify: node test scripts for these files + the svelte-check gate.
Done means: measurable no-re-read round trip in tests, no behavior change for first-time
opens, integration note written.

## Lane F — the editor says what the server is doing (frontend)

Owns: `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte` (and a new small
`LanguageServerStatusChip.svelte` beside it), their test files.

1. **Status chip** (TSK-784): small chip in the editor header — "C#: indexing…", "C#: ready",
   "C# server is off" — from `read_source_lsp_status` (refresh on file open) plus the
   `source-lsp-status-changed` event (contract #4) when capability `lspStatusEvents` exists.
   No polling. Old build (no `state` field): show nothing. Copy in plain English; the chip's
   tooltip carries `detail`.
2. **Calm the file-open storm.** Today opening a C# file fires semantic tokens + inlay hints
   immediately and diagnostics twice (`EditorPanel.svelte:93-116`, `DIAGNOSTICS_SETTLE_MS`).
   While `state` is `starting`/`indexing`: skip inlay hints and the duplicate diagnostics
   pass; fire them once when the status event flips to `ready`. When status is unknown (old
   build), keep today's behavior unchanged.
3. The C# on/off switch shipped in Settings stays as is; the chip reflects `disabled`.

Tests (write first): chip renders per state; storm calls deferred while indexing and fired
once on ready; unknown-status keeps legacy behavior. Verify: node tests + svelte-check gate.
Done means: chip live, storm staggered, integration note written (exact event subscription
code if any wiring belongs to `+page.svelte`).

## After lanes

1. Integrator: branch `tsk-799-code-intelligence`, commit per-lane by path, wire all
   integration notes, run the full gate (svelte-check, node suites, `cargo check`,
   `pnpm build`), browser-walk the /next shell at the dev URL (browser preview must behave
   exactly as before), push, open PR. Old-shell diff and `.vscode/` stay untouched.
2. ONE milestone review (moderate — correctness of the multiplexer, the scheduler, and the
   no-re-read restore; no nitpick stacking), fixer finishes, controller merges.
3. User desktop pass with `MCB_TIMING=1` to confirm where the old 10s actually went.
