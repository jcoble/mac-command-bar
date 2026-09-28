import assert from 'node:assert/strict';

import { taskWorkflowDefinition } from '../src/lib/shell/workflows/taskWorkflowTemplate.ts';

const definition = taskWorkflowDefinition({
  plan: 'claude',
  implement: 'codex',
  review: 'antigravity'
});

assert.deepEqual(
  definition.roles.map((role) => [role.id, role.providerPolicy.provider]),
  [
    ['planner', 'claude'],
    ['implementer', 'codex'],
    ['reviewer', 'antigravity'],
    ['verifier', 'antigravity'],
    ['pr-author', 'codex'],
    ['pr-reviewer', 'antigravity'],
    ['pr-merger', 'codex']
  ]
);
assert.deepEqual(
  definition.nodes.map((node) => [node.id, node.dependsOn, node.approvalGate !== null]),
  [
    ['plan', [], false],
    ['plan-review', ['plan'], true],
    ['implement', ['plan', 'plan-review'], true],
    ['review', ['plan', 'implement'], false],
    ['verify', ['review'], false],
    ['open-pr', ['implement', 'verify'], false],
    ['pr-review', ['implement', 'open-pr'], false],
    ['merge-pr', ['plan', 'open-pr', 'pr-review'], false]
  ]
);
assert.equal(definition.concurrency.global, 1);
assert.equal(definition.concurrency.workflow, 1);
assert.ok(
  definition.roles.every((role) =>
    ['codex', 'claude', 'antigravity'].every((provider) =>
      role.providerPolicy.allowedProviders.includes(provider)
    )
  )
);
assert.equal(definition.budgets.maximumActiveAgents, 1);
assert.equal(definition.budgets.maximumWorktrees, 0);
assert.equal(definition.nodes.find((node) => node.id === 'plan-review')?.roleId, 'reviewer');
assert.deepEqual(definition.nodes.find((node) => node.id === 'plan-review')?.condition, { redoNodeId: 'plan' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'review')?.condition, { redoNodeId: 'implement' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'pr-review')?.condition, { redoNodeId: 'implement' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'merge-pr')?.condition, { ownerMergeNodeId: 'plan' });
assert.equal(definition.roles.find((role) => role.id === 'verifier')?.outputContract, 'VerificationReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-author')?.outputContract, 'PullRequestReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-reviewer')?.outputContract, 'ReviewReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-merger')?.outputContract, 'PullRequestMergeReceipt');

console.log('task workflow template tests passed');
