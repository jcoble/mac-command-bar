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

import type { ProjectGitFileStatus, ProjectGitStatus } from '../src/lib/tauriSource.ts';

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

// ── per-file menu ───────────────────────────────────────────────────────────

const actions = sourceControlFileActions(file('src/zebra.ts'), '/repo');
assert.deepEqual(
  actions.map((action) => action.id),
  ['view', 'copy-path', 'copy-relative-path', 'open-in-editor', 'reveal-in-finder'],
  'the menu offers exactly the five read-side actions, in the order the panel draws them'
);

assert.deepEqual(
  actions.map((action) => action.enabled),
  [true, true, true, true, true],
  'against a normal file in a known repository every action is available'
);

assert.deepEqual(
  actions.filter((action) => action.disabledReason !== null),
  [],
  'an available action carries no reason for being off'
);

assert.equal(
  new Set(actions.map((action) => action.id)).size,
  actions.length,
  'no action id appears twice'
);

const withoutRoot = sourceControlFileActions(file('src/zebra.ts'), '');
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
  '/repo'
);
assert.deepEqual(
  deleted.filter((action) => !action.enabled).map((action) => action.id),
  ['view', 'open-in-editor', 'reveal-in-finder'],
  'a deleted file has nothing on disk to show, open, or reveal'
);

console.log('sourceControlPanel.test.ts: ok');
