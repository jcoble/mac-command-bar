# Workbench Restructure — Implementation Plan

Date: 2026-08-13. Implements `docs/superpowers/specs/2026-08-13-workbench-restructure-design.md`.
Wave worktree: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave` (branch `tsk-808-assembly-wave`).
All paths below are relative to `tauri-svelte-preview/` unless they start with `core/`, `docs/`, or `scripts/` at the repo root.

## Goal

Restructure the `/next` workbench into three columns with one icon tab strip over the right panel and small corner tabs on the center pane, then rebuild all eight right-side panels on shared kit primitives so the whole app looks and behaves as one product.

## Architecture

The shell keeps its dockview Gridview for column geometry but loses two regions: the far-right icon rail (`activity`) is deleted, and the right column's per-view Paneview machinery is replaced by a plain always-mounted panel host that shows one of eight panels. The center Dockview shrinks from six panels to three (Session, Editor, Diff) driven by new corner tabs. Panels never import each other: every cross-panel action (open a file in the editor, open a diff, open a URL in the browser, hand an attachment to the composer, start a session) goes through one registered-handler module, `workbenchNavigation.ts`, which follows the same register/call pattern the codebase already uses for `registerStackHandlers`.

Wave 1 (Task 0) lands the geometry, the kit primitives, the navigation module, and re-hosts today's existing panel content into the new slots so the app is fully working. Wave 2 (Tasks 1–8) rebuilds each panel in parallel, each lane owning exactly one directory.

## Tech stack

Svelte 5 (runes: `$state`, `$derived`, `$props`, `$effect`), SvelteKit, TypeScript, Tailwind v4 (config-less, `@theme inline` in `src/lib/shell/styles/next.css`), shadcn-style vendored kit in `src/lib/components/ui/`, `@lucide/svelte` ^1.17.0 icons, dockview-core for geometry, Tauri v2 (Rust) backend in `src-tauri/` plus the `mcb_core` crate at `core/`.

---

## Global Constraints

Every lane must read this section. It is pasted verbatim from the design spec §7, plus the kit and code rules that bind all lanes.

### Gates

- `pnpm run check:svelte` ends `Files the /next shell owns: 0 error(s), 0 warning(s)`.
- Rust lanes: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test <module>` green, `cargo fmt -- --check` clean, one cargo at a time.
- Script tests via `node --experimental-strip-types`; update guards they own; never touch the known-red `nextTokens.test.mjs` allowlist or `conversationActivation.test.mjs`.
- Never bind port 5177; lane vite on 5181/5182/5183 only; sandbox usually blocks sockets — attempt once, report, move on; controller verifies visually at 1710×990 (verified via innerWidth eval).
- No commits/pushes by lanes; plain commit messages by whoever commits; no co-author trailers; no AI vendor/model mentions in code, comments, or reports; never log tokens/credentials/prompt text.
- Worktrees under `/Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>`, node_modules symlinked from the wave worktree, symlink removed before `git worktree remove`, removal verified with `git worktree list`.
- Reports first: every lane writes its receipt-report file before doing anything else; every claim carries file:line or command output, labelled verified/assumed.

### Code rules

