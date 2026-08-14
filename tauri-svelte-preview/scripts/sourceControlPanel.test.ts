/**
 * sourceControlPanel.test.ts — the pure parts of the Source control panel.
 *
 * The panel's two decisions that do not need a browser are which section a
 * changed file belongs in and which actions its context menu offers. Both are
 * plain functions over the status the backend already returns, so they are
 * checked here rather than through a rendered component.
 *
 * The sections module reaches the shared untracked-file test through
 * `gitPanelStore.svelte.ts`, which calls `$state(...)` at module scope for the
 * panel's own state. Node runs that file as plain JavaScript, so the same
 * one-line stand-in `gitPanelStore.test.mjs` uses goes in before the imports:
 * `$state(v)` hands back `v`, which is all the store relies on.
 */

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import type { ProjectGitFileStatus, ProjectGitStatus } from '../src/lib/tauriSource.ts';

const panelSource = readFileSync(
  new URL('../src/lib/shell/panels/sourceControl/SourceControlPanel.svelte', import.meta.url),
  'utf8'
);

assert.match(
  panelSource,
  /\{#if visible\}[\s\S]*?<ScrollArea/,
  'the heavy source-control body must not exist in the document while its tab is hidden'
);

// The store is a plain value bag; this is the shim, not a reactivity stand-in.
(globalThis as unknown as { $state: <T>(value: T) => T }).$state = (value) => value;

const { sourceControlDiffstat, sourceControlSections } = await import(
  '../src/lib/shell/panels/sourceControl/sourceControlSections.ts'
);
const { sourceControlFileActions } = await import(
  '../src/lib/shell/panels/sourceControl/sourceControlFileMenu.ts'
);

function file(
  relativePath: string,
  overrides: Partial<ProjectGitFileStatus> = {}
): ProjectGitFileStatus {
  return {
    relativePath,
    indexStatus: '',
    worktreeStatus: 'modified',
    status: 'modified',
    badge: 'M',
    ...overrides
  };
}

function untracked(relativePath: string): ProjectGitFileStatus {
  return file(relativePath, {
    indexStatus: '?',
    worktreeStatus: '?',
    status: 'untracked',
    badge: '?'
  });
}

// ── sections ────────────────────────────────────────────────────────────────

const emptySections = sourceControlSections(null);
assert.deepEqual(
  emptySections.map((section) => [section.id, section.label, section.files.length]),
  [
    ['untracked', 'Untracked', 0],
    ['changed', 'Changed', 0]
  ],
  'with no status at all both sections still exist, empty, so the panel keeps its shape'
);

const status: ProjectGitStatus = {
  branch: 'tsk-808-source-control',
  ahead: 2,
  behind: 0,
  hasUpstream: true,
  files: [
    file('src/zebra.ts'),
    untracked('docs/new-notes.md'),
    file('README.md', { indexStatus: 'M', worktreeStatus: '', status: 'staged' }),
    untracked('.vscode/')
  ]
};

const sections = sourceControlSections(status);
assert.deepEqual(
  sections.map((section) => [section.id, section.files.map((entry) => entry.relativePath)]),
  [
    ['untracked', ['.vscode/', 'docs/new-notes.md']],
    ['changed', ['README.md', 'src/zebra.ts']]
  ],
  'untracked files come first, everything else follows, each sorted by path'
);

assert.equal(
  sections.filter((section) => section.id === 'untracked').length,
  1,
  'a file never lands in two sections'
);

// ── diffstat ────────────────────────────────────────────────────────────────

assert.deepEqual(
  sourceControlDiffstat(null),
  { filesChanged: 0, untracked: 0 },
  'no repository loaded counts nothing rather than guessing'
);

assert.deepEqual(
  sourceControlDiffstat(status),
  { filesChanged: 4, untracked: 2 },
  'every file in the status counts as changed; the untracked ones are counted again on their own'
);

// ── per-file actions ────────────────────────────────────────────────────────

const READ_ONLY = 'This page can read the repository, not change it.';
const writable = { canWrite: true, readOnlyReason: READ_ONLY, busy: false };

const actions = sourceControlFileActions(file('src/zebra.ts'), '/repo', writable);
assert.deepEqual(
  actions.map((action) => action.id),
  [
    'view',
    'stage',
    'unstage',
    'discard',
    'copy-path',
    'copy-relative-path',
    'open-in-editor',
    'reveal-in-finder'
  ],
  'the actions come in the order the panel draws them, the three write ones next to the diff'
);

assert.deepEqual(
  actions.filter((action) => action.enabled).map((action) => action.id),
  ['view', 'stage', 'discard', 'copy-path', 'copy-relative-path', 'open-in-editor', 'reveal-in-finder'],
  'a file changed only in the working copy can be staged and discarded, but there is nothing to unstage'
);

assert.equal(
  new Set(actions.map((action) => action.id)).size,
  actions.length,
  'no action id appears twice'
);

const stagedOnly = sourceControlFileActions(
  file('README.md', { indexStatus: 'M', worktreeStatus: '', status: 'staged' }),
  '/repo',
  writable
);
assert.deepEqual(
  stagedOnly.filter((action) => action.enabled).map((action) => action.id),
  ['view', 'unstage', 'discard', 'copy-path', 'copy-relative-path', 'open-in-editor', 'reveal-in-finder'],
  'a file whose whole change is staged can be unstaged, and staging it again would do nothing'
);

const bothHalves = sourceControlFileActions(
  file('src/both.ts', { indexStatus: 'M', worktreeStatus: 'modified' }),
  '/repo',
  writable
);
assert.deepEqual(
  bothHalves.filter((action) => ['stage', 'unstage'].includes(action.id)).map((a) => a.enabled),
  [true, true],
  'a file with work on both sides of the index can go either way'
);

const readOnly = sourceControlFileActions(file('src/zebra.ts'), '/repo', {
  canWrite: false,
  readOnlyReason: READ_ONLY,
  busy: false
});
assert.deepEqual(
  readOnly.filter((action) => !action.enabled).map((action) => [action.id, action.disabledReason]),
  [
    ['stage', READ_ONLY],
    ['unstage', READ_ONLY],
    ['discard', READ_ONLY]
  ],
  'a page that cannot change the repository still reads it, and the three write actions say why they are off'
);

const midAction = sourceControlFileActions(file('src/zebra.ts'), '/repo', {
  ...writable,
  busy: true
});
assert.deepEqual(
  midAction.filter((action) => !action.enabled).map((action) => action.id),
  ['stage', 'unstage', 'discard'],
  'while a source-control action is running nothing else changes the repository'
);

const withoutRoot = sourceControlFileActions(file('src/zebra.ts'), '', writable);
assert.deepEqual(
  withoutRoot.filter((action) => action.enabled).map((action) => action.id),
  ['copy-relative-path'],
  'without a repository folder only the relative path can be answered, and the rest say why'
);
assert.ok(
  withoutRoot
    .filter((action) => !action.enabled)
    .every((action) => (action.disabledReason ?? '').length > 0),
  'every action that is off says why in plain words'
);

const deleted = sourceControlFileActions(
  file('src/gone.ts', { badge: 'D', worktreeStatus: 'deleted', status: 'deleted' }),
  '/repo',
  writable
);
assert.deepEqual(
  deleted.filter((action) => !action.enabled).map((action) => action.id),
  ['view', 'unstage', 'open-in-editor', 'reveal-in-finder'],
  'a deleted file has nothing on disk to show, open, or reveal, and nothing staged to take back'
);
assert.ok(
  deleted.find((action) => action.id === 'discard')?.enabled,
  'a deleted file can still be brought back by discarding the deletion'
);

console.log('sourceControlPanel.test.ts: ok');
