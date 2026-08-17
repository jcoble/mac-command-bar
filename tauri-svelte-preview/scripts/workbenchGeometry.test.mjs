/**
 * workbenchGeometry.test.mjs — a source-text guard on the workbench's shape.
 *
 * The arrangement this asserts cannot be checked by a type checker: it is which
 * components the page mounts, which regions the frame builds, and which files
 * are gone for good. Same style as `centerDock.test.mjs`, which does the same
 * job for the center surface.
 */
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (relative) => readFileSync(path.join(root, relative), 'utf8');
const exists = (relative) => existsSync(path.join(root, relative));

const page = read('src/routes/next/+page.svelte');
const shellFrame = read('src/lib/shell/components/ShellFrame.svelte');
const frame = read('src/lib/shell/layout/frame.ts');
const centerDock = read('src/lib/shell/layout/centerDock.ts');
const rightPanelTabs = read('src/lib/shell/components/RightPanelTabs.svelte');

// ── What the page no longer mounts ──────────────────────────────────────────

for (const tag of ['<RunButton', '<SessionBrowserButton', '<ActivityBar']) {
  assert.ok(
    !page.includes(tag),
    `the page must not mount ${tag}> — the Run panel, the History panel and the right tab strip replaced them`
  );
}
// The last thing left in the strip along the top was the language-intelligence
// switch, and a whole row of window height to say one word is not a trade worth
// making. It rides with the centre pane's pill tabs now, so the page mounts
// neither the strip nor the switch.
assert.ok(
  !page.includes('<div class="top-bar">'),
  'the page must not keep a strip along the top of the window'
);
assert.ok(
  !page.includes('LanguageIntelligenceControls'),
  'the language-intelligence switch belongs to the centre pane pill group now'
);

// ── Files that are gone for good ────────────────────────────────────────────

for (const file of [
  'src/lib/shell/components/ActivityBar.svelte',
  'src/lib/shell/components/CenterActivityDock.svelte',
  'src/lib/shell/components/ShellSidebar.svelte',
  'src/lib/shell/components/run/RunButton.svelte',
  'src/lib/shell/browser/SessionBrowserButton.svelte',
  'src/lib/shell/components/railUtilityEvents.ts',
  'src/lib/shell/resources/BottomBar.svelte'
]) {
  assert.ok(!exists(file), `${file} must be deleted`);
}

// ── The frame lost its far-right rail ───────────────────────────────────────

assert.ok(
  !shellFrame.includes('activity-region'),
  'ShellFrame must not keep a slot for the deleted far-right rail'
);
assert.ok(
  !shellFrame.includes('CenterActivityDock'),
  'ShellFrame must not import the deleted far-right rail'
);
assert.match(
  frame,
  /export type ShellRegionId = 'sessions' \| 'center' \| 'tools' \| 'dock';/,
  "frame.ts must offer exactly the three columns plus the bottom dock"
);
assert.ok(
  !frame.includes('ACTIVITY_STRIP_WIDTH'),
  'the far-right rail width must be gone from the frame'
);

// ── The center pane holds three surfaces ────────────────────────────────────

assert.match(
  centerDock,
  /export const CENTER_PANEL_IDS = \[\s*'session',\s*'editor',\s*'diff'\s*\] as const;/,
  'the center dock must hold exactly Session, Editor and Diff'
);

// ── The right panel names all eight tabs ────────────────────────────────────

for (const id of [
  'files',
  'source-control',
  'worktrees',
  'run',
  'context',
  'agents',
  'browser',
  'history'
]) {
  assert.ok(
    rightPanelTabs.includes(`id: '${id}'`),
    `the right tab strip must offer the ${id} tab`
  );
}

// ── No `:has()` anywhere in the new panels or in the kit ────────────────────

function filesUnder(relative) {
  const absolute = path.join(root, relative);
  if (!existsSync(absolute)) return [];
  const out = [];
  for (const entry of readdirSync(absolute)) {
    const child = path.join(absolute, entry);
    if (statSync(child).isDirectory()) out.push(...filesUnder(path.join(relative, entry)));
    else out.push(path.join(relative, entry));
  }
  return out;
}

for (const file of [...filesUnder('src/lib/shell/panels'), ...filesUnder('src/lib/components/ui')]) {
  const source = read(file);
  assert.ok(!source.includes(':has('), `${file} must not use a :has() selector`);
  assert.ok(!source.includes('has-['), `${file} must not use a has-[ utility`);
}

console.log('workbenchGeometry: three columns, three center surfaces, eight right tabs, and the deletions verified');
