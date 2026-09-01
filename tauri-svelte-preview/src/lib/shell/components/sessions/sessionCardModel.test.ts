/**
 * What a session card says, checked without a browser.
 *
 * Run it with:
 *   node --experimental-strip-types src/lib/shell/components/sessions/sessionCardModel.test.ts
 *
 * The rules worth pinning down are the two that can quietly lie: a session
 * marked done whose terminal is still running must not read "Done", and a fact
 * the store does not carry must be missing rather than blank.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { messageCountLabel, sessionFacts, sessionStatus } from './sessionCardModel.ts';
import type { OwnedSession } from '../../ownedSessions.ts';

const NOW = new Date('2026-07-30T12:00:00.000Z');

function session(overrides: Partial<OwnedSession> = {}): OwnedSession {
  return {
    ownedId: 'owned-1',
    executionEnvironment: 'local',
    agent: 'claude',
    viaCmux: false,
    source: 'scanned',
    title: 'A session',
    projectPath: '/Users/someone/dev/work/mac-command-bar',
    cwd: '/Users/someone/dev/work/mac-command-bar',
    resumeCommand: null,
    nativeSessionId: null,
    ptySessionId: null,
    state: 'live',
    completedAt: null,
    settledAt: null,
    branch: null,
    taskId: null,
    pullRequest: null,
    messageCount: null,
    latestTurnPreview: null,
    lastActivity: null,
    ...overrides
  };
}

test('a running terminal reads Running, attached or not', () => {
  assert.equal(sessionStatus(session({ state: 'live' })).word, 'Running');
  assert.equal(sessionStatus(session({ state: 'background' })).word, 'Running');
});

test('a session marked done whose terminal is still running says so', () => {
  const status = sessionStatus(session({ state: 'live', completedAt: '2026-07-30T09:00:00.000Z' }));
  assert.equal(status.word, 'Running');
  assert.match(status.hint, /still running/);
});

test('marked done with the terminal ended reads Done', () => {
  const status = sessionStatus(
    session({ state: 'exited', completedAt: '2026-07-30T09:00:00.000Z' })
  );
  assert.equal(status.word, 'Done');
  assert.equal(status.tone, 'done');
});

test('an ended terminal that was never marked done reads Stopped', () => {
  const status = sessionStatus(session({ state: 'exited' }));
  assert.equal(status.word, 'Stopped');
  assert.match(status.hint, /Start it again/);
});

test('a count of turns is a sentence, and zero is nothing at all', () => {
  assert.equal(messageCountLabel(1), '1 message');
  assert.equal(messageCountLabel(12), '12 messages');
  assert.equal(messageCountLabel(0), null);
  assert.equal(messageCountLabel(null), null);
});

test('facts the store does not carry are left out, not printed empty', () => {
  const labels = sessionFacts(session(), NOW).map((fact) => fact.label);
  assert.deepEqual(labels, ['Folder', 'Where it came from']);
});

test('facts the store does carry are all there, with the exact time kept', () => {
  const facts = sessionFacts(
    session({
      source: 'fresh',
      messageCount: 12,
      lastActivity: '2026-07-30T11:00:00.000Z',
      completedAt: '2026-07-30T11:30:00.000Z',
      resumeCommand: 'claude --resume abc123'
    }),
    NOW
  );
  const byLabel = new Map(facts.map((fact) => [fact.label, fact]));

  assert.equal(byLabel.get('Where it came from')?.value, 'Started here');
  assert.equal(byLabel.get('Conversation so far')?.value, '12 messages (at least)');
  assert.equal(byLabel.get('Last thing the scan saw')?.value, '1h ago');
  assert.ok((byLabel.get('Last thing the scan saw')?.exact ?? '').length > 0);
  assert.equal(byLabel.get('Starts back up with')?.value, 'claude --resume abc123');
});
