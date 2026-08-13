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
const backend = readFileSync(
  new URL('../src/lib/shell/newSession/newSessionBackend.ts', import.meta.url),
  'utf8'
);
const nativeSource = readFileSync(
  new URL('../src-tauri/src/main.rs', import.meta.url),
  'utf8'
);
const flip = readFileSync(
  new URL('../src/lib/shell/newSession/composerFlip.ts', import.meta.url),
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

assert.match(thread, /What should we build in/);
assert.match(thread, /class="thread-start-project-trigger"/);
assert.match(thread, /data-testid="new-session-thread-input"/);
assert.match(thread, /data-testid="new-session-thread-model"/);
assert.match(thread, /data-testid="new-session-thread-effort"/);
assert.match(thread, /data-testid="new-session-thread-access"/);
assert.match(thread, /data-testid="new-session-thread-project"/);
assert.match(thread, /data-testid="new-session-thread-branch"/);
assert.match(thread, /data-testid="new-session-thread-new-worktree"/);
assert.match(thread, /data-testid="new-session-thread-new-project"/);
assert.match(thread, /data-testid="new-session-thread-ref-search"/);
assert.match(thread, /Showing \{filteredRefs\.visible\.length\} of \{filteredRefs\.total\} refs/);
assert.match(thread, /needs a worktree/);
assert.match(thread, /disabled=\{!canSelectThreadStartGitRef\(ref, canCreateWorktree\)\}/);
assert.match(backend, /invoke<ProjectGitRef\[]>\('list_project_git_refs'/);
assert.match(nativeSource, /async fn list_project_git_refs\(/);
assert.match(nativeSource, /list_project_git_refs,/);
assert.match(thread, /groupProviderModels\(providerConfigs\)/);
assert.match(thread, /onSelect=\{\(\) => chooseModel\(group\.provider, model\.id\)\}/);
assert.match(thread, /disabled=\{!model\.available\}/);
assert.match(thread, /class="thread-start-menu-provider">\{group\.label\}/);
assert.match(thread, /await dockComposer\(\);[\s\S]*?await onSend\(request\)/);
assert.doesNotMatch(thread, /The session could not be started\. Check the rail for details\./);
assert.match(thread, /data-composer-state=\{docked \? 'docked' : 'hero'\}/);
assert.match(thread, /Nothing is created while this pane is a draft/);
assert.match(flip, /node\.animate\(/);
assert.match(flip, /duration: FLIP_DURATION_MS, easing: FLIP_EASING/);
assert.match(flip, /prefers-reduced-motion: reduce/);
assert.match(flip, /Math\.abs\(deltaX\) < 0\.5 && Math\.abs\(deltaY\) < 0\.5/);

assert.match(host, /import\('\.\/NewSessionThread\.svelte'\)/);
assert.match(host, /sessionProviderConfigs = \(input\.providerConfigs \?\? \[\]\)\.map/);
assert.match(host, /presetProjectPath = input\.projectPath\?\.trim\(\) \|\| null/);
assert.match(host, /\{presetProjectPath\}/);
assert.match(host, /providerConfigs=\{sessionProviderConfigs\}/);
assert.match(host, /onSend={submit}/);
assert.match(overlays, /<ThreadStartHost/);
assert.match(overlays, /providerConfigs: newSessionProviderConfigs\(\)/);
assert.match(overlays, /projectPath/);
assert.doesNotMatch(overlays, /<ThreadStartHost[^>]*providerConfigs/);
assert.doesNotMatch(overlays, /NewSessionHost|NewSessionDialog/);
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
