/**
 * The editor's two modes, tested where the rules live.
 *
 * Run it with:
 *   node --experimental-strip-types \
 *     src/lib/shell/editor/languageIntelligenceMode.test.ts
 *
 * Four promises are worth breaking a build over:
 *  1. Read mode is the default. A project nobody has switched on never starts
 *     a language server, whatever is stored or missing.
 *  2. The choice belongs to the project, so one project's server is not
 *     another's business.
 *  3. The choice survives a restart, and unreadable storage loses it quietly
 *     rather than breaking the editor.
 *  4. Launch restores ONE project — the open one. Restoring the whole list is
 *     exactly the "every server wakes at once" behaviour this replaces.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  LANGUAGE_INTELLIGENCE_STORAGE_KEY,
  languageIntelligenceLabel,
  languageIntelligenceOn,
  launchRestoreFor,
  mayUseLanguageServer,
  parseStoredLanguageIntelligence,
  readLanguageIntelligenceChoices,
  withLanguageIntelligenceChoice,
  workspaceKey,
  writeLanguageIntelligenceChoices,
  type ModeStorage
} from './languageIntelligenceMode.ts';

function fakeStorage(initial: Record<string, string> = {}): ModeStorage & {
  written: Record<string, string>;
} {
  const written: Record<string, string> = { ...initial };
  return {
    written,
    getItem: (key) => written[key] ?? null,
    setItem: (key, value) => {
      written[key] = value;
    }
  };
}

test('a project nobody has switched on is in read mode', () => {
  assert.equal(languageIntelligenceOn({}, '/projects/one'), false);
  assert.equal(languageIntelligenceOn({ '/projects/two': true }, '/projects/one'), false);
  assert.equal(languageIntelligenceOn({ '/projects/one': false }, '/projects/one'), false);
  assert.equal(languageIntelligenceOn({ '/projects/one': true }, null), false);
});

test('each project keeps its own choice', () => {
  let choices = withLanguageIntelligenceChoice({}, '/projects/one', true);
  assert.equal(languageIntelligenceOn(choices, '/projects/one'), true);
  assert.equal(languageIntelligenceOn(choices, '/projects/two'), false);

  choices = withLanguageIntelligenceChoice(choices, '/projects/two', true);
  choices = withLanguageIntelligenceChoice(choices, '/projects/one', false);
  assert.equal(languageIntelligenceOn(choices, '/projects/one'), false);
  assert.equal(
    languageIntelligenceOn(choices, '/projects/two'),
    true,
    "switching one project off must not switch off the project beside it"
  );
});

test('the same folder written two ways is one project', () => {
  assert.equal(workspaceKey('/projects/one/'), '/projects/one');
  assert.equal(workspaceKey('  /projects/one  '), '/projects/one');
  const choices = withLanguageIntelligenceChoice({}, '/projects/one/', true);
  assert.equal(languageIntelligenceOn(choices, '/projects/one'), true);
});

test('the choice survives a restart', () => {
  const storage = fakeStorage();
  const choices = withLanguageIntelligenceChoice({}, '/projects/one', true);
  assert.equal(writeLanguageIntelligenceChoices(choices, storage), true);
  assert.ok(storage.written[LANGUAGE_INTELLIGENCE_STORAGE_KEY]);
  assert.deepEqual(readLanguageIntelligenceChoices(storage), { '/projects/one': true });
});

test('unreadable storage loses the choice quietly', () => {
  assert.deepEqual(parseStoredLanguageIntelligence(null), {});
  assert.deepEqual(parseStoredLanguageIntelligence('not json'), {});
  assert.deepEqual(parseStoredLanguageIntelligence('[1,2,3]'), {});
  assert.deepEqual(parseStoredLanguageIntelligence('{"/projects/one":"yes"}'), {});
  assert.deepEqual(readLanguageIntelligenceChoices(null), {});
  assert.equal(writeLanguageIntelligenceChoices({}, null), false);
});

test('launch restores the open project only', () => {
  const choices = {
    '/projects/one': true,
    '/projects/two': true,
    '/projects/three': true
  };
  assert.deepEqual(launchRestoreFor(choices, '/projects/two'), {
    root: '/projects/two',
    enabled: true
  });
  assert.equal(
    launchRestoreFor(choices, null),
    null,
    'with no project open, nothing is restored and no server is started'
  );
  assert.deepEqual(
    launchRestoreFor(choices, '/projects/four'),
    { root: '/projects/four', enabled: false },
    'a project with no remembered choice opens in read mode'
  );
});

test('the editor asks for a language server only in full mode, in the desktop app', () => {
  const choices = { '/projects/one': true };
  assert.equal(
    mayUseLanguageServer({ projectRoot: '/projects/one', choices, nativeRuntime: true }),
    true
  );
  assert.equal(
    mayUseLanguageServer({ projectRoot: '/projects/one', choices, nativeRuntime: false }),
    false,
    'a browser tab has no language servers at all'
  );
  assert.equal(
    mayUseLanguageServer({ projectRoot: '/projects/two', choices, nativeRuntime: true }),
    false
  );
  assert.equal(mayUseLanguageServer({ projectRoot: null, choices, nativeRuntime: true }), false);
});

test('the switch says On or Off in plain words', () => {
  assert.equal(languageIntelligenceLabel(true), 'On');
  assert.equal(languageIntelligenceLabel(false), 'Off');
});
