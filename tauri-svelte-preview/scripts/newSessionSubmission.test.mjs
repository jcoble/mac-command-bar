import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildNewSessionRequest,
  selectLaunchAgent
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
const restartOwned = functionSource(
  'async function restartOwned(',
  'async function closeTerminal('
);

// The same selection value must paint the card and build the submitted request.
// This is the regression path captured by the native new-session proof: changing
// the selected card may not leave the request on its initial value.
{
  const selection = selectLaunchAgent('codex');
  const request = buildNewSessionRequest({
    cwd: '/Users/me/dev/work/thing',
    title: '',
    ...selection
  });

  assert.equal(selection.agent, 'codex');
  assert.equal(request?.agent, 'codex');
  assert.equal(request?.command, 'codex');
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
  /if \(!startsStructured && !service\)/,
  'only terminal launches require the terminal service'
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

console.log('newSessionSubmission.test.mjs passed');
