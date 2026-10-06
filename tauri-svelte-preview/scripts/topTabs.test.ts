import assert from 'node:assert/strict';
import test from 'node:test';

import {
  closeTopTab,
  normalizeTopTabs,
  openTopTab,
  parseTopTabKey,
  resolveActiveKey,
  topTabKey,
  topTabRow
} from '../src/lib/shell/layout/topTabsOps.ts';

const live = {
  editorPaths: ['/repo/a.ts', '/repo/b.ts', '/repo/new.ts'],
  browserTabIds: ['tab-1', 'tab-new']
};

test('the row keeps stored order, drops gone content, keeps singletons, appends new live tabs', () => {
  const row = topTabRow(
    ['browser:tab-1', 'diff', 'editor:/repo/gone.ts', 'editor:/repo/b.ts', 'browser:tab-gone', 'pull-requests', 'editor:/repo/a.ts'],
    live
  );
  assert.deepEqual(row, [
    'browser:tab-1',
    'diff',
    'editor:/repo/b.ts',
    'pull-requests',
    'editor:/repo/a.ts',
    'editor:/repo/new.ts',
    'browser:tab-new'
  ]);
});

test('the active key falls back to the last tab, or null when the row is empty', () => {
  const row = ['diff', 'editor:/repo/a.ts'];
  assert.equal(resolveActiveKey(row, 'diff'), 'diff');
  assert.equal(resolveActiveKey(row, 'git-history'), 'editor:/repo/a.ts');
  assert.equal(resolveActiveKey(row, null), 'editor:/repo/a.ts');
  assert.equal(resolveActiveKey([], 'diff'), null);
});

test('closing the active tab picks the right neighbour, then the left, then nothing', () => {
  const row = ['diff', 'git-history', 'pull-requests'];
  assert.deepEqual(closeTopTab(row, 'git-history', 'git-history'), {
    order: ['diff', 'pull-requests'],
    activeKey: 'pull-requests'
  });
  assert.deepEqual(closeTopTab(row, 'pull-requests', 'pull-requests'), {
    order: ['diff', 'git-history'],
    activeKey: 'git-history'
  });
  assert.deepEqual(closeTopTab(['diff'], 'diff', 'diff'), { order: [], activeKey: null });
});

test('closing an inactive tab keeps the active key', () => {
  assert.deepEqual(closeTopTab(['diff', 'git-history', 'pull-requests'], 'diff', 'pull-requests'), {
    order: ['diff', 'git-history'],
    activeKey: 'diff'
  });
});

test('opening a tab appends it once', () => {
  const once = openTopTab(['diff'], 'git-history');
  assert.deepEqual(once, ['diff', 'git-history']);
  assert.deepEqual(openTopTab(once, 'git-history'), ['diff', 'git-history']);
  assert.deepEqual(openTopTab(once, 'diff'), ['diff', 'git-history']);
});

test('a key round-trips for a path that contains a colon', () => {
  const ref = { kind: 'editor' as const, id: 'C:/repo/odd:name.ts' };
  const key = topTabKey(ref);
  assert.equal(key, 'editor:C:/repo/odd:name.ts');
  assert.deepEqual(parseTopTabKey(key), ref);
  assert.deepEqual(parseTopTabKey('browser:tab-1'), { kind: 'browser', id: 'tab-1' });
  assert.deepEqual(parseTopTabKey('diff'), { kind: 'diff', id: 'diff' });
  assert.equal(topTabKey({ kind: 'diff', id: 'diff' }), 'diff');
  assert.equal(parseTopTabKey('session'), null);
  assert.equal(parseTopTabKey('editor:'), null);
});

test('normalize drops junk, unknown kinds, duplicates, gone editors and browsers, and a stray active key', () => {
  const normalized = normalizeTopTabs(
    {
      order: [
        7,
        null,
        'terminal',
        'diff',
        'diff',
        'editor:/repo/a.ts',
        'editor:/repo/closed.ts',
        'browser:tab-1',
        'browser:tab-closed',
        'pull-requests'
      ],
      activeKey: 'editor:/repo/closed.ts'
    },
    { editorPaths: ['/repo/a.ts'], browserTabIds: ['tab-1'] }
  );
  assert.deepEqual(normalized, {
    order: ['diff', 'editor:/repo/a.ts', 'browser:tab-1', 'pull-requests'],
    activeKey: null
  });
  assert.deepEqual(
    normalizeTopTabs({ order: ['git-history'], activeKey: 'git-history' }, { editorPaths: [], browserTabIds: [] }),
    { order: ['git-history'], activeKey: 'git-history' }
  );
  assert.equal(normalizeTopTabs('nope', { editorPaths: [], browserTabIds: [] }), undefined);
  assert.equal(normalizeTopTabs([], { editorPaths: [], browserTabIds: [] }), undefined);
});
