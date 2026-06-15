import assert from 'node:assert/strict';
import {
  orchestrationAutoResolveTallyText,
  orchestrationArtifactChips,
  orchestrationAgentActivityItems,
  orchestrationAttentionQueue,
  orchestrationCurrentActivity,
  orchestrationDecisionQueueForRuns,
  orchestrationLinkChips,
  orchestrationLiveDigestItems,
  orchestrationLoopStageMetrics,
  orchestrationLoopTallyText,
  orchestrationRunDigest,
  orchestrationRunHandoffText,
  orchestrationRunMetrics,
  orchestrationRunSummaryText,
  orchestrationRunStage,
  orchestrationRunTimelineText,
  orchestrationStatusTone,
  orchestrationTimelineDetail,
  orchestrationTimelineItems
} from '../src/lib/orchestrationView.ts';

const run = {
  id: 'run-tsk-127',
  title: 'TSK-127 source browser loop',
  status: 'running',
  phase: 'ui-test-loop',
  progress: 72,
  projectID: 'mac-command-bar',
  projectName: 'MacCommandBar',
  projectPath: '/repo',
  rootLabel: 'main checkout',
  taskID: 'TSK-127',
  startedAt: '2026-06-10T12:00:00.000Z',
  updatedAt: '2026-06-10T12:05:00.000Z',
  summary: 'Watching agents fix and retest UI findings',
  agents: [
    {
      id: 'controller',
      provider: 'codex',
      role: 'orchestrator',
      status: 'running',
      title: 'Controller',
      lastActivity: '2026-06-10T12:05:00.000Z'
    },
    {
      id: 'fixer-1',
      provider: 'claude',
      role: 'fix-agent',
      status: 'waiting-for-approval',
      title: 'Fix agent',
      lastActivity: '2026-06-10T12:04:00.000Z'
    }
  ],
  steps: [
    {
      id: 'scenario-pass',
      kind: 'scenario',
      title: 'Customer trading-partner pass',
      status: 'succeeded',
      summary: 'Scenario created and executed',
      agentId: 'controller',
      startedAt: '2026-06-10T12:01:00.000Z',
      finishedAt: '2026-06-10T12:03:00.000Z'
    },
    {
      id: 'fix-batch',
      kind: 'fix',
      title: 'Fix rejected auth redirect',
      status: 'running',
      summary: 'Batching two findings for retest',
      agentId: 'fixer-1',
      startedAt: '2026-06-10T12:04:00.000Z',
      finishedAt: null
    }
  ],
  artifacts: [
    {
      id: 'handoff',
      kind: 'handoff',
      title: 'Scenario handoff',
      path: '/repo/.codex-artifacts/handoff.md',
      url: null,
      status: 'available'
    }
  ],
  links: [
    {
      kind: 'task',
      label: 'TSK-127',
      url: 'https://example.test/task'
    }
  ],
  events: [
    {
      schemaVersion: 1,
      id: 'evt-1',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:02:00.000Z',
      kind: 'scenario.executed',
      status: 'succeeded',
      title: 'Scenario executed',
      message: 'UI path passed',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'controller',
      agentProvider: 'codex',
      agentRole: 'orchestrator',
      stepId: 'scenario-pass',
      stepKind: 'scenario',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null,
      scenario: 'Customer trading-partner pass',
      issueID: null,
      retryAttempt: null,
      approvalSubject: null,
      blockerReason: null,
      decisionPrompt: null
    },
    {
      schemaVersion: 1,
      id: 'evt-2',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:06:00.000Z',
      kind: 'approval.required',
      status: 'waiting-for-approval',
      title: 'Needs sign-off',
      message: 'Manual decision before deleting dirty worktree',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'fixer-1',
      agentProvider: 'claude',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'fix',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null,
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      retryAttempt: null,
      approvalSubject: 'Delete dirty worktree?',
      blockerReason: 'Manual sign-off required before cleanup',
      decisionPrompt: 'Delete the dirty worktree?'
    },
    {
      schemaVersion: 1,
      id: 'evt-3',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:04:30.000Z',
      kind: 'test.retry',
      status: 'running',
      title: 'Retesting failed scenario',
      message: 'Retry after auth fix',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'fixer-1',
      agentProvider: 'claude',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'test',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null,
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      retryAttempt: 1,
      approvalSubject: null,
      blockerReason: null,
      decisionPrompt: null
    }
  ]
};

