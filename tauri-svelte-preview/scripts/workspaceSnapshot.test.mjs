import assert from 'node:assert/strict';
import { createDefaultSourceDockLayout } from '../src/lib/sourceDockLayout.ts';
import {
  createWorkspaceSnapshot,
  describeWorkspaceSnapshotRestoreReadiness,
  parseStoredWorkspaceSnapshot,
  restoreWorkspaceSnapshot,
  selectStartupWorkspaceSnapshot,
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
  browserUrl: '  http://localhost:5177/customer/dashboard  ',
  viewState: {
    contextPanelMode: 'stack',
    contextPanelPlacement: 'side',
    contextPanelCollapsed: true,
    editorInsightCollapsed: false,
    sidePanePosition: 'right',
    sourceChromeCompact: true,
    sourceActivityFilter: 'tsk-127',
    hiddenContextCardIDs: ['runtime', 'runtime', 'repo'],
    activeContextCardID: 'agents',
    sourceIntelligencePanel: 'git'
  },
  dockLayout,
  embeddedTerminal: {
    sessionID: 'terminal-123',
    cwd: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center',
    shell: '/bin/zsh',
    startedAt: 1_111
  },
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
assert.deepEqual(snapshot.embeddedTerminal, {
  sessionID: 'terminal-123',
  cwd: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center',
  shell: '/bin/zsh',
  startedAt: 1_111
});
assert.deepEqual(snapshot.openPaths, [
  '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts',
  '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/routes/+page.svelte'
]);
assert.deepEqual(snapshot.viewState, {
  contextPanelMode: 'stack',
  contextPanelPlacement: 'side',
  contextPanelCollapsed: true,
  editorInsightCollapsed: false,
  sidePanePosition: 'right',
  sourceChromeCompact: true,
  sourceActivityFilter: 'tsk-127',
  hiddenContextCardIDs: ['runtime', 'repo'],
  activeContextCardID: 'agents',
  sourceIntelligencePanel: 'git'
});
assert.equal(snapshot.dockLayout.activePanelByGroup.right, 'context');

const restored = restoreWorkspaceSnapshot(snapshot);
assert.deepEqual(restored.selectedProjectID, 'ediplatform');
assert.deepEqual(restored.selectedSourcePaths, {
  ediplatform: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts'
});
assert.equal(restored.selectedPath, '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/src/App.ts');
assert.equal(restored.selectedLine, 43);
assert.equal(restored.sourceActivityMode, 'conversations');
assert.equal(restored.sourceTerminalApp, 'Warp');
assert.equal(restored.browserUrl, 'http://localhost:5177/customer/dashboard');
assert.deepEqual(restored.viewState, snapshot.viewState);
assert.equal(restored.cwd, '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center');
assert.deepEqual(restored.embeddedTerminal, snapshot.embeddedTerminal);
assert.equal(restored.dockLayout.activePanelByGroup.right, 'context');

const olderSnapshot = createWorkspaceSnapshot({
  provider: 'claude',
  sessionID: 'claude-session',
  title: 'Older task',
  project: { id: 'mac-command-bar', name: 'MacCommandBar', path: '/repo' },
  cwd: '/repo',
  capturedAt: 500
});

const selectedProjectSnapshot = createWorkspaceSnapshot({
  provider: 'manual',
  sessionID: 'ediplatform',
  title: 'Latest selected project snapshot',
  project: {
    id: 'ediplatform',
    name: 'EdiPlatform',
    path: '/Users/blackcolours/dev/work/EdiPlatform'
  },
  cwd: '/Users/blackcolours/dev/work/EdiPlatform',
  selectedPath: '/Users/blackcolours/dev/work/EdiPlatform/EdiPlatform.Core/Services/FormatResolver.cs',
  capturedAt: 1_500
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

assert.equal(
  selectStartupWorkspaceSnapshot([olderSnapshot, selectedProjectSnapshot, snapshot], {
    activeSessionKey: snapshot.id,
    selectedProjectID: 'ediplatform',
    selectedProjectPath: '/Users/blackcolours/dev/work/EdiPlatform'
  })?.id,
  snapshot.id,
  'startup restore should prefer the active conversation snapshot'
);

assert.equal(
  selectStartupWorkspaceSnapshot([olderSnapshot, selectedProjectSnapshot, snapshot], {
    selectedProjectID: 'ediplatform',
    selectedProjectPath: '/Users/blackcolours/dev/work/EdiPlatform'
  })?.id,
  selectedProjectSnapshot.id,
  'startup restore should fall back to the newest snapshot for the stored project'
);

assert.equal(
  selectStartupWorkspaceSnapshot([olderSnapshot, selectedProjectSnapshot], {
    activeSessionKey: 'missing-session',
    selectedProjectID: 'missing-project',
    selectedProjectPath: '/missing/project'
  }),
  null,
  'startup restore should ignore stale active keys when no stored project snapshot matches'
);

const storedSnapshot = parseStoredWorkspaceSnapshot({
  ...snapshot,
  viewState: {
    contextPanelMode: 'stack',
    contextPanelPlacement: 'bottom',
    contextPanelCollapsed: false,
    editorInsightCollapsed: false,
    sidePanePosition: 'right',
    sourceChromeCompact: false,
    sourceActivityFilter: '  task trail  ',
    hiddenContextCardIDs: ['runtime', 'bad-card', 'repo'],
    activeContextCardID: 'worktrees',
    sourceIntelligencePanel: 'symbols'
  },
  embeddedTerminal: {
    sessionID: 'terminal-456',
    cwd: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center',
    shell: '/bin/zsh',
    startedAt: 2_222
  },
  browserUrl: ' http://localhost:6190/customer/trading-partners '
});

assert.deepEqual(
  storedSnapshot?.viewState,
  {
    contextPanelMode: 'stack',
    contextPanelPlacement: 'bottom',
    contextPanelCollapsed: false,
    editorInsightCollapsed: false,
    sidePanePosition: 'right',
    sourceChromeCompact: false,
    sourceActivityFilter: 'task trail',
    hiddenContextCardIDs: ['runtime', 'repo'],
    activeContextCardID: 'worktrees',
    sourceIntelligencePanel: 'symbols'
  },
  'stored snapshots should preserve normalized workspace view state'
);
assert.deepEqual(
  storedSnapshot?.embeddedTerminal,
  {
    sessionID: 'terminal-456',
    cwd: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center',
    shell: '/bin/zsh',
    startedAt: 2_222
  },
  'stored snapshots should preserve embedded terminal restore metadata'
);
assert.equal(
  storedSnapshot?.browserUrl,
  'http://localhost:6190/customer/trading-partners',
  'stored snapshots should preserve normalized browser dock URLs'
);
assert.equal(parseStoredWorkspaceSnapshot({ ...snapshot, provider: 'unknown' }), null);

assert.deepEqual(
  describeWorkspaceSnapshotRestoreReadiness(snapshot, {
    liveTerminalSessionIDs: ['terminal-123'],
    liveTerminalCwds: [],
    knownWorktreePaths: []
  }),
  {
    kind: 'live-terminal',
    tone: 'ready',
    label: 'Live terminal',
    detail: 'Can reattach to the saved embedded terminal session.',
    canRestoreWorkspace: true,
    canResumeEmbedded: true
  },
  'restore readiness should prefer a live saved terminal session'
);

assert.deepEqual(
  describeWorkspaceSnapshotRestoreReadiness(snapshot, {
    liveTerminalSessionIDs: [],
    liveTerminalCwds: [],
    knownWorktreePaths: ['/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center']
  }),
  {
    kind: 'ready',
    tone: 'ready',
    label: 'Worktree ready',
    detail: 'Saved worktree is still registered and can be restored.',
    canRestoreWorkspace: true,
    canResumeEmbedded: true
  },
  'restore readiness should recognize a still-registered saved worktree'
);

assert.deepEqual(
  describeWorkspaceSnapshotRestoreReadiness(snapshot, {
    knownWorktreePaths: ['/Users/blackcolours/dev/work/worktrees/EdiPlatform/other-task']
  }),
  {
    kind: 'missing-worktree',
    tone: 'blocked',
    label: 'Worktree missing',
    detail: 'Saved worktree is not in the current worktree scan; restore files cautiously.',
    canRestoreWorkspace: true,
    canResumeEmbedded: false
  },
  'restore readiness should warn when a saved worktree is no longer registered'
);

const commandOnlySnapshot = createWorkspaceSnapshot({
  provider: 'codex',
  sessionID: 'command-only',
  title: 'Command-only resume',
  project: { id: 'mac-command-bar', name: 'MacCommandBar', path: '/repo' },
  cwd: '/repo',
  resumeCommand: 'codex resume command-only',
  capturedAt: 3_000
});

assert.deepEqual(
  describeWorkspaceSnapshotRestoreReadiness(commandOnlySnapshot),
  {
    kind: 'needs-terminal',
    tone: 'warning',
    label: 'Needs terminal',
    detail: 'Restore can start a new embedded terminal and run the saved command.',
    canRestoreWorkspace: true,
    canResumeEmbedded: true
  },
  'restore readiness should flag command snapshots that need a fresh terminal'
);

const filesOnlySnapshot = createWorkspaceSnapshot({
  provider: 'manual',
  sessionID: 'files-only',
  title: 'Files only',
  project: { id: 'mac-command-bar', name: 'MacCommandBar', path: '/repo' },
  cwd: '/repo',
  capturedAt: 4_000
});

assert.deepEqual(
  describeWorkspaceSnapshotRestoreReadiness(filesOnlySnapshot),
  {
    kind: 'files-only',
    tone: 'neutral',
    label: 'Files only',
    detail: 'Restores panes, selected file, and open tabs; no resume command was saved.',
    canRestoreWorkspace: true,
    canResumeEmbedded: false
  },
  'restore readiness should describe snapshots without a resume command'
);
