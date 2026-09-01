/**
 * gitWorkingTreeActions.test.mjs — the working-tree actions the source-control
 * panel grew this round: discard, branch create/switch, stash/pop and amend.
 *
 * NOTHING HERE RUNS GIT. The service is driven with a stub backend that records
 * the calls it was asked to make, which is the whole point for the destructive
 * ones: what has to be provable is that a discard reaches the backend exactly
 * once, with exactly the paths that were asked for, and never as a side effect
 * of anything else.
 *
 * `gitPanelStore.svelte.ts` calls `$state(...)` at module scope, which the
 * Svelte compiler normally rewrites; Node runs it as plain JavaScript, so the
 * one-line stand-in below gives `$state(v)` its plain-value meaning first.
 */
globalThis.$state = (value) => value;

const assert = (await import('node:assert/strict')).default;

const { createGitPanelState } = await import('../src/lib/shell/git/gitPanelStore.svelte.ts');
const { createGitService } = await import('../src/lib/shell/git/gitService.ts');

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function status(files = []) {
  return { branch: 'main', ahead: 0, behind: 0, hasUpstream: true, files };
}

function changedFile(relativePath) {
  return {
    relativePath,
    indexStatus: '',
    worktreeStatus: 'modified',
    status: 'modified',
    badge: 'M'
  };
}

function makeBackend() {
  const calls = [];
  const record =
    (name, message) =>
    async (...args) => {
      calls.push([name, ...args]);
      return { message, status: status() };
    };
  return {
    calls,
    async readStatus() {
      return status([changedFile('src/a.ts')]);
    },
    async readDiff() {
      return null;
    },
    async readHistory() {
      return [];
    },
    listBranches: async (root) => {
      calls.push(['listBranches', root]);
      return {
        current: 'main',
        branches: [{ name: 'main', isCurrent: true, upstream: 'origin/main', subject: 'work' }]
      };
    },
    listStashes: async (root) => {
      calls.push(['listStashes', root]);
      return [{ index: 0, label: 'stash@{0}', description: 'WIP on main' }];
    },
    stage: record('stage', 'Staged'),
    unstage: record('unstage', 'Unstaged'),
    commit: record('commit', 'Committed'),
    amend: record('amend', 'Amended the last commit'),
    discard: record('discard', 'Discarded changes in 1 file'),
    discardAll: record('discardAll', 'Discarded every change'),
    createBranch: record('createBranch', 'Created'),
    switchBranch: record('switchBranch', 'Switched'),
    stash: record('stash', 'Stashed the working copy'),
    popStash: record('popStash', 'Restored the stashed changes'),
    fetch: record('fetch', 'Fetched'),
    pull: record('pull', 'Pulled'),
    push: record('push', 'Pushed')
  };
}

function makeService() {
  const backend = makeBackend();
  const state = createGitPanelState();
  const git = createGitService({ backend, state });
  git.activate('/repo');
  return { backend, state, git };
}

// ── discarding files ────────────────────────────────────────────────────────
{
  const { backend, state, git } = makeService();
  await settle();

  await git.discardPaths(['src/a.ts', '  src/b.ts  ', '']);
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'discard'),
    [['discard', '/repo', ['src/a.ts', 'src/b.ts']]],
    'a discard runs once, with the trimmed paths it was given and nothing else'
  );
  assert.equal(state.actionStatus, 'Discarded changes in 1 file');
  assert.equal(state.actionBusy, '');

  backend.calls.length = 0;
  await git.discardPaths([]);
  await git.discardPaths(['   ']);
  assert.equal(
    backend.calls.length,
    0,
    'an empty list never reaches the backend — an empty discard is not a discard of everything'
  );
}

// ── discarding everything ───────────────────────────────────────────────────
{
  const { backend, git } = makeService();
  await settle();

  await git.discardAll(false);
  await git.discardAll(true);
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'discardAll'),
    [
      ['discardAll', '/repo', false],
      ['discardAll', '/repo', true]
    ],
    'whether untracked files are deleted is carried through exactly as asked'
  );
}

