import { test } from 'node:test';
import assert from 'node:assert/strict';
import { stripTypeScriptTypes } from 'node:module';
import { normalizeProvider } from '../src/lib/shell/ownedSessions.ts';
import * as historyModel from '../src/lib/shell/history/sessionHistoryViewModel.ts';
import { readFileSync } from 'node:fs';

import {
  sessionHistoryLoadOutcome,
  type RepositoryCheckouts
} from '../src/lib/shell/history/sessionHistoryLoad.ts';
import { sessionHistoryActions, sessionTranscriptImport } from '../src/lib/shell/history/components/sessionHistoryActions.ts';
import type { SessionLibraryRecord } from '../src/lib/shell/history/library/sessionLibraryModel.ts';

function record(key: string): SessionLibraryRecord {
  return {
    key,
    source: 'provider',
    machine: 'local',
    ownedId: null,
    provider: 'test',
    nativeSessionId: key,
    canonicalCwd: '/work/project',
    title: key,
    description: null,
    projectGroupKey: 'project:p1',
    projectGroupLabel: 'project',
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
    new URL('../src/lib/shell/history/components/HistoryPanel.svelte', import.meta.url),
    'utf8'
  );
  assert.match(panel, /let historyLoadVersion = 0;/);
  assert.match(panel, /let detailLoadVersion = 0;/);
  assert.match(panel, /function releaseDetails[\s\S]*?detailLoadVersion \+= 1;/);
  assert.match(panel, /function stopHistoryLoads[\s\S]*?historyLoadVersion \+= 1;[\s\S]*?loadingProjectKey = null;/);
  assert.match(panel, /async function loadCardDetails[\s\S]*?\+\+detailLoadVersion/);
  assert.doesNotMatch(panel, /\bloadVersion\b/);
});


test('history import is offered only where a transcript reader exists', () => {
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationStore.svelte.ts', import.meta.url), 'utf8');
  const childRules = source.slice(source.indexOf('function normalizeChildProvider('), source.indexOf('function normalizedConversationChildren('));
  const normalizeChild = Function('normalizeProvider', 'hasTranscriptReader', 'asString', 'isRecord',
    stripTypeScriptTypes(childRules, { mode: 'strip' }) + '; return normalizedConversationChild;'
  )(normalizeProvider, historyModel.hasTranscriptReader, (value: unknown) => typeof value === 'string' ? value : null,
    (value: unknown) => value !== null && typeof value === 'object');
  for (const provider of ['codex', 'claude', 'antigravity', 'gemini', 'agy', 'opencode', 'anthropic', 'openai', 'claudecode']) {
    const row = { ...record(provider), provider, logPath: '/work/transcript.jsonl' };
    const canonical = normalizeProvider(provider).agent;
    const expected = canonical === 'codex' || canonical === 'claude';
    const child = normalizeChild({ provider, childId: 'child', parentId: 'root' }, null, 'antigravity', 'root', 1, 1, []);
    assert.equal(child.transcriptAvailable, expected, `${provider} child`);
    assert.equal(sessionTranscriptImport(row) !== null, expected, provider);
    assert.equal(sessionHistoryActions(row).find((action) => action.id === 'resume-assembly')?.enabled, expected, provider);
  }
});
