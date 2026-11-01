// What the step panel and the Cost tab read from a run: a step's links, its failed commands,
// its usage, an accepted review's open findings, and the Cost tab's lines and totals. Logic only.
import assert from 'node:assert/strict';
import type { WorkflowTimelineItem } from '../src/lib/shell/workflows/workflowTimeline.ts';
import type { WorkflowNodeRunRecord, WorkflowRunRecord, WorkflowUsageRow } from '../src/lib/shell/workflows/workflowTypes.ts';
import {
  acceptedWithFindings, costLines, costSummary, costTotal, failedCommands, money, stepLinks, stepUsageText
} from '../src/lib/shell/workflows/workflowStepInfo.ts';

const RUN = 'workflow-run-1';

function node(nodeId: string, extra: Partial<WorkflowNodeRunRecord> = {}): WorkflowNodeRunRecord {
  const attempt = extra.attempt ?? 1;
  return {
    id: `${RUN}:${nodeId}:${attempt}`, nodeId, roleId: nodeId, state: 'completed', attempt, depth: 0, ownedId: `owned-${nodeId}`,
    provider: 'claude', providerInstanceId: null, startedAtMs: 1_000, finishedAtMs: 61_000, outputContract: 'ImplementationReceipt',
    structuredOutput: null, artifacts: [], gate: null, leaseId: null, failure: null, usage: null, ...extra
  };
}

function run(nodes: WorkflowNodeRunRecord[], snapshot: Record<string, unknown> = {}): WorkflowRunRecord {
  return {
    id: RUN, workflowId: '/repo:.assembly/workflows/spec-to-pr.yml', state: 'running', phase: 'implementing',
    inputSnapshot: { task: { key: 'TSK-9' }, worktree: { path: '/wt/tsk-9', branch: 'tsk-9-thing' }, ...snapshot },
    inputHash: '', createdAtMs: 0, updatedAtMs: 0, nodes, tokensUsed: 0, toolTerminalsUsed: 0, worktreesAllocated: 0, lastSequence: 0,
    definition: {
      nodes: nodes.map((item) => ({ id: item.nodeId, title: item.nodeId, roleId: item.nodeId, dependsOn: [] })),
      roles: nodes.map((item) => ({ id: item.nodeId, modelPolicy: { value: item.provider ? 'sonnet' : null, overrides: {} } }))
    }
  } as unknown as WorkflowRunRecord;
}

function usage(step: string, attempt: number, extra: Partial<WorkflowUsageRow> = {}): WorkflowUsageRow {
  return { nodeRunId: `${RUN}:${step}:${attempt}`, model: 'claude-sonnet', inputTokens: 1_000, outputTokens: 200, cacheReadTokens: 3_000,
    cacheWriteTokens: 0, reportedCostMicros: 20_000, estimatedCostMicros: null, ...extra };
}

function item(kind: WorkflowTimelineItem['kind'], nodeId: string, attempt: number, atMs: number, title = ''): WorkflowTimelineItem {
  return { key: `${kind}:${nodeId}:${attempt}:${atMs}`, atMs, kind, tone: null, nodeId, attempt, title, detail: '', verdict: null,
    findingCount: null, receipt: null, url: null, files: [] };
}

{ // a_yaml_pr_steps_links_have_its_pr_from_command_output_and_the_runs_branch_and_worktree
  const pr = node('pr', { provider: '', structuredOutput: { commands: [
    { command: 'git push -u origin HEAD', exitCode: 0, output: 'pushed' },
    { command: 'gh pr create --fill', exitCode: 0, output: 'https://github.com/jcoble/mac-command-bar/pull/422\n' }
  ] }, artifacts: [{ id: 'a', kind: 'command-output', path: '/proof/pr-1-1.txt', url: null, digest: null }] });
  const links = stepLinks(run([pr]), pr);
  assert.deepEqual(links.map((link) => [link.label, link.value, link.url, link.path]), [
    ['Pull request', 'https://github.com/jcoble/mac-command-bar/pull/422', 'https://github.com/jcoble/mac-command-bar/pull/422', null],
    ['Branch', 'tsk-9-thing', null, null],
    ['Worktree', '/wt/tsk-9', null, '/wt/tsk-9'],
    ['Task', 'TSK-9', null, null],
    ['Workflow', '.assembly/workflows/spec-to-pr.yml', null, null],
    ['command-output', '/proof/pr-1-1.txt', null, '/proof/pr-1-1.txt']
  ]);
  // A receipt's own branch and worktree win over the run's.
  const build = node('build', { structuredOutput: { branch: 'own-branch', worktree: '/own', commit: 'abc' } });
  assert.deepEqual(stepLinks(run([build]), build).slice(0, 3).map((link) => link.value), ['abc', 'own-branch', '/own']);
}

{ // a_failed_command_step_names_each_failed_command_its_exit_and_its_output_tail
  const checks = node('checks', { provider: '', state: 'failed', structuredOutput: { commands: [
    { command: 'echo ok', exitCode: 0, output: 'ok' },
    { command: 'test -f gate-ok.txt', exitCode: 1, error: null, output: '' },
    { command: 'pnpm test', exitCode: 2, output: Array.from({ length: 30 }, (_, index) => `line ${index + 1}`).join('\n') + '\n' },
    { command: 'sleep 999', exitCode: null, error: 'Workflow command timed out', output: '' }
  ] } });
  const failed = failedCommands(checks);
  assert.deepEqual(failed.map((command) => [command.command, command.exit]), [
    ['test -f gate-ok.txt', 'exit 1'], ['pnpm test', 'exit 2'], ['sleep 999', 'Workflow command timed out']
  ]);
  assert.equal(failed[0].tail, '');
  assert.equal(failed[1].tail.split('\n').length, 10);
  assert.ok(failed[1].tail.endsWith('line 30'));
  // Nothing recorded means nothing invented.
  assert.deepEqual(failedCommands(node('checks', { structuredOutput: null })), []);
}

