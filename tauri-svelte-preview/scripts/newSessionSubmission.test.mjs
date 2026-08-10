import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildNewSessionRequest,
  selectLaunchAgent,
  startsStructuredSession
} from '../src/lib/shell/newSession/newSessionFlow.ts';

const page = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
const dialog = readFileSync(
  new URL('../src/lib/shell/components/newSession/NewSessionDialog.svelte', import.meta.url),
  'utf8'
);
const host = readFileSync(
  new URL('../src/lib/shell/components/newSession/NewSessionHost.svelte', import.meta.url),
  'utf8'
);
const sessionCard = readFileSync(
  new URL('../src/lib/shell/components/sessions/SessionCard.svelte', import.meta.url),
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
const selectOwned = functionSource(
  'async function selectOwned(',
  'function handoffInput('
);
const restartOwned = functionSource(
  'async function restartOwned(',
  'async function closeTerminal('
);

assert.match(
  dialog,
  /let selectedLaunch = \$state\(selectLaunchAgent\('codex'\)\)/,
  'Codex is selected before the new-session dialog first opens'
);
assert.match(
  dialog,
  /export function reset[\s\S]*pickAgent\('codex'\)/,
  'reopening the new-session dialog restores the Codex default'
);

// The same selection value must paint the card and build the submitted request.
// This is the regression path captured by the native new-session proof: changing
// the selected card may not leave the request on its initial value.
for (const agent of ['codex', 'claude']) {
  const selection = selectLaunchAgent(agent);
  const request = buildNewSessionRequest({
    cwd: '/Users/me/dev/work/thing',
    title: '',
    ...selection
  });

  assert.deepEqual(request, {
    cwd: '/Users/me/dev/work/thing',
    title: `${agent === 'codex' ? 'Codex' : 'Claude'} in thing`,
    agent,
    command: agent
  });
  assert.equal(startsStructuredSession(request), true, `${agent} must start structured`);
}

assert.match(
  dialog,
  /aria-pressed=\{selectedLaunch\.agent === option\.agent\}[\s\S]*onclick=\{\(\) => pickAgent\(option\.agent\)\}/,
  'the selected card and its click handler share selectedLaunch'
);
assert.match(
  dialog,
  /const draft = \$derived\(\{ cwd, title, \.\.\.selectedLaunch \}\)/,
  'the submitted draft reads the same selectedLaunch value as the card'
);
assert.match(
  dialog,
  /`Start \$\{selectedLaunch\.agent\} session`/,
  'the primary button names the agent that will be submitted'
);
assert.match(
  host,
  /<NewSessionDialog[\s\S]*\{onStart\}/,
  'the host forwards the request callback without substituting an agent'
);

assert.doesNotMatch(
  startNewSession,
  /if \(!service \|\| disposed\) return;/,
  'structured new sessions must not depend on the terminal service'
);
assert.match(
  startNewSession,
  /if \(disposed\) return;/,
  'new-session submission still stops after page disposal'
);
assert.match(
  startNewSession,
  /if \(!startsStructuredSession\(request\) && !service\)/,
  'only terminal launches require the terminal service'
);
assert.match(
  startNewSession,
  /origin: startsStructuredSession\(request\) \? \('app' as const\) : \('external' as const\)/,
  'the same request predicate owns the persisted structured origin'
);

{
  const structuredBranch = startNewSession.indexOf("if (owned.origin === 'app')");
  const structuredSelect = startNewSession.indexOf('await selectOwned(owned.ownedId, true);', structuredBranch);
  const structuredFailure = startNewSession.indexOf("state: 'exited'", structuredBranch);
  assert.ok(structuredBranch >= 0 && structuredSelect >= 0 && structuredFailure >= 0);
  assert.ok(
    structuredSelect < structuredFailure,
    'the new app-owned row must activate its structured surface before connection can reject'
  );
}
assert.match(
  selectOwned,
  /const structuredActivation = ensureStructuredConversation\(\{[\s\S]*if \(!propagateStructuredFailure\)[\s\S]*await structuredActivation;[\s\S]*throw error;/,
  'new-session activation must be able to await and propagate its structured connection failure'
);

assert.doesNotMatch(
  restartOwned,
  /if \(!service \|\| disposed \|\| restarting\.has\(ownedId\)\) return;/,
  'structured restarts must not depend on the terminal service'
);
assert.match(
  restartOwned,
  /if \(disposed \|\| restarting\.has\(ownedId\)\) return;/,
  'restart keeps the disposal and duplicate-click guards'
);
assert.match(
  restartOwned,
  /if \(!service\) \{/,
  'the terminal service is checked only after the structured restart path'
);
assert.match(
  restartOwned,
  /nativeSessionMode: 'resume'/,
  'the structured restart path explicitly resumes the native conversation'
);

assert.match(
  dialog,
  /console\.(?:debug|warn)\('mcb next: new-session submit'/,
  'new-session submit attempts must be visible in future frontend logs'
);

assert.match(
  startNewSession,
  /updateOwnedSession\(owned\.ownedId, \{[\s\S]*state: 'exited',[\s\S]*lastError: describeError\(error\)[\s\S]*\}\);/,
  'a rejected structured start must retain its diagnostic on the session record'
);
assert.match(
  sessionCard,
  /presentAgentError\(session\.lastError\)/,
  'session cards must humanize retained structured-start errors'
);
assert.match(
  sessionCard,
  /data-testid="session-card-error"[\s\S]*\{presentedError\.summary\}/,
  'session cards must render the humanized structured-start error summary'
);

console.log('newSessionSubmission.test.mjs passed');
