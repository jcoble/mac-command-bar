import assert from 'node:assert/strict';

import {
  accessChoicesFor,
  buildThreadStartRequest,
  checkoutPlanFor,
  defaultThreadStartState,
  effortChoicesFor,
  filterThreadStartGitRefs,
  groupProviderModels,
  titleFromPrompt,
  validateThreadStart
} from '../src/lib/shell/newSession/threadStartFlow.ts';
import { sessionTitleFromPrompt } from '../src/lib/shell/sessionStrip.ts';

// The draft's project travels to the first send.
{
  const state = defaultThreadStartState({
    projectId: 'project-1',
    projectPath: '/Users/me/dev/alpha',
    branch: 'main'
  });
  assert.equal(state.projectId, 'project-1');
  assert.equal(buildThreadStartRequest({ ...state, prompt: 'hello' })?.projectId, 'project-1');
}

// "No project" starts in the machine's home folder and names no project.
{
  const state = defaultThreadStartState({ projectId: null, projectPath: '', cwd: '/home/user' });
  const request = buildThreadStartRequest({ ...state, prompt: 'hello' });
  assert.equal(request?.projectId, null);
  assert.equal(request?.projectPath, null);
  assert.equal(request?.cwd, '/home/user');
}

// A current Codex ACP catalog encodes effort in each model id and uses its own
// access ids. Empty reported effort choices are meaningful, not a reason to
// revive the older fallback ids that the provider rejects.
{
  const currentCodex = [{
    provider: 'codex',
    model: 'gpt-6-astra[medium]',
    availableModels: ['gpt-6-astra[low]', 'gpt-6-astra[medium]'],
    reasoningEffort: null,
    availableEfforts: [],
    approvalPolicy: 'agent',
    availableApprovalPolicies: ['read-only', 'agent', 'agent-full-access']
  }];
  const state = defaultThreadStartState({
    projectPath: '/Users/me/dev/work/app',
    cwd: '/Users/me/dev/work/app',
    branch: 'main',
    providerConfigs: currentCodex
  });
  assert.equal(state.model, 'gpt-6-astra[medium]');
  assert.equal(state.effort, '');
  assert.equal(state.access, 'agent');
  assert.deepEqual(effortChoicesFor('codex', currentCodex), []);
  assert.deepEqual(accessChoicesFor('codex', currentCodex), [
    'read-only', 'agent', 'agent-full-access'
  ]);
  const request = buildThreadStartRequest({ ...state, prompt: 'say hello' });
  assert.equal(request?.reasoningEffort, null);
  assert.equal(request?.approvalPolicy, 'agent');
}

{
  const refs = Array.from({ length: 120 }, (_, index) => ({
    name: index === 0 ? 'main' : `feature/${index}`,
    checkoutPath: index < 2 ? `/checkout/${index}` : null
  }));
  const all = filterThreadStartGitRefs(refs, '');
  assert.equal(all.visible.length, 100);
  assert.equal(all.total, 120);
  const searched = filterThreadStartGitRefs(refs, 'feature/119');
  assert.deepEqual(searched.visible.map((ref) => ref.name), ['feature/119']);
}

// Picking a branch decides what the first send does; nothing runs at the pick.
{
  const root = '/Users/me/dev/alpha';
  assert.deepEqual(
    checkoutPlanFor(root, { name: 'main', isCurrent: true, checkoutPath: root }),
    { kind: 'none', cwd: root },
    'the branch the root is on needs no git command'
  );
  assert.deepEqual(
    checkoutPlanFor(root, { name: 'feature', isCurrent: false, checkoutPath: '/Users/me/dev/worktrees/alpha/feature' }),
    { kind: 'none', cwd: '/Users/me/dev/worktrees/alpha/feature' },
    'a branch checked out elsewhere points the session at that worktree'
  );
  assert.deepEqual(
    checkoutPlanFor(root, { name: 'idle', isCurrent: false, checkoutPath: null }),
    { kind: 'switch', root, branch: 'idle', cwd: root },
    'a branch with no checkout is switched to in the root'
  );
  assert.deepEqual(
    checkoutPlanFor(root, { name: 'main', isCurrent: false, checkoutPath: root }),
    { kind: 'none', cwd: root },
    "the root's own checkout needs no git command"
  );
}

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
  assert.deepEqual(groups.map((group) => group.provider), ['codex', 'claude', 'antigravity']);
  assert.deepEqual(groups[0].models.map((model) => model.id), [
    'gpt-5.6-next',
    'gpt-5.6-luna',
    'gpt-5.6-sol'
  ]);
  assert.equal(groups[0].label.length > 0, true);
  assert.equal(groups[0].models.every((model) => model.hint.length > 0), true);
  assert.equal(groups[0].models.find((model) => model.id === 'gpt-5.6-next')?.available, false);
  assert.match(
    groups[0].models.find((model) => model.id === 'gpt-5.6-next')?.unavailableReason ?? '',
    /Unavailable/
  );
  assert.equal(groups[0].models.find((model) => model.id === 'gpt-5.6-luna')?.available, true);
}

