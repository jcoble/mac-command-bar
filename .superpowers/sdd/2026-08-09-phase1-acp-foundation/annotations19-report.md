# Lane annotations19 report: persisted browser-overlay annotations

Status: implementation complete; required verification green.

## Outcome

- The three annotation commands add, list, and delete rows through the manager's shared `SessionStore` (`agent_conversation/mod.rs:24-77`, `manager.rs:320-349`).
- Session selection loads stored annotations once, and add/remove/clear write only on mutation; there is no polling or timer (`+page.svelte:632-653`, `sessionBrowserState.svelte.ts:81-124`).
- Database row ids are the only annotation identities. The removed frontend path no longer generates ids with `Date.now()` (`sessionBrowserOps.ts:24-25, 168-196`).
- Restart persistence is covered at the manager/file-database boundary and by the TypeScript command/state test (`manager.rs:4198-4231`, `sessionBrowserState.test.ts:180-232`).

## Files touched

- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs` — serializable annotation response and the three thin commands.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs` — shared-store methods and file-database restart/delete test.
- `tauri-svelte-preview/src-tauri/src/main.rs` — exactly three single-line handler registrations.
- `tauri-svelte-preview/src/lib/shell/browser/sessionAnnotationPersistence.ts` — typed command calls and stored-row presentation.
- `tauri-svelte-preview/src/lib/shell/browser/sessionBrowserState.svelte.ts` — activation load plus add/delete/clear write-through behavior.
- `tauri-svelte-preview/src/lib/shell/browser/sessionBrowserOps.ts` — database ids and transient append/replace/remove operations.
- `tauri-svelte-preview/src/lib/shell/browser/SessionBrowserOverlay.svelte` — asynchronous persistence plumbing and existing error surface.
- `tauri-svelte-preview/src/lib/shell/browser/SessionBrowserAnnotationLayer.svelte` — database id type plumbing.
- `tauri-svelte-preview/src/routes/next/+page.svelte` — one load on session selection.
- `tauri-svelte-preview/scripts/sessionBrowserState.test.ts` — command contract, reload, and deletion logic test; replaces the touched `.mjs` file.
- `tauri-svelte-preview/package.json` — points the existing script at the TypeScript test.
- This report.

## Receipts

- **Verified:** `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation` — **99 passed, 0 failed, 0 ignored, 265 filtered**. Includes `session_annotations_survive_manager_restart` and the requested contract test.
- **Verified:** exact contract invocation — **1 passed, 0 failed, 0 ignored, 363 filtered**.
- **Verified:** `pnpm run check:svelte` — `Files the /next shell owns: 0 error(s), 0 warning(s)`; the command separately reports 16 existing old-shell errors outside this gate.
- **Verified:** `pnpm run test:session-browser-state` — `sessionBrowserState: all checks passed`.
- **Verified:** `cargo fmt --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml` completed before the Rust suite.
- **Verified:** `git diff --check` produced no output.
- **Verified:** no app or browser was launched; no port was bound; no file was staged, committed, or pushed.

## Storage finding

- The previous annotation holder was pure ephemeral Svelte state (`sessionBrowserState.svelte.ts`); it did not use local storage or another durable frontend store. The same list remains only as transient render state populated from backend rows.
- `rg -n "localStorage|sessionStorage|persist|storage|bySession|annotations" .../sessionBrowserState.svelte.ts .../sessionBrowserOps.ts .../sessionAnnotationPersistence.ts` finds the transient map and backend list call, with no local/session storage use.
- `rg -n "annotation-.*Date\\.now|Date\\.now\\(\\).*annotation|addSessionAnnotation|sessionBrowserState\\.test\\.mjs" tauri-svelte-preview/src tauri-svelte-preview/scripts tauri-svelte-preview/package.json` returns no matches. The frontend-generated identity path and touched JavaScript test path are gone.

## Assumptions and boundaries

- **Verified:** annotation text is passed directly to the store and is never written to logs by this change.
- **Verified:** clear-after-send and discard-all delete each stored row through the required delete command; no extra bulk command or compatibility path was added.
- **Assumed:** the existing `SessionStore` annotation schema and timestamp semantics from Track B remain authoritative; this lane did not change them.