{ // a_steps_usage_comes_from_its_usage_rows_when_its_record_has_none
  const rows = [usage('build', 1), usage('build', 1, { model: 'claude-haiku', inputTokens: 500, cacheReadTokens: 0, outputTokens: 0, reportedCostMicros: null, estimatedCostMicros: 0 }), usage('build', 2)];
  assert.equal(stepUsageText(rows, `${RUN}:build:1`), '5k in, 200 out · $0.02');
  assert.equal(stepUsageText(rows, `${RUN}:checks:1`), null);
  assert.equal(stepUsageText([usage('build', 1, { inputTokens: 40, cacheReadTokens: 0, outputTokens: 5, reportedCostMicros: 3_000 })], `${RUN}:build:1`), '40 in, 5 out · <$0.01');
  assert.equal(stepUsageText([usage('build', 1, { reportedCostMicros: null })], `${RUN}:build:1`), '4k in, 200 out · not priced');
}

{ // a_review_accepted_with_an_open_finding_says_so_and_who_accepted_it
  const review = node('code-review', { structuredOutput: { verdict: 'fail', findings: ['src/a.ts:3: off by one'], findingCount: 1 } });
  const answered = [item('owner', 'code-review', 1, Date.UTC(2026, 9, 22, 15, 0), 'You answered code-review: accept')];
  const accepted = acceptedWithFindings(run([review], { answers: [{ step: 'code-review', decision: 'accept', by: 'owner' }] }), review, answered);
  assert.ok(accepted?.startsWith('Accepted by you with 1 finding open · '), accepted ?? 'null');
  const byOrchestrator = acceptedWithFindings(run([review], { answers: [{ step: 'code-review', decision: 'accept', by: 'orchestrator', why: 'nit' }] }), review, []);
  assert.equal(byOrchestrator, 'Accepted by the orchestrator with 1 finding open');
  // A clean pass, or a step whose last answer was not accept, reads as before.
  const clean = node('code-review', { structuredOutput: { verdict: 'pass', findings: [] } });
  assert.equal(acceptedWithFindings(run([clean], { answers: [{ step: 'code-review', decision: 'accept' }] }), clean, []), null);
  assert.equal(acceptedWithFindings(run([review], { answers: [{ step: 'code-review', decision: 'another-round' }] }), review, []), null);
  assert.equal(acceptedWithFindings(run([review]), review, []), null);
}

{ // money_shows_sub_cent_costs_as_under_a_cent
  assert.equal(money(0), '$0.00');
  assert.equal(money(4_000), '<$0.01');
  assert.equal(money(9_999), '<$0.01');
  assert.equal(money(17_850), '$0.02');
  assert.equal(money(1_070_000), '$1.07');
  assert.equal(costTotal([usage('a', 1, { reportedCostMicros: 3_000 }), usage('b', 1, { reportedCostMicros: 2_000 })]), '<$0.01');
  assert.equal(costTotal([usage('a', 1, { reportedCostMicros: null, estimatedCostMicros: 4_000 })]), '<$0.01 est.');
  assert.equal(costTotal([usage('a', 1, { reportedCostMicros: null })]), '');
}

{ // the_cost_tab_lists_every_attempt_with_its_time_and_the_known_model
  // spec ran three times without usage rows; build ran twice, with rows; checks is a command step.
  const nodes = [
    node('spec', { attempt: 3, startedAtMs: 100_000, finishedAtMs: 160_000 }),
    node('build', { attempt: 2, startedAtMs: 300_000, finishedAtMs: 420_000 }),
    node('checks', { provider: '', startedAtMs: 500_000, finishedAtMs: 505_000 })
  ];
  const timeline = [
    item('start', 'spec', 1, 0), item('finish', 'spec', 1, 30_000),
    item('start', 'spec', 2, 40_000), item('review', 'spec', 2, 52_000),
    item('start', 'spec', 3, 100_000),
    item('start', 'build', 1, 200_000), item('finish', 'build', 1, 290_000),
    item('start', 'build', 2, 300_000)
  ];
  const rows = [usage('build', 1), usage('build', 2)];
  const lines = costLines(run(nodes), rows, timeline, 999_000);
  assert.deepEqual(lines.map((line) => [line.step, line.attempt, line.model, line.time, line.row !== null, line.engine]), [
    ['spec', 1, 'sonnet', '30s', false, false],
    ['spec', 2, 'sonnet', '12s', false, false],
    ['spec', 3, 'sonnet', '1m', false, false],
    ['build', 1, 'claude-sonnet', '1m', true, false],
    ['build', 2, 'claude-sonnet', '2m', true, false],
    ['checks', 1, 'engine', '5s', false, true]
  ]);
  // An earlier attempt with no end in the log has no time.
  assert.equal(costLines(run(nodes), rows, [item('start', 'spec', 1, 0)], 999_000)[0].time, '—');

  assert.deepEqual(costSummary(rows, lines), { total: '$0.04', note: 'Partial: 3 step attempts not measured' });
  assert.deepEqual(costSummary([], lines), { total: 'Not measured', note: '' });
  assert.deepEqual(costSummary([usage('build', 1, { reportedCostMicros: null })], lines), { total: 'Not priced', note: 'Partial: 3 step attempts not measured' });
  assert.deepEqual(costSummary(null, lines), { total: '—', note: '' });
  assert.deepEqual(costSummary(rows, lines.filter((line) => line.row || line.engine)), { total: '$0.04', note: '' });
}

console.log('workflowStepInfo tests passed');