// ── one action at a time ────────────────────────────────────────────────────
{
  const { backend, state, git } = makeService();
  await settle();

  state.actionBusy = 'commit';
  await git.discardPaths(['src/a.ts']);
  assert.equal(
    backend.calls.filter(([name]) => name === 'discard').length,
    0,
    'a discard cannot start while another action is running'
  );
  state.actionBusy = '';
}

// ── branches ────────────────────────────────────────────────────────────────
{
  const { backend, state, git } = makeService();
  await settle();

  const list = await git.listBranches();
  assert.equal(list.branches.length, 1);
  assert.equal(list.current, 'main');

  await git.createBranch('  tsk-809-next  ', true);
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'createBranch'),
    [['createBranch', '/repo', 'tsk-809-next', true]]
  );

  backend.calls.length = 0;
  await git.createBranch('   ', true);
  assert.equal(backend.calls.length, 0, 'an empty name never reaches git');
  assert.equal(state.actionError, 'Type a branch name first.');

  await git.switchBranch('main');
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'switchBranch'),
    [['switchBranch', '/repo', 'main']]
  );
}

// ── stash and pop ───────────────────────────────────────────────────────────
{
  const { backend, git } = makeService();
  await settle();

  await git.stashChanges(true, '  before the rebase  ');
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'stash'),
    [['stash', '/repo', true, 'before the rebase']]
  );

  await git.popStash(null);
  await git.popStash(2);
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'popStash'),
    [
      ['popStash', '/repo', null],
      ['popStash', '/repo', 2]
    ]
  );

  const stashes = await git.listStashes();
  assert.equal(stashes.length, 1);
}

// ── amend ───────────────────────────────────────────────────────────────────
{
  const { backend, state, git } = makeService();
  await settle();

  state.commitMessage = '  fix the parser  ';
  await git.amendCommit();
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'amend'),
    [['amend', '/repo', 'fix the parser']],
    'the amend takes the box’s message, trimmed'
  );
  assert.equal(state.commitMessage, '', 'a finished amend empties the box, as a commit does');

  backend.calls.length = 0;
  state.commitMessage = '';
  await git.amendCommit();
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'amend'),
    [['amend', '/repo', '']],
    'an empty box is a valid amend: it keeps the message the commit already has'
  );
}

// ── pull and push, and the read-only scope the panel can point at ───────────
{
  const { backend, state, git } = makeService();
  await settle();

  backend.calls.length = 0;
  await git.runRemoteAction('pull');
  await git.runRemoteAction('push');
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'pull' || name === 'push'),
    [
      ['pull', '/repo'],
      ['push', '/repo']
    ],
    'pull and push each reach the backend once, for the folder the panel is on'
  );
  assert.equal(state.actionStatus, 'Pushed', 'the panel repeats what git said');
  assert.equal(state.actionBusy, '', 'nothing is left marked busy afterwards');
}

{
  // A failing push keeps git's own words: the panel shows this text unchanged.
  const backend = makeBackend();
  backend.push = async () => {
    throw new Error('fatal: could not read Username for https://github.com');
  };
  const state = createGitPanelState();
  const git = createGitService({ backend, state });
  git.activate('/repo');
  await settle();

  await git.runRemoteAction('push');
  assert.equal(
    state.actionError,
    'fatal: could not read Username for https://github.com',
    'the failure text is git’s, word for word'
  );
  assert.equal(state.actionBusy, '');
}

{
  // The scope picker re-points the panel by activating another checkout; every
  // read after that names that folder, and nothing about the old one lingers.
  const { backend, state, git } = makeService();
  await settle();

  backend.calls.length = 0;
  git.activate('/work/tsk-1');
  await settle();
  assert.equal(state.root, '/work/tsk-1', 'the panel now reads the chosen checkout');
  assert.equal(state.selectedPath, '', 'the old folder’s selected file does not follow');

  await git.stagePaths(['src/a.ts']);
  assert.deepEqual(
    backend.calls.filter(([name]) => name === 'stage'),
    [['stage', '/work/tsk-1', ['src/a.ts']]],
    'root-parameterised commands go to the chosen checkout, not the session folder'
  );
}

console.log('gitWorkingTreeActions tests passed');
