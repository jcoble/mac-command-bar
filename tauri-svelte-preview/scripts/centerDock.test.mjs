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
  /grid-template-rows:\s*44px minmax\(0, 1fr\)/,
  'the center region must reserve a horizontal surface strip above the dock'
);
assert.match(
  shellFrame,
  /\.shell-center-dock \.dv-tabs-and-actions-container\)[\s\S]*?display:\s*none/,
  'the former horizontal center tab headers must be hidden'
);
assert.match(
  shellFrame,
  /<CenterActivityDock activeId=\{activeCenterPanel\} onSelect=\{selectCenterPanel\}[\s\S]*?<div class="center-dock-host"/,
  'ShellFrame must mount the horizontal surface strip above the center dock'
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
    `the top strip must keep the ${id} surface reachable`
  );
}
assert.match(
  centerActivityDock,
  /label: 'Session History'/,
  'the dock must keep the user-facing Session History label'
);
assert.match(
  centerActivityDock,
  /<span class="dock-label">\{surface\.label\}<\/span>/,
  'the horizontal strip must show readable surface labels beside its icons'
);
assert.doesNotMatch(
  centerActivityDock,
  /ResourcePopover|UsagePopover|utility-group/,
  'the center surface strip must contain only center-surface navigation'
);
assert.match(
  activityBar,
  /<ResourcePopover \/>[\s\S]*?<UsagePopover \/>/,
  'Resources and Stats must live at the bottom of the far-right picker strip'
);
assert.match(
  shellLayout,
  /id: 'activity',[\s\S]*?direction: 'right', referencePanel: 'center'[\s\S]*?id: 'tools',[\s\S]*?direction: 'right', referencePanel: 'center'/,
  'the picker strip must be added outside the right tool panel'
);
assert.doesNotMatch(
  shellOverlays,
  /ResourcePopover|UsagePopover|a10-resource-usage-popovers/,
  'Resources and Usage must no longer float at the window top-right'
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

console.log('centerDock: top surface strip, far-right picker, visible-only surfaces, and pure switching verified');