- TypeScript only. `any` is a last resort and needs a comment saying why.
- **No `:has()` selectors and no `has-[` Tailwind utilities.** Reveal row actions with the `group` / `group-hover` pattern the kit already uses.
- 12px minimum font anywhere a person reads text. `scripts/checkSvelteNext.mjs:132-174` enforces this and fails the gate.
- CSS grid/flex layouts. No magic-number absolute positioning except where an overlay genuinely demands it.
- Every animation ends at rest — finite duration, no infinite loops, opacity and transform only, and skipped under `motion-safe`.
- Plain-English UI copy. No invented jargon, no product codenames in anything a person reads.
- Shadcn kit only for controls. No panel-local buttons, rows, headers, chips, or empty states.
- Read `src/lib/components/ui/DESIGN.md` before styling anything. It is the binding kit contract: which color slot means what, the 13px content floor (`text-sm` is 12px and `text-xs` is 11px inside `/next`, not Tailwind's 14/12), the 8/12/16px radius scale, the 4/8/12/16 spacing scale, and the four things every interactive control owes (24px+ hit target, visible focus ring, hover state, disabled state).
- Colors come from shadcn slots (`bg-card`, `text-muted-foreground`, `border-input`) which `src/lib/shell/styles/next.css:132-241` maps onto the shell tokens in `src/lib/shell/styles/nextTokens.css`. `nextTokens.css` is a closed list — `scripts/nextTokens.test.mjs` fails the build if a token is added that no component reads. Reach for an existing token before inventing one.
- All git shelling in Rust uses `Command::new("git").args([...])`. Never a joined shell string.
- Any new `#[tauri::command]` must be added to the `tauri::generate_handler![...]` list at `src-tauri/src/main.rs:5678-5842`, or the existing registration test at `src-tauri/src/main.rs:8732` fails.

### File ownership

Wave 2 lanes own only the directories named in their task. These files are **frozen** after Wave 1 — a lane that needs one changed reports it and the merge agent or controller applies it:

- `src/routes/next/+page.svelte`
- `src/lib/components/ui/**` (the kit)
- `src/lib/shell/workbenchNavigation.ts`
- `src/lib/shell/layout/workbenchTabs.ts`
- `src/lib/shell/components/RightPanel.svelte`, `RightPanelTabs.svelte`, `CenterCornerTabs.svelte`, `UtilityStrip.svelte`, `SettingsGearButton.svelte`
- `src/lib/shell/components/SessionRail.svelte`, `WorktreeAgentRow.svelte`, `SessionsColumn.svelte`, `SessionHoverCard.svelte` (the recently rebuilt left rail — do not restyle)
- `src/lib/shell/components/EditorPanel.svelte` and `src/lib/shell/components/editor/**` (internals)
- `src/lib/shell/components/conversation/**`
- `src/lib/shell/components/ShellFrame.svelte`, `src/lib/shell/layout/**` (except `workbenchTabs.ts`, which is also frozen)

---

## Task 0 — Wave 1 shell lane (sequential, lands before everything else)

One implementer. Nothing in Wave 2 starts until this is merged into `tsk-808-assembly-wave`.

### Goal

Land the new geometry, the shared kit primitives, the navigation module, and tab persistence, and re-host every existing panel's content into its new slot so the app is fully working at the end of this task — just with today's panel content in tomorrow's arrangement.

### Current state (verified anchors)

- `src/routes/next/+page.svelte` is 1640 lines. Markup runs 1374–1640. Region snippets: `sessionsArea` 1380-1392, `toolsArea` 1393-1405, `activityArea` 1406-1412, `dockArea` 1413-1415, `sessionArea` 1416-1431, `editorArea` 1432-1448, `browserArea` 1449-1454, `diffArea` 1459, `sessionLibraryArea` 1460-1471, `agentsArea` 1472-1479. `<main class="next-shell">` 1481-1573. Top bar 1485-1492 (`<RunButton />` 1486, `<SessionBrowserButton />` 1487, `<LanguageIntelligenceControls />` 1491). `<ShellFrame>` 1495-1541. `<ShellOverlays>` 1544-1560. `<BottomBar />` 1572. Style block 1575-1640.
- `src/lib/shell/components/ShellFrame.svelte` — `Props` at :28-67, controls object at :48-65, parking stage at :197-218, `tools-region` grid at :250-254. Region ids come from `src/lib/shell/layout/frame.ts:30` (`'sessions' | 'center' | 'tools' | 'activity' | 'dock'`); `ACTIVITY_STRIP_WIDTH = 44` at `frame.ts:38`; the activity region is added at `frame.ts:229-236`.
- `src/lib/shell/components/CenterActivityDock.svelte` is the **far-right icon rail** to delete. Rendered at `ShellFrame.svelte:208-209`. Surface group :36-47, utility group :97-122 (Resources, Usage) dispatching `RAIL_UTILITY_REQUEST_EVENT` from `src/lib/shell/components/railUtilityEvents.ts:1`.
- `src/lib/shell/components/ActivityBar.svelte` is the **old right-side tab row** to delete. Rendered at `+page.svelte:1406-1412`, placed by `ShellFrame.svelte:205`. Its roster is `SIDEBAR_VIEWS` at `src/lib/shell/layout/sidebarViews.ts:71-79`; settings gear at `ActivityBar.svelte:81-91`.
- `src/lib/shell/components/ShellSidebar.svelte` hosts today's right-panel content: `PANES` map at :89-97, pane component imports at :55-61, `onReady` controls at :106-110.
- Center Dockview: `src/lib/shell/layout/centerDock.ts`, `CENTER_PANEL_IDS` at :77-85 (`session`, `editor`, `browser`, `diff`, `session-library`, `agents`), default layout `buildDefault` at :194-218. Layout keys at `src/lib/shell/layout/layoutStorage.ts:28,36,44`.
- Settings opens through `ShellOverlays.svelte:132-137` (`export function openSettings()`) → `SettingsHost.svelte` `open()` at :55-84 → lazy `SettingsDialog.svelte`.
- `src/lib/shell/resources/BottomBar.svelte` is the Resources strip currently at the page root (`+page.svelte:1572`). `ResourcePopover.svelte` and `UsagePopover.svelte` are mounted by `ShellOverlays.svelte:167,174` and driven by the rail utility events.
- `src/lib/shell/openFileBus.ts` — `OpenFileRequest` at :10-19, `requestOpenFile` at :26, `onOpenFile` at :35. This is the existing file-open path; `EditorPanel` subscribes, `ExplorerPanel` and `GitDiffView` produce.
- `src/lib/shell/shellPanels.ts:124-152` — the panel activation registry (`editor`, `git`, `browser`, `explorer`, `context`, `worktrees`, `stacks`, `problems`) and `readSelection()` at :84-97 (`rail.activeOwnedId` → `rail.owned` → `folderFor(session)` → `{ root, projects }`).
- Session store: `src/lib/shell/stores/sessionRailStore.svelte.ts:11-23` — `rail.owned`, `rail.available`, `rail.activeOwnedId`.
- Kit: `src/lib/components/ui/`. `buttonVariants` at `button/variants.ts:14-40`; `icon-sm` = `size-7` = 28px at `variants.ts:32`. `HoverActions` / `HoverActionButton` already exist at `hover-actions/` — **do not recreate them**, consume them.

### Files created

**Kit primitives** (in `src/lib/components/ui/`, matching the existing folder + `index.ts` convention; DESIGN.md §"Divergences" already precedents app-owned components living here alongside `icon-button` and `segmented-control`):

1. `src/lib/components/ui/panel-header/panel-header.svelte` + `index.ts`, exporting `PanelHeader` (and `Root`).
2. `src/lib/components/ui/list-row/list-row.svelte` + `index.ts`, exporting `ListRow` (and `Root`).
3. `src/lib/components/ui/chip/chip.svelte` + `chip/variants.ts` + `index.ts`, exporting `Chip`, `chipVariants`, `type ChipTone`.
4. `src/lib/components/ui/empty-state/empty-state.svelte` + `index.ts`, exporting `EmptyState` (and `Root`).

**Shell components** (in `src/lib/shell/components/`):

5. `RightPanel.svelte` — the right column host: tab strip on top, one always-mounted panel body per tab, utility strip at the bottom.
6. `RightPanelTabs.svelte` — the eight-icon tab strip.
7. `CenterCornerTabs.svelte` — the Session/Editor/Diff corner tabs.
8. `UtilityStrip.svelte` — the slim always-visible Resources/Usage strip at the bottom of the right panel.
9. `SettingsGearButton.svelte` — the lower-left settings gear.

**Modules**:

10. `src/lib/shell/workbenchNavigation.ts` — cross-panel navigation.
11. `src/lib/shell/layout/workbenchTabs.ts` — tab ids, defaults, and per-session persistence.

**Panel host stubs** (one directory per Wave 2 lane, each containing the re-hosted existing content so the app works at wave end):

12. `src/lib/shell/panels/files/FilesPanel.svelte`
13. `src/lib/shell/panels/sourceControl/SourceControlPanel.svelte`
14. `src/lib/shell/panels/worktrees/WorktreesPanel.svelte`
15. `src/lib/shell/panels/run/RunPanel.svelte`
16. `src/lib/shell/panels/context/SessionContextPanel.svelte`
17. `src/lib/shell/panels/agents/AgentsPanel.svelte`
18. `src/lib/shell/panels/browser/BrowserPanel.svelte`
19. `src/lib/shell/panels/history/HistoryPanel.svelte`

**Tests**:

20. `scripts/workbenchNavigation.test.ts`
21. `scripts/workbenchTabs.test.ts`
22. `scripts/workbenchGeometry.test.mjs`

### Files modified

- `src/routes/next/+page.svelte` — remove the `RunButton` and `SessionBrowserButton` imports (:35, :36) and their markup (:1486, :1487); remove the `ActivityBar` import (:26) and the `activityArea` snippet (1406-1412); replace the `ShellSidebar` import (:33) and `toolsArea` snippet (1393-1405) with `RightPanel`; move `BottomBar` (import :172, markup :1572) into `UtilityStrip`; drop `browser`, `sessionLibrary`, and `agents` from the `center` prop object (1497-1504); add `CenterCornerTabs` and `SettingsGearButton`; register the workbench navigation handlers.
- `src/lib/shell/components/ShellFrame.svelte` — delete the `CenterActivityDock` import (:26) and the `activity-region` slot (:208-210); remove `activity` from `Props` (:42) and from the `regions` object (:112); trim the `center` Props object (:31-38) to `{ session, editor, diff }`; remove the `tools-tabs` row from `.tools-region` (:205, :250-254) since the tab strip now lives inside `RightPanel`.
- `src/lib/shell/layout/frame.ts` — remove `'activity'` from `ShellRegionId` (:30), delete `ACTIVITY_STRIP_WIDTH` (:38) and the activity region creation block (:229-236). Bump `GRID_LAYOUT_KEY` in `src/lib/shell/layout/layoutStorage.ts:28` from `mac-command-bar.next.grid-layout-v2` to `mac-command-bar.next.grid-layout-v3` so a stored four-region layout cannot restore a region that no longer exists.
- `src/lib/shell/layout/centerDock.ts` — reduce `CENTER_PANEL_IDS` (:77-85) to `['session', 'editor', 'diff']`; update `buildDefault` (:194-218) to open `session` alone in the conversation group with `editor` and `diff` stacked in the display group; bump `CENTER_LAYOUT_KEY` (`layoutStorage.ts:44`) to `mac-command-bar.next.center-layout-v6` and drop the v4 migration block.
- `src/lib/shell/shellPanels.ts` — add a `history` and an `agents` loader alongside the existing eight so all eight right tabs can gate their loads through `panelActivation.ts` the way `source-control`, `worktrees`, `stacks`, and `context` already do.
- `src/lib/shell/components/ShellOverlays.svelte` — keep `openSettings()` (:132-137) exactly as it is; keep `ResourcePopover` (:167) and `UsagePopover` (:174) mounted but re-point their anchor source from `railUtilityEvents` to props passed by `UtilityStrip`.
- `src/lib/shell/components/conversation/ConversationComposer.svelte` and `ConversationSurface.svelte` — add a programmatic focus path (see "Composer focus" below).

### Files deleted

- `src/lib/shell/components/ActivityBar.svelte` (the old right tab row).
- `src/lib/shell/components/CenterActivityDock.svelte` (the far-right icon rail).
- `src/lib/shell/components/ShellSidebar.svelte` (replaced by `RightPanel.svelte`).
- `src/lib/shell/components/run/RunButton.svelte` (moves into the Run panel; Task 4 rebuilds the run affordance).
- `src/lib/shell/browser/SessionBrowserButton.svelte` (the History panel replaces it).
- `src/lib/shell/layout/paneStack.ts` and `src/lib/shell/layout/sidePaneRegistry.ts` — **only if** `git grep -n "paneStack\|sidePaneRegistry" -- src scripts` shows no importer other than `ShellSidebar.svelte:43` and the two tests below. Verified importers today: `sidePaneRegistry.ts:10`, `ShellSidebar.svelte:43`, `scripts/paneLayout.test.mjs:8`, `scripts/layoutStorage.test.mjs:16`. If deleted, also delete `scripts/paneLayout.test.mjs` and remove the `test:pane-layout` script from `package.json`, and remove the `paneStack` imports from `scripts/layoutStorage.test.mjs`.
- `src/lib/shell/components/ContextPanel.svelte` is **not** deleted here — Task 0 re-hosts it into the Context tab so nothing is lost mid-wave. Task 5 deletes it.

### Existing tests that assert the old structure and must be updated

These currently encode the geometry being replaced. Update each to assert the new structure; do not weaken an assertion to make it pass.

- `scripts/centerDock.test.mjs:13,17,84,227` — reads `CenterActivityDock.svelte` and `ActivityBar.svelte` and asserts the `activity-region` slot markup and `<SessionBrowserButton />` in the top bar.
- `src/lib/shell/components/editor/editorPanelLanguageServer.test.ts:69` — asserts `/<RunButton \/>/` in the top bar.
- `scripts/menuOpenSweep.test.ts:17` — reads `src/lib/shell/components/run/RunButton.svelte`.
- `scripts/sidebarViews.test.mjs` — covers `SIDEBAR_VIEWS` and `ACTIVE_VIEW_KEY`, both superseded by `workbenchTabs.ts`.
- `scripts/layoutStorage.test.mjs:16` and `scripts/paneLayout.test.mjs:8` — see the deletion rule above.
- `scripts/sessionRowActions.test.ts:446` asserts the rail has no `paneStack`; that assertion stays true and needs no change.

### Interfaces

**Produces — `src/lib/shell/workbenchNavigation.ts`.** This is the exact public API. Wave 2 lanes spell these names identically.

```ts
import type { OpenFileRequest } from './openFileBus.ts';
import type { ConversationAttachment } from './conversation/conversationTypes.ts';

export type CenterTabId = 'session' | 'editor' | 'diff';
export type RightTabId =
  | 'files' | 'source-control' | 'worktrees' | 'run'
  | 'context' | 'agents' | 'browser' | 'history';

export const CENTER_TAB_IDS: readonly CenterTabId[];
export const RIGHT_TAB_IDS: readonly RightTabId[];

export interface OpenDiffRequest {
  /** Repository root the file belongs to. */
  projectRoot: string;
  /** Path relative to `projectRoot`. */
  relativePath: string;
}

export interface OpenUrlRequest {
  url: string;
}

export interface ComposerHandoff {
  ownedId: string;
  attachments?: ConversationAttachment[];
  /** Appended to the composer's current draft, on its own line. */
  appendText?: string;
}

export interface StartSessionRequest {
  prompt: string;
  cwd: string;
  projectPath: string;
  title: string;
  provider?: 'codex' | 'claude';
}

export interface WorkbenchNavigationHandlers {
  showCenterTab(id: CenterTabId): void;
  showRightTab(id: RightTabId): void;
  openDiff(request: OpenDiffRequest): void | Promise<void>;
  openUrl(request: OpenUrlRequest): void | Promise<void>;
  focusComposer(handoff: ComposerHandoff): void | Promise<void>;
  startSession(request: StartSessionRequest): Promise<string | null>;
}

/** The page registers the real handlers once, on mount. */
export function registerWorkbenchNavigation(next: Partial<WorkbenchNavigationHandlers>): void;
export function clearWorkbenchNavigation(): void;

/** Callers. Each is a no-op (never a throw) when no handler is registered. */
export function showCenterTab(id: CenterTabId): void;
export function showRightTab(id: RightTabId): void;
/** Calls `requestOpenFile(request)` then `showCenterTab('editor')`. */
export function openFileInEditor(request: OpenFileRequest): void;
/** Calls the registered `openDiff` then `showCenterTab('diff')`. */
export function openDiffForFile(request: OpenDiffRequest): Promise<void>;
/** Calls `showRightTab('browser')` then the registered `openUrl`. */
export function openUrlInBrowser(request: OpenUrlRequest): Promise<void>;
/** Attaches, appends text, moves focus to the composer, and shows the Session tab. */
export function focusComposerWith(handoff: ComposerHandoff): Promise<void>;
/** Creates an owned session at `cwd` and sends `prompt` as its first message. Resolves to the ownedId, or null. */
export function startWorkbenchSession(request: StartSessionRequest): Promise<string | null>;
```

Handler registration in `+page.svelte`, wiring to functions that already exist there:

- `showCenterTab` → sets the page's center tab state and calls `frameControls?.showCenterPanel(id)`.
- `showRightTab` → sets the page's right tab state (which `RightPanel` reads as a prop).
- `openDiff` → `gitService`'s existing selection path, then `showCenterPanel('diff')`. `GitDiffView.svelte` takes no props and reads `gitPanel.selectedDiff` itself (`src/lib/shell/git/gitPanelStore.svelte.ts`), so the handler selects the file in that store rather than passing anything to the view.
- `openUrl` → the browser store's navigate path (`src/lib/shell/browser/browserStore.svelte.ts`).
- `focusComposer` → `setConversationAttachments(ownedId, attachments)` (`src/lib/shell/conversation/conversationStore.svelte.ts:816`), `setConversationDraft(ownedId, draft)` (:1036), then the new composer focus call below.
- `startSession` → the page's existing `startNewSession(request: ThreadStartRequest)` at `+page.svelte:871-918`, with `createNewWorktree: false`, `branch: ''`, and provider/model/effort defaults filled from the page's current provider config.

**Composer focus.** No programmatic focus API exists today — verified: `ConversationComposer.svelte` has no `export function` and no parent binds it; its textarea host is bound internally only at `ConversationComposer.svelte:254` (`bind:this={promptHost}`). Task 0 adds the smallest real path: `ConversationComposer.svelte` gains `export function focus(): void { promptHost?.focus(); }`, `ConversationSurface.svelte` adds `let composer = $state<{ focus(): void } | null>(null)` with `bind:this={composer}` on the composer and its own `export function focusComposer(): void { composer?.focus(); }`, and `+page.svelte` binds the surface and calls that from the `focusComposer` handler.

**Produces — `src/lib/shell/layout/workbenchTabs.ts`.**

```ts
import type { LayoutStorage } from './layoutStorage.ts';
import type { CenterTabId, RightTabId } from '../workbenchNavigation.ts';

export const CENTER_TAB_KEY = 'mac-command-bar.next.center-tab-v1';
export const RIGHT_TAB_KEY = 'mac-command-bar.next.right-tab-v1';
export const DEFAULT_CENTER_TAB: CenterTabId = 'session';
export const DEFAULT_RIGHT_TAB: RightTabId = 'files';

export function isCenterTabId(value: unknown): value is CenterTabId;
export function isRightTabId(value: unknown): value is RightTabId;

/** Both keys hold `Record<ownedId, tabId>`. A null ownedId reads/writes the `''` slot,
 *  which is what a shell with no session selected uses. Anything unreadable, corrupt,
 *  or naming a tab that no longer exists comes back as the default. */
export function readCenterTab(storage: LayoutStorage, ownedId: string | null): CenterTabId;
export function writeCenterTab(storage: LayoutStorage, ownedId: string | null, id: CenterTabId): boolean;
export function readRightTab(storage: LayoutStorage, ownedId: string | null): RightTabId;
export function writeRightTab(storage: LayoutStorage, ownedId: string | null, id: RightTabId): boolean;
export function clearWorkbenchTabs(storage: LayoutStorage): void;
```

`LayoutStorage`, `loadLayout`, `saveLayout`, and `clearLayout` already exist in `src/lib/shell/layout/layoutStorage.ts` — reuse them rather than touching `localStorage` directly.

**Produces — kit primitive APIs.** These are contracts; Wave 2 lanes consume them exactly.

```svelte
<!-- panel-header/panel-header.svelte -->
interface Props {
  title: string;
  /** Renders a `count`-toned Chip beside the title when not null. */
  count?: number | null;
  /** Right-aligned action slot — put IconButtons or a DropdownMenu trigger here. */
  actions?: Snippet;
  /** Optional second line under the title, for a branch name or a path. */
  children?: Snippet;
  class?: string;
  'data-testid'?: string;
}
```

```svelte
<!-- list-row/list-row.svelte -->
interface Props {
  /** The row's content. */
  children: Snippet;
  /** HoverActionButtons. Rendered inside a HoverActions cluster, revealed on hover. */
  actions?: Snippet;
  /** Names the action group for a screen reader, e.g. "Worktree actions". */
  actionsLabel?: string;
  selected?: boolean;
  disabled?: boolean;
  onclick?: (event: MouseEvent) => void;
  class?: string;
  'data-testid'?: string;
}
```

`ListRow` renders `class="group relative"` on its container so `HoverActions`' `group-hover` trigger works, positions the cluster with `class="absolute top-1/2 right-2 -translate-y-1/2"`, and owns the 18px glyph rule for its own cluster:

```css
.row :global([data-slot='hover-actions'] svg) { width: 18px; height: 18px; }
```

This moves glyph sizing into the kit. Today it is owned by the consuming row's stylesheet (`WorktreeAgentRow.svelte:791-792`), which is why panels have drifted. `WorktreeAgentRow.svelte` keeps its own rule and is not touched.

```ts
// chip/variants.ts
export type ChipTone = 'neutral' | 'count' | 'attention' | 'live' | 'good' | 'bad';
export const chipVariants: (props?: { tone?: ChipTone }) => string;
```

```svelte
<!-- chip/chip.svelte -->
interface Props {
  children: Snippet;
  /** `attention` is the needs-you amber; `live`/`good`/`bad` are the shell status colors. */
  tone?: ChipTone;
  class?: string;
  'data-testid'?: string;
}
```

`Chip` reads only tokens already defined in `nextTokens.css` — the same ones `WorktreeAgentRow.svelte` uses for `.needs-you` (:704-713), `.failed` (:723-731), and `.status.working` (:674-687), plus `--color-live`/`--color-live-bg` and `--color-good`/`--color-good-bg` already read by `hover-action-button.svelte:52-53`. Do not add a token to `nextTokens.css`.

```svelte
<!-- empty-state/empty-state.svelte -->
interface Props {
  title: string;
  /** One or two plain sentences saying what will appear here and when. */
  body?: string;
  /** A single lucide icon, rendered at 24px above the title. */
  icon?: Snippet;
  /** Optional call to action. */
  actions?: Snippet;
  class?: string;
  'data-testid'?: string;
}
```

**Produces — `RightPanel.svelte` contract.** Wave 2 lanes do not modify this file; they only need to know that their panel component is mounted with these props:

```ts
interface PanelProps {
  /** True while this panel's tab is the selected one. Gate loads and polling on it. */
  visible: boolean;
  /** The active session's working folder, or '' when nothing is selected. From `readSelection().root`. */
  root: string;
  /** The active session's ownedId, or null. From `rail.activeOwnedId`. */
  ownedId: string | null;
}
```

Every panel component in `src/lib/shell/panels/*/` takes exactly these three props and nothing else. Anything else it needs, it reads from a store or a service.

**The eight tabs.** Order, ids, labels, and icons are fixed here so `RightPanelTabs.svelte` and every lane agree:

| Order | `RightTabId` | Label | Icon (`@lucide/svelte/icons/…`) | Panel component |
|---|---|---|---|---|
| 1 | `files` | Files | `files` | `panels/files/FilesPanel.svelte` |
| 2 | `source-control` | Source control | `git-branch` | `panels/sourceControl/SourceControlPanel.svelte` |
| 3 | `worktrees` | Worktrees | `layers` | `panels/worktrees/WorktreesPanel.svelte` |
| 4 | `run` | Run | `play` | `panels/run/RunPanel.svelte` |
| 5 | `context` | Context | `activity` | `panels/context/SessionContextPanel.svelte` |
| 6 | `agents` | Agents | `bot` | `panels/agents/AgentsPanel.svelte` |
| 7 | `browser` | Browser | `globe-2` | `panels/browser/BrowserPanel.svelte` |
| 8 | `history` | History | `history` | `panels/history/HistoryPanel.svelte` |

Tab buttons are `IconButton size="sm"` (28px, `icon-sm`) with 18px glyphs and the label as tooltip, matching the settled button standard. The strip is a flex row with 4px gaps and a bottom hairline; the selected tab carries `aria-current="page"` and a filled `bg-secondary` background.

**Center corner tabs.** `CenterCornerTabs.svelte` renders three small tabs in the center pane's upper-right corner: Session (`messages-square`), Editor (`file-code-2`), Diff (`git-compare-arrows`). Icon plus short label, `Button size="xs"` (24px) so the strip stays out of the way, selected tab filled. It takes `activeId: CenterTabId` and `onSelect(id: CenterTabId): void`.

**Utility strip.** `UtilityStrip.svelte` is a 28px flex row pinned to the bottom of `RightPanel`, holding two buttons: Resources and Usage. Each opens its existing surface as a popover anchored to its own button — `ResourcePopover.svelte` and `UsagePopover.svelte` stay the surfaces, and the `railUtilityEvents.ts` window-CustomEvent hop is deleted along with the rail that needed it. `BottomBar.svelte`'s live summary line (RSS, CPU, process count, from `resourceSampleStore.svelte`) moves into the Resources button's label so the strip still shows the numbers at a glance. Delete `src/lib/shell/components/railUtilityEvents.ts` and `src/lib/shell/resources/BottomBar.svelte` once `UtilityStrip` carries their behavior.

**Settings gear.** `SettingsGearButton.svelte` is an `IconButton size="sm"` with the `settings` glyph, pinned to the lower-left corner of the app below the session rail, calling the page's `overlays?.openSettings()`. It takes one prop: `onOpenSettings(): void`.

### Implementation steps

1. Write the receipt report file first, per Global Constraints.
2. Write the three failing tests (see below) and confirm each fails for the right reason.
3. Build the four kit primitives. Read `DESIGN.md` first and follow its color, type, spacing, and hit-target rules.
4. Build `workbenchTabs.ts` and `workbenchNavigation.ts`.
5. Build `RightPanelTabs.svelte`, `UtilityStrip.svelte`, `RightPanel.svelte`, `CenterCornerTabs.svelte`, `SettingsGearButton.svelte`.
6. Create the eight panel directories, each re-hosting today's content unchanged so nothing regresses:
   - `FilesPanel.svelte` renders the existing `ExplorerPanel.svelte`.
   - `SourceControlPanel.svelte` renders the existing `GitPanel.svelte`.
   - `WorktreesPanel.svelte` renders the existing `WorktreeManagerPane.svelte`.
   - `RunPanel.svelte` renders the existing `StacksPane.svelte`.
   - `SessionContextPanel.svelte` renders the existing `ContextPanel.svelte` (the activity dashboard; Task 5 replaces it).
   - `AgentsPanel.svelte` renders the existing `AgentActivityPane.svelte`.
   - `BrowserPanel.svelte` renders the existing `SessionBrowserOverlay.svelte` content, docked rather than overlaid; if docking it cleanly is not achievable inside this task, mount a button that opens the existing overlay and report it — Task 7 owns the real docking.
   - `HistoryPanel.svelte` renders the existing `SessionLibraryWorkspace.svelte` with `placement="right"`.
7. Shrink the center Dockview to three panels and bump both layout keys.
8. Delete the activity region from `frame.ts` and `ShellFrame.svelte`.
9. Rewrite `+page.svelte`'s markup region, register the navigation handlers, and add the composer focus path.
10. Delete the files listed under "Files deleted".
11. Update the existing tests listed above.
12. Run the gates.

### Tests to write (failing first)

**`scripts/workbenchTabs.test.ts`** — pure module, plain node, no compilation needed (it has no runes). Asserts:
- `readCenterTab` and `readRightTab` return the defaults for an empty storage, for unparseable JSON, and for a stored id that is not in the roster.
- A value written for one ownedId does not leak to another.
- `writeCenterTab` returns `false` when the storage throws (a full quota), and reading afterwards still returns a valid id.
- `RIGHT_TAB_IDS` is exactly the eight ids in the table above, in that order.

**`scripts/workbenchNavigation.test.ts`** — pure module. Asserts:
- Every caller is a no-op and does not throw when no handler is registered.
- `registerWorkbenchNavigation` merges partially (registering `showCenterTab` alone leaves the others unregistered).
- `openFileInEditor` calls `requestOpenFile` and then `showCenterTab('editor')`, in that order.
- `openUrlInBrowser` calls `showRightTab('browser')` before `openUrl`.
- `clearWorkbenchNavigation` unregisters everything.

**`scripts/workbenchGeometry.test.mjs`** — source-text guard in the style of `scripts/centerDock.test.mjs`, which already does exactly this. Reads the source files and asserts:
- `src/routes/next/+page.svelte` contains neither `<RunButton` nor `<SessionBrowserButton` nor `<ActivityBar`.
- `ActivityBar.svelte`, `CenterActivityDock.svelte`, and `ShellSidebar.svelte` do not exist on disk.
- `ShellFrame.svelte` has no `activity-region` slot and no `CenterActivityDock` import.
- `frame.ts`'s `ShellRegionId` does not include `'activity'`.
- `centerDock.ts`'s `CENTER_PANEL_IDS` is exactly `['session', 'editor', 'diff']`.
- `RightPanelTabs.svelte` names all eight tab ids.
- No file under `src/lib/shell/panels/` or `src/lib/components/ui/` contains `:has(` or `has-[`.

Register all three in `package.json` as `test:workbench-tabs`, `test:workbench-navigation`, and `test:workbench-geometry`.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/workbenchTabs.test.ts
node --experimental-strip-types scripts/workbenchNavigation.test.ts
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
node --experimental-strip-types scripts/centerDock.test.mjs
node --experimental-strip-types scripts/layoutStorage.test.mjs
node --experimental-strip-types scripts/sessionWorkspaces.test.ts
node --experimental-strip-types scripts/openFileBus.test.mjs
node --experimental-strip-types scripts/nextTokens.test.mjs
node --experimental-strip-types scripts/panelActivation.test.mjs
node --experimental-strip-types src/lib/shell/components/editor/editorPanelLanguageServer.test.ts
node --experimental-strip-types scripts/menuOpenSweep.test.ts
```

Passing looks like: `check:svelte` ends `Files the /next shell owns: 0 error(s), 0 warning(s).`, and each test script exits 0 printing its own `… passed` line.

### Done means

- [ ] Receipt report written before any code, every claim carrying file:line or command output.
- [ ] Four kit primitives exist with the exact props above and are exported from their `index.ts`.
- [ ] `workbenchNavigation.ts` and `workbenchTabs.ts` exist with the exact signatures above.
- [ ] Right panel shows the eight tabs in the specified order; selecting one fills the panel; the selection survives a reload and is remembered per session.
- [ ] Center pane shows Session/Editor/Diff corner tabs; Session is the default; the selection survives a reload and is remembered per session.
- [ ] Resources and Usage open from the bottom strip of the right panel.
- [ ] Settings gear sits in the lower-left corner and opens the existing settings surface.
- [ ] Far-right icon rail, old right tab row, top-bar Run button, and top-bar session-browser button are gone from the page and their files are deleted.
- [ ] Top bar holds only the Language Intelligence controls.
- [ ] Every one of the eight panels shows its existing content and the app is fully usable.
- [ ] All three new tests pass; every listed existing test updated and passing.
- [ ] `check:svelte` clean.
- [ ] No `:has()` or `has-[` anywhere in new code; no font below 12px; no AI vendor or model names in code, comments, or the report.
- [ ] Lane worktree removed (`rm` the node_modules symlink, `git worktree remove`, `git worktree prune`, verified with `git worktree list`) and reported.

---

## Wave 2 — eight parallel panel lanes

Each lane below is a complete cold-start dispatch spec. A lane implementer sees only their task plus Global Constraints. Every lane:

- Writes its receipt report file before doing anything else.
- Owns only the directories named in "Owns". Everything else, especially the frozen list in Global Constraints, is read-only. A lane that needs a frozen file changed writes the exact change into its report and stops touching it.
- Consumes the kit primitives and `workbenchNavigation` from Task 0. Their APIs are reproduced in each task so no lane has to read another task.
- Receives `{ visible, root, ownedId }` as its panel component's props and nothing else.
- Ends with `pnpm run check:svelte` clean plus its own listed tests.

### Shared consumed contracts (identical in every lane)

```ts
// src/lib/components/ui/panel-header/index.js
PanelHeader — { title: string; count?: number | null; actions?: Snippet; children?: Snippet; class?: string }
// src/lib/components/ui/list-row/index.js
ListRow — { children: Snippet; actions?: Snippet; actionsLabel?: string; selected?: boolean; disabled?: boolean; onclick?: (e: MouseEvent) => void; class?: string }
// src/lib/components/ui/chip/index.js
Chip — { children: Snippet; tone?: 'neutral' | 'count' | 'attention' | 'live' | 'good' | 'bad'; class?: string }
// src/lib/components/ui/empty-state/index.js
EmptyState — { title: string; body?: string; icon?: Snippet; actions?: Snippet; class?: string }
// src/lib/components/ui/hover-actions/index.js  (pre-existing — do not recreate)
HoverActions — { children: Snippet; label?: string; class?: string }
HoverActionButton — { label: string; children: Snippet; tone?: 'default' | 'primary' | 'info' | 'success'; size?: 'xs' | 'sm'; disabled?: boolean; onclick?: (e: MouseEvent) => void; class?: string }
// Also available, all pre-existing: Button, IconButton, SegmentedControl, Badge, Card, Collapsible,
// ContextMenu, Dialog, AlertDialog, DropdownMenu, Input, Label, ScrollArea, Select, Separator,
// Slider, Switch, Tabs, Tooltip — all under $lib/components/ui/<name>/index.js.
// NOT available: Popover, Sheet, Textarea. Use DropdownMenu for a popover, Dialog for a sheet,
// and a plain <textarea> styled to match `input.svelte` for multi-line text.

// src/lib/shell/workbenchNavigation.ts
showCenterTab(id: 'session' | 'editor' | 'diff'): void
showRightTab(id: RightTabId): void
openFileInEditor(request: { path: string; projectRoot?: string; line?: number; column?: number }): void
openDiffForFile(request: { projectRoot: string; relativePath: string }): Promise<void>
openUrlInBrowser(request: { url: string }): Promise<void>
focusComposerWith(handoff: { ownedId: string; attachments?: ConversationAttachment[]; appendText?: string }): Promise<void>
startWorkbenchSession(request: { prompt: string; cwd: string; projectPath: string; title: string; provider?: 'codex' | 'claude' }): Promise<string | null>
```

---

## Task 1 — History panel

Implements spec §3.1. Reference screenshots `173.png`–`178.png`.

### Owns

- `src/lib/shell/panels/history/**` (all new work goes here)
- `src/lib/shell/sessionLibrary/**` (existing session-library model, service, store, context menu, and workspace component)
- `src/lib/shell/history/**` (existing history view model)
- `core/src/scanners/sessions.rs` (the session scanner — this lane's Rust)
- `scripts/sessionHistoryPanel.test.ts` (new)

### Frozen

`src/routes/next/+page.svelte`, `src/lib/components/ui/**`, `src/lib/shell/workbenchNavigation.ts`, `src/lib/shell/layout/**`, `SessionRail.svelte`, `WorktreeAgentRow.svelte`, `SessionsColumn.svelte`, `EditorPanel.svelte` and `src/lib/shell/components/editor/**`, `src/lib/shell/components/conversation/**`, and every other lane's `src/lib/shell/panels/*/` directory.

### Current state (verified anchors)

- Data comes from `listAgentSessionsFromTauri()` at `src/lib/tauriSource.ts:1238-1244`, invoking `'list_agent_sessions'` and returning `AgentSession[]` (type at `tauriSource.ts:271-301`).
- The Rust side is `list_agent_sessions` at `src-tauri/src/main.rs:1524-1528`, calling `scan_sessions()` from the `mcb_core` crate. `AgentSessionRecord` is at `core/src/scanners/sessions.rs:39-…`; `scan_sessions()` is at `core/src/scanners/sessions.rs:136`. The scanner walks real `.jsonl` transcript files (`jsonl_files()` at :1547, with the two provider paths gathered at :149-156 and :201-212), so **the log path is available at scan time**.
- `SessionLibraryRecord` at `src/lib/shell/sessionLibrary/sessionLibraryModel.ts:21-45`; `buildSessionLibrary(owned, available)` at :194-197.
- Project grouping already exists and is the richer one to build on: `src/lib/shell/history/sessionHistoryViewModel.ts` — `SessionHistoryProjectGroup` at :41-49 (has a real `count` field), `SessionHistoryWorktreeGroup` at :31-39, `SessionHistoryRow` at :22-29, `buildSessionHistoryViewModel(records, options)` at :245-248, collapse helpers at :319-356, windowed paging at :84-111 (`SESSION_HISTORY_ROW_WINDOW = 25` at :9).
- `SessionLibraryActionHandlers` at `sessionLibraryService.ts:19-25`; `createSessionLibraryService(source, handlers)` at :109-126 returning `SessionLibraryService` (:36-43).
- The current action roster is `sessionContextMenuRoster(options)` at `sessionLibraryContextMenu.ts:96-152`, six entries. `continue-new-session` (:109-114) and `view-log` (:115-120) are hardcoded `enabled: false`.
- `AgentSession` carries no log or transcript path: `git grep -n "logPath\|transcriptPath\|sessionFile" src/lib` returns nothing.
- Generic path helpers already exist: `openPathFromTauri` (`'open_path'`) at `tauriSource.ts:870` and `revealPathFromTauri` (`'reveal_path'`) at :873-874.

### New Rust (this lane's only backend work)

Add a log path to the scanner so View Log / Open Log / Reveal Log / Copy Log Path can be real instead of disabled.

1. In `core/src/scanners/sessions.rs`, add to `AgentSessionRecord` (after `resume_commands` at :48):

```rust
/// Absolute path of the transcript file this record was scanned out of, so the
/// app can open, reveal, or read it. `None` when a record was merged from more
/// than one file and no single path describes it.
#[serde(skip_serializing_if = "Option::is_none")]
pub log_path: Option<String>,
```

2. Populate it at every construction site inside `scan_sessions()` and its helpers, from the `PathBuf` the record was parsed from. `parse_claude_jsonl(input: &str, project_path: &str)` at :754 has no path argument — extend it to `parse_claude_jsonl(input: &str, project_path: &str, log_path: &Path)` and pass the file through from :212. In the merge path (`existing`/`candidate` at :1110-1140), keep the existing record's `log_path` and drop the candidate's, matching how `project_path` is merged at :1110-1111.
3. Mirror the field on the TypeScript side: add `logPath?: string | null;` to `AgentSession` at `src/lib/tauriSource.ts:271-301`, and to `SessionLibraryRecord` at `sessionLibraryModel.ts:21-45` as `logPath: string | null`, populated in `buildSessionLibrary`.

No new `#[tauri::command]` is needed, so the handler registry at `src-tauri/src/main.rs:5678-5842` is untouched.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `HoverActions`, `HoverActionButton`, `Card`, `Collapsible`, `DropdownMenu`, `AlertDialog`, `Button`, `IconButton`, `ScrollArea`.
- Navigation: `openFileInEditor` (for View Log / Open Log), `startWorkbenchSession` (for Continue in New Session), `showCenterTab`.
- Existing: `buildSessionHistoryViewModel`, `createSessionHistoryCollapseState`, `toggleSessionHistoryGroup`, `visibleSessionHistoryRows`, `extendSessionHistoryWindow` from `sessionHistoryViewModel.ts`; `buildSessionLibrary` from `sessionLibraryModel.ts`; `rail.owned` / `rail.available` from `src/lib/shell/stores/sessionRailStore.svelte.ts:11-23`; `openPathFromTauri` and `revealPathFromTauri` from `src/lib/tauriSource.ts:870-874`.

### Produces

- `src/lib/shell/panels/history/HistoryPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/history/SessionHistoryCard.svelte` — one card.
- `src/lib/shell/panels/history/sessionHistoryActions.ts`:

```ts
export type SessionHistoryActionId =
  | 'resume-worktree' | 'continue-new-session' | 'view-log' | 'copy-resume-command'
  | 'open-log' | 'reveal-log' | 'open-working-directory' | 'copy-session-id'
  | 'copy-log-path' | 'delete';

export interface SessionHistoryAction {
  id: SessionHistoryActionId;
  label: string;
  enabled: boolean;
  /** Plain English, shown as the item's tooltip when it is off. Null when enabled. */
  disabledReason: string | null;
  destructive: boolean;
}

/** The nine menu actions plus Delete, in the order spec §3.1 lists them. */
export function sessionHistoryActions(record: SessionLibraryRecord): SessionHistoryAction[];
```

### Behavior

- Cards grouped by project, one count Chip per group header, using `SessionHistoryProjectGroup.count`. Groups collapse; collapse state lives in component `$state` seeded from `createSessionHistoryCollapseState()`.
- Card at rest: title, then one meta line — provider icon, message count, subagent count, age, model id. Use `Chip tone="count"` for counts and `text-sm` (12px) for the meta line.
- Hover actions on the card via `HoverActions`: Resume (`tone="primary"`), Continue in new session, Expand/hide details, and a more-actions `DropdownMenu` trigger.
- Expanded card: an action row (Resume in Worktree, Continue in New Session, View Log), a First Prompt block with a Copy button, Latest Turns (the last few user and agent snippets from `SessionLibraryRecord.latestTurns`), and a Subagents list (name, type Chip, message count, log link).
- More-actions menu, in this order: Resume in Worktree, Continue in New Session, Copy Resume Command, Open Log, Reveal Log, Open Working Directory, Copy Session ID, Copy Log Path, Delete. Delete is destructive and confirms through `AlertDialog`.
- Action backing, all real after the Rust change above:
  - Resume in Worktree → the existing resume path (`SessionLibraryService.resume`).
  - Continue in New Session → `startWorkbenchSession({ prompt: record.firstPrompt ?? '', cwd: record.canonicalCwd, projectPath: record.projectPath ?? record.canonicalCwd, title: record.title })`.
  - View Log → `openFileInEditor({ path: record.logPath })`, which puts the transcript in the center Editor tab.
  - Open Log → `openPathFromTauri(record.logPath)`. Reveal Log → `revealPathFromTauri(record.logPath)`. Open Working Directory → `openPathFromTauri(record.canonicalCwd)`.
  - Copy Resume Command → `record.available?.resumeCommands[0]`. Copy Session ID → `record.nativeSessionId ?? record.key`. Copy Log Path → `record.logPath`.
  - Delete → the existing `SessionLibraryService.delete`.
- Anything whose data is missing on a given record (no `logPath`, no `resumeCommands`) ships **disabled with honest hint text** through `SessionHistoryAction.disabledReason` and is named in the report. `record.firstPrompt` and `record.latestTurns` are documented as staying empty until the backend exposes the full transcript (`sessionLibraryModel.ts:40-42`) — if they are empty at implementation time, render the First Prompt and Latest Turns blocks only when populated and say so in the report rather than fabricating content.
- The old `SessionLibraryWorkspace.svelte` and `SessionContextMenu.svelte` are replaced by this panel. Delete them, along with `sessionLibraryContextMenu.ts`, once nothing imports them; verify with `git grep`.

### Tests

**`scripts/sessionHistoryPanel.test.ts`** (new, failing first):
- `sessionHistoryActions` returns exactly the ten ids above, in that order.
- A record with `logPath: null` returns `view-log`, `open-log`, `reveal-log`, and `copy-log-path` all `enabled: false`, each with a non-empty `disabledReason`.
- A record with a `logPath` returns those four enabled with `disabledReason: null`.
- A record with no `ownedId` returns `delete` disabled.
- `delete` is the only action with `destructive: true`.

**Rust**, in `core/src/scanners/sessions.rs`'s existing `#[cfg(test)] mod tests`:
- `log_path_is_recorded_for_a_scanned_session` — a parsed record carries the file it came from.
- `merged_records_keep_the_first_log_path` — merging two records does not blank the path.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/sessionHistoryPanel.test.ts
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path core/Cargo.toml scanners::sessions
cargo fmt --manifest-path core/Cargo.toml -- --check
```

Passing: `check:svelte` ends `Files the /next shell owns: 0 error(s), 0 warning(s).`; the script test exits 0; cargo prints `test result: ok`; `cargo fmt --check` prints nothing.

### Done means

- [ ] Receipt report written first; every claim carries file:line or command output, labelled verified or assumed.
- [ ] Cards grouped by project with a count chip per group.
- [ ] Card meta line, hover actions, expanded detail (action row, First Prompt with Copy, Latest Turns, Subagents), and the nine-item more-actions menu plus Delete all present.
- [ ] `log_path` added to the scanner, populated, serialized, and surfaced through to `SessionLibraryRecord.logPath`.
- [ ] View Log opens the transcript in the center Editor tab.
- [ ] Every action either works or is disabled with plain-English hint text; the report lists which and why.
- [ ] Built entirely on the kit primitives — no panel-local button, row, header, chip, or empty state.
- [ ] All tests green, `check:svelte` clean, `cargo fmt --check` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names in code, comments, or report.
- [ ] Lane worktree removed and verified with `git worktree list`; reported.

---

## Task 2 — Source Control panel

Implements spec §3.2. Reference screenshots `179.png`–`184.png`.

### Owns

- `src/lib/shell/panels/sourceControl/**`
- `src/lib/shell/git/**`
- `src/lib/shell/components/git/**`
- `src/lib/shell/components/GitPanel.svelte`
- `scripts/sourceControlPanel.test.ts` (new)

### Frozen

Everything in the Global Constraints frozen list, plus `src/lib/shell/components/GitDiffView.svelte` — it is the **center** Diff tab, not part of this panel, and it takes no props (verified: no `$props()` line; it reads `gitPanel.selectedDiff` from `src/lib/shell/git/gitPanelStore.svelte.ts` directly, doc comment says so). This lane may change what is selected in `gitPanelStore`; it may not change `GitDiffView.svelte`.

### Current state (verified anchors)

The backend for this panel already exists in full. **No new Rust commands are required** — the spec's §5 line about adding status/diffstat/log/stage/commit commands is already satisfied. Verified commands, all registered at `src-tauri/src/main.rs:5723-5748` and all using `Command::new("git").args([...])`:

| Frontend wrapper (`src/lib/tauriSource.ts`) | Invoke command | Rust | Returns |
|---|---|---|---|
| `readProjectGitStatusFromTauri(root)` | `project_git_status` | `main.rs:1298-1303` | `ProjectGitStatus \| null` |
| `readSourceGitDiffFromTauri(root, path)` | `read_source_git_diff` | `main.rs:1305-1312` | `SourceGitDiff \| null` |
| `stageGitPathsFromTauri(root, paths)` | `stage_git_paths` | `main.rs:1314-1319` | `GitActionResult \| null` |
| `unstageGitPathsFromTauri(root, paths)` | `unstage_git_paths` | `main.rs:1321-1326` | `GitActionResult \| null` |
| `commitGitRepositoryFromTauri(root, message)` | `commit_git_repository` | `main.rs:1328-1335` | `GitActionResult \| null` |
| `readGitCommitHistoryFromTauri(root, limit)` | `read_git_commit_history` | `main.rs:1358-1368` | `GitCommitHistoryEntry[]` |
| — | `read_git_commit_files` | `main.rs:1448-1458` | per-commit file list |
| — | `read_git_commit_file_diff` | `main.rs:1460-1471` | per-commit file diff |

Types at `src/lib/tauriSource.ts:117-269`:

```ts
export type ProjectGitFileStatus = { relativePath: string; indexStatus: string; worktreeStatus: string; status: string; badge: string; };
export type ProjectGitStatus = { branch: string | null; ahead: number; behind: number; hasUpstream: boolean; files: ProjectGitFileStatus[]; };
export type GitActionResult = { message: string; status: ProjectGitStatus; };
export type SourceGitDiff = { relativePath: string; status: string; diff: string; isBinary: boolean; originalContent: string | null; modifiedContent: string | null; };
export type GitCommitHistoryEntry = { shortSha: string; sha: string; subject: string; author: string; committedAt: string; refs: string; parentShas: string[]; parentCount: number; taskID: string | null; taskSource: string | null; };
```

Existing UI to rebuild on the kit, not to reinvent: `src/lib/shell/components/git/SourceControlPanes.svelte` (222 lines, top-level composition), `ChangesPane.svelte` (588 — stage all at :214-217, commit/amend at :220-226, per-file stage/unstage at :163-169, generated commit message at :267-277), `GraphPane.svelte` (461 — commit timeline, backed by `src/lib/shell/git/gitGraphLanes.ts`), `RepoPane.svelte` (185), `BranchMenu.svelte` (253), `sourceControlContextMenu.ts` (215), `commitSuggestion.ts` (104), `discardConfirm.ts` (116). Service layer: `src/lib/shell/git/gitService.ts` (704), `gitPanelStore.svelte.ts` (361), `gitBackendExtra.ts` (264), `gitCommitFilesService.ts` (252), `gitCommitFilesStore.svelte.ts` (159), `parseUnifiedDiff.ts` (312).

Activation is already wired: `shellPanels.ts:130` — `git: (root) => gitService.activate(root)`.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `HoverActions`, `HoverActionButton`, `Button`, `IconButton`, `ContextMenu`, `DropdownMenu`, `Collapsible`, `ScrollArea`, `Separator`. Use a plain `<textarea>` styled to match `input.svelte` for the commit message box — there is no `Textarea` kit component.
- Navigation: `openDiffForFile({ projectRoot, relativePath })` for clicking a changed file; `openFileInEditor` for "Open" on a file.
- Existing: everything in the table above, plus `gitService`, `gitPanelStore`, `gitGraphLanes.ts`, `commitSuggestion.ts`.

### Produces

- `src/lib/shell/panels/sourceControl/SourceControlPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/sourceControl/ChangedFileRow.svelte`
- `src/lib/shell/panels/sourceControl/CommitTimeline.svelte`
- `src/lib/shell/panels/sourceControl/sourceControlFileMenu.ts`:

```ts
export type SourceControlFileActionId =
  | 'view' | 'copy-path' | 'copy-relative-path' | 'open-in-editor' | 'reveal-in-finder';

export interface SourceControlFileAction {
  id: SourceControlFileActionId;
  label: string;
  enabled: boolean;
  disabledReason: string | null;
}

export function sourceControlFileActions(
  file: ProjectGitFileStatus,
  root: string
): SourceControlFileAction[];
```

- `src/lib/shell/panels/sourceControl/sourceControlSections.ts`:

```ts
export interface SourceControlSection {
  id: 'untracked' | 'changed';
  label: string;
  files: ProjectGitFileStatus[];
}

export interface SourceControlDiffstat {
  filesChanged: number;
  untracked: number;
}

/** Untracked files first, then everything else, each sorted by relativePath. */
export function sourceControlSections(status: ProjectGitStatus | null): SourceControlSection[];
export function sourceControlDiffstat(status: ProjectGitStatus | null): SourceControlDiffstat;
```

### Behavior

- Header via `PanelHeader`: current branch against its base, plus the total diffstat as Chips. The right-side `actions` slot holds a single disabled more-actions `DropdownMenu` trigger reserving space for the follow-up wave, with the hint "Push, pull, and pull requests arrive in a later update."
- Commit message box, then **Stage All** and **Commit** — these two writes only. Wire them to the existing `stageGitPathsFromTauri` and `commitGitRepositoryFromTauri` paths already used at `ChangesPane.svelte:214-226`.
- Two sections, Untracked and Changed, each a collapsible list of `ListRow`s showing the status letter, the path, and the per-file diffstat.
- Clicking a file calls `openDiffForFile({ projectRoot: root, relativePath: file.relativePath })`, which puts the diff in the center Diff tab.
- Per-file context menu: View (same as clicking), Copy Path, Copy Relative Path, Open in editor, Reveal in Finder. Use the existing `openPathFromTauri`/`revealPathFromTauri` wrappers at `tauriSource.ts:870-874`.
- Commit timeline below: subject, author, date; expanding a commit lists its files (via `read_git_commit_files`); hovering a subject shows the full commit message in a `DropdownMenu`-based popover, matching `180.png`.
- **Not in v1** — do not build: push, pull, sync, rebase, fast-forward, Create PR, Change Base Ref, tree view. If existing code for these is reachable from this panel, remove the affordance from the panel; leave the underlying service functions and `src/lib/shell/components/git/pr/**` alone rather than deleting backend capability the follow-up wave needs.

### Tests

**`scripts/sourceControlPanel.test.ts`** (new, failing first):
- `sourceControlSections(null)` returns two sections, both empty.
- Untracked files land in `untracked` and modified files in `changed`, each sorted by path.
- `sourceControlDiffstat` counts files changed and untracked correctly, and returns zeros for a null status.
- `sourceControlFileActions` returns exactly the five ids in order, all enabled for a normal file.
- No action id duplicates.

Also keep green: `scripts/gitPanelStore.test.mjs`, `scripts/gitGraphLanes.test.mjs`, `scripts/gitCommitFiles.test.mjs`, `scripts/gitHistoryPaging.test.mjs`, `scripts/parseUnifiedDiff.test.mjs`, `scripts/commitSuggestion.test.mjs`, `scripts/sourceControlContextMenu.test.ts`, `scripts/gitWorkingTreeActions.test.mjs`, `scripts/gitBranchPicker.test.mjs`, `scripts/gitDiscardConfirm.test.mjs`, `scripts/gitBridge.test.mjs`, `scripts/gitTaskLinks.test.mjs`, `scripts/gitGraphViewModel.test.mjs`. Update any that assert markup this lane changes; do not weaken an assertion to make it pass.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/sourceControlPanel.test.ts
node --experimental-strip-types scripts/gitPanelStore.test.mjs
node --experimental-strip-types scripts/gitGraphLanes.test.mjs
node --experimental-strip-types scripts/gitCommitFiles.test.mjs
node --experimental-strip-types scripts/gitHistoryPaging.test.mjs
node --experimental-strip-types scripts/parseUnifiedDiff.test.mjs
node --experimental-strip-types scripts/commitSuggestion.test.mjs
node --experimental-strip-types scripts/sourceControlContextMenu.test.ts
node --experimental-strip-types scripts/gitWorkingTreeActions.test.mjs
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Header shows branch against base and total diffstat; the more-actions slot exists and is honestly disabled.
- [ ] Commit message box, Stage All, and Commit work against a real repository.
- [ ] Untracked and Changed sections with per-file status letter and diffstat.
- [ ] Clicking a file opens its diff in the center Diff tab.
- [ ] Per-file context menu has all five actions and each works.
- [ ] Commit timeline with expandable per-commit files and a hover commit-message popover.
- [ ] No push/pull/sync/rebase/PR/base-ref affordance anywhere in the panel.
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 3 — Worktrees panel

Implements spec §3.3. Reference screenshot `187.png`.

### Owns

- `src/lib/shell/panels/worktrees/**`
- `src/lib/shell/worktrees/**`
- `src/lib/shell/components/worktrees/**`
- `src/lib/worktreeSafety.ts`
- `scripts/worktreeAgentPrompts.test.ts` (new)

### Frozen

The Global Constraints list. Note especially `src/lib/shell/components/WorktreeAgentRow.svelte` — despite the name it is a **session rail row**, not a worktree row, and it is part of the recently rebuilt left rail. Do not touch it.

### Current state (verified anchors)

- Inventory data: `listWorktrees(root)` at `src/lib/shell/worktrees/worktreesBackend.ts:57-59` → `listProjectWorktreesFromTauri(root)` at `src/lib/tauriSource.ts:1200` → invoke `'list_project_worktrees'` → `ProjectWorktree[]` (TS type at `tauriSource.ts:209-222`; Rust struct at `src-tauri/src/main.rs:250-265`; command at `main.rs:1473-1478`; sync helper `list_project_worktrees_sync` at `main.rs:3286-3330` running `git worktree list --porcelain`).
- The row view model is `WorktreeManagerRow` at `src/lib/shell/worktrees/worktreeManagerRows.ts:84-122`, built by `buildWorktreeManagerRows(input)` at :360-417. It already models everything the buttons need: `folderGone`, `safety`, `lane`, `canRemove`, `blockedReason`, `chips`, `aheadBehind`, `sessions`, and `commands: { audit: string; backup: string; cleanup: string }` (:116-117).
- **The copy-paste blocks to replace** are at `src/lib/shell/components/worktrees/WorktreeDetail.svelte:150-195` — the "Commands to run yourself" heading, a Copy button per command, and a `<pre>` per command. The `commands` array driving them is a `$derived` at `WorktreeDetail.svelte:58-80`, sourced from `row.commands`.
- The three command strings come from `src/lib/worktreeSafety.ts`: `worktreeAuditCommand(worktree)` at :552, `worktreeBackupCommand(worktree, now)` at :588, `worktreeCleanupCommand(worktree, primaryPath)` at :610-613. `WorktreeSafetySummary` is at :7-21 and `buildWorktreeSafetySummary(worktree, options)` at :95-98.
- Service functions already exist and back the real cleanup: `worktreeManagerService.ts` — `activate(input)` :82-94, `refresh()` :97-134, `removeWorktree(path)` :163-178, `clearWorktreeEntry(path, branch)` :194-219, `forceRemoveWorktree(path)` :226-247, `archiveWorktree(path)` :250-265. Backend wrappers: `worktreesBackend.ts` — `archiveWorktree(root, path)` :69-74 and `removeWorktree(root, path, force)` :83-93 (invoke `'remove_project_worktree'`), gated by `readBackendCapabilities()` :102-112 against `WORKTREE_FORCE_REMOVE_CAPABILITY` :42 and `WORKTREE_PRUNE_SINGLE_CAPABILITY` :54.
- Activation is wired at `shellPanels.ts:142-147`.
- **No new Rust is needed.** Archive and remove already exist; the agent spawn is a frontend call.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `HoverActions`, `HoverActionButton`, `Button`, `IconButton`, `Collapsible`, `DropdownMenu`, `AlertDialog`, `ScrollArea`, `Select` (for the filter).
- Navigation: `startWorkbenchSession({ prompt, cwd, projectPath, title })` — this is how each button spawns its agent session. It resolves to the new session's `ownedId`, and the session appears in the left rail like any other.
- Existing: everything in the anchors above.

### Produces

- `src/lib/shell/panels/worktrees/WorktreesPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/worktrees/WorktreeRow.svelte` (rebuilt on `ListRow`). This replaces the existing `src/lib/shell/components/worktrees/WorktreeRow.svelte` (229 lines), which this lane also owns and deletes once nothing imports it — same filename, different directory, and only the new one survives.
- `src/lib/shell/panels/worktrees/worktreeAgentPrompts.ts` — the prompt-template module spec §3.3 calls for:

```ts
import type { WorktreeManagerRow } from '../../worktrees/worktreeManagerRows.ts';

export type WorktreeAgentActionId = 'inspect' | 'archive-and-remove' | 'remove';

export interface WorktreeAgentPrompt {
  /** The session title shown in the left rail, e.g. "Inspect tsk-808-assembly-wave". */
  title: string;
  /** The first message sent to the session. */
  prompt: string;
  /** The worktree folder the session runs in. */
  cwd: string;
  /** The repository the worktree belongs to. */
  projectPath: string;
}

export function worktreeAgentPrompt(
  action: WorktreeAgentActionId,
  row: WorktreeManagerRow,
  primaryPath: string | null
): WorktreeAgentPrompt;

export interface WorktreeAgentAction {
  id: WorktreeAgentActionId;
  label: string;
  enabled: boolean;
  disabledReason: string | null;
  destructive: boolean;
}

/** Inspect, Archive & Remove, Remove — in that order. */
export function worktreeAgentActions(row: WorktreeManagerRow): WorktreeAgentAction[];
```

### Behavior

- Keep the inventory: per-repository worktree list, status chips, and the filter, all rebuilt on `PanelHeader`, `ListRow`, and `Chip`. Nothing about what the list shows changes; only how it is built.
- Replace `WorktreeDetail.svelte:150-195` entirely with three buttons: **Inspect**, **Archive & Remove**, **Remove**.
- Each button calls `startWorkbenchSession(worktreeAgentPrompt(action, row, primaryPath))`. The session shows up in the left rail like any other session and the user watches it work.
- The prompts are the safety mechanism, and each must instruct the agent, in this order, to: inspect the worktree's state first; explain in plain English what it found; warn about anything risky it sees — uncommitted changes, unpushed commits, an unmerged branch, a lock; and only then carry out the requested cleanup and report exactly what it did. The Inspect prompt stops after the explanation and performs no cleanup. The two destructive prompts must say explicitly that the agent must not delete anything it found risky without saying so first.
- Build each prompt from what the row already knows — `row.folderName`, `row.branch`, `row.path`, `row.repo`, `row.safety.reason`, `row.safety.recommendation`, `row.blockedReason`, `row.aheadBehindLabel`, `row.sessionsLabel` — and from the existing command strings `row.commands.audit`, `row.commands.backup`, `row.commands.cleanup` as the concrete steps to consider. Those command strings stop being copy-paste UI and become the substance of the prompt.
- `worktreeAgentActions` disables Archive & Remove and Remove for a primary checkout (`row.isPrimary`) with the reason "This is the repository's main checkout." and for `!row.canRemove` with `row.blockedReason`.
- Never delete dirty or unmerged work silently: the destructive buttons spawn an agent, they do not call `removeWorktree`/`archiveWorktree` directly. Keep the existing service functions — the agent session is what invokes the real cleanup, through the same commands.
- `RemoveWorktreeDialog.svelte`'s type-the-folder-name confirmation stays for the direct path if this lane keeps one; if the three buttons fully replace direct removal, delete the dialog and report it.

### Tests

**`scripts/worktreeAgentPrompts.test.ts`** (new, failing first):
- `worktreeAgentActions` returns exactly `['inspect', 'archive-and-remove', 'remove']` in order.
- A primary-checkout row returns both destructive actions disabled with a non-empty reason; Inspect stays enabled.
- A row with `canRemove: false` returns the destructive actions disabled carrying `row.blockedReason`.
- `worktreeAgentPrompt('inspect', row, null)` produces a prompt that mentions inspecting and explaining, and does **not** contain the cleanup command.
- `worktreeAgentPrompt('remove', row, primary)` contains the row's path and branch, contains the cleanup command, and contains an explicit instruction to report risky findings before removing anything.
- Every prompt's `cwd` equals `row.path` and its `title` contains `row.folderName`.

Also keep green: `scripts/worktreeManager.test.mjs`, `scripts/worktreeSafety.test.mjs`, `scripts/worktreeCleanupPlan.test.mjs`, `scripts/worktreeCleanupRunbook.test.mjs`.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/worktreeAgentPrompts.test.ts
node --experimental-strip-types scripts/worktreeManager.test.mjs
node --experimental-strip-types scripts/worktreeSafety.test.mjs
node --experimental-strip-types scripts/worktreeCleanupPlan.test.mjs
node --experimental-strip-types scripts/worktreeCleanupRunbook.test.mjs
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Inventory preserved (list, status chips, filter) and rebuilt on the kit primitives.
- [ ] `WorktreeDetail.svelte:150-195`'s copy-paste blocks are gone.
- [ ] Inspect, Archive & Remove, and Remove each spawn a session that appears in the left rail.
- [ ] Each prompt instructs the agent to inspect, explain in plain English, warn about risk, then act — and Inspect does not act.
- [ ] Destructive buttons are disabled with an honest reason for a primary checkout and for a blocked row.
- [ ] Nothing deletes dirty or unmerged work without the agent surfacing it first.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 4 — Run panel

Implements spec §3.4. Reference: the T3 "Add action" top-bar button and its Add Action dialog.

### Owns

- `src/lib/shell/panels/run/**`
- `src/lib/shell/stacks/**`
- `src/lib/shell/components/run/**`
- `src/lib/shell/components/stacks/**`
- `scripts/runActions.test.ts` (new)
- `scripts/stackStore.test.mjs` (existing, extend)

### Frozen

The Global Constraints list. Note that `registerStackHandlers` is called from `+page.svelte:1214-1218`, which is frozen — this lane consumes the registered handlers, it does not change the registration.

### Current state (verified anchors)

Internally this feature is called "stacks"; on screen it is "Run". `stackService.ts`'s own doc comment says renaming the functions would mean renaming the storage keys, so **keep the internal names** and change only what a person reads.

- `StackDefinition` at `src/lib/shell/stacks/stackStore.svelte.ts:71-82`: `{ id, name, script, cwd, env?: StackEnvVar[] }`. `StackEnvVar` at :54-60. `StackDraft` at :639-644. `StackRow extends StackStatus` at :123-129. `StackStatus` at :112-120. `StackState` at :109.
- **Missing fields relative to the target**: no `keybinding`, no `previewUrl`, no run-on-worktree-creation flag, no open-preview flag. Verified across `StackDefinition`, `StackDraft`, and `RunConfigurationDialog.svelte`'s save path (:82-87).
- Persistence keys at `stackStore.svelte.ts:134,137`: `STACK_DEFINITIONS_STORAGE_KEY = 'mac-command-bar.next.stacks.definitions'` and `STACK_RUNS_STORAGE_KEY = 'mac-command-bar.next.stacks.runs'`. Storage is localStorage, not app data.
- Store functions: `addStack(draft)` :684, `updateStack(stackId, draft)` :712, `removeStack(stackId)` :744, `visibleStackRows()` :852, `allStackRows()` :867, `hydrateStacks()` :562, `describeStackProblem(draft)` :651, `parseStackDefinitions` :196, `serializeStackDefinitions` :215, `recordStackStart` :761, `recordStackExit` :784, `deriveStackState` :397, `describeStackState` :436, `buildStackRows` :455.
- Handlers: `StackHandlers` at `stackService.ts:73-80` (`onStartStack`, `onStopStack`, `onSelectSession`), `StackStartRequest` at :61-70, `registerStackHandlers(next)` at :88, `activateStacks(input)` at :117. `startStack`, `stopStack`, and `refreshStacks` are exported from `stackService` and used at `RunButton.svelte:32-34`.
- Starting a run today spawns a **full terminal session** — `onStartStack` at `+page.svelte:933-954` creates an owned session and calls `service.startOwned(owned, host, { runCommandDirectly: true })`.
- Live output infrastructure exists: `src/lib/shell/terminalService.ts` (`tauriTerminalBackend` ~:180-207) with commands `start_terminal_session`, `write_terminal_session`, `resize_terminal_session`, `close_terminal_session`, `read_terminal_session_scrollback`, `list_terminal_sessions`, and the event channel `terminalOutputEvent = 'terminal_output'` at `src/lib/tauriSource.ts:41`, consumed by `listenToTerminalOutput()` at `tauriSource.ts:731-742`.
- Running-process discovery: `listRuntimeContextsFromTauri(projects)` at `tauriSource.ts:1295-1303` → `'list_runtime_contexts'` → `RuntimeContext[]`. This is a **port scan**, not a process registry — `stackService.ts:11-16` says it is read at exactly three moments and nothing polls. Keep that discipline.
- **There is no keybinding-capture control anywhere in the app.** Verified: every `keybind`/`shortcut` hit is display-only (`src/lib/shell/overlay/actionSurfaceModel.ts:32`, `dropdown-menu-shortcut.svelte`, `PalettePanel.svelte:14`). This lane builds one.
- `RunConfigurationDialog.svelte` (219 lines) has Name (:116), Command (:131), Folder (:146), and Environment variables (:162). It is the base to extend.
- **No new Rust is needed.** Persistence stays in localStorage (the spec permits extending the existing store, and it exists); process spawn and output streaming reuse the terminal infrastructure above.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `HoverActions`, `HoverActionButton`, `Button`, `IconButton`, `Dialog`, `Input`, `Label`, `Switch`, `DropdownMenu`, `ScrollArea`.
- Navigation: `openUrlInBrowser({ url })` — called when an action with a preview URL runs and its open-preview toggle is on.
- Existing: the stack store and service functions above, `listenToTerminalOutput`, `readTerminalSessionScrollback`.

### Produces

Extend `StackDefinition` and `StackDraft` at `stackStore.svelte.ts:71-82` and `:639-644` with four optional fields. Optional, because `parseStackDefinitions` (:196) must keep reading every already-saved definition — a stored value that predates these fields is valid and must survive a round trip:

```ts
export interface StackDefinition {
  id: string;
  name: string;
  script: string;
  cwd: string;
  env?: StackEnvVar[];
  /** The key combination that runs this action, as captured, e.g. "Cmd+Shift+R". Absent when none is set. */
  keybinding?: string;
  /** A page to open when this action runs. Absent when the action serves no page. */
  previewUrl?: string;
  /** Run this action automatically whenever a worktree is created. */
  runOnWorktreeCreation?: boolean;
  /** Open `previewUrl` in the Browser panel when this action runs. */
  openPreviewOnRun?: boolean;
}
```

New files:

- `src/lib/shell/panels/run/RunPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/run/RunActionRow.svelte`
- `src/lib/shell/panels/run/AddActionDialog.svelte` — the T3-shaped dialog.
- `src/lib/shell/panels/run/KeybindingField.svelte` — the capture control.
- `src/lib/shell/panels/run/RunningProcessRow.svelte`
- `src/lib/shell/panels/run/keybindingCapture.ts`:

```ts
/** A captured combination, normalized. Modifiers always in this order: Cmd, Ctrl, Alt, Shift. */
export function formatKeybinding(event: KeyboardEvent): string | null;
/** True when the event is only a modifier being held, so the field keeps waiting. */
export function isModifierOnly(event: KeyboardEvent): boolean;
/** Which saved action a keydown should run, or null. First match wins. */
export function matchKeybinding(
  event: KeyboardEvent,
  definitions: readonly StackDefinition[]
): StackDefinition | null;
```

- `src/lib/shell/panels/run/runOutputTail.ts`:

```ts
export const RUN_OUTPUT_TAIL_LINES = 200;
/** Appends a chunk to a tail, keeping at most RUN_OUTPUT_TAIL_LINES lines. */
export function appendOutputTail(tail: readonly string[], chunk: string): string[];
```

### Behavior

- Panel lists the project's saved actions. Each row: name, keybinding `Chip` when set, a one-line command preview, and a run button. Rows come from `visibleStackRows()`, which already filters to the active project.
- **Add action** button in the `PanelHeader` `actions` slot opens `AddActionDialog` with: Name, Keybinding (a capture field), Command, Preview URL (optional), a Switch for "Run automatically on worktree creation", and a Switch for "Open preview automatically when this action runs". Keep the existing Folder and Environment-variables fields from `RunConfigurationDialog.svelte` — they are already saved and dropping them would lose data.
- The keybinding field captures a real key combination: focus it, press the combination, and it displays what was captured. `isModifierOnly` keeps it waiting while only modifiers are down. An Escape clears the field.
- Below the list, running processes with a live output tail plus stop and restart per process. Subscribe to `listenToTerminalOutput` for the run's session and seed the tail from `read_terminal_session_scrollback`; keep at most `RUN_OUTPUT_TAIL_LINES` lines. Unsubscribe when `visible` goes false — nothing polls, matching the discipline at `stackService.ts:11-16`.
- Stop calls the registered `onStopStack(ownedId)`; restart is stop followed by `startStack`.
- When an action with `openPreviewOnRun: true` and a `previewUrl` starts, call `openUrlInBrowser({ url: previewUrl })`.
- `runOnWorktreeCreation` is stored and honored if a worktree-creation hook exists. **It does not** — worktree creation is explicitly out of scope per spec §8, and the new-session pane's "New worktree" stays disabled. So store the flag and label it honestly in the dialog: "Takes effect once worktree creation lands." Report this.
- The top-bar Run button is already deleted by Task 0. This panel is the only place actions run from, plus the keybindings.

### Tests

**`scripts/runActions.test.ts`** (new, failing first):
- `formatKeybinding` normalizes modifier order and returns null for a modifier-only event.
- `isModifierOnly` is true for a bare Shift and false for Shift+R.
- `matchKeybinding` returns the right definition, returns null when nothing matches, and returns the first match when two definitions share a combination.
- `appendOutputTail` keeps at most `RUN_OUTPUT_TAIL_LINES` lines, splits a multi-line chunk correctly, and handles a chunk with no trailing newline.

**`scripts/stackStore.test.mjs`** (existing, extend). It already compiles the runes module through the Svelte compiler — follow that pattern, do not add a bundler:
- A definition saved with `keybinding`, `previewUrl`, `runOnWorktreeCreation`, and `openPreviewOnRun` survives `serializeStackDefinitions` → `parseStackDefinitions` unchanged.
- A definition saved **without** the four new fields still parses, and the fields come back undefined rather than throwing.
- `parseStackDefinitions` rejects a non-string `keybinding` and a non-boolean toggle by dropping the field, not the definition.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/runActions.test.ts
node --experimental-strip-types scripts/stackStore.test.mjs
node --experimental-strip-types scripts/menuOpenSweep.test.ts
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Saved actions list with name, keybinding chip, command preview, and a working run button.
- [ ] Add action dialog has Name, Keybinding capture, Command, Preview URL, and both toggles, and keeps Folder and Environment variables.
- [ ] Keybinding capture works and a captured combination actually runs its action.
- [ ] Running processes show a live output tail with working stop and restart.
- [ ] An action with a preview URL and the open-preview toggle on opens the Browser panel when it runs.
- [ ] `runOnWorktreeCreation` is stored and labeled honestly as not yet taking effect; reported.
- [ ] Definitions saved before this change still load; round-trip test proves it.
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 5 — Context panel

Implements spec §3.5. Read-only in v1.

### Owns

- `src/lib/shell/panels/context/**`
- `src/lib/shell/context/**`
- `src/lib/shell/components/ContextPanel.svelte` (the old activity dashboard — this lane deletes it)
- `scripts/sessionContext.test.ts` (new)

### Frozen

The Global Constraints list, especially `src/lib/shell/components/conversation/**` and `src/lib/shell/conversation/**` — this lane reads the conversation store, it does not change it.

### Current state (verified anchors)

**Name collision, read this first.** `src/lib/shell/components/ContextPanel.svelte` (1196 lines) is **not** a session-context panel. Its own doc comment (:1-13) says it shows six cards about what is going on around the active project — recorded agent runs, running processes, leftover browsers, agent sessions, worktrees, repositories. It is an activity dashboard, backed by `src/lib/shell/context/contextService.ts` (320), `contextStore.svelte.ts` (439), and `contextPanelHooks.svelte.ts` (51). Task 0 re-hosted it into the Context tab only so nothing was lost mid-wave. **This lane replaces it** and deletes it, because everything it shows is now covered by the Resources strip (processes), the Worktrees panel, and the History panel.

The data the real Context panel needs already exists in the conversation store:

- `ConversationMetadata` at `src/lib/shell/conversation/conversationTypes.ts:568-576`: `{ model: string | null; effort: string | null; approvalPolicy: string | null; usedTokens: number | null; contextWindow: number | null }`.
- `ConversationUsage` at `conversationTypes.ts:563-567`: `{ inputTokens?, outputTokens?, usedTokens?, contextWindow? }`.
- Read both with `getConversationSession(ownedId)` at `src/lib/shell/conversation/conversationStore.svelte.ts:156-158`, returning `ConversationWorkspaceState | null` with `metadata` at :73 and `usage` inherited from `ConversationSessionState`.
- `AgentConversationConfigState` and `readAgentConversationConfig(ownedId)` at `src/lib/shell/conversation/conversationConfig.ts:3-18` — `{ model, availableModels, reasoningEffort, availableEfforts, approvalPolicy, availableApprovalPolicies }`. "Access mode" is `approvalPolicy`.
- `ConversationAttachment` at `conversationTypes.ts:600-606`: `{ id, name, mimeType, path, previewUrl }`. A session's current attachments are `getConversationSession(ownedId)?.attachments` (field at `conversationStore.svelte.ts:72`).
- **Files read or touched: no ledger exists.** The only source is the tool-call payload at `conversationTypes.ts:452-468`, whose `path?: string` and `locations?: AgentConfigValue` fields providers populate opportunistically. There is no aggregation anywhere in `src/lib`. This lane builds the derivation (below).
- Events are readable with `listAgentConversationEventsFromTauri(ownedId, fromSequence)` at `src/lib/tauriSource.ts:1269-1276`, returning `AgentConversationEvent[]`.

**No new Rust is needed** — this is read-only aggregation over data the manager already holds, exactly as spec §5 says.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `Card`, `Collapsible`, `ScrollArea`, `Separator`.
- Navigation: `openFileInEditor({ path })` for a row in the files-touched list.
- Existing: `getConversationSession`, `readAgentConversationConfig`, `listAgentConversationEventsFromTauri`.

### Produces

- `src/lib/shell/panels/context/SessionContextPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/context/sessionContextModel.ts`:

```ts
import type { ConversationMetadata, ConversationUsage } from '../../conversation/conversationTypes.ts';

export interface SessionContextFact {
  label: string;
  /** The value to show, already formatted. */
  value: string;
  /** True when the provider did not report this. The view shows "not reported". */
  missing: boolean;
}