assert.equal(orchestrationStatusTone('waiting-for-approval'), 'attention');
assert.equal(orchestrationStatusTone('failed'), 'bad');
assert.equal(orchestrationStatusTone('running'), 'live');
assert.equal(orchestrationStatusTone('succeeded'), 'good');

const metrics = orchestrationRunMetrics(run);
assert.equal(metrics.agentCount, 2);
assert.equal(metrics.stepCount, 2);
assert.equal(metrics.artifactCount, 1);
assert.equal(metrics.linkCount, 1);
assert.equal(metrics.eventCount, 3);
assert.equal(metrics.retryCount, 1);
assert.equal(metrics.approvalCount, 1);
assert.equal(metrics.decisionCount, 1);
assert.equal(metrics.scenarioCount, 2);
assert.equal(metrics.issueCount, 0);
assert.equal(metrics.testCount, 1);
assert.equal(metrics.retestCount, 1);
assert.equal(metrics.fixCount, 1);
assert.equal(metrics.resolvedCount, 0);
assert.equal(metrics.verifiedCount, 0);
assert.equal(metrics.delegationCount, 0);
assert.equal(metrics.handoffCount, 1);
assert.ok(metrics.attentionCount >= 2);
assert.ok(metrics.runningCount >= 2);
assert.equal(
  orchestrationLoopTallyText(metrics),
  '2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off'
);
assert.equal(
  orchestrationAutoResolveTallyText(metrics),
  '0 found · 0 fixed · 1 retested · 0 UI verified · 1 needs decision'
);
assert.deepEqual(orchestrationRunStage(run, metrics), {
  label: 'Needs sign-off',
  tone: 'attention',
  title: '1 decision · 1 sign-off'
});
assert.deepEqual(
  orchestrationLoopStageMetrics(metrics).map((stage) => [
    stage.id,
    stage.label,
    stage.value,
    stage.tone,
    stage.title
  ]),
  [
    ['scenario', 'Scen', 2, 'good', '2 scenarios mapped'],
    ['issue', 'Issue', 0, 'idle', '0 issues found'],
    ['test', 'Test', 1, 'live', '1 test run'],
    ['retest', 'Retest', 1, 'live', '1 retest run'],
    ['fix', 'Fix', 1, 'live', '1 fix batched'],
    ['resolved', 'Done', 0, 'idle', '0 resolved'],
    ['verified', 'UI', 0, 'idle', '0 UI verifications passed'],
    ['decision', 'Decide', 1, 'attention', '1 decision needed'],
    ['approval', 'Sign', 1, 'attention', '1 sign-off needed']
  ]
);

