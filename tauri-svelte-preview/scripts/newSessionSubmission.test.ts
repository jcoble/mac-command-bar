import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildThreadStartRequest,
  validateThreadStart
} from '../src/lib/shell/newSession/threadStartFlow.ts';

const page = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
const surface = readFileSync(
  new URL('../src/lib/shell/newSession/DraftSessionSurface.svelte', import.meta.url),
  'utf8'
);
const backend = readFileSync(
  new URL('../src/lib/shell/newSession/newSessionBackend.ts', import.meta.url),
  'utf8'
);
const nativeSource = readFileSync(
  new URL('../src-tauri/src/main.rs', import.meta.url),
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

// The draft surface is a session, not a form: the ordinary composer, with the
// pickers riding on it.
assert.match(surface, /data-testid="draft-session-surface"/);
assert.match(surface, /<ConversationComposer/);
assert.match(surface, /data-testid="draft-session-project"/);
assert.match(surface, /data-testid="draft-session-branch"/);
assert.match(surface, /data-testid="draft-session-provider"/);
assert.match(surface, /data-testid="draft-session-new-project"/);
assert.match(surface, /data-testid="draft-session-ref-search"/);
assert.match(surface, /Showing \{filteredRefs\.visible\.length\} of \{filteredRefs\.total\} branches/);
assert.match(surface, /disabled=\{!canSelectThreadStartGitRef\(ref, canCreateWorktree\)\}/);
assert.match(surface, /groupProviderModels\(providerConfigs\)/);
assert.match(surface, /Nothing is created until you send\./);
// A project with no repository behind it answers with nothing, so the reply is
// read as a list only when it really is one — see newSessionGitRefs.test.ts.
assert.match(backend, /invoke<ProjectGitRef\[] \| null>\('list_project_git_refs'/);
assert.match(backend, /Array\.isArray\(refs\) \? refs : \[]/);
assert.match(nativeSource, /async fn list_project_git_refs\(/);
assert.match(nativeSource, /list_project_git_refs,/);

assert.match(page, /function providerConfigsForNewSession\(\): ThreadStartProviderConfig\[\]/);
assert.match(page, /function openNewSessionForProject\(projectPath\?: string\): void/);
assert.match(page, /mostRecentProjectPath\(\)/);
assert.doesNotMatch(page, /const providerConfigs = \$derived/);

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

// It answers with the new session's id, so a panel that starts a session on
// somebody's behalf can point at the one it just made.
assert.match(startNewSession, /async function startNewSession\(request: ThreadStartRequest\): Promise<string>/);
assert.match(startNewSession, /return owned\.ownedId;/);
assert.match(startNewSession, /createFreshSession\(\{ cwd: request\.cwd, title: request\.title \}\)/);
assert.match(startNewSession, /projectPath: request\.projectPath/);
assert.match(startNewSession, /branch: request\.branch/);
assert.match(startNewSession, /await selectOwned\(owned\.ownedId, true\)/);
assert.doesNotMatch(startNewSession, /ensureStructuredConversation|setAgentConversationConfig\(/);
assert.match(startNewSession, /await sendStructuredMessage\(owned\.ownedId, request\.prompt, null, \{/);
assert.match(startNewSession, /model: request\.model/);
assert.match(startNewSession, /approvalPolicy: request\.approvalPolicy/);
assert.match(startNewSession, /throw new Error\(message\)/);
assert.match(startNewSession, /could not start \$\{owned\.agent\} session: \$\{detail\}/);
assert.match(conversationService, /reasoningEffort\?: string \| null;/);
assert.match(conversationService, /nativeSessionMode: input\.nativeSessionMode \?\? 'resume'/);

console.log('newSessionSubmission.test.ts passed');
