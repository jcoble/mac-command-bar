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
    ['reviewer', 'antigravity']
  ]
);
assert.deepEqual(
  definition.nodes.map((node) => [node.id, node.dependsOn, node.approvalGate !== null]),
  [
    ['plan', [], true],
    ['implement', ['plan'], true],
    ['review', ['implement'], false]
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

console.log('task workflow template tests passed');