const autoResolveRun = {
  ...run,
  id: 'run-auto-resolve',
  title: 'Auto-resolve browser loop',
  status: 'succeeded',
  phase: 'verified',
  progress: 100,
  agents: [],
  steps: [],
  artifacts: [],
  links: [],
  events: [
    {
      ...run.events[0],
      id: 'auto-issue',
      runId: 'run-auto-resolve',
      kind: 'issue.found',
      status: 'needs-fix',
      title: 'Issue found: AUTH-7',
      message: 'Google redirect loop',
      stepId: 'issue-auth-7',
      stepKind: 'issue'
    },
    {
      ...run.events[0],
      id: 'auto-delegated',
      runId: 'run-auto-resolve',
      timestamp: '2026-06-10T12:03:00.000Z',
      kind: 'batch.delegated',
      status: 'running',
      title: 'Fix batch delegated',
      message: '1 issue · 1 resolved',
      agentId: 'fixer-1',
      agentProvider: 'codex',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'fix'
    },
    {
      ...run.events[0],
      id: 'auto-resolved',
      runId: 'run-auto-resolve',
      timestamp: '2026-06-10T12:04:00.000Z',
      kind: 'fix.resolved',
      status: 'succeeded',
      title: 'Fix resolved: AUTH-7',
      message: '1 resolved',
      agentId: 'fixer-1',
      agentProvider: 'codex',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'fix'
    },
    {
      ...run.events[0],
      id: 'auto-verified',
      runId: 'run-auto-resolve',
      timestamp: '2026-06-10T12:05:00.000Z',
      kind: 'ui.verified',
      status: 'succeeded',
      title: 'UI verified: Google auth callback',
      message: 'Scenario passed in browser',
      agentId: 'tester-1',
      agentProvider: 'claude',
      agentRole: 'ui-tester',
      stepId: 'retest-auth-7',
      stepKind: 'retest'
    }
  ]
};
const autoResolveMetrics = orchestrationRunMetrics(autoResolveRun);
assert.equal(autoResolveMetrics.issueCount, 1);
assert.equal(autoResolveMetrics.delegationCount, 1);
assert.equal(autoResolveMetrics.resolvedCount, 1);
assert.equal(autoResolveMetrics.verifiedCount, 1);
assert.equal(
  orchestrationLoopTallyText(autoResolveMetrics),
  '1 issue · 1 resolved · 1 UI verified · 1 delegation'
);
assert.deepEqual(orchestrationRunStage(autoResolveRun, autoResolveMetrics), {
  label: 'UI verified',
  tone: 'good',
  title: '1 UI verification'
});
assert.equal(
  orchestrationAutoResolveTallyText(autoResolveMetrics),
  '1 found · 1 fixed · 0 retested · 1 UI verified · 0 needs decision'
);

const batchedAutoResolveRun = {
  ...autoResolveRun,
  id: 'run-batched-auto-resolve',
  events: [
    {
      ...autoResolveRun.events[0],
      id: 'batch-issues',
      runId: 'run-batched-auto-resolve',
      issueCount: 4,
      message: '4 issues'
    },
    {
      ...autoResolveRun.events[1],
      id: 'batch-delegated',
      runId: 'run-batched-auto-resolve',
      delegatedCount: 2,
      fixCount: 2,
      message: '2 delegated · 2 fixes'
    },
    {
      ...autoResolveRun.events[2],
      id: 'batch-resolved',
      runId: 'run-batched-auto-resolve',
      resolvedCount: 3,
      message: '3 resolved'
    },
    {
      ...autoResolveRun.events[3],
      id: 'batch-verified',
      runId: 'run-batched-auto-resolve',
      verifiedCount: 2,
      message: '2 UI verified'
    }
  ]
};
const batchedAutoResolveMetrics = orchestrationRunMetrics(batchedAutoResolveRun);
assert.equal(batchedAutoResolveMetrics.issueCount, 4);
assert.equal(batchedAutoResolveMetrics.fixCount, 2);
assert.equal(batchedAutoResolveMetrics.resolvedCount, 3);
assert.equal(batchedAutoResolveMetrics.verifiedCount, 2);
assert.equal(batchedAutoResolveMetrics.delegationCount, 2);
assert.equal(
  orchestrationLoopTallyText(batchedAutoResolveMetrics),
  '4 issues · 2 fixes · 3 resolved · 2 UI verified · 2 delegations'
);