export interface SessionContextUsage {
  usedTokens: number | null;
  contextWindow: number | null;
  /** 0-100, or null when either number is missing. Never guessed. */
  percentUsed: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface SessionFileTouch {
  path: string;
  /** How many tool calls referenced this path. */
  count: number;
  /** Milliseconds, of the most recent reference. */
  lastTouchedMs: number;
}

/** Model, effort, and access mode. A null field comes back with missing: true. */
export function sessionContextFacts(metadata: ConversationMetadata | null): SessionContextFact[];

/** Never computes a percentage from a partial pair, and never invents a context window. */
export function sessionContextUsage(
  metadata: ConversationMetadata | null,
  usage: ConversationUsage | null | undefined
): SessionContextUsage;

/** Pulls `path` and `locations` out of toolCall / toolCallUpdate payloads,
 *  de-duplicates by path, and sorts by most recently touched. */
export function sessionFilesTouched(events: readonly AgentConversationEvent[]): SessionFileTouch[];
```

### Behavior

- Four sections, all read-only: **Session** (model, effort, access mode), **Context usage** (used tokens against the context window, with a percentage only when both numbers are real), **Files touched** (a `ListRow` per path, click opens it in the center Editor tab), and **Attachments** (name, type, size where known).
- Where a number is not available for a provider, show the words **"not reported"**. Never invent, never estimate, never fill in a plausible context window. `SessionContextFact.missing` and the nullable fields on `SessionContextUsage` are what make this enforceable, and the tests below check it.
- Files touched is derived, not stored: read the session's events once when the panel becomes visible, run `sessionFilesTouched`, and re-derive on the next visibility change. Do not poll and do not subscribe to the live event stream — the derivation is cheap enough to redo and a subscription would duplicate the conversation store's work.
- Empty states through `EmptyState`: no session selected, a session with no files touched yet, a session with no attachments.
- Delete `src/lib/shell/components/ContextPanel.svelte`, `src/lib/shell/context/contextService.ts`, `contextStore.svelte.ts`, `contextPanelHooks.svelte.ts`, and the `context` entry in `shellPanels.ts:133-141` — but **only after** checking `git grep` for other importers. `shellPanels.ts` is frozen for this lane, so write the exact removal into the report for the merge agent to apply. If the Playwright card activation at `shellPanels.ts:138-140` has no other home, say so in the report rather than deleting it silently: `activatePlaywright()` also serves the Resources strip.

### Tests

**`scripts/sessionContext.test.ts`** (new, failing first):
- `sessionContextFacts(null)` returns three facts, all `missing: true`.
- A metadata with `model` set and `effort` null returns the model with `missing: false` and the effort with `missing: true`.
- `sessionContextUsage` returns `percentUsed: null` when `usedTokens` is present but `contextWindow` is null, and again when the reverse is true.
- `sessionContextUsage` returns a correct percentage only when both are real numbers.
- `sessionFilesTouched` extracts paths from `toolCall` and `toolCallUpdate` payloads, de-duplicates a path referenced three times into one entry with `count: 3`, ignores events with no path, and sorts most-recent-first.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/sessionContext.test.ts
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
node --experimental-strip-types scripts/panelActivation.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Panel shows model, effort, and access mode for the selected session.
- [ ] Token and context usage shown from real reported numbers only, with a percentage only when both numbers exist.
- [ ] Any unavailable number reads "not reported"; nothing is invented.
- [ ] Files touched derived from tool-call events; clicking one opens it in the center Editor tab.
- [ ] Attachments listed.
- [ ] Read-only — no control in this panel changes anything.
- [ ] Old activity-dashboard `ContextPanel.svelte` and its service/store deleted, or the blocker reported with the exact change the merge agent should apply to `shellPanels.ts`.
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 6 — Agents panel

Implements spec §3.6. Reference screenshot `190.png`. Workflow authoring is explicitly out of scope.

### Owns

- `src/lib/shell/panels/agents/**`
- `src/lib/shell/workflows/**` and `src/lib/shell/components/workflows/**` (the existing workflow surfaces this panel supersedes)
- `scripts/agentsPanel.test.ts` (new)

### Frozen

The Global Constraints list, especially `src/lib/shell/components/conversation/**` — including `ConversationAgentTree.svelte`, which stays where it is as the center conversation's own inline strip. This lane builds a separate right-panel view over the same data.

### Current state (verified anchors)

- `ConversationChildAgent` at `src/lib/shell/conversation/conversationTypes.ts:578-585`: `{ childId, parentId, provider, label, state, updatedAtMs }`. **There is no message count and no log path on this type.**
- Store fields on `ConversationWorkspaceState` at `src/lib/shell/conversation/conversationStore.svelte.ts:74-76`: `children: ConversationChildAgent[]`, `selectedChildId: string | null`, `childTimeline: ConversationTimelineEntry[]`.
- `children` is populated **only from a full snapshot** — `conversationStore.svelte.ts:549,789`, inside `applyAgentConversationSnapshot` (`children: snapshot.children`). `'children.updated'` exists as an `AgentEventType` at `conversationTypes.ts:243` but no reducer assigns `.children` from it. So the list refreshes when a snapshot lands, not continuously.
- `childTimeline` exists only for the currently selected child: `setConversationSelectedChild(ownedId, childId)` at `conversationStore.svelte.ts:991-995` clears it, and `applyChildConversationTranscript` at :800-815 fills it only when `current.selectedChildId === childId`. It is fed by `readChildConversationTranscript(input)` at `src/lib/shell/conversation/conversationService.ts:95-107`, invoking `'read_agent_conversation_transcript'`.
- `ConversationTranscriptSnapshot` at `conversationTypes.ts:594-598`: `{ messages, metadata, children }`.
- The existing inline renderer is `src/lib/shell/components/conversation/ConversationAgentTree.svelte` (36 lines), mounted at `ConversationSurface.svelte:374`. It shows a dot, a label, and the state — no count, no log link. It stays.
- The existing right-side agents view is `src/lib/shell/components/workflows/AgentActivityPane.svelte`, and the center agents surface was `WorkflowControlCenter.svelte` (removed from the center by Task 0). This lane replaces both.
- **No new Rust is needed** — spec §5 says no new backend expected for Agents, and anything missing ships disabled with an honest hint.

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `HoverActions`, `HoverActionButton`, `ScrollArea`.
- Navigation: `openFileInEditor` for a log link when one is available; `showCenterTab('session')` when selecting a child should bring the conversation forward.
- Existing: `getConversationSession(ownedId)` at `conversationStore.svelte.ts:156-158`, `setConversationSelectedChild` at :991-995, `readChildConversationTranscript` at `conversationService.ts:95-107`.

### Produces

- `src/lib/shell/panels/agents/AgentsPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/agents/AgentRow.svelte`
- `src/lib/shell/panels/agents/agentActivityModel.ts`:

```ts
import type { ConversationChildAgent, ConversationTimelineEntry } from '../../conversation/conversationTypes.ts';

export type AgentStatus = 'working' | 'done' | 'failed' | 'idle';

export interface AgentActivityRow {
  childId: string;
  label: string;
  status: AgentStatus;
  /** One plain-English line about what this agent is doing, or '' when nothing is known. */
  activity: string;
  /** Message count, or null when it cannot be known for this agent. */
  messageCount: number | null;
  /** Absolute path of a log to open, or null when none is available. */
  logPath: string | null;
  updatedAtMs: number;
}

