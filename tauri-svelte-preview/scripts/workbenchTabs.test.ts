import assert from 'node:assert/strict';

import type { LayoutStorage } from '../src/lib/shell/layout/layoutStorage.ts';
import {
  CENTER_TAB_KEY,
  DEFAULT_CENTER_TAB,
  DEFAULT_RIGHT_TAB,
  RIGHT_TAB_KEY,
  clearWorkbenchTabs,
  isCenterTabId,
  isRightTabId,
  readCenterTab,
  readRightTab,
  writeCenterTab,
  writeRightTab
} from '../src/lib/shell/layout/workbenchTabs.ts';
import { CENTER_TAB_IDS, RIGHT_TAB_IDS } from '../src/lib/shell/workbenchNavigation.ts';

/** A localStorage stand-in. `failWrites` makes every write throw, the way a
 * storage at its quota does. */
function fakeStorage(seed: Record<string, string> = {}): LayoutStorage & {
  failWrites: boolean;
  entries: Map<string, string>;
} {
  const entries = new Map(Object.entries(seed));
  return {
    entries,
    failWrites: false,
    getItem(key: string): string | null {
      return entries.get(key) ?? null;
    },
    setItem(key: string, value: string): void {
      if (this.failWrites) throw new Error('storage is full');
      entries.set(key, value);
    },
    removeItem(key: string): void {
      entries.delete(key);
    }
  };
}

// ── The roster ──────────────────────────────────────────────────────────────

assert.deepEqual(
  [...RIGHT_TAB_IDS],
  ['files', 'source-control', 'worktrees', 'run', 'context', 'agents', 'browser', 'history'],
  'the right panel offers exactly the eight tabs, in the settled order'
);
assert.deepEqual(
  [...CENTER_TAB_IDS],
  ['session', 'editor', 'diff'],
  'the center pane offers exactly Session, Editor and Diff'
);
assert.equal(DEFAULT_CENTER_TAB, 'session', 'the center pane opens on the session');
assert.equal(DEFAULT_RIGHT_TAB, 'files', 'the right panel opens on the file tree');

assert.ok(isCenterTabId('editor'));
assert.ok(!isCenterTabId('browser'), 'browser is a right tab, never a center one');
assert.ok(isRightTabId('history'));
assert.ok(!isRightTabId('problems'));
assert.ok(!isRightTabId(null));

// ── Defaults for anything unusable ──────────────────────────────────────────

const empty = fakeStorage();
assert.equal(readCenterTab(empty, 'one'), DEFAULT_CENTER_TAB, 'nothing stored reads as the default');
assert.equal(readRightTab(empty, 'one'), DEFAULT_RIGHT_TAB);
assert.equal(readCenterTab(empty, null), DEFAULT_CENTER_TAB, 'no session reads as the default too');

const corrupt = fakeStorage({ [CENTER_TAB_KEY]: '{not json', [RIGHT_TAB_KEY]: '[1,2,3]' });
assert.equal(readCenterTab(corrupt, 'one'), DEFAULT_CENTER_TAB, 'unparseable JSON reads as the default');
assert.equal(readRightTab(corrupt, 'one'), DEFAULT_RIGHT_TAB, 'a stored value of the wrong shape reads as the default');

const retired = fakeStorage({
  [CENTER_TAB_KEY]: JSON.stringify({ one: 'session-library' }),
  [RIGHT_TAB_KEY]: JSON.stringify({ one: 'problems' })
});
assert.equal(readCenterTab(retired, 'one'), DEFAULT_CENTER_TAB, 'a tab that no longer exists reads as the default');
assert.equal(readRightTab(retired, 'one'), DEFAULT_RIGHT_TAB);

// ── One session's choice is its own ─────────────────────────────────────────

const perSession = fakeStorage();
assert.equal(writeCenterTab(perSession, 'one', 'editor'), true);
assert.equal(writeRightTab(perSession, 'one', 'worktrees'), true);
assert.equal(readCenterTab(perSession, 'one'), 'editor');
assert.equal(readRightTab(perSession, 'one'), 'worktrees');
assert.equal(readCenterTab(perSession, 'two'), DEFAULT_CENTER_TAB, 'one session’s tab does not leak to another');
assert.equal(readRightTab(perSession, 'two'), DEFAULT_RIGHT_TAB);

assert.equal(writeCenterTab(perSession, null, 'diff'), true);
assert.equal(readCenterTab(perSession, null), 'diff', 'a shell with no session picked keeps its own slot');
assert.equal(readCenterTab(perSession, 'one'), 'editor', 'and that slot is not the one a session uses');

// ── A storage that refuses to write ─────────────────────────────────────────

const full = fakeStorage();
writeCenterTab(full, 'one', 'editor');
writeRightTab(full, 'one', 'run');
full.failWrites = true;
assert.equal(writeCenterTab(full, 'one', 'diff'), false, 'a refused write is reported, never thrown');
assert.equal(writeRightTab(full, 'one', 'agents'), false);
assert.equal(readCenterTab(full, 'one'), 'editor', 'and reading afterwards still answers with a real tab');
assert.equal(readRightTab(full, 'one'), 'run');

// ── Forgetting everything ───────────────────────────────────────────────────

const cleared = fakeStorage();
writeCenterTab(cleared, 'one', 'editor');
writeRightTab(cleared, 'one', 'browser');
clearWorkbenchTabs(cleared);
assert.equal(cleared.entries.size, 0, 'clearing removes both keys');
assert.equal(readCenterTab(cleared, 'one'), DEFAULT_CENTER_TAB);
assert.equal(readRightTab(cleared, 'one'), DEFAULT_RIGHT_TAB);

console.log('workbenchTabs: roster, defaults, per-session storage, and quota safety passed');