const longRunningLoopRun = {
  ...run,
  id: 'run-long-e2e-loop',
  title: '/run-e2e-tests orchestration',
  status: 'running',
  progress: 86,
  agents: [
    {
      id: 'orchestrator',
      provider: 'codex',
      role: 'orchestrator',
      status: 'waiting-for-approval',
      title: 'Controller',
      lastActivity: '2026-06-10T12:07:00.000Z'
    },
    {
      id: 'fixer-1',
      provider: 'codex',
      role: 'fix-agent',
      status: 'succeeded',
      title: 'Fix batch agent',
      lastActivity: '2026-06-10T12:04:00.000Z'
    },
    {
      id: 'tester-1',
      provider: 'claude',
      role: 'ui-tester',
      status: 'succeeded',
      title: 'UI retest agent',
      lastActivity: '2026-06-10T12:05:00.000Z'
    }
  ],
  steps: [],
  artifacts: [],
  events: [
    {
      ...run.events[0],
      id: 'loop-scenario',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:00:00.000Z',
      kind: 'scenario.created',
      status: 'succeeded',
      title: 'Scenario created: Google auth callback',
      message: 'Scenario ready for browser execution',
      agentId: 'orchestrator',
      agentProvider: 'codex',
      agentRole: 'orchestrator',
      stepKind: 'scenario',
      scenario: 'Google auth callback',
      scenarioCount: 1
    },
    {
      ...run.events[0],
      id: 'loop-issues',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:01:00.000Z',
      kind: 'issue.found',
      status: 'needs-fix',
      title: 'Issues found: auth callback',
      message: '3 findings from browser run',
      agentId: 'tester-1',
      agentProvider: 'claude',
      agentRole: 'ui-tester',
      stepKind: 'issue',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      issueCount: 3
    },
    {
      ...run.events[0],
      id: 'loop-fix-batch',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:03:00.000Z',
      kind: 'batch.delegated',
      status: 'running',
      title: 'Fix batch delegated: AUTH-7',
      message: '2 fixes delegated to implementation agents',
      agentId: 'fixer-1',
      agentProvider: 'codex',
      agentRole: 'fix-agent',
      stepKind: 'fix',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      fixCount: 2,
      delegatedCount: 2
    },
    {
      ...run.events[0],
      id: 'loop-retest',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:04:00.000Z',
      kind: 'retest.started',
      status: 'running',
      title: 'UI retest started: AUTH-7',
      message: 'Browser retest after delegated fixes',
      agentId: 'tester-1',
      agentProvider: 'claude',
      agentRole: 'ui-tester',
      stepKind: 'retest',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      retryAttempt: 2,
      retestCount: 1
    },
    {
      ...run.events[0],
      id: 'loop-ui-proof',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:05:00.000Z',
      kind: 'ui.verified',
      status: 'succeeded',
      title: 'UI verified: Google auth callback',
      message: '2 fixes verified in browser retest',
      agentId: 'tester-1',
      agentProvider: 'claude',
      agentRole: 'ui-tester',
      stepKind: 'retest',
      artifactKind: 'trace',
      artifactPath: '/repo/.codex-artifacts/auth-trace.zip',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      resolvedCount: 2,
      verifiedCount: 2
    },
    {
      ...run.events[0],
      id: 'loop-decision',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:06:00.000Z',
      kind: 'approval.required',
      status: 'waiting-for-approval',
      title: 'Approval required: merge fix batch',
      message: 'Awaiting owner sign-off',
      agentId: 'orchestrator',
      agentProvider: 'codex',
      agentRole: 'orchestrator',
      stepKind: 'approval',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      approvalSubject: 'Merge delegated fixes?',
      blockerReason: 'Manual approval required before merge',
      decisionPrompt: 'Approve merge and handoff?',
      approvalCount: 1,
      decisionCount: 1
    },
    {
      ...run.events[0],
      id: 'loop-handoff',
      runId: 'run-long-e2e-loop',
      timestamp: '2026-06-10T12:07:00.000Z',
      kind: 'handoff.available',
      status: 'succeeded',
      title: 'Handoff available',
      message: 'Copyable loop summary ready',
      agentId: 'orchestrator',
      agentProvider: 'codex',
      agentRole: 'orchestrator',
      stepKind: 'handoff',
      artifactKind: 'handoff',
      artifactPath: '/repo/.codex-artifacts/run-e2e-tests-handoff.md'
    }
  ]
};
const longRunningMetrics = orchestrationRunMetrics(longRunningLoopRun);
assert.equal(
  orchestrationLoopTallyText(longRunningMetrics),
  '1 scenario · 3 issues · 1 test · 1 retest · 2 fixes · 2 resolved · 2 UI verified · 2 delegations · 1 handoff · 1 decision · 1 sign-off'
);
assert.equal(
  orchestrationAutoResolveTallyText(longRunningMetrics),
  '3 found · 2 fixed · 1 retested · 2 UI verified · 1 needs decision'
);
const longRunningDigest = orchestrationRunDigest(longRunningLoopRun);
assert.deepEqual(longRunningDigest.stage, {
  label: 'Needs sign-off',
  tone: 'attention',
  title: '1 decision · 1 sign-off'
});
assert.deepEqual(
  [
    longRunningDigest.activeAgent?.id,
    longRunningDigest.activeAgent?.activity,
    longRunningDigest.waitingDecision?.title,
    longRunningDigest.retestStatus?.label,
    longRunningDigest.retestStatus?.title,
    longRunningDigest.latestArtifact?.label,
    longRunningDigest.latestArtifact?.path
  ],
  [
    'orchestrator',
    'Handoff available',
    'Approval required: merge fix batch',
    'UI proof',
    'UI verified: Google auth callback',
    'Handoff',
    '/repo/.codex-artifacts/run-e2e-tests-handoff.md'
  ]
);
assert.deepEqual(longRunningDigest.compact, {
  currentStage: 'Needs sign-off',
  stageTone: 'attention',
  issueCount: 3,
  fixCount: 2,
  retestCount: 1,
  latestArtifact: {
    label: 'Handoff',
    title: 'Handoff available',
    href: null,
    path: '/repo/.codex-artifacts/run-e2e-tests-handoff.md'
  },
  latestHandoff: {
    label: 'Handoff',
    title: 'Handoff available',
    href: null,
    path: '/repo/.codex-artifacts/run-e2e-tests-handoff.md'
  },
  signOffNeeded: true,
  waitingDecision: 'Approval required: merge fix batch',
  tally: '3 found · 2 fixed · 1 retested · 2 UI verified · 1 needs decision'
});