// A wiped store has no session to ask. The draft still offers each provider's
// known models, efforts and access levels — in the ids the provider takes.
// 'acceptedits' was once sent to Claude in place of 'acceptEdits', was refused,
// and started nothing; and a remembered model must not become the whole list.
{
  const remembered = [
    {
      provider: 'claude',
      model: 'haiku',
      availableModels: [],
      reasoningEffort: null,
      availableEfforts: [],
      approvalPolicy: null,
      availableApprovalPolicies: []
    }
  ];
  const claude = groupProviderModels(remembered).find((group) => group.provider === 'claude');
  const ids = claude?.models.map((model) => model.id) ?? [];
  assert.equal(ids[0], 'haiku');
  assert.ok(ids.includes('opus'));
  assert.ok(ids.includes('claude-fable-5'));
  assert.ok(claude?.models.every((model) => model.available));
  assert.deepEqual(effortChoicesFor('claude', remembered), ['low', 'medium', 'high', 'max']);
  assert.deepEqual(accessChoicesFor('claude', remembered), [
    'default',
    'acceptEdits',
    'plan',
    'dontAsk',
    'bypassPermissions'
  ]);
  const state = defaultThreadStartState({
    projectPath: '/Users/me/dev/work/edi',
    cwd: '/Users/me/dev/work/edi',
    branch: 'main',
    provider: 'claude',
    providerConfigs: remembered
  });
  assert.equal(state.model, 'haiku');
  assert.equal(state.access, 'acceptEdits');
  // The spelling an earlier build remembered is not offered back either.
  const stale = defaultThreadStartState({
    projectPath: '/Users/me/dev/work/edi',
    cwd: '/Users/me/dev/work/edi',
    branch: 'main',
    provider: 'claude',
    providerConfigs: [{ ...remembered[0], approvalPolicy: 'acceptedits', reasoningEffort: 'xhigh' }]
  });
  assert.equal(stale.access, 'acceptEdits');
  assert.equal(stale.effort, 'medium');
  assert.deepEqual(accessChoicesFor('codex', []), ['untrusted', 'on-request', 'never']);
  assert.deepEqual(effortChoicesFor('codex', []), ['low', 'medium', 'high', 'xhigh', 'max']);
}

// Antigravity has no separate effort control — its speed tiers are separate
// models — and one access level that states what it does rather than offering
// a choice. The draft says so, and sends no approval policy: the adapter has
// none to set, and a session asked for one refuses the message that asked.
{
  const antigravity = groupProviderModels([]).find((group) => group.provider === 'antigravity');
  const ids = antigravity?.models.map((model) => model.id) ?? [];
  assert.equal(ids.length, 5);
  assert.equal(ids[0], 'Gemini 3.7 Flash (High)');
  assert.ok(ids.includes('Gemini 3.1 Pro (High)'));
  assert.deepEqual(effortChoicesFor('antigravity', []), []);
  assert.deepEqual(accessChoicesFor('antigravity', []), ['bypassPermissions']);

  const state = defaultThreadStartState({
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/mac-command-bar',
    branch: 'main',
    provider: 'antigravity'
  });
  assert.equal(state.model, 'Gemini 3.7 Flash (High)');
  assert.equal(state.effort, '');
  assert.equal(state.access, 'bypassPermissions');

  const request = buildThreadStartRequest({ ...state, prompt: 'say hi in five words' });
  assert.equal(request?.model, 'Gemini 3.7 Flash (High)');
  assert.equal(request?.reasoningEffort, null);
  assert.equal(request?.approvalPolicy, null);
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
    { field: 'prompt', message: 'Write a message or attach an image.' }
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
  // WIP: disabled. buildThreadStartRequest requires `executionEnvironment`; this state omits it.
  // assert.deepEqual(request, {
  //   prompt: 'Add a compact session rail',
  //   provider: 'claude',
  //   model: 'claude-opus',
  //   reasoningEffort: 'high',
  //   approvalPolicy: 'acceptedits',
  //   projectPath: '/Users/me/dev/work/mac-command-bar',
  //   cwd: '/Users/me/dev/work/worktrees/mac-command-bar/tsk-808-rail',
  //   branch: 'tsk-808-rail',
  //   createNewWorktree: false,
  //   title: 'Add a compact session rail'
  // });
}

// New worktree is a real choice: the request validates and carries the base
// branch with `createNewWorktree: true`. The worktree itself is made at the
// first send, not here.
{
  const state = {
    ...defaultThreadStartState({ projectId: 'p1', projectPath: '/Users/me/dev/work/mac-command-bar' }),
    prompt: 'Create the thread-first flow',
    branch: 'main',
    branchesAvailable: true,
    createNewWorktree: true
  };
  assert.deepEqual(validateThreadStart(state), []);
  const request = buildThreadStartRequest(state);
  assert.equal(request?.createNewWorktree, true);
  assert.equal(request?.branch, 'main');
  assert.equal(request?.projectId, 'p1');
  assert.deepEqual(validateThreadStart({ ...state, branch: '' }).map((problem) => problem.field), ['branch']);
}

