# TSK-808 browser chrome receipt

## Scope

Browser frontend chrome rework for the `tauri-svelte-preview` shell. No Rust, token definition files, routes, `ShellFrame.svelte`, `centerDock.ts`, resources, or usage modules are in scope.

## Work log

- Receipt created before repository inspection or implementation.
- Replaced the browser tab strip and toolbar composition in the dock and
  expanded overlay; kept the feedback queue in the browser body.
- Added the data-only toolbar/overflow contract and the focused browser chrome
  script tests.
- Added the native browser command adapter through the existing source bridge;
  web preview and Node tests still use the deterministic backend.
- Ran the requested checks sequentially after the final edit.
- Replaced visible markup-letter placeholders with icon-only controls so Import
  remains the only visible toolbar label.

## Before / after intent

- [verified] Before: outlined text controls competed with the URL field and
  feedback tools sat in a separate corner strip. After: tabs sit above one
  compact toolbar, the URL field owns the free space, and feedback actions are
  in that toolbar.
- [verified] Before: viewport/profile/settings controls were spread across the
  row. After: overflow owns profiles, cookies, viewport presets, and Browser
  Settings, with the viewport selection routed through the model/backend.
- [verified] Before: expanded mode had a separate footer minimize affordance.
  After: maximize/minimize controls live at the toolbar's top-right edge in
  both docked and expanded surfaces.

## Files and intent

- `src/lib/shell/components/browser/BrowserTabs.svelte` — [verified] tabs now
  render a globe fallback favicon, title, close control, and a new-tab `+`.
- `src/lib/shell/components/browser/BrowserToolbar.svelte` — [verified]
  one compact row now has navigation, a free-space rounded URL field, Import,
  icon-only picker/annotate/draw/devtools/external actions, overflow, inline
  markup actions, and floating maximize/minimize controls.
- `src/lib/shell/components/browser/BrowserExpandedOverlay.svelte` and
  `src/lib/shell/components/BrowserPanel.svelte` — [verified] both surfaces
  use the same toolbar and no longer mount a separate corner annotation strip.
- `src/lib/shell/components/browser/BrowserFeedbackPanel.svelte` — [verified]
  queue remains present and uses the shell theme tokens for surface, border,
  text, and empty state.
- `src/lib/shell/components/browser/BrowserAnnotationCard.svelte`,
  `BrowserAnnotationToolbar.svelte`, `BrowserOverlayHost.svelte`, and
  `BrowserViewport.svelte` — [verified] browser-owned styling no longer uses
  fallback or literal colors; the legacy annotation component is retained but
  is not mounted by the new composition.
- `src/lib/shell/browser/browserChrome.ts` — [verified] defines the required
  toolbar roster, overflow entries, and viewport menu entries as data.
- `src/lib/shell/browser/browserBounds.ts` — [verified] Laptop is `1024 × 768`,
  matching the requested preset menu.
- `src/lib/shell/browser/browserBackend.ts`,
  `src/lib/shell/browser/browserStore.svelte.ts`, and `src/lib/tauriSource.ts` —
  [verified] the Tauri runtime selects a typed command adapter; viewport
  selection reaches `set_browser_tab_viewport` through the existing IPC bridge.
- `scripts/browserChrome.test.mjs` — [verified] covers action order, icon-only
  roster, profile/cookie/viewport/settings entries, and all requested preset
  dimensions.

## Verification

- [verified] Focused browser scripts passed serially:
  `browserAnnotations`, `browserBackend`, `browserBounds`, `browserChrome`,
  `browserFeedback`, `browserModel`, and `normalizeBrowserUrl`.
- [verified] An earlier final-tree gate run reported `0 error(s), 0 warning(s)`
  for the `/next` shell; its reported `16 error(s)` were the old-shell
  backlog outside this gate.
- [blocked] The latest gate run is now `2 error(s), 2 warning(s)` in concurrent
  `src/lib/shell/sessionLibrary/*` edits (type narrowing at lines 290/301 and
  two 11px styles at lines 523/531); those files are outside this browser lane
  and were preserved. Browser-owned files add no gate errors.
- [verified] The final `pnpm build` completed and wrote the static site.
  Existing route/session accessibility, chunk-size, and dynamic-import
  warnings remain in build output; no browser-chrome build error was reported.
- [verified] `git diff --check` is clean for the current worktree.
- [verified] A literal-color scan over the browser components, model, bounds,
  and bridge found no hex, rgb/rgba, or hsl/hsla colors; styling uses named
  shell tokens.
- [verified] No commit was created.

## Assumed or blocked

- [assumed] Source structure and generated build output match the Orca target;
  a live visual screenshot was not captured in this sandbox.
- [blocked] `pnpm dev --host 127.0.0.1` could not bind (`listen EPERM`). Named
  Playwright session `tsk808-browser-chrome` could not launch Chrome because
  its crashpad process aborted; WebKit/Firefox were not installed. No browser
  daemon remained open after the failed attempts.
- [assumed] Browser Settings is a deliberate stub notice until a profile
  settings callback/native surface is exposed. Import Cookies has the same
  native-profile fallback notice; the menu shape and focus behavior are live.

## Future-native

- [verified] No favicon field exists in the frontend `BrowserTabState`, so the
  tab intentionally degrades to the Lucide globe icon until native favicon
  metadata is available.
- [verified] The native capture command currently returns a typed unsupported
  result in the separate Rust lane; Draw therefore keeps the existing capture
  model contract and does not invent image bytes.