const tallySnapshotRun = {
  ...run,
  id: 'run-tally-snapshot',
  title: 'Nested envelope tally run',
  status: 'running',
  agents: [],
  steps: [],
  artifacts: [],
  links: [],
  events: [
    {
      ...run.events[0],
      id: 'tally-scenario',
      runId: 'run-tally-snapshot',
      kind: 'scenario.passed',
      status: 'succeeded',
      title: 'Scenario passed: Google auth callback',
      message: 'Aggregate tally snapshot',
      stepKind: 'scenario',
      scenario: 'Google auth callback',
      scenarioCount: 2,
      issueCount: 3,
      fixCount: 2,
      retestCount: 2,
      resolvedCount: 1,
      verifiedCount: 1,
      decisionCount: 1,
      approvalCount: 1
    },
    {
      ...run.events[0],
      id: 'tally-issue-a',
      runId: 'run-tally-snapshot',
      timestamp: '2026-06-10T12:01:00.000Z',
      kind: 'issue.found',
      status: 'needs-fix',
      title: 'Issue found: AUTH-7',
      message: 'Callback redirect loop',
      stepKind: 'issue',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7'
    },
    {
      ...run.events[0],
      id: 'tally-issue-b',
      runId: 'run-tally-snapshot',
      timestamp: '2026-06-10T12:02:00.000Z',
      kind: 'issue.found',
      status: 'needs-fix',
      title: 'Issue found: SETTINGS-2',
      message: 'Save button never enables',
      stepKind: 'issue',
      scenario: 'Settings save',
      issueID: 'SETTINGS-2'
    },
    {
      ...run.events[0],
      id: 'tally-proof',
      runId: 'run-tally-snapshot',
      timestamp: '2026-06-10T12:03:00.000Z',
      kind: 'ui.verified',
      status: 'succeeded',
      title: 'UI verified: Google auth callback',
      message: 'Trace captured',
      stepKind: 'retest',
      scenario: 'Google auth callback',
      issueID: 'AUTH-7',
      artifactKind: 'trace',
      artifactPath: '/repo/.codex-artifacts/auth-trace.zip'
    },
    {
      ...run.events[1],
      id: 'tally-decision',
      runId: 'run-tally-snapshot',
      timestamp: '2026-06-10T12:04:00.000Z',
      title: 'Approval required: partial fix batch',
      message: 'Awaiting owner sign-off',
      scenario: null,
      issueID: null
    }
  ]
};
const tallySnapshotMetrics = orchestrationRunMetrics(tallySnapshotRun);
assert.equal(tallySnapshotMetrics.scenarioCount, 2);
assert.equal(tallySnapshotMetrics.issueCount, 3);
assert.equal(tallySnapshotMetrics.fixCount, 2);
assert.equal(tallySnapshotMetrics.retestCount, 2);
assert.equal(tallySnapshotMetrics.resolvedCount, 1);
assert.equal(tallySnapshotMetrics.verifiedCount, 1);
assert.equal(tallySnapshotMetrics.decisionCount, 1);
assert.equal(tallySnapshotMetrics.approvalCount, 1);
assert.deepEqual(orchestrationRunDigest(tallySnapshotRun).compact, {
  currentStage: 'Needs sign-off',
  stageTone: 'attention',
  issueCount: 3,
  fixCount: 2,
  retestCount: 2,
  latestArtifact: {
    label: 'UI proof',
    title: 'UI verified: Google auth callback',
    href: null,
    path: '/repo/.codex-artifacts/auth-trace.zip'
  },
  latestHandoff: null,
  signOffNeeded: true,
  waitingDecision: 'Approval required: partial fix batch',
  tally: '3 found · 1 fixed · 2 retested · 1 UI verified · 1 needs decision'
});