/** Maps the provider's raw state string onto the four statuses. Unknown states become 'idle'. */
export function agentStatus(state: string): AgentStatus;

/** Rows for one session's children, most recently updated first.
 *  `timelineByChild` supplies a count only for children whose transcript has been read. */
export function agentActivityRows(
  children: readonly ConversationChildAgent[],
  timelineByChild: Readonly<Record<string, readonly ConversationTimelineEntry[]>>
): AgentActivityRow[];
```

### Behavior

- List the selected session's subagents. Each `ListRow`: name/label, a status `Chip` (`live` for working, `good` for done, `bad` for failed, `neutral` for idle), an activity line, a message count when known, and a log link when available.
- **Message count is only knowable for a child whose transcript has been read**, because `ConversationChildAgent` carries no count and `childTimeline` exists only for the selected child (verified above). So: show the count for the selected child, and show nothing — not a zero, not a guess — for the others. `AgentActivityRow.messageCount: null` is what carries that. Selecting a row calls `setConversationSelectedChild` and `readChildConversationTranscript`, which is what makes its count appear.
- **Log link ships disabled with an honest hint** unless a real path is available. `ConversationChildAgent` has no log path today; report this as a disabled-with-hint feature per spec §3.6. The hint text: "This agent's log is not available yet."
- Empty state copy, verbatim from spec §3.6: title **"No agents yet"**, body **"When this thread spawns subagents or runs a workflow, they show up here with live status, activity, and token usage."**
- The list refreshes when a conversation snapshot lands. Do not poll. Note in the report that `'children.updated'` is emitted but not reduced (`conversationTypes.ts:243` with no assignment in the store), so the list is snapshot-driven — this is a real limitation to record, not something to work around inside this lane by writing to a frozen store.
- Workflow authoring is out of scope. Delete `WorkflowControlCenter.svelte` and `AgentActivityPane.svelte` and the workflow surfaces this panel supersedes, but **only** the UI — leave the workflow Tauri commands at `src-tauri/src/main.rs:5784-5793` registered, since the later wave needs them. Verify with `git grep` before each deletion and report anything you could not delete cleanly.

### Tests

**`scripts/agentsPanel.test.ts`** (new, failing first):
- `agentStatus` maps `'active'` to `'working'`, `'failed'` to `'failed'`, a completed state to `'done'`, and an unrecognized string to `'idle'`.
- `agentActivityRows` sorts most recently updated first.
- A child with no timeline entry returns `messageCount: null`, not `0`.
- A child with a timeline of three entries returns `messageCount: 3`.
- Every row returns `logPath: null` given today's `ConversationChildAgent` shape (this test documents the disabled-with-hint state and will need changing when a log path lands).
- `agentActivityRows([], {})` returns an empty array.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/agentsPanel.test.ts
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
node --experimental-strip-types scripts/conversationTimeline.test.ts
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Selected session's subagents listed with label, status chip, activity line, message count when known, and a log link or an honest disabled hint.
- [ ] Selecting an agent reads its transcript and its message count appears.
- [ ] Message count is blank rather than zero for agents whose transcript has not been read.
- [ ] Empty state uses the exact copy from the spec.
- [ ] No workflow authoring anywhere in the panel; workflow Tauri commands left registered.
- [ ] The snapshot-driven refresh limitation is recorded in the report.
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 7 — Browser panel

Implements spec §3.7. Reference screenshots `185.png`, `186.png`. This is the largest Wave 2 lane.

### Owns

- `src/lib/shell/panels/browser/**`
- `src/lib/shell/browser/**`
- `scripts/browserAnnotationComposite.test.ts` (new)
- `scripts/browserPanelBounds.test.ts` (new)

### Frozen

The Global Constraints list, especially `src/lib/shell/components/conversation/**`. This lane hands attachments to the composer through `focusComposerWith` and never touches the composer itself.

### Current state (verified anchors)

- The browser is a **native Tauri view positioned by bounds**, not a DOM iframe. `BrowserBackend` interface at `src/lib/shell/browser/browserBackend.ts:50-66`; input types at :3-48; `createBrowserBackend()` at :289-291 picking `TauriBrowserBackend` natively or `InMemoryBrowserBackend` otherwise. All commands route through `invokeBrowserCommandFromTauri<T>` at `src/lib/tauriSource.ts:916-926`.
- Rust side: commands registered at `src-tauri/src/main.rs:5826-5841`; `set_browser_tab_bounds` at `src-tauri/src/browser.rs:1212-1225`; `capture_browser_viewport` at `browser.rs:1316-1321` returning `BrowserMarkupCapture` (`browser.rs:143-149`: `mime_type`, `bytes`, `width`, `height`, `source_hash`).
- **Screenshot capture already exists.** TS type `BrowserMarkupCapture` at `src/lib/shell/browser/browserTypes.ts:85-91`: `{ mimeType: string; bytes: readonly number[]; width: number; height: number; sourceHash: string }`.
- `BrowserPresentationMode` at `browserTypes.ts:7` is `'docked' | 'floating' | 'maximized' | 'collapsed'`. Bounds are only pushed for `isExpandedBrowserMode(mode)` — `browserModel.ts:459-477` calls `show_browser_tab` and `set_browser_tab_bounds` only for floating and maximized. **`docked` currently sets no bounds**, so hosting the browser inside the right panel needs its own bounds wiring.
- `clampBrowserFloatingBounds` at `browserBounds.ts:88-109`, with `minWidth = 360`, `minHeight = 240`, `edgeGap = 24` at :28-30. `BrowserFloatingBounds` is `{ x, y, width, height }` in window-space CSS pixels.
- `BrowserInteractionMode` at `browserTypes.ts:9` is `'browse' | 'picking' | 'annotating' | 'drawing'` — **no region mode and no erase mode**. Toolbar ids at `browserChrome.ts:4-28`: `'import' | 'grab' | 'annotate' | 'draw' | 'devtools' | 'external' | 'overflow'` — `grab` is the closest thing to Select; there is no Region and no Erase.
- `SessionBrowserAnnotationLayer.svelte` (303 lines) draws with **absolutely positioned `<div>`s**, not canvas and not SVG — marker boxes at :124-141, drag box at :144-150, pending box at :152-156, pointer drag at :66-89. **It cannot composite annotations onto a captured image**; there is no `drawImage` and no raster export anywhere in the 15 browser files.
- `browserAnnotations.ts` exports bounds/sanitize helpers at :12-15 and :21-37, `createImmutableBrowserFeedbackAttachment` at :61, `selectionFromElementInput` at :117, `pendingMarkupFromInput` at :170, `canAddBrowserAnnotation` at :193. `BrowserFeedbackAttachment` at `browserTypes.ts:106-122` carries `selector`, `accessibleName`, `textSnippet`, `note`, `intent`, `imageAttachmentId`, `sourceHash`.
- Overlay state: `sessionBrowserState.svelte.ts` — `openSessionBrowserOverlay` :50-52, `closeSessionBrowserOverlay` :54-56, `toggleSessionBrowserOverlay` :58-64, `setSessionBrowserAnnotateMode` :77.
- **The element-picker result is never received.** `arm_browser_element_picker` at `browser.rs:1299-1305` returns `()` and the pick arrives on the event `BROWSER_ELEMENT_SELECTED_EVENT = "browser-element-selected"` (`browser.rs:30`, emitted at :345-347). `git grep "browser-element-selected" src` returns **zero** frontend matches, while `acceptBrowserElementSelection` exists at `browserModel.ts:532` and is exposed at :872. This lane adds the missing `listen()`.
- Composer attachment contract: `ConversationAttachment` at `conversationTypes.ts:600-606`; `setConversationAttachments(ownedId, attachments)` at `conversationStore.svelte.ts:816`; the save wrapper is `saveConversationClipboardImage(ownedId, file)` at `conversationService.ts:149-160`, invoking `'save_agent_conversation_attachment'` with `{ ownedId, mimeType, bytes }`. `SessionBrowserOverlay.svelte:196-206` already attaches clipboard images — a working precedent.
- **No new Rust is needed.** Capture exists; compositing is client-side per spec §3.7's own fallback.

### Consumes

- Kit: `PanelHeader`, `EmptyState`, `Button`, `IconButton`, `SegmentedControl` (for the four tools), `Input`, `DropdownMenu`, `Tooltip`.
- Navigation: `focusComposerWith({ ownedId, attachments, appendText })` — the one way this panel reaches the composer.
- Existing: the browser backend and model above, `saveConversationClipboardImage`, `clampBrowserFloatingBounds`.

### Produces

- `src/lib/shell/panels/browser/BrowserPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/browser/BrowserToolbar.svelte` — Select / Region / Draw / Erase plus the address field.
- `src/lib/shell/panels/browser/AnnotationCanvas.svelte` — the drawing surface.
- `src/lib/shell/panels/browser/BrowserMiniComposer.svelte` — "Describe the change… / Attach".
- Extend `BrowserInteractionMode` at `browserTypes.ts:9` to `'browse' | 'picking' | 'region' | 'drawing' | 'erasing'`. `'annotating'` folds into `'region'`; migrate every reader.
- `src/lib/shell/panels/browser/annotationComposite.ts`:

```ts
export type AnnotationShape =
  | { kind: 'region'; x: number; y: number; width: number; height: number }
  | { kind: 'stroke'; points: readonly { x: number; y: number }[] }
  | { kind: 'element'; x: number; y: number; width: number; height: number; tag: string };

/** Drops any shape whose id is in `erased`, and any shape smaller than one pixel. */
export function liveShapes(
  shapes: readonly { id: string; shape: AnnotationShape }[],
  erased: ReadonlySet<string>
): { id: string; shape: AnnotationShape }[];

