import assert from 'node:assert/strict';
import test from 'node:test';
import { mapWorkspaceSnapshotPaths, parseRemoteWorkspacePath, qualifyWorkspaceResult, remoteWorkspacePath, remoteWorkspaceRequest, sessionWorkspaceRoot } from '../src/lib/workspacePaths.ts';

test('a workspace keeps its machine even when another session is selected', () => {
  const first = remoteWorkspacePath('workbox', '/home/me/project/a.ts');
  const second = remoteWorkspacePath('other', '/home/me/project/a.ts');
  assert.notEqual(first, second);
  assert.deepEqual(remoteWorkspaceRequest({ path: first, content: second }), { profileId: 'workbox', args: { path: '/home/me/project/a.ts', content: second } });
  assert.deepEqual(parseRemoteWorkspacePath(first), { profileId: 'workbox', path: '/home/me/project/a.ts' });
});
test('cross-machine and mixed local/remote file operations fail before invoking', () => {
  const source = remoteWorkspacePath('workbox', '/a');
  assert.throws(() => remoteWorkspaceRequest({ source, target: remoteWorkspacePath('other', '/b') }), /cross machines/);
  assert.throws(() => remoteWorkspaceRequest({ source, target: '/b' }), /mix Mac and remote/);
  assert.equal(remoteWorkspaceRequest({ path: '/Users/me/a' }), null);
});
test('qualifies returned file and checkout paths without rewriting content', () => {
  assert.deepEqual(qualifyWorkspaceResult({ '/repo': [{ path: '/repo/a', content: '/literal', relativePath: 'a' }] }, 'box'), {
    'assembly-remote://box/repo': [{ path: 'assembly-remote://box/repo/a', content: '/literal', relativePath: 'a' }]
  });
});
test('remote sessions without a machine never fall back to local filesystem', () => {
  assert.equal(sessionWorkspaceRoot({ cwd: '/repo', executionEnvironment: 'remote' }), '');
  assert.equal(sessionWorkspaceRoot({ cwd: '/repo', executionEnvironment: 'remote', remoteProfileId: 'box' }), 'assembly-remote://box/repo');
  assert.equal(sessionWorkspaceRoot({ cwd: '/repo', executionEnvironment: 'local' }), '/repo');
});

test('saved workspace paths round trip without retaining a client profile or rewriting drafts', () => {
  const stored = { openPaths: ['/home/me/a.txt'], activePath: '/home/me/a.txt', filesInspectionRoot: '/home/me', expandedPathsByRoot: { '/home/me': ['/home/me/sub'] }, fileStates: { '/home/me/a.txt': { content: '/home/me/leave-content-alone' } } };
  const projected = mapWorkspaceSnapshotPaths(stored, 'workbox');
  assert.equal((projected as typeof stored).activePath, 'assembly-remote://workbox/home/me/a.txt');
  assert.deepEqual(mapWorkspaceSnapshotPaths(projected, null), stored);
});
