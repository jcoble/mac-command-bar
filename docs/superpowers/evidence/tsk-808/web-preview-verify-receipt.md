# TSK-808 Web Preview Verification Receipt

Status: verification complete; native follow-up explicitly listed below

This receipt records the Playwright verification of `http://localhost:5179/next` against the eight-item punch list in `ui-feedback/PUNCH-LIST.md`, with screenshots and console observations captured below.

## Setup

- Worktree: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave`
- Expected implementation commit: `7a07d8e`
- Preview: `pnpm dev --port 5179` from `tauri-svelte-preview/`
- Browser session: `tsk808-webverify`

## Progress observations

- Initial shell and sidebar render in layered dark grays with no pure-black canvas; the small `Resources`/`Usage` controls and some disabled source-control controls still use visible borders.
- Session History opens from the left-rail `Find a session` row. Header, Local Mac host, Workspace/Project/All tabs, search, grouped rows, inline details, and disabled future-native actions are present. The context-menu DOM is created, but its fixed-position menu is rendered outside the viewport inside Dockview's transformed overlay; this is a proven broken-render case to evaluate for the permitted trivial fix.
- Browser chrome renders a tab strip, URL field, right-side icon actions, floating maximize/minimize controls, overflow menu, and viewport-preset submenu. Embedded page content is unavailable in this web preview and shows the expected structured-clone/native degradation.
- Diff tab and a full center-tab cycle completed without overlapping or duplicated panes.
- Resource Manager, Space, Usage, Stats & Usage, and source-control empty states render cleanly; real process/quota/repository data is absent without the Tauri backend.

## Permitted broken-render fix

The row context menu was proven broken in the web preview before the fix: the menu DOM existed, but Dockview's transformed `.dv-render-overlay` made the fixed-position coordinates relative to that overlay. The measured pre-fix menu rectangle was `x=1349` with `innerWidth=1200`, so the menu was entirely off-screen (captured in `04-session-history-context-menu.png`).

I applied the one permitted trivial fix, uncommitted, in `tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte` (eight changed diff lines): normalize pointer coordinates to the transformed overlay and clamp the menu's left edge to the overlay width. No other source files were changed. After the fix the menu measured `x=891..1107` inside the `1600px` viewport and rendered all six actions; see `21-session-history-context-menu-final.png` and the post-HMR recheck `23-session-history-context-menu-final-hmr.png`. The intermediate `20-session-history-context-menu-fixed.png` records the first coordinate correction before the final clamp.

## Console and degradation capture

- Playwright console capture: `docs/superpowers/evidence/tsk-808/web-preview-console.txt`.
- Result: 19 total messages, 0 errors, 7 warnings. Every warning is the repeated Svelte `binding_property_non_reactive` warning for `bind:this={hosts[view.id]}` in `src/lib/shell/components/ShellSidebar.svelte:324:8`.
- The browser surface displayed `Failed to execute 'structuredClone' on 'Window': #<Object> could not be cloned.` when the embedded page was requested. It was page content, not a console error, and is recorded as expected web-preview/native degradation.
- No `tauri-invoke-unavailable` error was emitted by the isolated page console; the handled browser-only degradation appeared as the structured-clone message above instead.
- The source-control pane degrades cleanly for a stopped/no-agent session: commit-message generation is disabled with `No active agent session is running. Start an agent conversation to generate this message.` and New pull request is disabled. See `22-source-control-readonly.png`.
- The New pull request control stayed disabled under the browser read-only guard, so I did not force-open it through DOM/script state. This preserves the real degradation path; the panel's generation/push/gh behavior is explicitly marked native-only below.

## Screenshot index

- Shell/theme: `01-shell-initial.png` (full shell), `18-theme-sidebar-settings-dialog.png` (sidebar/shell state), with the visible outlined controls noted above.
- Session History: `02-session-history-collapsed.png`, `03-session-history-expanded.png`, `21-session-history-context-menu-final.png`, `23-session-history-context-menu-final-hmr.png`; `04-session-history-context-menu.png` is the pre-fix off-screen failure and `20-session-history-context-menu-fixed.png` is the intermediate fix state.
- Browser chrome: `06-browser-tab-chrome.png`, `07-browser-toolbar-wide.png`, `08-browser-overflow-menu.png`, `09-browser-viewport-submenu.png`; `05-browser-toolbar.png` records the empty embedded-page state.
- Diff/layout: `10-diff-tab.png`, `11-layout-after-tab-cycle.png`.
- Resources/Space: `12-resources-popover.png`, `13-space-view.png`, `14-resource-manager-full.png`.
- Usage/Stats: `15-usage-popover.png`, `16-stats-usage.png`.
- Source control/PR and native resume degradation: `17-source-control-pane.png`, `19-resume-in-worktree-web-degradation.png`, `22-source-control-readonly.png`.

## Verdicts

| Punch-list item | Verdict | One-line evidence |
|---|---|---|
| 1. Houston theme / no contrast-theme shell | PARTIAL-WEB | `01-shell-initial.png`: layered dark grays and no pure-black canvas; visible outlined `Resources`/`Usage` and disabled source-control controls remain. |
| 2. Session History tab and grouped rows | PARTIAL-WEB | `02-session-history-collapsed.png` and `03-session-history-expanded.png`: header, filters, search, grouped rows, inline action/detail blocks render; actual resume/continuation remains backend-bound. |
| 3. Browser tab chrome | PARTIAL-WEB | `07-browser-toolbar-wide.png`, `08-browser-overflow-menu.png`, `09-browser-viewport-submenu.png`: requested chrome and menus render; embedded webview content is unavailable in preview. |
| 4. Resource Manager and Space | PARTIAL-WEB | `12-resources-popover.png`, `13-space-view.png`, `14-resource-manager-full.png`: empty/explanatory states and popover spacing are clean; live PTY attribution and scans require native. |
| 5. Usage popover and Stats & Usage | PARTIAL-WEB | `15-usage-popover.png`, `16-stats-usage.png`: empty quota/analytics states are clean and non-overlapping; live quota ingestion requires native. |
| 6. Session details and context menus | VERIFIED-WEB | `03-session-history-expanded.png` and `23-session-history-context-menu-final-hmr.png`: inline details plus Resume/Continue/View Log/Copy ID/Archive/Delete menu entries are visible, with disabled native-only actions clearly explained. |
| 7. Agent commit message and New pull request flow | NEEDS-NATIVE | `22-source-control-readonly.png`: source-control structure and graceful no-agent state render, but ACP generation, gh availability, push, and PR creation cannot run in web preview. |
| 8. Diff tab center-layout regression | VERIFIED-WEB | `10-diff-tab.png` and `11-layout-after-tab-cycle.png`: Diff and a full center-tab cycle show no overlapping, duplicated, or corrupted panes. |

## NEEDS-NATIVE explicitly

- Real embedded Tauri webview behavior, including page rendering and webview-backed annotation/element-picker/devtools/external actions.
- Real PTY process-tree attribution numbers and Space scanning/reclaim behavior.
- Real provider quota endpoint reads and non-zero local usage ingestion/analytics.
- ACP agent-generated commit messages plus gh-backed push/PR generation and status checks.
- Resume in Worktree terminal startup/reattach and the corresponding live session actions.

Browser cleanup: stopped tsk808-webverify (daemon + Chrome helper tree)
Server cleanup: stopped pnpm dev port 5179
