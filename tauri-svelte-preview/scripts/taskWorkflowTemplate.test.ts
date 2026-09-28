import assert from 'node:assert/strict';

import {
  savedWorkflowDefinition,
  taskWorkflowDefinition,
  type SavedWorkflowStage
} from '../src/lib/shell/workflows/taskWorkflowTemplate.ts';

const definition = taskWorkflowDefinition({
  plan: 'claude',
  implement: 'codex',
  review: 'antigravity'
});

assert.deepEqual(
  definition.roles.map((role) => [role.id, role.providerPolicy.provider]),
  [
    ['specifier', 'claude'],
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
    ['spec', [], false],
    ['spec-review', ['spec'], true],
    ['plan', ['spec', 'spec-review'], false],
    ['plan-review', ['plan'], true],
    ['implement', ['plan', 'plan-review'], true],
    ['review', ['plan', 'implement'], false],
    ['verify', ['review'], false],
    ['open-pr', ['implement', 'verify'], false],
    ['pr-review', ['implement', 'verify', 'open-pr'], false],
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
assert.equal(definition.roles.find((role) => role.id === 'specifier')?.outputContract, 'SpecReceipt');
assert.deepEqual(definition.nodes.find((node) => node.id === 'spec-review')?.condition, { redoNodeId: 'spec' });
assert.equal(definition.nodes.find((node) => node.id === 'plan-review')?.roleId, 'reviewer');
assert.deepEqual(definition.nodes.find((node) => node.id === 'plan-review')?.condition, { redoNodeId: 'plan' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'review')?.condition, { redoNodeId: 'implement' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'pr-review')?.condition, { redoNodeId: 'implement' });
assert.deepEqual(definition.nodes.find((node) => node.id === 'merge-pr')?.condition, { ownerMergeNodeId: 'plan' });
assert.equal(definition.roles.find((role) => role.id === 'verifier')?.outputContract, 'VerificationReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-author')?.outputContract, 'PullRequestReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-reviewer')?.outputContract, 'ReviewReceipt');
assert.equal(definition.roles.find((role) => role.id === 'pr-merger')?.outputContract, 'PullRequestMergeReceipt');

const stage = (
  id: string,
  outputContract: SavedWorkflowStage['outputContract'],
  redoStageId: string | null = null
): SavedWorkflowStage => ({
  id,
  title: id,
  instructions: `Complete ${id} for the approved plan.`,
  provider: outputContract === 'ImplementationReceipt' || outputContract === 'PullRequestReceipt' ? 'codex' : 'claude',
  outputContract,
  redoStageId,
  approvalPrompt: null
});

const approved = {
  id: 'approved-plan',
  name: 'Approved plan to PR',
  stages: [
    stage('implement', 'ImplementationReceipt'),
    stage('review', 'ReviewReceipt', 'implement'),
    stage('verify', 'VerificationReceipt'),
    stage('open-pr', 'PullRequestReceipt'),
    stage('pr-review', 'ReviewReceipt', 'implement'),
    stage('merge', 'PullRequestMergeReceipt')
  ]
};
const saved = savedWorkflowDefinition(approved);
assert.equal(saved.name, approved.name);
assert.throws(() => savedWorkflowDefinition({ ...approved, id: definition.id }), /reserved/);
assert.deepEqual(saved.nodes.map((node) => node.id), approved.stages.map((item) => item.id));
assert.equal(saved.nodes.some((node) => node.id === 'plan'), false);
assert.deepEqual(saved.nodes.find((node) => node.id === 'review')?.dependsOn, ['implement']);
assert.deepEqual(saved.nodes.find((node) => node.id === 'open-pr')?.dependsOn, ['verify', 'implement']);
assert.deepEqual(saved.nodes.find((node) => node.id === 'pr-review')?.dependsOn, ['open-pr', 'implement']);
assert.deepEqual(saved.nodes.find((node) => node.id === 'merge')?.dependsOn, ['pr-review', 'open-pr']);
assert.equal(saved.roles.find((item) => item.id === 'implement')?.workspacePolicy.kind, 'shared-current');
assert.equal(saved.roles.find((item) => item.id === 'merge')?.workspacePolicy.kind, 'read-only-current');
const approvedWithGate = savedWorkflowDefinition({
  ...approved,
  stages: [{ ...approved.stages[0], approvalPrompt: 'Approve implementation before review.' }, ...approved.stages.slice(1)]
});
assert.equal(approvedWithGate.nodes[0].approvalGate?.prompt, 'Approve implementation before review.');
assert.equal(savedWorkflowDefinition({ ...approved, stages: approved.stages.slice(0, -1) }).nodes.some(
  (node) => node.id === 'merge'
), false);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [...approved.stages.slice(0, -1), stage('update-pr', 'PullRequestReceipt')]
}), /one Open PR stage/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [stage('implement', 'ImplementationReceipt'), stage('review', 'ReviewReceipt', 'implement'),
    stage('open-pr', 'PullRequestReceipt')]
}), /Open PR follows implementation, review, and verification/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [stage('implement', 'ImplementationReceipt'), stage('verify', 'VerificationReceipt'),
    stage('review', 'ReviewReceipt', 'implement'), stage('open-pr', 'PullRequestReceipt')]
}), /Open PR follows implementation, review, and verification/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: approved.stages.slice(0, 4)
}), /Review the opened PR/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [...approved.stages.slice(0, 4), stage('late-implement', 'ImplementationReceipt'),
    ...approved.stages.slice(4)]
}), /After Open PR/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [...approved.stages, stage('after-merge', 'VerificationReceipt')]
}), /Merge must be the last stage/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [stage('review', 'ReviewReceipt', 'implement'), stage('implement', 'ImplementationReceipt')]
}), /earlier plan or implementation/);
assert.throws(() => savedWorkflowDefinition({
  ...approved,
  stages: [stage('implement', 'ImplementationReceipt'), stage('implement', 'ImplementationReceipt')]
}), /unique id/);

console.log('task workflow template tests passed');
