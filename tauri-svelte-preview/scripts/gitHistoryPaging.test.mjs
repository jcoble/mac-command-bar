/**
 * gitHistoryPaging.test.mjs — reading more of the commit history, and saying
 * honestly how much of it is on screen.
 *
 * THE BUG THIS EXISTS TO PREVENT. The panel asks the app for a number of
 * commits and the app answers with fewer. Two completely different things look
 * identical from here: the repository has no more commits, or this build of the
 * app will not read that far back. Today's desktop build stops at 80 and the
 * dev server's read-only bridge stops at 200, so on any real repository the
 * short answer is the app's limit, not the end of the history. A list that says
 * "all 80 commits" about a repository with four thousand of them is a lie the
 * user has no way to catch — so the wording below never claims to know which of
 * the two happened, and these tests hold it to that.
 *
 * `gitPanelStore.svelte.ts` calls `$state(...)` at module scope, which the
 * Svelte compiler normally rewrites; Node runs it as plain JavaScript, so the
 * one-line stand-in below gives `$state(v)` its plain-value meaning first.
 */
globalThis.$state = (value) => value;

const assert = (await import('node:assert/strict')).default;

const store = await import('../src/lib/shell/git/gitPanelStore.svelte.ts');
const service = await import('../src/lib/shell/git/gitService.ts');

const {
  canLoadMoreGitHistory,
  createGitPanelState,
  describeGitBranchTitle,
  describeGitHistoryCount,
  describeGitHistoryFooter,
  isGitHistoryComplete
} = store;

const {
  COMMIT_HISTORY_CEILING,
  COMMIT_HISTORY_LIMIT,
  COMMIT_HISTORY_PAGE,
  createGitService,
  nextCommitHistoryLimit
} = service;

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** One commit record, shaped like the backend's `GitCommitHistoryEntry`. */
function commit(index) {
  const sha = String(index).padStart(40, '0');
  return {
    shortSha: sha.slice(0, 7),
    sha,
    subject: `commit ${index}`,
    author: 'Someone',
    committedAt: '2026-07-30T09:00:00+01:00',
    refNames: '',
    parentShas: index > 1 ? [String(index - 1).padStart(40, '0')] : []
  };
}

const emptyStatus = {
  branch: 'main',
  ahead: 0,
  behind: 0,
  hasUpstream: true,
  files: []
};

/**
 * A backend holding `total` commits, which never answers with more than `cap` of
 * them however many are asked for. `cap` is how a real app build behaves.
 */
function makeBackend({ total = 1000, cap = Infinity } = {}) {
  const calls = [];
  const all = Array.from({ length: total }, (_, index) => commit(total - index));
  const refuse = async () => {
    throw new Error('this test never writes');
  };
  return {
    calls,
    async readStatus(root) {
      calls.push(['readStatus', root]);
      return emptyStatus;
    },
    async readDiff() {
      return null;
    },
    async readHistory(root, limit) {
      calls.push(['readHistory', root, limit]);
      return all.slice(0, Math.min(limit, cap));
    },
    stage: refuse,
    unstage: refuse,
    commit: refuse,
    fetch: refuse,
    pull: refuse,
    push: refuse
  };
}

// ── the one rule the whole feature rests on ─────────────────────────────────
{
  assert.equal(isGitHistoryComplete(24, 24), false, 'a full answer may have more behind it');
  assert.equal(isGitHistoryComplete(124, 80), true, 'a short answer has nothing more to give');
  assert.equal(isGitHistoryComplete(24, 0), true, 'an empty repository is complete');
}

// ── each click asks for one page more, and stops at the ceiling ─────────────
{
  assert.equal(nextCommitHistoryLimit(0), COMMIT_HISTORY_LIMIT + COMMIT_HISTORY_PAGE);
  assert.equal(
    nextCommitHistoryLimit(COMMIT_HISTORY_LIMIT),
    COMMIT_HISTORY_LIMIT + COMMIT_HISTORY_PAGE
  );
  assert.equal(nextCommitHistoryLimit(124), 224);
  assert.equal(nextCommitHistoryLimit(COMMIT_HISTORY_CEILING), COMMIT_HISTORY_CEILING);
  assert.equal(
    nextCommitHistoryLimit(COMMIT_HISTORY_CEILING - 1),
    COMMIT_HISTORY_CEILING,
    'the last step lands exactly on the ceiling rather than past it'
  );
}

