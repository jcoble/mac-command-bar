import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  sessionHistoryLoadOutcome,
  type RepositoryCheckouts
} from '../src/lib/shell/history/sessionHistoryLoad.ts';
import { sessionHistoryActions } from '../src/lib/shell/panels/history/sessionHistoryActions.ts';
import type { SessionLibraryRecord } from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';

function record(key: string): SessionLibraryRecord {
  return {
    key,
    source: 'provider',
    ownedId: null,
    provider: 'test',
    nativeSessionId: key,
    canonicalCwd: '/work/project',
    title: key,
    description: null,
    projectPath: '/work/project',
    projectRoot: '/work/project',
    model: null,
    state: 'resumable',
    runtimeState: null,
    lastActivity: null,
    updatedAt: null,
    messageCount: null,
    logPath: null,
    firstPrompt: null,
    latestTurns: [],
    subagents: [],
    owned: null,
    available: null
  };
}

function run(name: string, test: () => void): void {
  test();
  console.log(`pass: ${name}`);
}

run('a_failed_checkout_lookup_still_yields_the_sessions', () => {
  const records = [record('one'), record('two')];
  const outcome = sessionHistoryLoadOutcome({
    requestedKeys: new Set(['one', 'two']),
    refreshed: { status: 'fulfilled', value: records },
    checkouts: { status: 'rejected', reason: new Error('git unavailable') }
  });

  assert.equal(outcome.state, 'ready');
  assert.deepEqual(outcome.records, records);
  assert.deepEqual(outcome.checkouts, {});
});

run('a_scan_that_is_behind_reports_what_is_missing', () => {
  const outcome = sessionHistoryLoadOutcome({
    requestedKeys: new Set(['one', 'two']),
    refreshed: { status: 'fulfilled', value: [record('one')] },
    checkouts: { status: 'fulfilled', value: {} }
  });

  assert.equal(outcome.state, 'incomplete');
  assert.deepEqual(outcome.missingKeys, ['two']);
  assert.deepEqual(outcome.records.map(({ key }) => key), ['one']);
});

run('a_failed_refresh_is_the_only_failure', () => {
  const checkouts: RepositoryCheckouts = {
    '/work/project': [{ path: '/work/project', branch: 'main', isMain: true }]
  };
  const outcome = sessionHistoryLoadOutcome({
    requestedKeys: new Set(['one']),
    refreshed: { status: 'rejected', reason: new Error('scan failed') },
    checkouts: { status: 'fulfilled', value: checkouts }
  });

  assert.equal(outcome.state, 'failed');
  assert.deepEqual(outcome.records, []);
});

run('everything_present_is_ready', () => {
  const checkouts: RepositoryCheckouts = {
    '/work/project': [{ path: '/work/project', branch: 'main', isMain: true }]
  };
  const outcome = sessionHistoryLoadOutcome({
    requestedKeys: new Set(['one', 'two']),
    refreshed: { status: 'fulfilled', value: [record('one'), record('two')] },
    checkouts: { status: 'fulfilled', value: checkouts }
  });

  assert.equal(outcome.state, 'ready');
  assert.deepEqual(outcome.missingKeys, []);
  assert.deepEqual(outcome.checkouts, checkouts);
});

run('an_owned_history_row_opens_instead_of_importing_a_duplicate', () => {
  const owned = record('owned-session');
  owned.source = 'owned';
  owned.ownedId = 'owned-session';
  owned.provider = 'codex';
  owned.logPath = '/tmp/provider-transcript.jsonl';
  const action = sessionHistoryActions(owned).find((candidate) => candidate.id === 'resume-assembly');

  assert.equal(action?.label, 'Open Assembly Session');
  assert.equal(action?.enabled, true);
});

run('project_and_detail_reads_have_independent_lifecycles', () => {
  const panel = readFileSync(
    new URL('../src/lib/shell/panels/history/HistoryPanel.svelte', import.meta.url),
    'utf8'
  );
  assert.match(panel, /let historyLoadVersion = 0;/);
  assert.match(panel, /let detailLoadVersion = 0;/);
  assert.match(panel, /function releaseDetails[\s\S]*?detailLoadVersion \+= 1;/);
  assert.match(panel, /function stopHistoryLoads[\s\S]*?historyLoadVersion \+= 1;[\s\S]*?loadingProjectKey = null;/);
  assert.match(panel, /async function loadCardDetails[\s\S]*?\+\+detailLoadVersion/);
  assert.doesNotMatch(panel, /\bloadVersion\b/);
});
