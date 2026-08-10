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
const shellOverlays = readFileSync(
  new URL('../src/lib/shell/components/ShellOverlays.svelte', import.meta.url),
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
  centerDock,
  /renderer: panel\.renderer/,
  'the center surface model must pass a panel-specific renderer to Dockview'
);

assert.match(
  shellFrame,
  /grid-template-columns:\s*minmax\(0, 1fr\) 48px/,
  'the center region must reserve a slim dock on its right edge'
);
assert.match(
  shellFrame,
  /\.shell-center-dock \.dv-tabs-and-actions-container\)[\s\S]*?display:\s*none/,
  'the former horizontal center tab headers must be hidden'
);
assert.match(
  shellFrame,
  /<CenterActivityDock activeId=\{activeCenterPanel\} onSelect=\{selectCenterPanel\}/,
  'ShellFrame must mount the activity dock against the live center state'
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
    `the right-side dock must keep the ${id} surface reachable`
  );
}
assert.match(
  centerActivityDock,
  /label: 'Session History'/,
  'the dock must keep the user-facing Session History label'
);
assert.match(
  centerActivityDock,
  /<ResourcePopover \/>[\s\S]*?<UsagePopover \/>/,
  'Resources and Usage must live in the dock utility group'
);
assert.doesNotMatch(
  shellOverlays,
  /ResourcePopover|UsagePopover|a10-resource-usage-popovers/,
  'Resources and Usage must no longer float at the window top-right'
);

console.log('centerDock: right activity dock, six surfaces, and visible-only Diff verified');