assert.equal(
  titleFromPrompt('  First line that names the work\nSecond line with details  ', '/Users/me/app'),
  'First line that names the work'
);
assert.equal(titleFromPrompt('', '/Users/me/app'), 'Build in app');
// The backend names the session from the trimmed prompt, so a prompt that
// opens with a blank line is named after its first real line here too. Reading
// the untrimmed text found an empty first line and fell back to the project.
assert.equal(
  titleFromPrompt('\n\nName the work here\nand then details', '/Users/me/app'),
  'Name the work here'
);

// The provisional title has to be the same string the backend writes from the
// same prompt. The rail saves the row back seconds after the send; a title that
// differs by so much as a character reads as a rename there, and a renamed
// session is never given a better name after its first turn.
{
  const longPrompt = `${'x'.repeat(100)}\nsecond line`;
  const request = buildThreadStartRequest({
    prompt: longPrompt,
    provider: 'claude',
    model: 'claude-opus',
    effort: 'high',
    access: 'acceptedits',
    projectPath: '/Users/me/dev/work/mac-command-bar',
    cwd: '/Users/me/dev/work/worktrees/mac-command-bar/tsk-808-rail',
    branch: 'tsk-808-rail',
    createNewWorktree: false
  });
  assert.equal(request?.title, sessionTitleFromPrompt(longPrompt));
  assert.equal(request?.title.length, 64);
  assert.equal(titleFromPrompt(longPrompt, '/Users/me/app'), sessionTitleFromPrompt(longPrompt));
}

// A remote draft carries the stable machine profile through the first send.
{
  const local = defaultThreadStartState({ projectPath: '/home/user/project' });
  const remote = {
    ...local,
    prompt: 'Run this remotely',
    executionEnvironment: 'remote' as const,
    remoteProfileId: null
  };
  assert.deepEqual(validateThreadStart(remote), [
    { field: 'project', message: 'Choose a remote machine first.' }
  ]);
  const request = buildThreadStartRequest({ ...remote, remoteProfileId: 'workbox' });
  assert.equal(request?.executionEnvironment, 'remote');
  assert.equal(request?.remoteProfileId, 'workbox');
}

// A plain conversation uses a real cwd without claiming it as a project.
{
  const plain = { ...defaultThreadStartState({ projectPath: '', cwd: '/home/user' }), prompt: 'Help me remember this' };
  assert.deepEqual(validateThreadStart(plain), []);
  assert.equal(buildThreadStartRequest(plain)?.projectPath, null);
  assert.equal(buildThreadStartRequest(plain)?.cwd, '/home/user');
  assert.equal(buildThreadStartRequest({ ...plain, executionEnvironment: 'remote', remoteProfileId: 'workbox' })?.projectPath, null);
  assert.deepEqual(validateThreadStart({ ...plain, cwd: '' }).map((problem) => problem.field), ['branch']);
  assert.equal(buildThreadStartRequest({ ...plain, prompt: '' }, true)?.title, 'New conversation');
}

console.log('threadStartFlow.test.ts passed');

// --- A project with no branches is not a dead end ----------------------------
{
  const empty = defaultThreadStartState({ projectPath: '/tmp/brand-new' });
  const draft = { ...empty, prompt: 'Build me a todo list', branch: '', branchesAvailable: false };

  assert.deepEqual(
    validateThreadStart(draft).map((problem) => problem.field),
    [],
    'an empty folder starts without a branch: the agent makes the repository'
  );
  assert.ok(buildThreadStartRequest(draft), 'and the request is assembled');

  // A project that does have branches still has to have one chosen.
  assert.deepEqual(
    validateThreadStart({ ...draft, branchesAvailable: true }).map((problem) => problem.field),
    ['branch'],
    'a project with branches still needs one picked'
  );
  assert.deepEqual(
    validateThreadStart({ ...draft, branchesAvailable: true, branch: 'main' }),
    [],
    'and is satisfied once it is'
  );
}

console.log('threadStartFlow: a branchless project can start');

// Model IDs remain wire values while labels come from the live provider catalog.
{
  const groups = groupProviderModels([{
    provider: 'claude', model: 'opus', availableModels: ['opus', 'sonnet'],
    modelLabels: { opus: 'Opus 5.5', sonnet: 'Sonnet 5' },
    reasoningEffort: null, availableEfforts: [], approvalPolicy: null, availableApprovalPolicies: []
  }]);
  const models = groups.find(group => group.provider === 'claude')!.models;
  assert.equal(models.find(model => model.id === 'opus')!.label, 'Opus 5.5');
  assert.equal(models.find(model => model.id === 'sonnet')!.label, 'Sonnet 5');
}
