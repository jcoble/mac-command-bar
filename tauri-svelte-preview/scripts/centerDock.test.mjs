import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const shellFrame = readFileSync(
  new URL('../src/lib/shell/components/ShellFrame.svelte', import.meta.url),
  'utf8'
);
const centerDock = readFileSync(
  new URL('../src/lib/shell/layout/centerDock.ts', import.meta.url),
  'utf8'
);
const centerActivityDock = readFileSync(
  new URL('../src/lib/shell/components/CenterActivityDock.svelte', import.meta.url),
  'utf8'
);
const activityBar = readFileSync(
  new URL('../src/lib/shell/components/ActivityBar.svelte', import.meta.url),
  'utf8'
);
const shellOverlays = readFileSync(
  new URL('../src/lib/shell/components/ShellOverlays.svelte', import.meta.url),
  'utf8'
);
const assistanceHost = readFileSync(
  new URL('../src/lib/shell/assistance/AssistanceHost.svelte', import.meta.url),
  'utf8'
);
const browserOverlayHost = readFileSync(
  new URL('../src/lib/shell/components/browser/BrowserOverlayHost.svelte', import.meta.url),
  'utf8'
);
const shellPage = readFileSync(
  new URL('../src/routes/next/+page.svelte', import.meta.url),
  'utf8'
);
const shellLayout = readFileSync(
  new URL('../src/lib/shell/layout/frame.ts', import.meta.url),
  'utf8'
);

const centerDockStart = shellFrame.indexOf('centerDock = createCenterDock(');
const parentLayout = shellFrame.indexOf('frame.layout(gridHost.clientWidth, gridHost.clientHeight);');

assert.notEqual(centerDockStart, -1, 'ShellFrame must create the center Dockview');
assert.notEqual(parentLayout, -1, 'ShellFrame must lay out the parent Gridview');
assert.ok(
  parentLayout < centerDockStart,
  'the parent Gridview must have real bounds before center Dockview restore/build'
);

assert.match(
  shellFrame,
  /id: 'diff',[\s\S]*?renderer: 'onlyWhenVisible'/,
  'the Monaco Diff surface must detach when another center surface is active'
);
assert.match(
  shellFrame,
  /\{\s*id: 'browser',\s*title: 'Browser',\s*element: browserSlot,\s*group: 'display',\s*renderer: 'onlyWhenVisible'\s*\}/,
  'the browser HTML host must detach when another center surface is active'
);
assert.match(
  centerDock,
  /renderer: panel\.renderer/,
  'the center surface model must pass a panel-specific renderer to Dockview'
);

assert.match(
  shellFrame,
  /\.tools-region[\s\S]*?grid-template-rows:\s*40px minmax\(0, 1fr\)/,
  'the right tool region must reserve a horizontal picker row above its pane'
);
assert.match(
  shellFrame,
  /\.shell-center-dock \.dv-tabs-and-actions-container\)[\s\S]*?display:\s*none/,
  'the former horizontal center tab headers must be hidden'
);
assert.match(
  shellFrame,
  /<div class="slot tools-region"[\s\S]*?<div class="tools-tabs">\{@render activity\(\)\}<\/div>[\s\S]*?<div class="tools-pane">\{@render tools\(\)\}<\/div>/,
  'ShellFrame must mount the horizontal view picker directly above the right tool pane'
);
assert.match(
  shellFrame,
  /<div class="slot activity-region"[^>]*>[\s\S]*?<CenterActivityDock activeId=\{activeCenterPanel\} onSelect=\{selectCenterPanel\}/,
  'ShellFrame must mount the center surface rail in the far-right activity region'
);
assert.match(
  shellFrame,
  /selectCenterPanel\(id: CenterPanelId\)[\s\S]*?centerDock\?\.activatePanel\(id\)/,
  'dock clicks must use the existing centerDock activation state machine'
);

