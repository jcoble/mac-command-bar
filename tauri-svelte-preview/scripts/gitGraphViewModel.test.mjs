import assert from 'node:assert/strict';

import {
  buildGitCommitGraphRows,
  buildGitGraphViewModel,
  buildGitRepositoryGraphRows,
  gitCommitParentHint,
  gitCommitRefSummary,
  gitRepositoryDirtySummary,
  gitRepositorySyncSummary
} from '../src/lib/gitGraphViewModel.ts';

const dirtyRepo = {
  projectID: 'mac-command-bar',
  projectName: 'MacCommandBar',
  repo: 'mac-command-bar',
  path: '/repo/mac-command-bar',
  rootLabel: 'main checkout',
  branch: 'cdx/tsk-127-128-git-panel',
  taskID: null,
  isWorktree: false,
  isDirty: true,
  stagedCount: 1,
  unstagedCount: 2,
  untrackedCount: 1,
  dirtyCount: 4,
  ahead: 2,
  behind: 1,
  hasUpstream: true,
  lastCommitSha: 'abc1234',
  lastCommitSubject: 'feat: add TSK-129 graph rows',
  lastCommitAt: '2026-06-15T12:00:00Z',
  dirtySinceEpochMs: 1_000,
  dirtyStatusFingerprint: 'dirty',
  error: null
};

assert.deepEqual(gitRepositoryDirtySummary(dirtyRepo, { now: 121_000 }), {
  isDirty: true,
  dirtyCount: 4,
  stagedCount: 1,
  unstagedCount: 2,
  untrackedCount: 1,
  label: '4 changed · staged 1 / unstaged 2 / untracked 1 · 2m',
  countLabel: '4 changed',
  detailLabel: 'staged 1 / unstaged 2 / untracked 1',
  tone: 'dirty',
  dirtySinceEpochMs: 1_000,
  dirtyAgeMs: 120_000,
  dirtyAgeLabel: '2m'
});

assert.deepEqual(gitRepositorySyncSummary(dirtyRepo), {
  ahead: 2,
  behind: 1,
  hasUpstream: true,
  label: 'ahead 2 / behind 1',
  detailLabel: 'local and upstream both have unique commits',
  tone: 'diverged'
});

assert.deepEqual(
  gitRepositorySyncSummary({
    ...dirtyRepo,
    isDirty: false,
    dirtyCount: 0,
    stagedCount: 0,
    unstagedCount: 0,
    untrackedCount: 0,
    branch: 'main',
    taskID: null,
    ahead: 0,
    behind: 0,
    hasUpstream: false
  }),
  {
    ahead: 0,
    behind: 0,
    hasUpstream: false,
    label: 'no upstream',
    detailLabel: 'no upstream configured',
    tone: 'no-upstream'
  }
);

const repoRows = buildGitRepositoryGraphRows([dirtyRepo], { now: 121_000 });
assert.equal(repoRows.length, 1);
assert.equal(repoRows[0].id, 'repository:/repo/mac-command-bar');
assert.equal(repoRows[0].branchLabel, 'cdx/tsk-127-128-git-panel');
assert.equal(repoRows[0].headLabel, 'abc1234');
assert.equal(repoRows[0].taskID, 'TSK-127');
assert.deepEqual(
  repoRows[0].taskReferences.map((task) => task.id),
  ['TSK-127', 'TSK-128', 'TSK-129']
);
assert.deepEqual(repoRows[0].taskSearchTargets[1], {
  kind: 'notion-search',
  id: 'TSK-128',
  label: 'TSK-128',
  query: 'TSK-128'
});
assert.equal(
  repoRows[0].compactLabel,
  'mac-command-bar · cdx/tsk-127-128-git-panel · 4 changed · staged 1 / unstaged 2 / untracked 1 · 2m · ahead 2 / behind 1'
);
assert.ok(repoRows[0].searchText.includes('TSK-129'));

const mergeCommit = {
  shortSha: 'def4567',
  sha: 'def4567890abcdef',
  subject: 'merge task 130 history helper',
  author: 'Ada Lovelace',
  committedAt: '2026-06-15T12:30:00Z',
  refs: 'HEAD -> cdx/tsk-127-128-git-panel, origin/cdx/tsk-127-git-panel, tag: v0.1.0',
  parentShas: ['1111111111111111', '2222222222222222'],
  parentCount: 2,
  taskID: 'TSK-127',
  taskSource: 'refs'
};

assert.deepEqual(gitCommitParentHint(mergeCommit), {
  kind: 'merge',
  label: '2 parents',
  parentCount: 2,
  parentShas: ['1111111111111111', '2222222222222222'],
  firstParentSha: '1111111111111111',
  isMerge: true
});

assert.deepEqual(gitCommitRefSummary(mergeCommit.refs), {
  labels: [
    'HEAD -> cdx/tsk-127-128-git-panel',
    'origin/cdx/tsk-127-git-panel',
    'tag: v0.1.0'
  ],
  headLabels: ['HEAD -> cdx/tsk-127-128-git-panel'],
  branchLabels: ['cdx/tsk-127-128-git-panel'],
  remoteLabels: ['origin/cdx/tsk-127-git-panel'],
  tagLabels: ['tag: v0.1.0'],
  label: 'HEAD -> cdx/tsk-127-128-git-panel, origin/cdx/tsk-127-git-panel, tag: v0.1.0'
});

const rootCommit = {
  shortSha: '0000001',
  sha: '000000100000010000001',
  subject: 'initial import',
  author: 'Grace Hopper',
  committedAt: '2026-06-14T09:00:00Z',
  refs: '',
  parentShas: [],
  parentCount: 0,
  taskID: null,
  taskSource: null
};

const commitRows = buildGitCommitGraphRows([mergeCommit, rootCommit]);
assert.equal(commitRows[0].graphKind, 'head');
assert.equal(commitRows[0].topologyLabel, 'HEAD');
assert.equal(commitRows[0].parentHint.kind, 'merge');
assert.equal(commitRows[0].taskID, 'TSK-127');
assert.deepEqual(
  commitRows[0].taskReferences.map((task) => task.id),
  ['TSK-127', 'TSK-128', 'TSK-130']
);
assert.deepEqual(
  commitRows[0].ownershipBadges.map((badge) => badge.label),
  ['HEAD', 'UPSTREAM', 'TAG', 'TSK-127', 'MERGE']
);
assert.ok(commitRows[0].metaLabel.includes('2 parents'));
assert.ok(commitRows[0].detailLabel.includes('2 parents'));
assert.equal(commitRows[1].graphKind, 'root');
assert.equal(commitRows[1].topologyLabel, 'ROOT');
assert.equal(commitRows[1].parentHint.label, 'root commit');
assert.equal(commitRows[1].refs.label, 'no refs');

const viewModel = buildGitGraphViewModel(
  {
    repositories: [dirtyRepo],
    commits: [mergeCommit, rootCommit]
  },
  { now: 121_000 }
);
assert.deepEqual(viewModel.taskIDs, ['TSK-127', 'TSK-128', 'TSK-129', 'TSK-130']);
assert.deepEqual(
  viewModel.taskSearchTargets.map((target) => target.query),
  ['TSK-127', 'TSK-128', 'TSK-129', 'TSK-130']
);
assert.deepEqual(viewModel.summary, {
  repositoryCount: 1,
  dirtyRepositoryCount: 1,
  commitCount: 2,
  mergeCommitCount: 1,
  taskCount: 4
});