/** Which shape an erase click at (x, y) removes — the topmost hit, or null. */
export function shapeAtPoint(
  shapes: readonly { id: string; shape: AnnotationShape }[],
  x: number,
  y: number
): string | null;

/** Scales shapes from layer coordinates onto the captured image's pixel grid. */
export function scaleShapes(
  shapes: readonly AnnotationShape[],
  layer: { width: number; height: number },
  image: { width: number; height: number }
): AnnotationShape[];

/** Burns the shapes onto the capture with a canvas and returns a PNG File named
 *  after the page, ready for `saveConversationClipboardImage`. */
export function compositeAnnotations(
  capture: BrowserMarkupCapture,
  shapes: readonly AnnotationShape[],
  layer: { width: number; height: number }
): Promise<File>;
```

- `src/lib/shell/panels/browser/browserPanelBounds.ts`:

```ts
/** Converts a host element's rect into window-space bounds for the native view,
 *  clamped by the existing clampBrowserFloatingBounds rules. */
export function boundsForHost(
  rect: { x: number; y: number; width: number; height: number },
  window: { width: number; height: number }
): { x: number; y: number; width: number; height: number };

/** Expanded means: stretch left to `leftEdge` (the session rail's right edge). */
export function expandedBoundsForHost(
  rect: { x: number; y: number; width: number; height: number },
  leftEdge: number,
  window: { width: number; height: number }
): { x: number; y: number; width: number; height: number };
```

- `src/lib/shell/panels/browser/browserAttachmentNote.ts`:

```ts
export interface BrowserAttachmentNote {
  url: string;
  /** The picked element's selector, when Select was used. */
  selector: string | null;
  /** The picked element's tag, e.g. "svg". */
  tag: string | null;
  /** What the user typed in the mini composer. */
  description: string;
}

