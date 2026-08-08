# Work Package A9 — Browser frontend receipt

Status: complete for the frontend lane. No Rust, Cargo, Tauri configuration, shell-frame wiring, route wiring, or database work was performed here.

## Owned files and symbols

### Browser model and contracts

- `src/lib/shell/browser/browserTypes.ts`
  - `BrowserPresentationMode`, `BrowserInteractionMode`, `BrowserViewportPreset`, `BrowserFloatingBounds`.
  - `BrowserFeedbackAttachment` is readonly and carries exactly workspace/tab/generation, URL/title, bounded selector/name/snippet, note/intent, image id, timestamp, and source hash.
  - `BrowserTabState`, `BrowserWorkspaceState`, `BrowserWorkspaceCapture`, selection/markup types, conversation bridge/preview types, and `createBrowserWorkspace`.
  - No page-owned sensitive fields are represented.
- `src/lib/shell/browser/browserBounds.ts`
  - Responsive/mobile/tablet/laptop/desktop presets, custom-size normalization, and `clampBrowserFloatingBounds` (`clampFloatingBounds` alias).
- `src/lib/shell/browser/browserPresentation.ts`
  - Pure four-mode transition and restore-target rules.
- `src/lib/shell/browser/browserAnnotations.ts`
  - Selector/snippet/class/annotation caps, bounded selection and markup construction, source hashing, immutable feedback construction, and exact-target checks.
- `src/lib/shell/browser/browserBackend.ts`
  - Typed future-command interface for create/bounds/show/hide/navigation/history/close/picker/capture plus optional devtools/external commands.
  - `InMemoryBrowserBackend` / `createFakeBrowserBackend` records deterministic calls, history, visibility, viewport, and deterministic capture bytes without network or native views.
- `src/lib/shell/browser/browserModel.ts`
  - Required exports: `activateBrowserWorkspace`, `deactivateBrowserWorkspace`, `createBrowserTab`, `selectBrowserTab`, `closeBrowserTab`, `navigateActiveBrowserTab`, `setBrowserViewport`, `setBrowserPresentationMode`, `expandBrowserFrom`, `restoreBrowserToDock`, `minimizeBrowserToPrevious`, `collapseBrowserToControl`, `beginBrowserElementPicker`, `acceptBrowserElementSelection`, `cancelBrowserAnnotation`, `queueBrowserAnnotation`, `removeBrowserAnnotation`, `captureBrowserWorkspace`, `formatBrowserFeedback`, and `stageBrowserFeedbackPreview`.
  - `createBrowserModel` supplies an isolated state/backend handle for tests and future controller wiring.
  - Unsafe/non-http(s), userinfo, file/data/javascript/custom-protocol addresses are rejected before backend calls.
  - Presentation changes never recreate a tab. Cold activation normalizes stale maximized state back to docked.
  - Grab defaults to context/no note; annotation requires a note plus Change/Question; Draw queues markup against backend-provided capture bytes.
  - Feedback preview validates exact owner/generation and draft snapshot. Commit updates the supplied conversation bridge only after the preview remains current, removes the queue item last, and never sends.

### Compatibility facade

- `src/lib/shell/browser/browserStore.svelte.ts`
  - Preserves the existing `browser`, `activateBrowser`, `setBrowserUrl`, `reloadBrowserFrame`, `captureBrowserState`, and `restoreBrowserState` exports and legacy URL snapshot shape.
  - Adds the reactive workspace/backend handles and adapters used by the new composition surfaces.
  - Routes feedback staging through the existing conversation draft and attachment service APIs via an owner-scoped bridge.

### Components

- `src/lib/shell/components/BrowserPanel.svelte` is composition-only and renders the dock surface only in `docked` mode, so expanded/collapsed presentation has one visible browser host.
- `src/lib/shell/components/browser/BrowserTabs.svelte`
- `src/lib/shell/components/browser/BrowserToolbar.svelte`
- `src/lib/shell/components/browser/BrowserViewport.svelte`
- `src/lib/shell/components/browser/BrowserFeedbackPanel.svelte`
- `src/lib/shell/components/browser/BrowserAnnotationToolbar.svelte`
- `src/lib/shell/components/browser/BrowserAnnotationCard.svelte`
- `src/lib/shell/components/browser/BrowserOverlayHost.svelte`
- `src/lib/shell/components/browser/BrowserExpandedOverlay.svelte`

The surfaces include accessible names, kebab-case test ids, viewport presets, Back/Forward/Reload/address controls, capability-gated Import/Devtools/Profile/Settings controls, Grab/Annotate/Draw controls, screenshot markup tool labels, feedback review/copy/stage/remove actions, and the dock/floating/maximized/collapsed affordances.

