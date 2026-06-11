import assert from 'node:assert/strict';
import {
  buildWorktreeCleanupBrief,
  buildWorktreeSafetySummary,
  worktreeAuditCommand,
  worktreeCleanupCommand
} from '../src/lib/worktreeSafety.ts';

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
  assert.match(summary.auditCommand, /status --short --branch/);
  assert.match(summary.backupCommand, /stash push --include-untracked/);
  assert.match(summary.cleanupPlan, /Audit before cleanup/);
  assert.match(summary.cleanupPlan, /Backup dirty\/untracked work/);
}

{
  const summary = buildWorktreeSafetySummary(worktree(), {
    primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
    activeSessionPaths: [
      '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-127-command-center/ediplatform-web'
    ],
    now
  });
  assert.equal(summary.kind, 'blocked');
  assert.equal(summary.badge, 'Active');
  assert.equal(summary.activeSessionCount, 1);
  assert.match(summary.reason, /Active session/);
  assert.match(summary.cleanupPlan, /Do not remove while active sessions point here/);
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
  const command = worktreeAuditCommand(worktree({ path: "/tmp/audit ' quote" }));
  assert.match(command, /git -C '\/tmp\/audit '\\'' quote' status --short --branch/);
  assert.match(command, /log --oneline --decorate --max-count=8/);
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

{
  const brief = buildWorktreeCleanupBrief(
    [
      worktree({
        path: '/Users/blackcolours/dev/work/EdiPlatform',
        branch: 'main',
        taskID: null
      }),
      worktree({
        path: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-120-clean',
        branch: 'cdx/tsk-120-clean',
        taskID: 'TSK-120',
        lastActivity: '2026-05-20T12:00:00.000Z'
      }),
      worktree({
        path: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-121-dirty',
        branch: 'cdx/tsk-121-dirty',
        taskID: 'TSK-121',
        isDirty: true
      }),
      worktree({
        path: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-122-review',
        branch: 'cdx/tsk-122-review',
        taskID: 'TSK-122',
        lastActivity: null
      })
    ],
    {
      primaryPath: '/Users/blackcolours/dev/work/EdiPlatform',
      activeSessionPaths: [
        '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-120-clean'
      ],
      now,
      staleAfterDays: 14
    }
  );

  assert.equal(brief.headline, '2 blocked · 1 review · 1 stale · 1 main');
  assert.equal(brief.cleanupCandidateCount, 0);
  assert.deepEqual(brief.taskIDs, ['TSK-120', 'TSK-121', 'TSK-122']);
  assert.match(brief.report, /Worktree cleanup brief/);
  assert.match(brief.report, /Blocked: 2/);
  assert.match(brief.report, /Cleanup candidates:/);
  assert.match(brief.report, /cdx\/tsk-120-clean/);
  assert.match(brief.report, /Needs attention:/);
  assert.match(brief.report, /cdx\/tsk-121-dirty/);
  assert.match(brief.report, /Review:/);
  assert.match(brief.report, /cdx\/tsk-122-review/);
}
