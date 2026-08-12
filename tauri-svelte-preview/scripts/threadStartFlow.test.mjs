import assert from 'node:assert/strict';

import {
  buildThreadStartRequest,
  defaultThreadStartState,
  groupProviderModels,
  titleFromPrompt,
  validateThreadStart
} from '../src/lib/shell/newSession/threadStartFlow.ts';

const providerConfigs = [
  {
    provider: 'codex',
    model: 'gpt-5.6-luna',
    availableModels: ['gpt-5.6-sol', 'gpt-5.6-luna'],
    reasoningEffort: 'high',
    availableEfforts: ['low', 'medium', 'high', 'max'],
    approvalPolicy: 'on-request',
    availableApprovalPolicies: ['on-request', 'never']
  },
  {
    provider: 'claude',
    model: 'claude-sonnet',
    availableModels: ['claude-opus', 'claude-sonnet'],
    reasoningEffort: 'medium',
    availableEfforts: ['low', 'medium', 'high'],
    approvalPolicy: 'acceptedits',
    availableApprovalPolicies: ['acceptedits', 'plan']
  }
];

// Provider data stays grouped and current values are retained even when a
// provider reports a model that is not in its advertised list.
{
  const groups = groupProviderModels([
    ...providerConfigs,
    { ...providerConfigs[0], model: 'gpt-5.6-next', availableModels: ['gpt-5.6-luna'] }
  ]);
  assert.deepEqual(groups.map((group) => group.provider), ['codex', 'claude']);
  assert.deepEqual(groups[0].models.map((model) => model.id), [
    'gpt-5.6-next',
    'gpt-5.6-luna',
    'gpt-5.6-sol'
  ]);
  assert.equal(groups[0].label.length > 0, true);
  assert.equal(groups[0].models.every((model) => model.hint.length > 0), true);
}

// Opening a thread is a draft only; the pure default is already usable once a
// project and an existing checkout are supplied.
{
  const state = defaultThreadStartState({
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/mac-command-bar',
    branch: 'main',
    providerConfigs
  });
  assert.equal(state.prompt, '');
  assert.equal(state.provider, 'codex');
  assert.equal(state.model, 'gpt-5.6-luna');
  assert.equal(state.effort, 'high');
  assert.equal(state.access, 'on-request');
  assert.equal(state.createNewWorktree, false);
  assert.deepEqual(validateThreadStart(state), [
    { field: 'prompt', message: 'Describe what you want to build.' }
  ]);

  const claudeState = defaultThreadStartState({
    projectPath: state.projectPath,
    cwd: state.cwd,
    branch: state.branch,
    provider: 'claude',
    providerConfigs
  });
  assert.equal(claudeState.provider, 'claude');
  assert.equal(claudeState.model, 'claude-sonnet');
  assert.equal(claudeState.effort, 'medium');
  assert.equal(claudeState.access, 'acceptedits');
}

// Every picker value is carried into the request assembled for the existing
// session-creation path.
{
  const request = buildThreadStartRequest({
    prompt: '  Add a compact session rail  ',
    provider: 'claude',
    model: 'claude-opus',
    effort: 'high',
    access: 'acceptedits',
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/worktrees/mac-command-bar/tsk-808-rail',
    branch: 'tsk-808-rail',
    createNewWorktree: false
  });
  assert.deepEqual(request, {
    prompt: 'Add a compact session rail',
    provider: 'claude',
    model: 'claude-opus',
    reasoningEffort: 'high',
    approvalPolicy: 'acceptedits',
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/worktrees/mac-command-bar/tsk-808-rail',
    branch: 'tsk-808-rail',
    createNewWorktree: false,
    title: 'Add a compact session rail'
  });
}

// This build can list worktrees but has no creation command. Choosing the
// toggle is therefore an explicit validation failure, never a silent fallback
// to the project checkout.
{
  const state = {
    prompt: 'Create the thread-first flow',
    provider: 'codex',
    model: 'gpt-5.6-luna',
    effort: 'max',
    access: 'on-request',
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/mac-command-bar',
    branch: 'tsk-808-thread',
    createNewWorktree: true
  };
  assert.deepEqual(validateThreadStart(state), [
    {
      field: 'worktree',
      message: 'New worktree creation is not available in this build. Choose an existing checkout.'
    }
  ]);
  assert.equal(buildThreadStartRequest(state), null);
}

assert.equal(
  titleFromPrompt('  First line that names the work\nSecond line with details  ', '/Users/me/app'),
  'First line that names the work'
);
assert.equal(titleFromPrompt('', '/Users/me/app'), 'Build in app');
assert.equal(titleFromPrompt('x'.repeat(100), '/Users/me/app').length, 72);

console.log('threadStartFlow.test.mjs passed');
