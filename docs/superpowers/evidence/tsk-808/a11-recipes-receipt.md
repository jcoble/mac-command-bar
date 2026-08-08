# TSK-808 Work Package A11 — Assistance Recipes Receipt

Status: in progress

This receipt is append-only for the A11 implementation lane. Claims will be marked verified or assumed as evidence is collected.

## Start

- Receipt created before repository edits, per dispatch instructions.

## Dispatch/spec reconciliation (verified 2026-08-08)

- [Verified] The raw dispatch packet is the final packet at `a11-dispatch-spec-raw.log:52215-52650`; its ordered edits, test names, commands, done-means, and plan-vs-tree conflicts are the implementation baseline.
- [Verified] The amendment defines A11 at `2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md:2277-2332`; controller-owned shared seams are listed at `:2394-2409`.
- [Verified] Current HEAD is `ba46633` (native CLI handoff), immediately after `d800313` (browser overlay and Workbench action FAB). These newer commits are part of the current tree and must be consumed, not duplicated.
- [Verified] `+page.svelte` now mounts `ShellOverlays` once and passes browser overlay/action-surface contracts at `src/routes/next/+page.svelte:1686-1698`; browser action model/store are in `src/lib/shell/overlay/actionSurfaceModel.ts` and `actionSurfaceStore.svelte.ts`.
- [Verified] `ShellOverlays.svelte` already owns `BrowserOverlayHost` and `WorkbenchActionFab` at `src/lib/shell/components/ShellOverlays.svelte:119-153`; A11 must add one assistance host there rather than a second overlay/page host.
- [Verified] Native handoff wiring is present in `conversationService.ts:278-335`, `+page.svelte:1005-1092`, `ConversationSurface.svelte` props, and Rust `agent_conversation/handoff.rs`; A11 will not duplicate or edit those controller/shared seams.
- [Verified] `sendStructuredMessage` remains a text/image prompt submission at `src/lib/shell/conversation/conversationService.ts:397-450`, so the packet's correlated structured-output gate remains an assumed controller integration receipt, not an A11-owned edit.

## Test-first evidence

- [Verified] Added the five named tests before implementation: `tauri-svelte-preview/scripts/assistanceRecipes.test.mjs`, `assistanceContext.test.mjs`, `assistanceService.test.mjs`, `assistanceAudit.test.mjs`, and `assistanceUiContract.test.mjs`.
- [Verified] First failing test run: `node --experimental-strip-types scripts/assistanceRecipes.test.mjs` failed with `ERR_MODULE_NOT_FOUND` for the not-yet-created `src/lib/shell/assistance/assistanceRecipeRegistry.ts`. This is the expected red test before implementation.
- [Verified] `node --experimental-strip-types scripts/assistanceRecipes.test.mjs` now passes.
- [Verified] `node --experimental-strip-types scripts/assistanceContext.test.mjs` now passes.
- [Verified] `node --experimental-strip-types scripts/assistanceService.test.mjs` now passes.
- [Verified] `node --experimental-strip-types scripts/assistanceAudit.test.mjs` now passes.
- [Verified] `node --experimental-strip-types scripts/assistanceUiContract.test.mjs` now passes.
- [Verified] Final A11 test tails: `assistanceRecipes: all tests passed`; `assistanceContext: all tests passed`; `assistanceService: all tests passed`; `assistanceAudit: all tests passed`; `assistanceUiContract: all tests passed`.
- [Verified] `pnpm check:svelte` exits 0 with `Files the /next shell owns: 0 error(s), 0 warning(s). Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.`
- [Verified] `pnpm check` exits 0.
- [Verified] Final focused-test tails after output-hash/read-only checks: all five named tests report `all tests passed`.
- [Verified] `pnpm build` exits 0 and writes the static site. Build output contains pre-existing Svelte accessibility/unused-selector warnings outside the `/next` gate; no build error occurred.
- [Verified] `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml` ran 229 tests serially: 227 passed, 1 ignored, and 1 failed only with the accepted environmental error `Could not bind native C# bridge: Operation not permitted (os error 1)` in `lsp::tests::concurrent_native_csharp_ensure_calls_share_one_canonical_root_endpoint`.

## Implementation receipt

