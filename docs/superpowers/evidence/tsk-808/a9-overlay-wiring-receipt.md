# TSK-808 A9 browser-overlay wiring receipt

## Work log

- 2026-08-08: Receipt created before repository inspection or implementation. Status: assumed starting point; no wiring edits made yet.
- 2026-08-08: Cross-checked the emitted raw-log specification against the checked-out tree. Verified that `+page.svelte` still has one docked `BrowserPanel` in `browserArea`, `ShellOverlays.svelte` has no browser overlay or action-surface props, and the action-surface model/FAB files named by the contract are absent. The Rust browser command registration remains outside this lane and is intentionally untouched.

## Result

- [Verified] The one docked `BrowserPanel` remains at `tauri-svelte-preview/src/routes/next/+page.svelte:1511`; the center roster still maps `browser` to that snippet at `:1545-1552`.
- [Verified] `ShellOverlays.svelte` now mounts exactly one `BrowserOverlayHost` and exactly one `WorkbenchActionFab` at `:126-153`.
- [Verified] Static mount count from the checked-out tree is `BrowserPanel mounts: 1`, `BrowserOverlayHost mounts: 1`, and `WorkbenchActionFab mounts: 1`; the layout-file and `src-tauri/` diff checks returned no paths.
- [Verified] Browser callbacks and the `WorkbenchAction[]` are separate values. The callback bundle is declared at `ShellOverlays.svelte:40-66`; the action array is typed at `:83-86` and passed separately at `:153`.
- [Verified] Browser handlers route to the existing browser store/model functions and carry owner, workspace, target, and generation in `deriveWorkbenchActionContext()` at `+page.svelte:582-600`.
- [Verified] The action surface keeps disabled actions visible with a reason, uses a pure reducer and context filter, and chooses fan, horizontal, or bottom-sheet geometry without clipping in `actionSurfaceModel.ts:63-240`.
- [Verified] The FAB is a single fixed root-level control with a 48px target and 16px safe edge in `WorkbenchActionFab.svelte:188-269`; its confirmation/error surface and keyboard handling are in `:122-185`.
- [Assumed] The browser action list is intentionally limited to the browser routes named by the A9 wiring contract. Other context-specific action registries remain outside this browser-overlay lane.

## Files and changed line anchors

- [Verified] `tauri-svelte-preview/src/routes/next/+page.svelte:13-22,36-68` adds the icon, browser store/model, browser type, and action-surface imports.
- [Verified] `tauri-svelte-preview/src/routes/next/+page.svelte:235-600` adds the active center signal, safe browser callbacks, native-capability fallback message, browser action descriptors, overlay callback bundle, and context derivation.
- [Verified] `tauri-svelte-preview/src/routes/next/+page.svelte:1511` is the unchanged docked browser anchor; `:1554-1557` records the active center panel; `:1595-1607` passes the workspace, action array, context, and callback bundle to `ShellOverlays`.
- [Verified] `tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte:17-26,30-66,83-99,126-153` adds the browser overlay imports, shared callback types, typed props, and complete handler forwarding. The overlay explicitly leaves developer tools disabled in web/dev mode at `:151`.
- [Verified] `tauri-svelte-preview/src/lib/shell/components/WorkbenchActionFab.svelte:1-352` is the single global action control. It owns only surface layout, focus, confirmation, and error display; business work remains in the passed actions.
- [Verified] `tauri-svelte-preview/src/lib/shell/overlay/actionSurfaceModel.ts:1-240` adds the contract types, reducer, context filter, and collision-aware layout function.
- [Verified] `tauri-svelte-preview/src/lib/shell/overlay/actionSurfaceStore.svelte.ts:1-17` adds the shell-wide reducer-backed state holder.
- [Verified] `tauri-svelte-preview/scripts/actionSurfaceModel.test.mjs:1-58` adds reducer, context-filter, fan, and bottom-sheet coverage.
- [Verified] `docs/superpowers/evidence/tsk-808/a9-overlay-wiring-receipt.md:1-` is this append-only receipt. No layout files and no `src-tauri/` files were edited by this lane.

## Verification inventory and results

