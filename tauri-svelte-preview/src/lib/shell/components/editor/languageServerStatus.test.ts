/**
 * What the editor's language-server chip says, and when extra lookups wait.
 *
 * Run it with:
 *   node --experimental-strip-types \
 *     src/lib/shell/components/editor/languageServerStatus.test.ts
 *
 * Two things here can quietly lie and so are pinned down:
 *  1. A build that does not report a state, and a browser tab with no desktop
 *     app behind it, must produce NO chip at all — never a guessed one.
 *  2. The wait that holds back inline hints and the second diagnostics read
 *     must always end: on ready, on release, or on its own time limit. A wait
 *     that never ends looks exactly like a hang.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createLanguageServerGate,
  describeLanguageServer,
  languageDisplayName,
  languageServerIsBusy,
  readLanguageServerDetail,
  readLanguageServerState,
  statusMessageIsAboutThisFile
} from './languageServerStatus.ts';

// ── Reading the two additive fields off whatever the backend sent ───────────

test('a missing status has no state and no detail', () => {
  assert.equal(readLanguageServerState(null), null);
  assert.equal(readLanguageServerState(undefined), null);
  assert.equal(readLanguageServerDetail(null), null);
});

test('a build too old to report a state gives nothing away', () => {
  const oldBuildAnswer = {
    language: 'csharp',
    languageID: 'csharp',
    available: true,
    serverName: 'csharp-ls',
    command: 'csharp-ls',
    args: [],
    reason: null
  };
  assert.equal(readLanguageServerState(oldBuildAnswer), null);
  assert.equal(describeLanguageServer('csharp', oldBuildAnswer), null);
});

test('a state the app has never heard of is treated as no answer', () => {
  assert.equal(readLanguageServerState({ state: 'reticulating' }), null);
  assert.equal(readLanguageServerState({ state: 42 }), null);
});

test('every state the backend promises is read back', () => {
  for (const state of ['not-running', 'starting', 'indexing', 'ready', 'disabled'] as const) {
    assert.equal(readLanguageServerState({ state }), state);
  }
});

test('the detail sentence is read when there is one, and skipped when blank', () => {
  assert.equal(
    readLanguageServerDetail({ detail: 'Loading the EdiPlatform solution' }),
    'Loading the EdiPlatform solution'
  );
  assert.equal(readLanguageServerDetail({ detail: null }), null);
  assert.equal(readLanguageServerDetail({ detail: '   ' }), null);
  assert.equal(readLanguageServerDetail({ detail: 7 }), null);
});

// ── Language names a person recognises ──────────────────────────────────────

test('languages are named the way people write them', () => {
  assert.equal(languageDisplayName('csharp'), 'C#');
  assert.equal(languageDisplayName('typescript'), 'TypeScript');
  assert.equal(languageDisplayName('javascript'), 'JavaScript');
  assert.equal(languageDisplayName('rust'), 'Rust');
  assert.equal(languageDisplayName('svelte'), 'Svelte');
});

test('a language nobody mapped still reads as a word, not an id', () => {
  assert.equal(languageDisplayName('kotlin'), 'Kotlin');
  assert.equal(languageDisplayName(''), 'This file');
});

// ── What the chip says ──────────────────────────────────────────────────────

test('there is no chip when there is no status at all', () => {
  assert.equal(describeLanguageServer('csharp', null), null);
});

test('the chip reads plainly in every state', () => {
  const say = (state: string) => describeLanguageServer('csharp', { state, detail: null });

  assert.equal(say('ready')?.label, 'C#: ready');
  assert.equal(say('ready')?.tone, 'ready');

  assert.equal(say('indexing')?.label, 'C#: indexing…');
  assert.equal(say('indexing')?.tone, 'working');

  assert.equal(say('starting')?.label, 'C#: starting…');
  assert.equal(say('starting')?.tone, 'working');

  assert.equal(say('not-running')?.label, 'C#: not running');
  assert.equal(say('not-running')?.tone, 'off');

  assert.equal(say('disabled')?.label, 'C# server is off');
  assert.equal(say('disabled')?.tone, 'off');
});

test('with no sentence from the server the chip explains itself anyway', () => {
  const chip = describeLanguageServer('csharp', { state: 'indexing', detail: null });
  assert.equal(chip?.tooltip, 'The C# language server is reading the project.');
});

test("the server's own sentence wins over the standby wording", () => {
  const chip = describeLanguageServer('csharp', {
    state: 'indexing',
    detail: 'Loading the EdiPlatform solution'
  });
  assert.equal(chip?.tooltip, 'Loading the EdiPlatform solution');
});

// ── Which pushed messages belong to the file on screen ──────────────────────

const CSHARP_UPDATE = {
  root: '/Users/someone/dev/work/EdiPlatform',
  language: 'csharp',
  state: 'indexing',
  detail: 'Loading the EdiPlatform solution'
};

test('a message about the open file and its project is taken', () => {
  assert.equal(
    statusMessageIsAboutThisFile(CSHARP_UPDATE, '/Users/someone/dev/work/EdiPlatform', 'csharp'),
    true
  );
});

test('a message about another project or another language is ignored', () => {
  assert.equal(
    statusMessageIsAboutThisFile(CSHARP_UPDATE, '/Users/someone/dev/work/OtherApp', 'csharp'),
    false
  );
  assert.equal(
    statusMessageIsAboutThisFile(CSHARP_UPDATE, '/Users/someone/dev/work/EdiPlatform', 'rust'),
    false
  );
});

test('with no file open, no message applies', () => {
  assert.equal(statusMessageIsAboutThisFile(CSHARP_UPDATE, null, 'csharp'), false);
  assert.equal(
    statusMessageIsAboutThisFile(CSHARP_UPDATE, '/Users/someone/dev/work/EdiPlatform', null),
    false
  );
});

test('a message that is not shaped like one is ignored', () => {
  assert.equal(statusMessageIsAboutThisFile(null, '/repo', 'csharp'), false);
  assert.equal(statusMessageIsAboutThisFile('indexing', '/repo', 'csharp'), false);
  assert.equal(statusMessageIsAboutThisFile({ root: '/repo' }, '/repo', 'csharp'), false);
});

// ── When the extra lookups wait ─────────────────────────────────────────────

test('only a server that is starting up or reading the project makes work wait', () => {
  assert.equal(languageServerIsBusy('starting'), true);
  assert.equal(languageServerIsBusy('indexing'), true);
  assert.equal(languageServerIsBusy('ready'), false);
  assert.equal(languageServerIsBusy('not-running'), false);
  assert.equal(languageServerIsBusy('disabled'), false);
  // Unknown means an old build or a browser tab: behave exactly as before.
  assert.equal(languageServerIsBusy(null), false);
});

test('with no answer about the server, nothing waits', async () => {
  const gate = createLanguageServerGate();
  let finished = false;
  await gate.waitUntilReady().then(() => (finished = true));
  assert.equal(finished, true);
});

test('while the project is being read, work waits and then runs once', async () => {
  const gate = createLanguageServerGate();
  gate.setState('indexing');

  let finishedCount = 0;
  const first = gate.waitUntilReady().then(() => finishedCount++);
  const second = gate.waitUntilReady().then(() => finishedCount++);

  await Promise.resolve();
  assert.equal(finishedCount, 0, 'nothing should run while the project is being read');

  gate.setState('ready');
  await Promise.all([first, second]);
  assert.equal(finishedCount, 2, 'every waiting job runs once the server is ready');
});

test('a server that turns out to be off or missing stops the waiting too', async () => {
  const offGate = createLanguageServerGate();
  offGate.setState('starting');
  const waiting = offGate.waitUntilReady();
  offGate.setState('disabled');
  await waiting;

  const goneGate = createLanguageServerGate();
  goneGate.setState('indexing');
  const alsoWaiting = goneGate.waitUntilReady();
  goneGate.setState('not-running');
  await alsoWaiting;
});

test('closing the panel releases whatever was waiting', async () => {
  const gate = createLanguageServerGate();
  gate.setState('starting');
  const waiting = gate.waitUntilReady();
  gate.releaseAll();
  await waiting;
});

test('a server that never gets ready still lets the work run in the end', async () => {
  const scheduled: Array<{ run: () => void; afterMs: number }> = [];
  const gate = createLanguageServerGate({
    maxWaitMs: 20_000,
    schedule: (run, afterMs) => {
      const job = { run, afterMs };
      scheduled.push(job);
      return () => {
        const at = scheduled.indexOf(job);
        if (at >= 0) scheduled.splice(at, 1);
      };
    }
  });

  gate.setState('indexing');
  let finished = false;
  const waiting = gate.waitUntilReady().then(() => (finished = true));

  await Promise.resolve();
  assert.equal(finished, false);
  assert.equal(scheduled.length, 1);
  assert.equal(scheduled[0].afterMs, 20_000);

  scheduled[0].run();
  await waiting;
  assert.equal(finished, true, 'the time limit must let the work through');
});

test('the time limit is called off once the server is ready', async () => {
  let cancelled = false;
  const gate = createLanguageServerGate({
    schedule: () => () => {
      cancelled = true;
    }
  });
  gate.setState('starting');
  const waiting = gate.waitUntilReady();
  gate.setState('ready');
  await waiting;
  assert.equal(cancelled, true, 'a finished wait must not leave a timer behind');
});

test('the gate remembers the last state it was told', () => {
  const gate = createLanguageServerGate();
  assert.equal(gate.state, null);
  gate.setState('indexing');
  assert.equal(gate.state, 'indexing');
  assert.equal(gate.isBusy(), true);
  gate.setState('ready');
  assert.equal(gate.isBusy(), false);
});
