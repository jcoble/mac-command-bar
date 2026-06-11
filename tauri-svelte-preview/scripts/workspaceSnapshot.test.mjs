import assert from 'node:assert/strict';
import { createDefaultSourceDockLayout } from '../src/lib/sourceDockLayout.ts';
import {
  createWorkspaceSnapshot,
  restoreWorkspaceSnapshot,
  snapshotStorageKey,
  upsertWorkspaceSnapshot
} from '../src/lib/workspaceSnapshot.ts';

const dockLayout = createDefaultSourceDockLayout();

const snapshot = createWorkspaceSnapshot({
  provider: 'codex',
  sessionID: '019c-session',
  title: 'Review checkout flow',
  model: 'gpt-5.5 xhigh',
  project: {
    id: 'ediplatform',
    name: 'EdiPlatform',
    path: '/Users/blackcolours/dev/work/EdiPlatform/'
  },
  cwd: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/',
  worktreePath: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/',
  branch: 'cdx/tsk-127-command-center',
  selectedPath: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts',
  selectedLine: 42.7,
  openPaths: [
    '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts',
    '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts',
    '   ',
    '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/routes/+page.svelte'
  ],
  sourceActivityMode: 'conversations',
  sourceTerminalApp: 'Warp',
  dockLayout,
  resumeCommand: 'cd /Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center && codex resume 019c-session',
  capturedAt: 1_000
});

assert.equal(snapshotStorageKey, 'mac-command-bar.workspace-snapshots');
assert.equal(snapshot.id, 'codex:019c-session');
assert.equal(snapshot.title, 'Review checkout flow');
assert.equal(snapshot.project.path, '/Users/blackcolours/dev/work/EdiPlatform');
assert.equal(snapshot.cwd, '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center');
assert.equal(snapshot.worktreePath, '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center');
assert.equal(snapshot.selectedLine, 43);
assert.deepEqual(snapshot.openPaths, [
  '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts',
  '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/routes/+page.svelte'
]);
assert.equal(snapshot.dockLayout.activePanelByGroup.right, 'context');

const restored = restoreWorkspaceSnapshot(snapshot);
assert.deepEqual(restored.selectedProjectID, 'ediplatform');
assert.deepEqual(restored.selectedSourcePaths, {
  ediplatform: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts'
});
assert.equal(restored.selectedLine, 43);
assert.equal(restored.sourceActivityMode, 'conversations');
assert.equal(restored.sourceTerminalApp, 'Warp');
assert.equal(restored.cwd, '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center');
assert.equal(restored.dockLayout.activePanelByGroup.right, 'context');

const olderSnapshot = createWorkspaceSnapshot({
  provider: 'claude',
  sessionID: 'claude-session',
  title: 'Older task',
  project: { id: 'mac-command-bar', name: 'MacCommandBar', path: '/repo' },
  cwd: '/repo',
  capturedAt: 500
});

const updatedSnapshot = createWorkspaceSnapshot({
  ...snapshot,
  title: 'Review checkout flow resumed',
  selectedLine: 9,
  capturedAt: 2_000
});

assert.deepEqual(
  upsertWorkspaceSnapshot([olderSnapshot, snapshot], updatedSnapshot, 8).map((item) => [
    item.id,
    item.title,
    item.selectedLine
  ]),
  [
    ['codex:019c-session', 'Review checkout flow resumed', 9],
    ['claude:claude-session', 'Older task', null]
  ],
  'upserting a snapshot should replace the same provider/session and keep newest first'
);

assert.equal(
  upsertWorkspaceSnapshot([updatedSnapshot, olderSnapshot], olderSnapshot, 1).length,
  1,
  'snapshot history should be bounded'
);