const genericArtifactRun = {
  ...run,
  id: 'run-generic-artifact',
  title: 'Generic report artifact run',
  status: 'running',
  agents: [],
  steps: [],
  artifacts: [
    {
      id: 'browser-report',
      kind: 'report',
      title: 'Browser preview artifact report',
      path: '/repo/.codex-artifacts/browser-report.md',
      url: null,
      status: 'available'
    }
  ],
  events: []
};
assert.equal(
  orchestrationRunMetrics(genericArtifactRun).handoffCount,
  0,
  'generic report artifacts should not count as orchestration handoffs'
);
assert.equal(
  orchestrationRunDigest(genericArtifactRun).compact.latestHandoff,
  null,
  'generic report artifacts should not populate the compact handoff reference'
);

assert.deepEqual(
  orchestrationAgentActivityItems(run).map((agent) => [
    agent.id,
    agent.label,
    agent.tone,
    agent.activity,
    agent.detail
  ]),
  [
    [
      'fixer-1',
      'claude fix-agent',
      'attention',
      'Needs sign-off',
      'Manual decision before deleting dirty worktree'
    ],
    ['controller', 'codex orchestrator', 'live', 'Customer trading-partner pass', 'Scenario created and executed']
  ]
);

const timeline = orchestrationTimelineItems(run, 3);
assert.deepEqual(
  timeline.map((item) => item.id),
  ['evt-2', 'evt-3', 'fix-batch']
);
assert.equal(timeline[0].tone, 'attention');
assert.equal(timeline[0].agentLabel, 'claude fix-agent fixer-1');
assert.equal(timeline[0].issueID, 'AUTH-7');
assert.equal(timeline[0].scenario, 'Google auth callback');
assert.match(orchestrationTimelineDetail(timeline[0]), /issue AUTH-7/);
assert.match(orchestrationTimelineDetail(timeline[0]), /scenario Google auth callback/);
assert.match(orchestrationTimelineDetail(timeline[0]), /sign-off Delete dirty worktree\?/);
assert.match(orchestrationTimelineDetail(timeline[0]), /blocker Manual sign-off required before cleanup/);
assert.match(orchestrationTimelineDetail(timeline[0]), /decision Delete the dirty worktree\?/);
assert.match(orchestrationTimelineDetail(timeline[1]), /retry 1/);