- [Verified] Added stable recipe/surface contracts, strict plain-object and size checks, shell-fragment rejection, allow-listed field patches, and expiring proposal schemas in `tauri-svelte-preview/src/lib/shell/assistance/assistanceTypes.ts:1-126` and `assistanceSchemas.ts:1-189`.
- [Verified] Added one versioned `none|proposal` registry for all declared recipes, with duplicate/unknown registration refusal and no execute member in `tauri-svelte-preview/src/lib/shell/assistance/assistanceRecipeRegistry.ts:1-78`.
- [Verified] Added synchronous bounded/delimited fact capture, deterministic hashing, identity/fact/target revalidation, and expiry checks in `tauri-svelte-preview/src/lib/shell/assistance/assistanceContext.ts:1-130`.
- [Verified] Added injected structured-output and typed surface-adapter service with request correlation, owner/generation/fact/output/expiry/expected-value checks, cancellation, read-only recipe refusal, and no direct backend/product calls in `tauri-svelte-preview/src/lib/shell/assistance/assistanceService.ts:1-301`.
- [Verified] Added presentation-only request state in `tauri-svelte-preview/src/lib/shell/assistance/assistanceStore.svelte.ts:1-50`.
- [Verified] Added redacted additive audit events, bounded detail, serialization, and reload restoration in `tauri-svelte-preview/src/lib/shell/assistance/assistanceAudit.ts:1-99`.
- [Verified] Added proposal/action/host UI contracts with provenance, confidence, select/deselect, Apply selected, Dismiss, Retry, Continue without AI, and the preview/confirmation/revalidation/audit safety note in `tauri-svelte-preview/src/lib/shell/assistance/AssistanceAction.svelte:1-57`, `AssistanceProposal.svelte:1-106`, and `AssistanceHost.svelte:1-80`.
- [Verified] Added exactly one `<AssistanceHost />` beside the already-committed browser overlay and Workbench action FAB in `tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte:19,155`; no layout file, `ShellFrame.svelte`, `ShellSidebar.svelte`, `+page.svelte`, browser module, conversation module, workflow authority, controller seam, manifest, capability, or lock file was edited.
- [Verified] Added the five named tests: `tauri-svelte-preview/scripts/assistanceRecipes.test.mjs:1-77`, `assistanceContext.test.mjs:1-53`, `assistanceService.test.mjs:1-122`, `assistanceAudit.test.mjs:1-30`, and `assistanceUiContract.test.mjs:1-26`.
- [Verified] `git diff --check` and an explicit trailing-whitespace scan pass. No commit was created; `git status --short` contains only the A11 receipt, tests, assistance modules, and the two-line `ShellOverlays.svelte` change.

## Done-means and limits

- [Verified] No database touched: this lane contains no database, query, or persistence mutation; audit serialization is local to the injected audit boundary.
- [Verified] No direct mutation path: service code calls only the injected structured-output function/object and typed surface adapter; source and test checks reject direct backend/product calls.
- [Verified] No second workflow runtime or action-surface authority: the existing workflow/action-surface/browser/native-handoff seams were consumed as current-tree authorities; A11 adds only its presentation store and one overlay host.
- [Verified] Hostile text, secret exclusion/redaction, hash changes, stale identity, patch allow-listing, cancellation, audit order/reload, provider failure, and deterministic non-blocking behavior are covered by the focused tests.
- [Assumed] A2/A4 still needs to expose the correlated structured-output request and controller-owned nullable orchestration recipe/hash/confirmation/result fields. The service accepts that seam through injection; it does not edit `conversationTypes.ts`, `orchestration.rs`, `main.rs`, or registrations.
- [Assumed] Existing Git, Problems, Run, Browser, Context, form/save, and worktree typed adapters will be supplied by the controller/integration lane. The host exposes contextual entry points and callback contracts, but this lane does not invent direct adapters or wire backend calls into panels.
- [Deferred] Rebuilt-native-app proof remains outstanding: native proposal drafts/reviews across surfaces, native hostile-text containment, native changed-target refusal, native safe-field application through the real executor, native cancellation with zero external change, native restart/reload audit persistence, and native provider-failure Save usability. The required native rebuild/interaction proof was not claimed here.
- [Deferred] `pnpm tauri:build` and rebuilt desktop acceptance remain controller/native-proof work; the requested web build and serialized Rust suite were run and recorded above.
- [Verified] No browser or Playwright session was opened by this lane, so there is no browser process to clean up.

Stop result: A11-owned recipe contracts, tests, service, audit, presentation UI, and single-host integration are implemented and statically/build verified. Controller-owned correlated-output/orchestration integration and rebuilt-native proof remain explicit handoff items; no commit was made.
