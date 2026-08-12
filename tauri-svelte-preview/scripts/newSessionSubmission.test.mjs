import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildThreadStartRequest,
  validateThreadStart
} from '../src/lib/shell/newSession/threadStartFlow.ts';

const page = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
const thread = readFileSync(
  new URL('../src/lib/shell/newSession/NewSessionThread.svelte', import.meta.url),
  'utf8'
);
const host = readFileSync(
  new URL('../src/lib/shell/newSession/ThreadStartHost.svelte', import.meta.url),
  'utf8'
);
const overlays = readFileSync(
  new URL('../src/lib/shell/components/ShellOverlays.svelte', import.meta.url),
  'utf8'
);
const conversationService = readFileSync(
  new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url),
  'utf8'
);

function functionSource(signature, nextSignature) {
  const start = page.indexOf(signature);
  const end = page.indexOf(nextSignature, start);
  assert.notEqual(start, -1, `${signature} must exist`);
  assert.notEqual(end, -1, `${signature} must have a bounded source block`);
  return page.slice(start, end);
}

const startNewSession = functionSource(
  'async function startNewSession(',
  'async function onStartStack('
);

assert.match(thread, /What should we build in <span>{projectName}<\/span>\?/);
assert.match(thread, /data-testid="new-session-thread-input"/);
assert.match(thread, /data-testid="new-session-thread-model"/);
assert.match(thread, /data-testid="new-session-thread-effort"/);
assert.match(thread, /data-testid="new-session-thread-access"/);
assert.match(thread, /data-testid="new-session-thread-project"/);
assert.match(thread, /data-testid="new-session-thread-branch"/);
assert.match(thread, /data-testid="new-session-thread-new-worktree"/);
assert.match(thread, /groupProviderModels\(providerConfigs\)/);
assert.match(thread, /onSelect=\{\(\) => chooseModel\(group\.provider, model\.id\)\}/);
assert.match(thread, /Nothing is created while this pane is a draft/);

assert.match(host, /import\('\.\/NewSessionThread\.svelte'\)/);
assert.match(host, /onSend={submit}/);
assert.match(overlays, /<ThreadStartHost/);
assert.doesNotMatch(overlays, /NewSessionHost|NewSessionDialog/);

// The first-send request keeps every chosen picker value and rejects the
// unsupported worktree branch before any route-owned side effect can run.
const request = buildThreadStartRequest({
  prompt: 'Build the new thread pane',
  provider: 'codex',
  model: 'gpt-5.6-luna',
  effort: 'max',
  access: 'on-request',
  projectPath: '/Users/me/dev/work/mac-command-bar',
  cwd: '/Users/me/dev/work/worktrees/mac-command-bar/tsk-808-thread',
  branch: 'tsk-808-thread',
  createNewWorktree: false
});
assert.equal(request?.prompt, 'Build the new thread pane');
assert.equal(request?.cwd.endsWith('tsk-808-thread'), true);
assert.equal(request?.branch, 'tsk-808-thread');
assert.equal(request?.model, 'gpt-5.6-luna');
assert.equal(request?.reasoningEffort, 'max');
assert.equal(request?.approvalPolicy, 'on-request');
assert.deepEqual(validateThreadStart({ ...request, prompt: request.prompt }), []);

assert.match(startNewSession, /async function startNewSession\(request: ThreadStartRequest\): Promise<boolean>/);
assert.match(startNewSession, /createFreshSession\(\{ cwd: request\.cwd, title: request\.title \}\)/);
assert.match(startNewSession, /projectPath: request\.projectPath/);
assert.match(startNewSession, /branch: request\.branch/);
assert.match(startNewSession, /await selectOwned\(owned\.ownedId, true, request\.reasoningEffort/);
assert.match(startNewSession, /setAgentConversationConfig\(/);
assert.match(startNewSession, /await sendStructuredMessage\(owned\.ownedId, request\.prompt\)/);
assert.match(startNewSession, /return false;/);
assert.match(conversationService, /reasoningEffort\?: string \| null;/);
assert.match(conversationService, /nativeSessionMode: input\.nativeSessionMode \?\? 'resume'/);

console.log('newSessionSubmission.test.mjs passed');
