/**
 * The branch picker in the new-session pane must survive a project that has no
 * git repository behind it.
 *
 * `list_project_git_refs` answers with nothing in that case. Passing the
 * non-list on to the picker threw while the picker was opening, which left the
 * picker marked open with no content on screen and no way to dismiss it — the
 * whole window stopped taking clicks.
 */
import assert from 'node:assert/strict';

interface TauriInternals {
  transformCallback(callback: unknown, once?: boolean): number;
  unregisterCallback(id: number): void;
  convertFileSrc(path: string): string;
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
}

let answer: unknown = null;
let failure: string | null = null;
const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

// The backend module counts its calls through a runes store. Outside a Svelte
// build `$state` is just the value it was handed.
(globalThis as unknown as { $state: <T>(value: T) => T }).$state = (value) => value;

(globalThis as unknown as { window: { __TAURI_INTERNALS__: TauriInternals } }).window = {
  __TAURI_INTERNALS__: {
    transformCallback: () => 1,
    unregisterCallback: () => {},
    convertFileSrc: (path: string) => path,
    invoke: async (command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (failure) throw failure;
      return answer;
    }
  }
};

const { listGitRefs, switchBranch } = await import('../src/lib/shell/newSession/newSessionBackend.ts');
const { filterThreadStartGitRefs, refNote } = await import('../src/lib/shell/newSession/threadStartFlow.ts');

// A folder with no repository: nothing comes back, and the picker sees an
// empty list rather than something it cannot read.
answer = null;
const empty = await listGitRefs('local', '/tmp/not-a-repository');
assert.equal(empty.status, 'ok');
assert.deepEqual(empty.status === 'ok' ? empty.value : null, []);
assert.deepEqual(
  filterThreadStartGitRefs(empty.status === 'ok' ? empty.value : [], ''),
  { visible: [], total: 0 }
);

// Anything else that is not a list is read the same way.
answer = { message: 'no repository here' };
const odd = await listGitRefs('local', '/tmp/not-a-repository');
assert.equal(odd.status, 'ok');
assert.deepEqual(odd.status === 'ok' ? odd.value : null, []);

// A real list still comes through untouched.
answer = [{ name: 'main', isDefault: true, isCurrent: true, checkoutPath: '/tmp/repo', lastCommitMs: 1 }];
const listed = await listGitRefs('local', '/tmp/repo');
assert.equal(listed.status, 'ok');
assert.equal(listed.status === 'ok' ? listed.value.length : 0, 1);
assert.equal(filterThreadStartGitRefs(listed.status === 'ok' ? listed.value : [], 'ma').total, 1);

// An empty folder path is still a question the pane answers itself.
const noRoot = await listGitRefs('local', '   ');
assert.equal(noRoot.status, 'failed');

// The project's machine answers: this Mac directly, a remote one through its server.
calls.length = 0;
answer = [];
await listGitRefs('workbox', '/srv/repo');
await switchBranch('local', '/tmp/repo', 'feature');
await switchBranch('workbox', '/srv/repo', 'feature');
assert.deepEqual(calls.map((call) => [call.command, call.args]), [
  ['remote_workspace', { profileId: 'workbox', operation: 'list_project_git_refs', args: { root: '/srv/repo' } }],
  ['switch_git_branch', { root: '/tmp/repo', name: 'feature' }],
  ['remote_workspace', { profileId: 'workbox', operation: 'switch_git_branch', args: { root: '/srv/repo', name: 'feature' } }]
]);

// A failed switch hands back git's own words.
failure = 'error: Your local changes would be overwritten by checkout';
const refused = await switchBranch('local', '/tmp/repo', 'feature');
assert.deepEqual(refused, { status: 'failed', message: 'error: Your local changes would be overwritten by checkout' });
failure = null;

// The note beside a branch: "current" for the root's own branch, "in another
// worktree" only when it is checked out somewhere other than the root.
const root = '/tmp/repo';
assert.equal(refNote({ name: 'main', isCurrent: true, checkoutPath: root }, root), 'current');
assert.equal(refNote({ name: 'side', isCurrent: false, checkoutPath: '/tmp/worktrees/side' }, root), 'in another worktree');
assert.equal(refNote({ name: 'main', isCurrent: false, checkoutPath: root }, root), '');
assert.equal(refNote({ name: 'idle', isCurrent: false, checkoutPath: null }, root), '');

console.log('new session git refs tests passed');