// ── a repository with plenty of history: load more grows the list ───────────
{
  const backend = makeBackend({ total: 1000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();
  assert.equal(state.history.length, COMMIT_HISTORY_LIMIT);
  assert.equal(state.historyRequested, COMMIT_HISTORY_LIMIT);
  assert.equal(state.historyComplete, false);
  assert.equal(describeGitHistoryCount(state), '24 so far');
  assert.equal(describeGitHistoryFooter(state), 'Showing 24 so far.');
  assert.equal(canLoadMoreGitHistory(state), true);

  backend.calls.length = 0;
  await git.loadMoreHistory();
  await settle();
  assert.deepEqual(
    backend.calls,
    [['readHistory', '/repo', 124]],
    'one read, for the whole list at the bigger size'
  );
  assert.equal(state.history.length, 124, 'the list is replaced, not appended to');
  assert.equal(state.historyPaged, true);
  assert.equal(state.historyComplete, false);
  assert.equal(describeGitHistoryFooter(state), 'Showing 124 so far.');

  // The newest commit is still first: a page read from the top keeps the order
  // the graph's columns were worked out from.
  assert.equal(state.history[0].subject, 'commit 1000');
}

// ── an app build that reads fewer commits than we ask for ───────────────────
{
  const backend = makeBackend({ total: 4000, cap: 80 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();
  await git.loadMoreHistory();
  await settle();

  assert.equal(state.history.length, 80, 'the app answered with its own limit');
  assert.equal(state.historyComplete, true, 'asking again the same way would give the same 80');
  assert.equal(canLoadMoreGitHistory(state), false, 'so the button goes away');
  assert.equal(describeGitHistoryCount(state), '80');

  const footer = describeGitHistoryFooter(state);
  assert.equal(
    footer,
    'Showing 80 commits. We asked for 124 and this is all that came back, so it is ' +
      'everything this app will show for this repository.'
  );
  assert.equal(/all 80 commits/.test(footer), false, 'it never calls 80 of 4000 "all of them"');
}

// ── a small repository: the first read already has everything ───────────────
{
  const backend = makeBackend({ total: 7 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();

  assert.equal(state.history.length, 7);
  assert.equal(state.historyComplete, true);
  assert.equal(canLoadMoreGitHistory(state), false, 'nothing to load, so nothing is offered');
  assert.equal(describeGitHistoryCount(state), '7');
  assert.equal(
    describeGitHistoryFooter(state),
    'Showing all 7 commits.',
    'asking for 24 and getting 7 really is the whole history'
  );
}

// ── refreshing keeps the size the list has grown to ─────────────────────────
{
  const backend = makeBackend({ total: 1000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();
  await git.loadMoreHistory();
  await settle();

  backend.calls.length = 0;
  await git.refreshHistory();
  await settle();
  assert.deepEqual(
    backend.calls,
    [['readHistory', '/repo', 124]],
    'a refresh after loading more does not drop back to the first 24'
  );
  assert.equal(state.history.length, 124);
}

// ── pointing the panel at another repository starts over ────────────────────
{
  const backend = makeBackend({ total: 1000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();
  await git.loadMoreHistory();
  await settle();

  git.activate('/other');
  await settle();
  assert.equal(state.historyRequested, COMMIT_HISTORY_LIMIT);
  assert.equal(state.historyPaged, false);
  assert.equal(state.history.length, COMMIT_HISTORY_LIMIT);
}

// ── the ceiling: this panel stops asking somewhere ──────────────────────────
{
  const backend = makeBackend({ total: 10000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();
  for (let click = 0; click < 20; click += 1) {
    await git.loadMoreHistory();
    await settle();
  }

  assert.equal(state.historyRequested, COMMIT_HISTORY_CEILING);
  assert.equal(state.history.length, COMMIT_HISTORY_CEILING);
  assert.equal(state.historyCeiling, true);
  assert.equal(canLoadMoreGitHistory(state), false);
  assert.equal(
    describeGitHistoryFooter(state),
    'Showing 500 commits — the most this app reads at one time. Older commits are not in this list.'
  );

  const readsAtCeiling = backend.calls.filter(
    (call) => call[0] === 'readHistory' && call[2] === COMMIT_HISTORY_CEILING
  );
  assert.equal(readsAtCeiling.length, 1, 'the ceiling is asked for once, not on every click');
}

// ── a failed "load more" keeps what is already on screen ────────────────────
{
  const backend = makeBackend({ total: 1000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();

  backend.readHistory = async () => {
    throw new Error('fatal: bad object HEAD');
  };
  await git.loadMoreHistory();
  await settle();

  assert.equal(state.history.length, COMMIT_HISTORY_LIMIT, 'the 24 already read stay on screen');
  assert.equal(state.historyError, 'fatal: bad object HEAD');
  assert.equal(state.historyLoadingMore, false);
}

// ── a failed refresh has nothing left to show, and says so ──────────────────
{
  const backend = makeBackend({ total: 1000 });
  const state = createGitPanelState();
  const git = createGitService({ backend, state });

  git.activate('/repo');
  await settle();

  backend.readHistory = async () => {
    throw new Error('fatal: bad object HEAD');
  };
  await git.refreshHistory();
  await settle();

  assert.equal(state.history.length, 0);
  assert.equal(state.historyError, 'fatal: bad object HEAD');
  assert.equal(describeGitHistoryFooter(state), '', 'no list, no count line');
}

// ── the branch hover text spells everything out ─────────────────────────────
{
  assert.equal(describeGitBranchTitle(null), 'No repository loaded');

  assert.equal(
    describeGitBranchTitle({
      branch: 'codex/outbound-rule-generation',
      ahead: 0,
      behind: 0,
      hasUpstream: true,
      files: []
    }),
    'Branch: codex/outbound-rule-generation\nUp to date with the remote.'
  );

  assert.equal(
    describeGitBranchTitle({
      branch: 'tsk-798-polish-wave',
      ahead: 1,
      behind: 3,
      hasUpstream: true,
      files: []
    }),
    'Branch: tsk-798-polish-wave\n' +
      '1 commit here that the remote does not have.\n' +
      '3 commits on the remote that you do not have.'
  );

  assert.equal(
    describeGitBranchTitle({
      branch: 'new-idea',
      ahead: 0,
      behind: 0,
      hasUpstream: false,
      files: []
    }),
    'Branch: new-idea\nThis branch has no matching branch on the remote yet.'
  );
}

console.log('gitHistoryPaging: all tests passed');
