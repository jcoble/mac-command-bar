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
 *  3. The SQLite value is normalized, and unreadable data loses the choice
 *     quietly rather than breaking the editor.
 *  4. Launch restores ONE project — the open one. Restoring the whole list is
 *     exactly the "every server wakes at once" behaviour this replaces.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  languageIntelligenceLabel,
  languageIntelligenceOn,
  launchRestoreFor,
  mayUseLanguageServer,
  normalizeLanguageIntelligenceChoices,
  withLanguageIntelligenceChoice,
  workspaceKey
} from './languageIntelligenceMode.ts';

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

test('the SQLite value is normalized and unreadable values are ignored', () => {
  assert.deepEqual(normalizeLanguageIntelligenceChoices(null), {});
  assert.deepEqual(normalizeLanguageIntelligenceChoices('not an object'), {});
  assert.deepEqual(normalizeLanguageIntelligenceChoices([1, 2, 3]), {});
  assert.deepEqual(normalizeLanguageIntelligenceChoices({ '/projects/one': 'yes' }), {});
  assert.deepEqual(
    normalizeLanguageIntelligenceChoices({ '/projects/one/': true, '': false }),
    { '/projects/one': true }
  );
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