/** The plain-English block appended to the composer draft alongside the image. */
export function formatAttachmentNote(note: BrowserAttachmentNote): string;
```

### Behavior

- The browser fills the right panel. `BrowserPanel` measures its host element and pushes bounds to the native view whenever the rect changes — mount, panel resize, window resize, tab switch, and expand/collapse. Use a `ResizeObserver` on the host plus a window resize listener; there is no such wiring today. Hide the native view when `visible` is false, otherwise it floats over whatever panel replaced it.
- An **expand** control stretches the view leftward over the center pane up to the session rail's right edge, via `expandedBoundsForHost`. Collapse restores. This is a bounds change, not a new presentation mode — keep using the existing model's floating/maximized path rather than inventing a fifth mode.
- Toolbar with four tools in a `SegmentedControl`: **Select**, **Region**, **Draw**, **Erase**.
  - Select arms the element picker (`arm_browser_element_picker` with `mode: 'grab'`) and highlights the element under the pointer. **Add the missing `listen('browser-element-selected', …)` subscription** and feed it to `acceptBrowserElementSelection` (`browserModel.ts:532`) — this is a real gap, verified as unwired. Show the captured element's tag as a `Chip`.
  - Region drags a rectangle. Draw is freehand. Erase removes the shape under the click via `shapeAtPoint`.
- `AnnotationCanvas.svelte` draws with a real `<canvas>`, replacing the absolutely-positioned `<div>` approach in `SessionBrowserAnnotationLayer.svelte` — a canvas is required anyway to composite, and keeping two drawing systems would guarantee they drift.
- The inline mini composer ("Describe the change… / Attach") does this on Attach: call `capture_browser_viewport`, run `compositeAnnotations` to burn the shapes onto the image, `saveConversationClipboardImage(ownedId, file)` to persist it, then `focusComposerWith({ ownedId, attachments: [saved], appendText: formatAttachmentNote(note) })`.
- **Nothing is sent until the user sends the prompt from the main composer.** This panel attaches and hands over focus; it never sends.
- Delete `SessionBrowserOverlay.svelte`, `SessionBrowserAnnotationLayer.svelte`, and `sessionBrowserState.svelte.ts` once the panel replaces them; `SessionBrowserButton.svelte` is already gone from Task 0. Verify each with `git grep` first and report anything still referenced.

### Tests

**`scripts/browserAnnotationComposite.test.ts`** (new, failing first) — pure functions only, no canvas needed except where noted:
- `liveShapes` drops erased ids and sub-pixel shapes.
- `shapeAtPoint` returns the topmost hit, returns null outside every shape, and hits a stroke within its tolerance.
- `scaleShapes` maps a shape from a 400×300 layer onto an 800×600 image by doubling both axes, and handles a non-uniform aspect ratio without distorting the two axes independently of the image's own ratio.
- `formatAttachmentNote` includes the URL and the description, includes the selector and tag only when present, and never emits an empty labeled line.

**`scripts/browserPanelBounds.test.ts`** (new, failing first):
- `boundsForHost` returns the rect unchanged when it is already inside the window and above the minimums.
- A rect narrower than `minWidth` (360) comes back at the minimum.
- A rect that would overflow the window's right edge is pulled back inside.
- `expandedBoundsForHost` extends `x` left to `leftEdge` and grows `width` by exactly the same amount, leaving `y` and `height` untouched.
- `expandedBoundsForHost` with a `leftEdge` to the right of the rect leaves the rect unchanged rather than producing a negative width.

Also keep green: `scripts/normalizeBrowserUrl.test.mjs`, `scripts/sessionBrowserState.test.ts` (update or delete it if this lane deletes `sessionBrowserState.svelte.ts`; if deleted, remove `test:session-browser-state` from `package.json`).

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/browserAnnotationComposite.test.ts
node --experimental-strip-types scripts/browserPanelBounds.test.ts
node --experimental-strip-types scripts/normalizeBrowserUrl.test.mjs
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Browser fills the right panel and its bounds track the host rect on mount, resize, and tab switch; it hides when the tab is not visible.
- [ ] Expand stretches it leftward to the session rail's edge; collapse restores.
- [ ] Select, Region, Draw, and Erase all work.
- [ ] The `browser-element-selected` event is subscribed and a picked element's tag shows as a chip.
- [ ] Attach burns the annotations onto a captured PNG, saves it, attaches it to the active session's composer with a structured note, and moves focus there.
- [ ] Nothing is sent until the user sends from the main composer.
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 8 — Files panel

Implements spec §3.8. The smallest lane.

### Owns

- `src/lib/shell/panels/files/**`
- `src/lib/shell/explorer/**`
- `src/lib/shell/components/ExplorerPanel.svelte`
- `src/lib/shell/components/explorer/**`
- `scripts/filesPanel.test.ts` (new)

### Frozen

The Global Constraints list, especially `EditorPanel.svelte` — this lane opens files through the existing bus, it does not touch the editor.

### Current state (verified anchors)

- `src/lib/shell/explorer/`: `explorerService.ts` (144), `explorerTree.ts` (143), `explorerStore.svelte.ts` (167). Component `src/lib/shell/components/ExplorerPanel.svelte`, currently mounted at `ShellSidebar.svelte:337` (deleted by Task 0; now hosted by `panels/files/FilesPanel.svelte`).
- **The file-open path to reuse** is `src/lib/shell/openFileBus.ts` — `requestOpenFile(request: OpenFileRequest)` at :26, with `OpenFileRequest` at :10-19 (`{ path, projectRoot?, line?, column? }`). `ExplorerPanel.svelte:45` already imports it. Its doc comment (:5-7) warns that the explorer is the bus's first producer and must **not** subscribe — a listener there would swallow the request the editor is waiting for. Keep that rule.
- Prefer `openFileInEditor` from `workbenchNavigation`, which calls `requestOpenFile` and then brings the Editor tab forward — one call instead of two.
- Active session cwd: `readSelection()` at `src/lib/shell/shellPanels.ts:84-97` resolves `rail.activeOwnedId` → `rail.owned` → `folderFor(session)` (cwd first, then projectPath — `shellPanels.ts:69-77`). The panel receives this as its `root` prop, so it needs no store read of its own.
- Activation: `explorerService.activate(root)` at `explorerService.ts:124-131`, idempotent, wired at `shellPanels.ts:132`.
- The store already supports what the panel needs: filtering via `filterSourceRecords` and a virtualized viewport via `setViewportHeight`/`setScrollTop`.
- **No new Rust is needed.**

### Consumes

- Kit: `PanelHeader`, `ListRow`, `Chip`, `EmptyState`, `IconButton`, `Collapsible`, `ScrollArea`, `Input` (for the filter).
- Navigation: `openFileInEditor({ path, projectRoot })`.
- Existing: `explorerService.activate`, `explorerStore`, `explorerTree.ts`.

### Produces

- `src/lib/shell/panels/files/FilesPanel.svelte` — props `{ visible, root, ownedId }`.
- `src/lib/shell/panels/files/FileTreeRow.svelte`
- `src/lib/shell/panels/files/fileTreeModel.ts`:

```ts
export interface FileTreeNode {
  path: string;
  name: string;
  /** Depth from the root, for indenting. Root children are 0. */
  depth: number;
  isDirectory: boolean;
  /** Directories only: how many entries are inside. */
  childCount: number;
  /** True when git ignores this path. Ignored files render dimmed. */
  ignored: boolean;
}

