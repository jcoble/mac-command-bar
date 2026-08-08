/**
 * prPanel.test.mjs — the source-control pull-request view model without a
 * browser or a real GitHub CLI. The backend owns side effects; this keeps the
 * panel's visible lifecycle deterministic and checks its degradation copy.
 */
const assert = (await import('node:assert/strict')).default;

const {
  beginPullRequestGeneration,
  beginPullRequestPush,
  canCreatePullRequest,
  createPullRequestFlowModel,
  failPullRequest,
  finishPullRequestCreation,
  finishPullRequestGeneration,
  noAgentMessage,
  noGhMessage,
  pullRequestActionLabel,
  setPullRequestChecks,
  setPullRequestContext
} = await import('../src/lib/shell/components/git/pr/prFlow.ts');

let model = createPullRequestFlowModel();
assert.equal(model.state, 'idle');
assert.equal(pullRequestActionLabel(model), 'Push & Create PR');

model = setPullRequestContext(model, { branch: 'feature/pr-flow', base: 'main' });
assert.equal(model.branch, 'feature/pr-flow');
assert.equal(model.base, 'main');
assert.equal(canCreatePullRequest(model, true), false, 'title is required before creation');

model = beginPullRequestGeneration(model);
assert.equal(model.state, 'generating');
assert.equal(pullRequestActionLabel(model), 'Generating title & description...');

model = finishPullRequestGeneration(model, {
  title: 'Add source-control pull requests',
  description: 'Adds the generated PR flow.'
});
assert.equal(model.state, 'ready');
assert.equal(canCreatePullRequest(model, true), true);
assert.equal(canCreatePullRequest(model, false), false, 'browser/read-only mode cannot create');

model = beginPullRequestPush(model);
assert.equal(model.state, 'pushing');
assert.equal(pullRequestActionLabel(model), 'Pushing & creating…');

model = finishPullRequestCreation(model, { number: 808, url: 'https://github.com/example/repo/pull/808' });
assert.equal(model.state, 'created');
assert.equal(model.number, 808);
assert.equal(model.url, 'https://github.com/example/repo/pull/808');
assert.equal(pullRequestActionLabel(model), 'PR #808');

model = setPullRequestChecks(model, {
  number: 808,
  url: model.url,
  state: 'OPEN',
  checks: 'pending',
  checkSummary: '0 passing · 1 pending · 0 failing'
});
assert.equal(model.state, 'created');
assert.equal(model.checks, 'pending');
assert.match(model.checkSummary, /pending/);

const noAgent = failPullRequest(createPullRequestFlowModel(), noAgentMessage());
assert.equal(noAgent.state, 'failed');
assert.equal(noAgent.error, 'No active agent session is running. Start an agent conversation to generate this text.');

const noGh = failPullRequest(createPullRequestFlowModel(), noGhMessage());
assert.equal(noGh.state, 'failed');
assert.equal(noGh.error, 'GitHub CLI (`gh`) is not installed or not on PATH');

console.log('prPanel.test.mjs: all assertions passed');
