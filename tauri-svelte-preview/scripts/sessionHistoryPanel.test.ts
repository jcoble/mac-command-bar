/**
 * The History panel's action roster.
 *
 * Every action a session card offers is either backed by something real or off
 * with a sentence saying why. These tests pin both halves of that, because the
 * failure they exist to catch is silent: a menu item that looks live, does
 * nothing when pressed, and says nothing about it.
 */
import assert from 'node:assert/strict';

import {
  SESSION_HISTORY_ACTION_IDS,
  sessionHistoryActions,
  type SessionHistoryActionId
} from '../src/lib/shell/panels/history/sessionHistoryActions.ts';
import type { SessionLibraryRecord } from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';

function record(overrides: Partial<SessionLibraryRecord> = {}): SessionLibraryRecord {
  return {
    key: 'provider:alpha|S9|/Users/dev/work/mac-command-bar',
    source: 'provider',
    ownedId: null,
    provider: 'alpha',
    nativeSessionId: 'S9',
    canonicalCwd: '/Users/dev/work/mac-command-bar',
    title: 'Fix the resume rail',
    description: null,
    projectPath: '/Users/dev/work/mac-command-bar',
    model: null,
    state: 'resumable',
    runtimeState: null,
    lastActivity: '2026-08-01T10:00:00Z',
    updatedAt: '2026-08-01T10:00:00Z',
    messageCount: 12,
    logPath: null,
    firstPrompt: null,
    latestTurns: [],
    subagents: [],
    owned: null,
    available: null,
    ...overrides
  };
}

function byId(actions: ReturnType<typeof sessionHistoryActions>, id: SessionHistoryActionId) {
  const action = actions.find((candidate) => candidate.id === id);
  assert.ok(action, `expected an action with id ${id}`);
  return action;
}

const LOG_ACTIONS: SessionHistoryActionId[] = [
  'view-log',
  'open-log',
  'reveal-log',
  'copy-log-path'
];

{
  // The card, the menu and the expanded action row all read this one list, so
  // its order is the order a person sees.
  const actions = sessionHistoryActions(record());
  assert.deepEqual(
    actions.map((action) => action.id),
    [
      'resume-assembly',
      'view-log',
      'copy-resume-command',
      'open-log',
      'reveal-log',
      'open-working-directory',
      'copy-session-id',
      'copy-log-path',
      'delete'
    ]
  );
  assert.deepEqual(actions.map((action) => action.id), [...SESSION_HISTORY_ACTION_IDS]);
}

{
  // No transcript file means no transcript actions, and each one says so.
  const actions = sessionHistoryActions(record({ logPath: null }));
  for (const id of LOG_ACTIONS) {
    const action = byId(actions, id);
    assert.equal(action.enabled, false, `${id} should be off without a log path`);
    assert.ok(
      typeof action.disabledReason === 'string' && action.disabledReason.length > 0,
      `${id} should say why it is off`
    );
  }
}

{
  const actions = sessionHistoryActions(
    record({ logPath: '/Users/dev/.sessions/alpha/S9.jsonl' })
  );
  for (const id of LOG_ACTIONS) {
    const action = byId(actions, id);
    assert.equal(action.enabled, true, `${id} should be on with a log path`);
    assert.equal(action.disabledReason, null);
  }
}

{
  // Only sessions this app started can be deleted.
  const withoutOwned = byId(sessionHistoryActions(record({ ownedId: null })), 'delete');
  assert.equal(withoutOwned.enabled, false);
  assert.ok(withoutOwned.disabledReason);

  const withOwned = byId(sessionHistoryActions(record({ ownedId: 'owned-1' })), 'delete');
  assert.equal(withOwned.enabled, true);
  assert.equal(withOwned.disabledReason, null);
}

{
  const actions = sessionHistoryActions(record({ ownedId: 'owned-1', logPath: '/tmp/S9.jsonl' }));
  assert.deepEqual(
    actions.filter((action) => action.destructive).map((action) => action.id),
    ['delete']
  );
}

{
  // A resume command only exists on a record the scanner produced.
  const withoutCommand = byId(sessionHistoryActions(record()), 'copy-resume-command');
  assert.equal(withoutCommand.enabled, false);
  assert.ok(withoutCommand.disabledReason);

  const withCommand = byId(
    sessionHistoryActions(
      record({
        available: {
          provider: 'alpha',
          id: 'S9',
          title: 'Fix the resume rail',
          model: null,
          projectPath: '/Users/dev/work/mac-command-bar',
          lastActivity: '2026-08-01T10:00:00Z',
          resumeCommands: ['alpha resume S9']
        }
      })
    ),
    'copy-resume-command'
  );
  assert.equal(withCommand.enabled, true);
  assert.equal(withCommand.disabledReason, null);
}

{
  // Nowhere to resume into, and nowhere to open. Resuming hands the
  // conversation back to its agent through the ACP adapter, and an adapter has
  // to be started in a folder, so a row without one says so.
  const actions = sessionHistoryActions(
    record({
      provider: 'claude',
      logPath: '/Users/dev/.sessions/claude/S9.jsonl',
      canonicalCwd: '',
      projectPath: null
    })
  );
  for (const id of ['resume-assembly', 'open-working-directory'] as SessionHistoryActionId[]) {
    const action = byId(actions, id);
    assert.equal(action.enabled, false, `${id} should be off without a folder`);
    assert.ok(action.disabledReason);
  }
  assert.equal(
    byId(actions, 'resume-assembly').disabledReason,
    'This session has no folder recorded.'
  );
}

{
  // A row with an agent this app can read, the id that agent knew the session
  // by, its transcript file and its folder is the one that resumes.
  const ready = byId(
    sessionHistoryActions(
      record({ provider: 'claude', logPath: '/Users/dev/.sessions/claude/S9.jsonl' })
    ),
    'resume-assembly'
  );
  assert.equal(ready.enabled, true);
  assert.equal(ready.disabledReason, null);

  // An agent whose transcripts this app cannot read says that, rather than
  // offering a resume that would land nowhere.
  const unreadable = byId(
    sessionHistoryActions(record({ logPath: '/Users/dev/.sessions/alpha/S9.jsonl' })),
    'resume-assembly'
  );
  assert.equal(unreadable.enabled, false);
  assert.equal(unreadable.disabledReason, 'This app cannot read transcripts written by this agent.');
}

{
  // Every record has some identity to copy, so this one is never off.
  assert.equal(byId(sessionHistoryActions(record()), 'copy-session-id').enabled, true);
  assert.equal(
    byId(sessionHistoryActions(record({ nativeSessionId: null })), 'copy-session-id').enabled,
    true
  );
}

console.log('sessionHistoryPanel: all tests passed');
