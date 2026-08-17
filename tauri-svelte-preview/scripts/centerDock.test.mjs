/**
 * centerDock.test.mjs — the center surface area and the two tab strips that
 * navigate the workbench.
 *
 * The middle of the shell is three Dockview panels — Session, Editor, Diff —
 * with the corner tabs in the center pane as their only visible navigation. The
 * eight panels on the right are a plain host with an icon strip over it and the
 * Resources/Usage strip under it. None of that is checkable by a type checker,
 * so it is pinned here, in source text.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (relative) => readFileSync(new URL(relative, import.meta.url), 'utf8');

const shellFrame = read('../src/lib/shell/components/ShellFrame.svelte');
const centerDock = read('../src/lib/shell/layout/centerDock.ts');
const centerCornerTabs = read('../src/lib/shell/components/CenterCornerTabs.svelte');
const rightPanel = read('../src/lib/shell/components/RightPanel.svelte');
const rightPanelTabs = read('../src/lib/shell/components/RightPanelTabs.svelte');
const utilityStrip = read('../src/lib/shell/components/UtilityStrip.svelte');
const shellOverlays = read('../src/lib/shell/components/ShellOverlays.svelte');
const assistanceHost = read('../src/lib/shell/assistance/AssistanceHost.svelte');
const shellPage = read('../src/routes/next/+page.svelte');
const shellLayout = read('../src/lib/shell/layout/frame.ts');

// ── The frame builds the parent grid before the center dock ────────────────

const centerDockStart = shellFrame.indexOf('centerDock = createCenterDock(');
const parentLayout = shellFrame.indexOf('frame.layout(gridHost.clientWidth, gridHost.clientHeight);');

assert.notEqual(centerDockStart, -1, 'ShellFrame must create the center Dockview');
assert.notEqual(parentLayout, -1, 'ShellFrame must lay out the parent Gridview');
assert.ok(
  parentLayout < centerDockStart,
  'the parent Gridview must have real bounds before center Dockview restore/build'
);

// ── Three center surfaces, and the diff detaches when it is not on screen ──

assert.match(
  centerDock,
  /export const CENTER_PANEL_IDS = \['session', 'editor', 'diff'\] as const;/,
  'the center dock holds exactly Session, Editor and Diff'
);
assert.match(
  shellFrame,
  /id: 'diff',[\s\S]*?renderer: 'onlyWhenVisible'/,
  'the Monaco Diff surface must detach when another center surface is active'
);
assert.match(
  centerDock,
  /renderer: panel\.renderer/,
  'the center surface model must pass a panel-specific renderer to Dockview'
);
assert.match(
  shellFrame,
  /\.shell-center-dock \.dv-tabs-and-actions-container\)[\s\S]*?display:\s*none/,
  'Dockview’s own horizontal tab headers stay hidden — the corner tabs replace them'
);

// ── One surface at a time: the three are stacked, never side by side ───────

// A fresh dock adds every surface into the group the first one made, so exactly
// one is on screen and the corner tabs are what change which.
assert.match(
  centerDock,
  /addPanelFor\(panel, \{ referencePanel: lead\.id, direction: 'within' \}\)/,
  'every center surface after the first must be stacked into the same group'
);
assert.doesNotMatch(
  centerDock,
  /direction: 'right'/,
  'no center surface may open beside another — that is what showed two at once'
);
assert.doesNotMatch(
  shellFrame,
  /group: '(display|conversation)'/,
  'the center panels no longer choose a side; they all stack'
);
// A layout stored by the older side-by-side geometry has the same three panel
// ids, so the id check alone would restore the split. The group count is what
// rejects it.
assert.match(
  centerDock,
  /dockGroupCount\(stored\) === 1/,
  'a stored center layout is only restored when its surfaces are in one group'
);

// ── The pill tabs float over the dock and cost it no height ────────────────
//
// They used to hold a row of their own above the dock. That row charged every
// surface the same strip of height to answer a question only asked now and
// then, so the group is laid over the pane instead and is invisible until the
// pointer or the keyboard focus is inside it.

assert.match(
  shellFrame,
  /<div class="slot center-region"[\s\S]*?<div class="center-tabs">\{@render centerTabs\(\)\}<\/div>[\s\S]*?<div class="center-dock-host"/,
  'ShellFrame must still mount the tabs inside the center region'
);
assert.match(
  shellFrame,
  /\.center-region \{[\s\S]*?grid-template-rows:\s*minmax\(0, 1fr\);/,
  'the dock gets the whole region; the tabs are laid over it'
);
assert.match(
  shellFrame,
  /\.center-region \{[\s\S]*?--center-pills-reveal:\s*0;[\s\S]*?--center-pills-events:\s*none;/,
  'the region is what decides whether the group is wanted'
);
assert.match(
  shellFrame,
  /\.center-region:hover,\s*\n\s*\.center-region:focus-within \{[\s\S]*?--center-pills-reveal:\s*1;[\s\S]*?--center-pills-events:\s*auto;/,
  'the pointer in the pane, or the keyboard focus inside it, reveals the group'
);
assert.match(
  shellFrame,
  /\.center-tabs \{[\s\S]*?position:\s*absolute;[\s\S]*?justify-content:\s*flex-end;[\s\S]*?pointer-events:\s*none;/,
  'the strip hangs the group off the pane’s upper-right corner and lets every ' +
    'click through to the surface underneath'
);
assert.match(
  centerCornerTabs,
  /opacity:\s*var\(--center-pills-reveal, 0\);[\s\S]*?pointer-events:\s*var\(--center-pills-events, none\);[\s\S]*?transition:\s*opacity \d+ms/,
  'the group reads the region’s two values and fades — one transition, one ending'
);
assert.match(
  centerCornerTabs,
  /setTimeout\([\s\S]*?HOLD_AFTER_SWITCH_MS\)/,
  'a switch holds the group on screen afterwards, on one timer'
);
assert.match(
  centerCornerTabs,
  /onDestroy\(releaseHold\)/,
  'that timer is cancelled when the shell goes away; nothing is left running'
);
for (const id of ['session', 'editor', 'diff']) {
  assert.match(
    centerCornerTabs,
    new RegExp(`id: '${id}'`),
    `the corner tabs must keep the ${id} surface reachable`
  );
}
// Capsules, and the one you are on is the filled one. `aria-current` carries
// that in the markup, so the same attribute a screen reader reads is the one
// the fill is painted from.
assert.match(
  centerCornerTabs,
  /aria-current=\{tab\.id === activeId \? 'page' : undefined\}/,
  'which surface you are on is stated once, on the tab itself'
);
assert.match(
  centerCornerTabs,
  /\.center-pills > :global\(button\) \{[\s\S]*?border-radius:\s*var\(--radius-pill\);[\s\S]*?background:\s*var\(--pill-surface\);/,
  'the tabs are capsules painted from the shared pill tokens'
);
assert.match(
  centerCornerTabs,
  /\.center-pills > :global\(button\[aria-current='page'\]\) \{[\s\S]*?background:\s*var\(--pill-surface-active\);/,
  'the selected capsule is the filled one'
);
assert.doesNotMatch(
  centerCornerTabs,
  /<button\b/,
  'the corner tabs must not hand-roll button controls'
);

// ── The far-right rail and the old tab row are gone ────────────────────────

assert.doesNotMatch(
  shellFrame,
  /activity|CenterActivityDock/,
  'ShellFrame must not keep the deleted far-right rail or its region'
);
assert.doesNotMatch(
  shellLayout,
  /'activity'|ACTIVITY_STRIP_WIDTH/,
  'the frame must not build a region for the deleted far-right rail'
);
assert.match(
  shellFrame,
  /<div class="slot tools-region"[^>]*>\{@render tools\(\)\}<\/div>/,
  'the right column is one region now: the panel host owns its own strips'
);

// ── The right panel: eight tabs, eight always-mounted bodies ───────────────

const RIGHT_TABS = [
  'files',
  'source-control',
  'worktrees',
  'run',
  'context',
  'agents',
  'browser',
  'history'
];
for (const id of RIGHT_TABS) {
  assert.match(
    rightPanelTabs,
    new RegExp(`id: '${id}'`),
    `the right tab strip must offer the ${id} tab`
  );
}
assert.equal(
  rightPanelTabs.indexOf("id: 'files'") < rightPanelTabs.indexOf("id: 'history'"),
  true,
  'Files opens the strip and History closes it'
);
assert.match(
  rightPanelTabs,
  /import \{ IconButton \} from '\$lib\/components\/ui\/icon-button\/index\.js'/,
  'the tab strip must consume the kit IconButton'
);
assert.match(
  rightPanelTabs,
  /label=\{tab\.label\}[\s\S]*?size="sm"[\s\S]*?<Icon class="size-\[18px\]"/,
  'every tab gets its tooltip, accessible label, 28px target and 18px glyph from IconButton'
);
assert.match(
  rightPanelTabs,
  /aria-current=\{tab\.id === activeId \? 'page' : undefined\}/,
  'the selected tab says so to a screen reader'
);
assert.doesNotMatch(
  rightPanelTabs,
  /<button\b/,
  'the icon-only tab strip must not hand-roll button controls'
);

assert.match(
  rightPanel,
  /grid-template-rows:\s*auto minmax\(0, 1fr\) auto;/,
  'the right column is tab strip, panel body, utility strip'
);
assert.equal(
  (rightPanel.match(/class="panel-body"/g) ?? []).length,
  RIGHT_TABS.length,
  'every one of the eight panels stays mounted; only the open one is displayed'
);
assert.match(
  rightPanel,
  /\.panel-body \{[\s\S]*?display:\s*none;[\s\S]*?\}\s*\.panel-body\.showing \{\s*display:\s*block;/,
  'switching tabs only changes which panel is displayed — nothing is torn down'
);
assert.doesNotMatch(
  rightPanel,
  /createPaneStack|Paneview/,
  'the right column is a plain host now, not a stack of collapsible panes'
);

// ── Resources and Usage: a strip at the bottom, surfaces at the page root ──

assert.match(
  utilityStrip,
  /data-testid="utility-resources"[\s\S]*?data-testid="utility-usage"/,
  'the bottom strip holds Resources and Usage, in that order'
);
assert.match(
  utilityStrip,
  /formatResourceBytes[\s\S]*?formatResourceCpu[\s\S]*?processCount/,
  'the live memory, CPU and process count read on the Resources button itself'
);
assert.match(
  utilityStrip,
  /onOpenUtility\(id, utilityAnchorFor\(event\.currentTarget\.getBoundingClientRect\(\)\)\)/,
  'the strip hands the overlay layer the exact rectangle of the button that was pressed'
);
assert.doesNotMatch(
  utilityStrip,
  /ResourcePopover|UsagePopover/,
  'the strip must not own the resource or quota surfaces'
);

assert.match(
  shellOverlays,
  /<ResourcePopover \/>[\s\S]*?<UsagePopover \/>/,
  'the global overlay layer owns both utility state owners'
);
assert.match(
  shellOverlays,
  /\{#if resourceManagerState\.open\}\s*<ResourceManagerPanel \/>/,
  'the Resource Manager is mounted above the layout, not inside the column that opens it'
);
assert.match(
  shellOverlays,
  /export async function openUtility\(id: UtilityId, anchor: UtilityAnchor\)[\s\S]*?await tick\(\);[\s\S]*?popoverTrigger\(id\)\?\.click\(\)/,
  'an anchored request must position first, then toggle the existing Resource or Usage trigger'
);
assert.match(
  shellOverlays,
  /\.rail-popover-host :global\(\.usage-popover \.card\)[\s\S]*?right:\s*calc\(100% \+ 12px\);[\s\S]*?bottom:\s*0/,
  'the quota card opens inward from the requested button rectangle'
);
assert.match(
  assistanceHost,
  /right:\s*calc\(44px \+ 16px\)[\s\S]*?z-index:\s*48/,
  'Assistance keeps its inset and its stacking level'
);

// ── The page: one switch handler, no session work inside it ────────────────

const centerSwitch = shellPage.match(
  /function handleCenterPanelShown\(id: string\): void \{([\s\S]*?)\n  \}/
)?.[1] ?? '';
assert.notEqual(centerSwitch, '', 'the page must own one explicit center-surface switch handler');
assert.doesNotMatch(
  centerSwitch,
  /selectOwned|ensureStructuredConversation/,
  'center-surface switches must never select or ensure a conversation'
);
assert.match(
  shellPage,
  /onCenterPanelShown=\{handleCenterPanelShown\}/,
  'Dockview surface announcements must use the pure center-surface handler'
);
assert.match(
  shellPage,
  /message=\{\[layoutError, centerTab === 'session' \? rail\.error : null\]/,
  'session activation errors must not be rendered over Diff or the editor'
);

// ── The browser lives in its own right-panel tab now ───────────────────────

assert.doesNotMatch(
  shellOverlays,
  /SessionBrowserOverlay|BrowserOverlayHost|BrowserExpandedOverlay/,
  'no window-wide browser overlay remains: the Browser panel hosts the view itself'
);

console.log('centerDock: three center surfaces, corner tabs, eight right tabs, bottom utility strip verified');