### Tests

- `scripts/browserModel.test.mjs`
- `scripts/browserBounds.test.mjs`
- `scripts/browserAnnotations.test.mjs`
- `scripts/browserFeedback.test.mjs`
- `scripts/browserBackend.test.mjs`

## Controller-owned wiring receipt (not applied in this lane)

The following is the exact intended wiring diff for the controller-owned seams. It is recorded here because `ShellOverlays.svelte`, `ShellFrame.svelte`, and `src/routes/next/+page.svelte` were explicitly out of scope.

```diff
diff --git a/src/lib/shell/components/ShellOverlays.svelte b/src/lib/shell/components/ShellOverlays.svelte
@@
 import NewSessionHost from './newSession/NewSessionHost.svelte';
 import PalettePanel from './PalettePanel.svelte';
 import SettingsHost from './SettingsHost.svelte';
+import BrowserOverlayHost from './browser/BrowserOverlayHost.svelte';
+import WorkbenchActionFab from './WorkbenchActionFab.svelte';
@@
 interface Props {
@@
+  browserWorkspace: BrowserWorkspaceState;
+  browserActions: WorkbenchAction[];
+  workbenchActionContext: WorkbenchActionContext;
 }
@@
 <NewSessionHost bind:this={newSessionHost} onStart={onStartNewSession} />
+<BrowserOverlayHost
+  workspace={browserWorkspace}
+  onExpand={browserActions.expand}
+  onPresentation={browserActions.setPresentation}
+  onMinimize={browserActions.minimize}
+  onCollapse={browserActions.collapse}
+/>
+<WorkbenchActionFab actions={browserActions} context={workbenchActionContext} />
```

```diff
diff --git a/src/lib/shell/components/ShellFrame.svelte b/src/lib/shell/components/ShellFrame.svelte
@@
-  // No overlay is mounted in ShellFrame. The single center browser slot stays:
-  center={{ session: sessionArea, editor: editorArea, browser: browserArea, ... }}
+  // Unchanged: ShellFrame owns the one docked BrowserPanel slot only.
+  center={{ session: sessionArea, editor: editorArea, browser: browserArea, ... }}
```

```diff
diff --git a/src/routes/next/+page.svelte b/src/routes/next/+page.svelte
@@
 import ShellOverlays from '$lib/shell/components/ShellOverlays.svelte';
+import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';
@@
 <ShellOverlays
@@
   onProblemsLocationChange={applyProblemsLocation}
+  {browserWorkspace}
+  browserActions={workbenchBrowserActions}
+  workbenchActionContext={deriveWorkbenchActionContext()}
 />
```

The Browser entries in `workbenchBrowserActions` route only to existing model/service functions: activate/expand, restore-to-dock, minimize-to-previous, collapse-to-control, reload, Grab, Annotate, Draw, and open-external. Session, Editor, Resources, History, and other action entries remain existing command/service routes; the FAB owns layout/focus/confirmation, not browser business logic. The context must carry the current owner/workspace/target/generation so stale feedback cannot be staged.

## Verification receipts

All five new scripts passed serially:

```text
browserModel: all tests passed
browserBounds: all tests passed
browserAnnotations: all tests passed
browserFeedback: all tests passed
browserBackend: all tests passed
```

Required legacy checks also passed:

```text
normalizeBrowserUrl: all tests passed
sessionWorkspaces: all tests passed
panelActivation: all tests passed
```

`pnpm check` passed (`tsc --noEmit`). `pnpm check:svelte` passed with `Files the /next shell owns: 0 error(s), 0 warning(s)`; its reported 16 errors are explicitly outside the gate in the existing old-shell backlog.

## Limitations and security notes

- Native `browser.rs`, bounded inspector, WKWebView snapshot/crop, z-order/input proof, and real native navigation remain pending the separate Rust lane. `BrowserViewport` is therefore a native-host presentation placeholder and the fake backend supplies deterministic bytes only.
- Real rebuilt-Tauri acceptance still needs three tabs, same/cross-origin inspector behavior, profile A/B isolation, native viewport sizes, drag/resize, Draw restore-without-reload, and complete named-process cleanup.
- No cookies, page storage, form values, full page markup, tokens, or provider screenshots are modeled or captured.
- No SQL/database was touched.

## Deviations

No deviations from the approved A9 frontend contract.

Repository note: I did not run `git commit`. During the lane the branch already advanced to `c9cd25c` (`tsk-808: A9 browser presentation model, feedback tools, and components`); the remaining working-tree edits are intentionally uncommitted. Unrelated concurrent `tauriSource.ts`/workflow files were preserved.