assert.equal(
  orchestrationCurrentActivity(run),
  'claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree'
);
assert.deepEqual(
  {
    stage: orchestrationRunDigest(run).stage,
    activeAgent: orchestrationRunDigest(run).activeAgent && [
      orchestrationRunDigest(run).activeAgent.id,
      orchestrationRunDigest(run).activeAgent.label,
      orchestrationRunDigest(run).activeAgent.status,
      orchestrationRunDigest(run).activeAgent.activity
    ],
    waitingDecision: orchestrationRunDigest(run).waitingDecision && [
      orchestrationRunDigest(run).waitingDecision.label,
      orchestrationRunDigest(run).waitingDecision.title
    ],
    retestStatus: orchestrationRunDigest(run).retestStatus && [
      orchestrationRunDigest(run).retestStatus.label,
      orchestrationRunDigest(run).retestStatus.title
    ],
    latestArtifact: orchestrationRunDigest(run).latestArtifact && [
      orchestrationRunDigest(run).latestArtifact.label,
      orchestrationRunDigest(run).latestArtifact.path
    ],
    autoResolveTally: orchestrationRunDigest(run).autoResolveTally
  },
  {
    stage: {
      label: 'Needs sign-off',
      tone: 'attention',
      title: '1 decision · 1 sign-off'
    },
    activeAgent: ['fixer-1', 'claude fix-agent', 'waiting-for-approval', 'Needs sign-off'],
    waitingDecision: ['Decision', 'Needs sign-off'],
    retestStatus: ['Retest', 'Retesting failed scenario'],
    latestArtifact: ['Handoff', '/repo/.codex-artifacts/handoff.md'],
    autoResolveTally: '0 found · 0 fixed · 1 retested · 0 UI verified · 1 needs decision'
  }
);
assert.equal(
  orchestrationRunTimelineText(run, 2),
  [
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix'
  ].join('\n')
);
assert.deepEqual(
  orchestrationAttentionQueue(run).map((item) => [item.label, item.title, item.agentLabel]),
  [['Decision', 'Needs sign-off', 'claude fix-agent fixer-1']]
);
assert.deepEqual(
  orchestrationLiveDigestItems(run, 4).map((item) => [
    item.label,
    item.title,
    item.tone,
    item.path
  ]),
  [
    ['Decision', 'Needs sign-off', 'attention', null],
    ['Retest', 'Retesting failed scenario', 'live', null],
    ['Handoff', 'Scenario handoff', 'idle', '/repo/.codex-artifacts/handoff.md']
  ]
);
assert.deepEqual(
  orchestrationDecisionQueueForRuns([
    {
      ...run,
      id: 'run-tsk-128',
      title: 'TSK-128 failed checkout loop',
      taskID: 'TSK-128',
      events: [
        {
          ...run.events[0],
          id: 'evt-4',
          runId: 'run-tsk-128',
          timestamp: '2026-06-10T12:07:00.000Z',
          kind: 'test.failed',
          status: 'failed',
          title: 'Checkout scenario failed',
          message: 'Browser test found a redirect loop',
          taskID: 'TSK-128'
        }
      ]
    },
    run
  ]).map((item) => [item.tone, item.runTitle, item.taskID, item.title, item.timestamp]),
  [
    ['bad', 'TSK-128 failed checkout loop', 'TSK-128', 'Checkout scenario failed', '2026-06-10T12:07:00.000Z'],
    ['attention', 'TSK-127 source browser loop', 'TSK-127', 'Needs sign-off', '2026-06-10T12:06:00.000Z']
  ]
);
assert.equal(
  orchestrationRunSummaryText(run),
  [
    'TSK-127 source browser loop',
    'Status: running · 72%',
    'Project: MacCommandBar · main checkout',
    'Task: TSK-127',
    'Phase: ui-test-loop',
    'Stage: Needs sign-off · 1 decision · 1 sign-off',
    'Current: claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree',
    'Active agent: claude fix-agent · waiting-for-approval · Needs sign-off',
    'Waiting decision: Needs sign-off - Manual decision before deleting dirty worktree',
    'Retest: Retesting failed scenario · issue AUTH-7 · scenario Google auth callback · retry 1 · Retry after auth fix',
    'Latest artifact: Scenario handoff · /repo/.codex-artifacts/handoff.md',
    'Tally: 2 done · 4 running · 0 failed · 2 attention · 1 retries · 1 sign-off',
    'Loop: 2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off',
    'Auto-resolve: 0 found · 0 fixed · 1 retested · 0 UI verified · 1 needs decision',
    'Needs attention: Needs sign-off',
    'Artifacts: 1 · Links: 1 · Events: 3',
    'Timeline:',
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix',
    '- live · fixer-1 · Fix rejected auth redirect - Batching two findings for retest',
    '- good · controller · Customer trading-partner pass - Scenario created and executed',
    '- good · codex orchestrator controller · Scenario executed - UI path passed',
    '- idle · Scenario handoff - /repo/.codex-artifacts/handoff.md'
  ].join('\n')
);
assert.equal(
  orchestrationRunHandoffText(run),
  [
    'Orchestration run handoff',
    'Run: TSK-127 source browser loop',
    'ID: run-tsk-127',
    'Status: running · 72% · Needs sign-off',
    'Project: MacCommandBar · main checkout',
    'Path: /repo',
    'Task: TSK-127',
    'Phase: ui-test-loop',
    'Stage: Needs sign-off · 1 decision · 1 sign-off',
    'Current: claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree',
    'Active agent: claude fix-agent · waiting-for-approval · Needs sign-off',
    'Waiting decision: Needs sign-off - Manual decision before deleting dirty worktree',
    'Retest status: Retesting failed scenario · issue AUTH-7 · scenario Google auth callback · retry 1 · Retry after auth fix',
    'Latest artifact: Scenario handoff · /repo/.codex-artifacts/handoff.md',
    'Loop tally: 2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off',
    'Auto-resolve tally: 0 found · 0 fixed · 1 retested · 0 UI verified · 1 needs decision',
    'Counts: 2 agents · 2 steps · 3 events · 1 artifacts · 1 links',
    '',
    'Needs attention:',
    '- Decision · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '',
    'Agents:',
    '- claude fix-agent · waiting-for-approval · Needs sign-off - Manual decision before deleting dirty worktree',
    '- codex orchestrator · running · Customer trading-partner pass - Scenario created and executed',
    '',
    'Recent timeline:',
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix',
    '- live · fixer-1 · Fix rejected auth redirect - Batching two findings for retest',
    '- good · controller · Customer trading-partner pass - Scenario created and executed',
    '- good · codex orchestrator controller · Scenario executed - UI path passed',
    '- idle · Scenario handoff - /repo/.codex-artifacts/handoff.md',
    '',
    'Artifacts:',
    '- handoff · Scenario handoff · /repo/.codex-artifacts/handoff.md',
    '',
    'Links:',
    '- task · TSK-127 · https://example.test/task'
  ].join('\n')
);
assert.deepEqual(orchestrationArtifactChips(run).map((chip) => [chip.kind, chip.path]), [
  ['handoff', '/repo/.codex-artifacts/handoff.md']
]);
assert.deepEqual(orchestrationLinkChips(run).map((chip) => [chip.label, chip.href]), [
  ['TSK-127', 'https://example.test/task']
]);

assert.deepEqual(
  orchestrationLiveDigestItems(autoResolveRun, 3).map((item) => [
    item.label,
    item.title,
    item.tone
  ]),
  [
    ['UI proof', 'UI verified: Google auth callback', 'good'],
    ['Resolved', 'Fix resolved: AUTH-7', 'good'],
    ['Fix', 'Fix batch delegated', 'live']
  ]
);