for (const id of ['session', 'editor', 'browser', 'diff', 'session-library', 'agents']) {
  assert.match(
    centerActivityDock,
    new RegExp(`id: '${id}'`),
    `the far-right rail must keep the ${id} surface reachable`
  );
}
assert.match(
  centerActivityDock,
  /label: 'Session History'/,
  'the dock must keep the user-facing Session History label'
);
assert.match(
  centerActivityDock,
  /import \{ IconButton \} from '\$lib\/components\/ui\/icon-button\/index\.js'/,
  'the surface rail must consume the kit IconButton'
);
assert.match(
  centerActivityDock,
  /label=\{surface\.label\}[\s\S]*?size="default"[\s\S]*?side="left"[\s\S]*?<Icon class="size-5"/,
  'every surface icon must get its tooltip, accessible label, 32px target, and 20px glyph from IconButton'
);
assert.doesNotMatch(
  centerActivityDock,
  /ResourcePopover|UsagePopover/,
  'Dockview must not own the resource or quota overlay mounts'
);
assert.match(
  centerActivityDock,
  /label="Resources"[\s\S]*?label="Usage and Stats"/,
  'the two rail utilities must have explicit IconButton labels'
);
assert.match(
  centerActivityDock,
  /requestUtility\(id: RailUtilityId, event: MouseEvent\)[\s\S]*?RAIL_UTILITY_REQUEST_EVENT[\s\S]*?railUtilityRequest\(id, event\.currentTarget\.getBoundingClientRect\(\)\)/,
  'rail utility IconButtons must dispatch an anchored request to the global overlay owners'
);
assert.match(
  centerActivityDock,
  /label="Usage and Stats"[\s\S]*?size="default"[\s\S]*?onclick=\{\(event\) => requestUtility\('usage', event\)\}[\s\S]*?<ChartNoAxesCombined class="size-5"/,
  'the Usage and Stats entry must use the 32px rail control and request the quota popover'
);
assert.match(
  centerActivityDock,
  /\.surface-group,[\s\S]*?\.utility-group[\s\S]*?gap:\s*8px/,
  'surface and utility buttons must retain an 8px vertical rhythm'
);
assert.match(
  centerActivityDock,
  /\.surface-action,[\s\S]*?\.utility-action[\s\S]*?width:\s*32px;[\s\S]*?height:\s*32px/,
  'surface and utility wrappers must match the 32px IconButton target'
);
assert.match(
  centerActivityDock,
  /variant=\{surface\.id === activeId \? 'secondary' : 'ghost'\}[\s\S]*?\.surface-action\.active::before[\s\S]*?width:\s*3px/,
  'the selected surface must combine a filled button treatment with a visible edge marker'
);
assert.doesNotMatch(
  centerActivityDock,
  /<button\b/,
  'the icon-only surface rail must not hand-roll button controls'
);
assert.match(
  activityBar,
  /<Tabs\.List variant="line"[\s\S]*?<Tabs\.Trigger[\s\S]*?value=\{view\.id\}/,
  'the right-pane picker must use the kit tabs line variant'
);
assert.doesNotMatch(
  activityBar,
  /ResourcePopover|UsagePopover|utility-group/,
  'right-pane tabs must contain only pane navigation and settings'
);
assert.match(
  shellLayout,
  /id: 'activity',[\s\S]*?direction: 'right', referencePanel: 'center'[\s\S]*?id: 'tools',[\s\S]*?direction: 'right', referencePanel: 'center'/,
  'the surface rail must be added outside the right tool panel'
);
assert.doesNotMatch(
  shellFrame,
  /\.shell-region-host-activity[\s\S]*?overflow:\s*visible/,
  'the quota overlay must not depend on escaping Dockview clipping'
);
assert.match(
  assistanceHost,
  /right:\s*calc\(44px \+ 16px\)[\s\S]*?z-index:\s*48/,
  'Assistance must sit one rail width left and below the rail stacking level'
);
assert.match(
  shellOverlays,
  /<ResourcePopover \/>[\s\S]*?<UsagePopover \/>/,
  'the global overlay layer must own both rail utility state owners'
);
assert.match(
  shellOverlays,
  /handleRailUtilityRequest[\s\S]*?isRailUtilityRequest\(event\.detail\)[\s\S]*?openRailUtility\(event\.detail\)/,
  'the global overlay layer must validate and handle each rail utility request'
);
assert.match(
  shellOverlays,
  /async function openRailUtility\(request: RailUtilityRequest\)[\s\S]*?await tick\(\);[\s\S]*?popoverTrigger\(request\.id\)\?\.click\(\)/,
  'an anchored rail request must position first, then toggle the existing Resource or Usage trigger'
);
assert.match(
  shellOverlays,
  /\.rail-popover-host :global\(\.usage-popover \.card\)[\s\S]*?right:\s*calc\(100% \+ 12px\);[\s\S]*?bottom:\s*0/,
  'the quota card must open inward from the requested rail button rectangle'
);

assert.match(
  shellPage,
  /function handleCenterPanelShown\(id: string\): void \{[\s\S]*?if \(id !== 'browser'\) deactivateBrowserWorkspace\(browserModelContext\(\)\);[\s\S]*?\n  \}/,
  'switching away from Browser must explicitly hide its native workspace'
);
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
  /message=\{\[layoutError, activeCenterPanelId === 'session' \? rail\.error : null\]/,
  'session activation errors must not be rendered over Diff or other center surfaces'
);
assert.match(
  shellPage,
  /browserSurfaceVisible=\{activeCenterPanelId === 'browser'\}/,
  'the global overlay host must know whether Browser is the active center surface'
);
assert.match(
  browserOverlayHost,
  /surfaceVisible && workspace\.presentation !== 'docked'/,
  'floating browser chrome must be absent after switching away from Browser'
);

console.log('centerDock: 32px far-right rail, global anchored utilities, right-pane line tabs, and pure switching verified');
