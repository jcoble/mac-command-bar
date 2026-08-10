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
  'the Monaco Diff surface must detach when another center tab is active'
);
assert.match(
  centerDock,
  /renderer: panel\.renderer/,
  'the center tab model must pass a panel-specific renderer to Dockview'
);

console.log('centerDock: parent layout and visible-only Diff renderer verified');