The browser-related script inventory was listed from `tauri-svelte-preview/` with:

```text
find scripts -maxdepth 1 -type f -name '*browser*test.mjs' -print | sort
find scripts -maxdepth 1 -type f -name '*Browser*test.mjs' -print | sort
find scripts -maxdepth 1 -type f -iname '*action*surface*test.mjs' -print | sort
find scripts -maxdepth 1 -type f -iname '*normalize*browser*test.mjs' -print | sort
```

The resulting existing A9 scripts were `scripts/normalizeBrowserUrl.test.mjs`, `scripts/browserModel.test.mjs`, `scripts/browserBounds.test.mjs`, `scripts/browserAnnotations.test.mjs`, `scripts/browserFeedback.test.mjs`, `scripts/browserBackend.test.mjs`, and the newly added `scripts/actionSurfaceModel.test.mjs`.

Each was run serially with the requested command form:

```text
node --experimental-strip-types scripts/normalizeBrowserUrl.test.mjs
normalizeBrowserUrl: all tests passed

node --experimental-strip-types scripts/browserModel.test.mjs
browserModel: all tests passed

node --experimental-strip-types scripts/browserBounds.test.mjs
browserBounds: all tests passed

node --experimental-strip-types scripts/browserAnnotations.test.mjs
browserAnnotations: all tests passed

node --experimental-strip-types scripts/browserFeedback.test.mjs
browserFeedback: all tests passed

node --experimental-strip-types scripts/browserBackend.test.mjs
browserBackend: all tests passed

node --experimental-strip-types scripts/actionSurfaceModel.test.mjs
actionSurfaceModel: all tests passed
```

- [Verified] The final required Svelte gate was:

```text
pnpm check:svelte
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

- [Verified] An earlier `pnpm check:svelte` run found one FAB-only warning and the same 16 old-shell errors; changing the confirmation element to a `div` and raising the context label to 12px removed the warning. The final gate above is the acceptance result.
- [Verified] The required build was:

```text
pnpm build
...
✓ built in 12.19s

Run npm run preview to preview your production build locally.

> Using @sveltejs/adapter-static
  Wrote site to "build"
  ✔ done
```

- [Verified] The build exited with status 0. Its warnings are existing old-shell accessibility/chunk warnings; no `/next` error was emitted.
- [Verified] The final whitespace check was:

```text
git diff --check
(no output)
```

- [Verified] The final static mount check was:

```text
printf 'BrowserPanel mounts: '; rg -c '<BrowserPanel' tauri-svelte-preview/src/routes/next/+page.svelte
printf 'BrowserOverlayHost mounts: '; rg -c '<BrowserOverlayHost' tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte
printf 'WorkbenchActionFab mounts: '; rg -c '<WorkbenchActionFab' tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte
printf 'forbidden layout edits: '; git diff --name-only | rg 'centerDock\.ts|layoutStorage\.ts|sidebarViews\.ts' || true
printf 'src-tauri edits in this diff: '; git diff --name-only | rg '^tauri-svelte-preview/src-tauri/' || true

BrowserPanel mounts: 1
BrowserOverlayHost mounts: 1
WorkbenchActionFab mounts: 1
forbidden layout edits: src-tauri edits in this diff:
```

## Native command boundary

- [Verified] This lane did not edit `src-tauri/`, did not add an invoke call, and keeps optional developer-tools/external-browser calls guarded. The browser store continues to use its in-memory web/dev backend, and drawing controls show the explicit deferred message instead of pretending to have native markup support.
- [Verified] A read-only check of the shared tree showed a separate native bridge commit is present in the worktree, but this lane does not certify that native registration, ACL, rebuilt-app behavior, child-view lifecycle, or device proof.
- [Assumed] Native acceptance remains deferred until the Rust command registration is available in the accepted rebuilt binary. Until then, web/dev behavior is the graceful fallback: the overlay remains usable, unsupported native controls stay disabled or report the bounded message, and no frontend path requires a native command to render.

## Stop state

- [Verified] No commit was created by this lane.
- [Verified] The final worktree still contains the separate lane's changes and this lane's uncommitted frontend/receipt files; they were not reset, staged, or removed.