/** The flat list to render, given which directories are open. Directories first,
 *  then files, each alphabetical, and a closed directory contributes no descendants. */
export function visibleFileTreeNodes(
  nodes: readonly FileTreeNode[],
  expanded: ReadonlySet<string>
): FileTreeNode[];

/** Toggling a directory that is closed opens it, and vice versa. Toggling a file is a no-op. */
export function toggleDirectory(
  expanded: ReadonlySet<string>,
  node: FileTreeNode
): Set<string>;
```

### Behavior

- File tree of the active session's checkout — the `root` prop, which is already the worktree cwd rather than the project path.
- Folders collapse and expand. Expansion state lives in component `$state`; it does not persist in v1.
- Clicking a file calls `openFileInEditor({ path: node.path, projectRoot: root })`, which opens it in the center Editor tab.
- Git-ignored files are **dimmed, not hidden** — pick one and be consistent, per spec §3.8. Dimming means `text-muted-foreground`; ignored files stay clickable.
- Filter field in the `PanelHeader` `actions` slot, backed by the store's existing `filterSourceRecords`.
- Empty states through `EmptyState`: no session selected, and an empty or unreadable folder.
- **No file operations in v1** — no rename, no move, no delete. Do not add a context menu offering them.
- Keep the virtualized viewport the store already supports; a large repository must not render every row.

### Tests

**`scripts/filesPanel.test.ts`** (new, failing first):
- `visibleFileTreeNodes` with nothing expanded returns only depth-0 nodes.
- Expanding one directory reveals its immediate children and no deeper descendants.
- Directories sort before files, and each group sorts alphabetically.
- `toggleDirectory` opens a closed directory and closes an open one, and returns a new Set rather than mutating the input.
- `toggleDirectory` on a file returns an equal set.
- An ignored file is present in the output with `ignored: true` — dimmed, not filtered out.

Also keep green: `scripts/explorerStore.test.mjs`, `scripts/explorerFileIcons.test.mjs`, `scripts/openFileBus.test.mjs`.

### Verification

```
cd tauri-svelte-preview
pnpm run check:svelte
node --experimental-strip-types scripts/filesPanel.test.ts
node --experimental-strip-types scripts/explorerStore.test.mjs
node --experimental-strip-types scripts/explorerFileIcons.test.mjs
node --experimental-strip-types scripts/openFileBus.test.mjs
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
```

### Done means

- [ ] Receipt report written first, claims carrying file:line or command output.
- [ ] Tree shows the active session's checkout and follows the selected session.
- [ ] Folders collapse and expand.
- [ ] Clicking a file opens it in the center Editor tab through `openFileInEditor`.
- [ ] Git-ignored files are dimmed, consistently, and still clickable.
- [ ] Filter works; the viewport stays virtualized.
- [ ] No rename, move, or delete anywhere in the panel.
- [ ] The panel does not subscribe to `onOpenFile` (it is a producer only).
- [ ] Built entirely on the kit primitives.
- [ ] All listed tests green; `check:svelte` clean.
- [ ] No `:has()`/`has-[`, no font under 12px, no AI vendor or model names.
- [ ] Lane worktree removed and verified; reported.

---

## Task 9 — Merge protocol

For the merge agent. These steps are order-independent: any of Tasks 1–8 can be merged in any order, and a lane that is not ready simply waits. Run them once per lane.

### Per-lane steps

**1. Read the lane's receipt report.** It is evidence, not a conclusion. Note every claim labelled "assumed" and every disabled-with-hint feature the lane reported.

**2. Review the diff against ownership.**

```
git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane> diff --stat tsk-808-assembly-wave...HEAD
git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane> diff --name-only tsk-808-assembly-wave...HEAD
```

Every changed path must fall inside that task's "Owns" list. A change to a frozen file is a stop: revert it in the lane, take the exact change the lane wrote into its report, and apply it yourself on the wave branch after the merge. Frozen files are `+page.svelte`, `src/lib/components/ui/**`, `workbenchNavigation.ts`, `layout/**`, `SessionRail.svelte`, `WorktreeAgentRow.svelte`, `SessionsColumn.svelte`, `EditorPanel.svelte` and `components/editor/**`, `components/conversation/**`, `ShellFrame.svelte`, and any other lane's `panels/*/` directory.

**3. Read the diff for correctness.** Specifically check: no `:has(` or `has-[`; no font size below 12px; no panel-local button, row, header, chip, or empty state that should have been a kit primitive; no `any` without a comment; every animation finite; no AI vendor or model name in code, comments, or copy; no token, credential, or prompt text logged; every new `#[tauri::command]` present in the handler list at `src-tauri/src/main.rs:5678-5842`.

**4. Run the gates in the lane worktree**, before merging.

```
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>/tauri-svelte-preview
pnpm run check:svelte
```

Must end `Files the /next shell owns: 0 error(s), 0 warning(s).`

Then the lane's own tests plus the always-affected guards:

```
node --experimental-strip-types scripts/workbenchGeometry.test.mjs
node --experimental-strip-types scripts/workbenchNavigation.test.ts
node --experimental-strip-types scripts/workbenchTabs.test.ts
node --experimental-strip-types scripts/nextTokens.test.mjs
node --experimental-strip-types scripts/panelActivation.test.mjs
```

Plus every test named in that task's Verification block. Never touch the known-red `nextTokens.test.mjs` allowlist and never modify `conversationActivation.test.mjs`.

**5. Rust lanes — one cargo at a time.** Only Task 1 has Rust in this wave (`core/src/scanners/sessions.rs`). If two Rust lanes ever arrive together, sequence them; concurrent rebuilds of the same crate corrupt its `bin`/`obj`.

```
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>
MSBUILDDISABLENODEREUSE=1 RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path core/Cargo.toml scanners::sessions
cargo fmt --manifest-path core/Cargo.toml -- --check
```

`cargo fmt --check` passes clean on the wave branch today, so any output is drift this lane introduced.

**6. Merge into the wave branch.**

```
cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
git merge --no-ff <lane-branch>
```

Conflicts should be impossible given ownership; a conflict means an ownership violation slipped through step 2. Resolve by taking the wave branch's version of the frozen file and re-applying the lane's own directory changes.

Re-run `pnpm run check:svelte` on the wave branch after each merge. Panels are independent, but the gate is not — a lane that passed alone can fail once another lane's file lands.

**7. Remove the lane worktree.** This is disk-critical and is done immediately, not at the end of the batch.

```
rm /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>/tauri-svelte-preview/node_modules
git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave worktree remove /Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>
git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave worktree prune
git -C /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave worktree list
```

Remove the `node_modules` symlink first or `git worktree remove` refuses. Never remove a dirty or unmerged worktree silently — report its path, branch, `git status --short`, and who owns the next action instead.

**8. Browser cleanup.** If the lane opened a Playwright or Chrome session, stop that exact session and its whole process tree — daemon, GPU helpers, renderers. Closing a page does not stop the daemon. Never kill another agent's session; check the session name, parent process, working directory, and elapsed time first if ownership is unclear.

### Report template

```
Lane: <task number and name>
Branch: <lane-branch>

Ownership: clean | violations: <file:line, and the exact change written into the lane's report>
Diff review: <one line — what landed, and anything that needed a fix>
check:svelte: Files the /next shell owns: 0 error(s), 0 warning(s).
Tests: <each command and its result>
Cargo: <command and result, or "no Rust in this lane">
Merged: <merge commit sha>
check:svelte after merge: <result>

Disabled-with-hint features this lane reported:
  - <feature> — <honest hint text> — <why, with file:line>

Assumed claims needing controller verification:
  - <claim> — <file:line the lane gave>

Worktree cleanup: removed <path>
Worktree cleanup: not removed - <path>, <branch>, <dirty status>, <reason>
Browser cleanup: stopped <session-name> (daemon + Chrome helper tree)
Browser cleanup: not stopped - <session-name>, <owner>, <reason>
```

### After all eight lanes

Run the full gate set on the wave branch, then hand back to the controller for the visual pass at 1710×990 (verified via an `innerWidth` eval) against the reference screenshots.

---

## Appendix — spec assumptions that did not hold

Recorded here so no lane rediscovers them.

1. **"New Rust commands as needed for status/log/stage/commit" (§3.2, §5) — already built.** Every command Source Control needs exists and is registered: `project_git_status`, `read_source_git_diff`, `stage_git_paths`, `unstage_git_paths`, `commit_git_repository`, `read_git_commit_history`, `read_git_commit_files`, `read_git_commit_file_diff` (`src-tauri/src/main.rs:1298-1471`, registered at :5723-5733). Task 2 needs no Rust at all.

2. **"A worktree-inspect Rust helper if the panel's current data is insufficient" (§5) — not needed.** `WorktreeManagerRow` (`worktreeManagerRows.ts:84-122`) already models dirty state, unmerged commits, prunability, locks, ahead/behind, sessions, and delete eligibility. Archive and remove backends exist (`worktreesBackend.ts:69-93`). Task 3 needs no Rust.

3. **"Actions persist per project (extend the existing run/actions storage if present; else a new store in app data)" (§3.4) — the store exists but is localStorage, not app data.** `STACK_DEFINITIONS_STORAGE_KEY = 'mac-command-bar.next.stacks.definitions'` (`stackStore.svelte.ts:134`). Task 4 extends it in place rather than moving to the SQLite pattern at `core/src/session_store.rs:94`, because moving storage would need a migration this wave does not have room for. Four fields are genuinely missing and are being added: `keybinding`, `previewUrl`, `runOnWorktreeCreation`, `openPreviewOnRun`.

4. **No keybinding-capture control exists anywhere.** Every `keybinding`/`shortcut` reference is display-only (`src/lib/shell/overlay/actionSurfaceModel.ts:32`, `dropdown-menu-shortcut.svelte`, `PalettePanel.svelte:14`). Task 4 builds one from scratch.

5. **"Data comes from the existing history scanner" (§3.1) — the scanner exists but reports no log path.** `scan_sessions()` at `core/src/scanners/sessions.rs:136` walks real `.jsonl` files but `AgentSessionRecord` (:39) has no path field, and `git grep "logPath\|transcriptPath\|sessionFile" src/lib` returns nothing. Without it, four of the nine spec'd actions (View Log, Open Log, Reveal Log, Copy Log Path) would all ship disabled. Task 1 adds `log_path: Option<String>` — the smallest real implementation — rather than shipping four dead menu items.

6. **Two of the History actions are hardcoded off today.** `continue-new-session` and `view-log` are `enabled: false` regardless of state (`sessionLibraryContextMenu.ts:109-120`), and four more (Copy Resume Command, Open Log, Reveal Log, Open Working Directory, Copy Log Path) are not in the roster at all. Task 1 replaces the roster with `sessionHistoryActions`.

7. **"Full T3-style annotation … Select / Region / Draw / Erase" (§3.7) — only two of the four exist.** `BrowserInteractionMode` is `'browse' | 'picking' | 'annotating' | 'drawing'` (`browserTypes.ts:9`); there is no region mode and no erase mode. Task 7 extends the union.

8. **The existing annotation layer cannot composite.** `SessionBrowserAnnotationLayer.svelte` draws with absolutely positioned `<div>`s (:124-156), not canvas or SVG, and has no `drawImage` or raster export. Spec §3.7 anticipates this ("burn annotations client-side onto a captured image in the app layer") — Task 7 replaces the layer with a real canvas.

9. **The element-picker result is emitted but never received.** `browser.rs:30` defines `BROWSER_ELEMENT_SELECTED_EVENT = "browser-element-selected"`, emitted at :345-347, and `acceptBrowserElementSelection` exists at `browserModel.ts:532` — but `git grep "browser-element-selected" src` returns zero frontend matches. Select cannot work until Task 7 adds the `listen()`.

10. **The browser's `docked` presentation mode sets no bounds.** `browserModel.ts:459-477` pushes bounds only for floating and maximized (`isExpandedBrowserMode`). Hosting the native view inside the right panel needs its own bounds wiring plus a `ResizeObserver`, neither of which exists. Task 7 builds both.

11. **No programmatic composer focus or insert-text API exists.** `ConversationComposer.svelte` has no `export function`, no parent binds it, and its textarea host is bound internally only (`:254`). Task 0 adds the smallest real path so Task 7 can hand off an attachment.

12. **`ContextPanel.svelte` is not a context panel.** It is a 1196-line activity dashboard about the project (:1-13), unrelated to spec §3.5's model/effort/tokens/files/attachments. Task 0 re-hosts it so nothing breaks mid-wave; Task 5 replaces and deletes it. There is also no "files read or touched" ledger anywhere — only opportunistic `path`/`locations` fields on tool-call payloads (`conversationTypes.ts:452-468`), which Task 5 aggregates.

13. **A per-subagent message count is not obtainable for unselected agents.** `ConversationChildAgent` (`conversationTypes.ts:578-585`) has no count field, and `childTimeline` exists only for the currently selected child (`conversationStore.svelte.ts:991-995`). Task 6 shows a count for the selected agent and blank — not zero — for the rest. `ConversationChildAgent` also has no log path, so Agents' log link ships disabled with a hint. Separately, `'children.updated'` is a declared event type (`conversationTypes.ts:243`) that no reducer consumes, so the subagent list is snapshot-driven rather than live.

14. **`HoverActionCluster` already exists** as `HoverActions` / `HoverActionButton` (`src/lib/components/ui/hover-actions/`), documented in `DESIGN.md:78-90`. Task 0 consumes it rather than building the primitive spec §4 names. Conversely, 18px glyph sizing is currently owned by the consuming row's stylesheet (`WorktreeAgentRow.svelte:791-792`), not by any kit component — which is why panels drifted. Task 0 moves that rule into `ListRow`.

15. **`Popover`, `Sheet`, and `Textarea` are not vendored.** Spec §4 says "existing shadcn kit components only", so lanes use `DropdownMenu` where they want a popover, `Dialog` where they want a sheet, and a plain `<textarea>` styled to match `input.svelte`. Do not vendor a new component without running `scripts/nextTokens.test.mjs` against it (`DESIGN.md:110-118`).

16. **The `/next` gate is prefix-based, not an enumerated file list.** `scripts/checkSvelteNext.mjs:40-45` owns `src/lib/shell/`, `src/lib/components/ui/`, `src/routes/next/`, and `src/lib/utils.ts`. Every new file in this plan lands under one of those prefixes, so nothing needs registering — but a file placed anywhere else would silently escape the gate.
