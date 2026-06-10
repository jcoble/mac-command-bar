import assert from 'node:assert/strict';
import { buildWorktreeSafetySummary, worktreeCleanupCommand } from '../src/lib/worktreeSafety.ts';

const now = new Date('2026-06-10T12:00:00.000Z');

function worktree(overrides = {}) {
  return {
    repo: 'EdiPlatform',
    path: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center',
    branch: 'cdx/tsk-127-command-center',
    taskID: 'TSK-127',
    isDirty: false,
    hasUnmergedCommits: false,
    lastActivity: '2026-06-09T12:00:00.000Z',
    deleteEligibility: 'requires-confirmation',
    ...overrides
  };
}

{
  const summary = buildWorktreeSafetySummary(worktree({ path: '/repo/main', branch: 'main' }), {
    primaryPath: '/repo/main/',
    now
  });
  assert.equal(summary.kind, 'protected');
  assert.equal(summary.badge, 'Main');
  assert.match(summary.cleanupPlan, /Do not remove the primary checkout/);
}

{
  const summary = buildWorktreeSafetySummary(worktree({ isDirty: true }), {
    primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
    now
  });
  assert.equal(summary.kind, 'blocked');
  assert.equal(summary.badge, 'Dirty');
  assert.match(summary.reason, /Uncommitted/);
  assert.match(summary.backupCommand, /stash push --include-untracked/);
  assert.match(summary.cleanupPlan, /Backup dirty\/untracked work/);
}

{
  const summary = buildWorktreeSafetySummary(worktree({ hasUnmergedCommits: true }), {
    primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
    now
  });
  assert.equal(summary.kind, 'blocked');
  assert.equal(summary.badge, 'Unmerged');
  assert.match(summary.recommendation, /push/);
}

{
  const summary = buildWorktreeSafetySummary(
    worktree({ lastActivity: '2026-05-20T12:00:00.000Z' }),
    {
      primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
      now,
      staleAfterDays: 14
    }
  );
  assert.equal(summary.kind, 'ready');
  assert.equal(summary.badge, 'Stale');
  assert.equal(summary.ageBucket, 'stale');
  assert.match(summary.recommendation, /cleanup candidate/);
}

{
  const summary = buildWorktreeSafetySummary(worktree({ lastActivity: null }), {
    primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
    now
  });
  assert.equal(summary.kind, 'review');
  assert.equal(summary.badge, 'Review');
  assert.equal(summary.activityLabel, 'activity unknown');
}

{
  const command = worktreeCleanupCommand(
    worktree({ path: "/tmp/has ' quote" }),
    '/Users/blackcolours/dev/work/EdiPlatform'
  );
  assert.match(command, /git -C '\/Users\/blackcolours\/dev\/work\/EdiPlatform' worktree remove/);
  assert.match(command, /'\/tmp\/has '\\'' quote'/);
  assert.match(command, /worktree prune/);
}
