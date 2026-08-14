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

// The backend module counts its calls through a runes store. Outside a Svelte
// build `$state` is just the value it was handed.
(globalThis as unknown as { $state: <T>(value: T) => T }).$state = (value) => value;

(globalThis as unknown as { window: { __TAURI_INTERNALS__: TauriInternals } }).window = {
  __TAURI_INTERNALS__: {
    transformCallback: () => 1,
    unregisterCallback: () => {},
    convertFileSrc: (path: string) => path,
    invoke: async () => answer
  }
};

const { listGitRefs } = await import('../src/lib/shell/newSession/newSessionBackend.ts');
const { filterThreadStartGitRefs } = await import('../src/lib/shell/newSession/threadStartFlow.ts');

// A folder with no repository: nothing comes back, and the picker sees an
// empty list rather than something it cannot read.
answer = null;
const empty = await listGitRefs('/tmp/not-a-repository');
assert.equal(empty.status, 'ok');
assert.deepEqual(empty.status === 'ok' ? empty.value : null, []);
assert.deepEqual(
  filterThreadStartGitRefs(empty.status === 'ok' ? empty.value : [], ''),
  { visible: [], total: 0 }
);

// Anything else that is not a list is read the same way.
answer = { message: 'no repository here' };
const odd = await listGitRefs('/tmp/not-a-repository');
assert.equal(odd.status, 'ok');
assert.deepEqual(odd.status === 'ok' ? odd.value : null, []);

// A real list still comes through untouched.
answer = [{ name: 'main', isDefault: true, isCurrent: true, checkoutPath: '/tmp/repo', lastCommitMs: 1 }];
const listed = await listGitRefs('/tmp/repo');
assert.equal(listed.status, 'ok');
assert.equal(listed.status === 'ok' ? listed.value.length : 0, 1);
assert.equal(filterThreadStartGitRefs(listed.status === 'ok' ? listed.value : [], 'ma').total, 1);

// An empty folder path is still a question the pane answers itself.
const noRoot = await listGitRefs('   ');
assert.equal(noRoot.status, 'failed');

console.log('new session git refs tests passed');
