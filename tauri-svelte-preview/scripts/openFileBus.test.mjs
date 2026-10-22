import assert from 'node:assert/strict';
import {
  onOpenFile,
  requestOpenConversationFile,
  requestOpenFile,
  resolveConversationFilePath
} from '../src/lib/shell/openFileBus.ts';

assert.equal(
  resolveConversationFilePath('src/App.svelte', '/work/session'),
  '/work/session/src/App.svelte',
  'relative conversation links resolve against the session cwd'
);
assert.equal(
  resolveConversationFilePath('/outside/App.svelte', '/work/session'),
  '/outside/App.svelte',
  'absolute conversation links pass through'
);
assert.equal(
  resolveConversationFilePath('~/notes/todo.md', '/work/session'),
  '~/notes/todo.md',
  'tilde-prefixed conversation links pass through'
);

// request before any subscriber is parked and delivered to the first subscriber
{
  requestOpenFile({ path: '/a.ts', line: 3 });
  const seen = [];
  const off = onOpenFile((request) => seen.push(request));
  assert.deepEqual(seen, [{ path: '/a.ts', line: 3 }], 'parked request delivered on subscribe');
  off();
}

// live delivery to all subscribers; unsubscribe stops delivery
{
  const a = [];
  const b = [];
  const offA = onOpenFile((request) => a.push(request.path));
  const offB = onOpenFile((request) => b.push(request.path));
  requestOpenFile({ path: '/b.ts' });
  assert.deepEqual(a, ['/b.ts']);
  assert.deepEqual(b, ['/b.ts']);
  offA();
  requestOpenFile({ path: '/c.ts' });
  assert.deepEqual(a, ['/b.ts'], 'unsubscribed listener not called');
  assert.deepEqual(b, ['/b.ts', '/c.ts']);
  offB();
}

// with a live subscriber nothing is parked: a later subscriber gets nothing old
{
  const first = [];
  const off = onOpenFile((request) => first.push(request.path));
  requestOpenFile({ path: '/d.ts' });
  off();
  const late = [];
  const offLate = onOpenFile((request) => late.push(request.path));
  assert.deepEqual(late, [], 'delivered requests are not replayed');
  offLate();
}

// A tool-call link outside the session's project opens the full path the call
// recorded, read-only, on the session's own machine.
{
  const seen = [];
  const off = onOpenFile((request) => seen.push(request));
  const outside = '/home/me/other-repo/src/remote.rs';
  const local = { cwd: '/work/session', executionEnvironment: 'local' };
  assert.equal(requestOpenConversationFile(local, 'remote.rs', { path: outside }), null);
  const remote = { cwd: '/work/session', executionEnvironment: 'remote', remoteProfileId: 'box' };
  assert.equal(requestOpenConversationFile(remote, outside, { root: '/work/session' }), null);
  assert.deepEqual(seen, [
    { path: outside, projectRoot: '/work/session', readOnly: true, line: undefined },
    {
      path: `assembly-remote://box${outside}`,
      projectRoot: 'assembly-remote://box/work/session',
      readOnly: true,
      line: undefined
    }
  ]);
  off();
}

console.log('openFileBus: all tests passed');
