import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  buildState,
  currentRunLink,
  parseAvailableGb,
  parseListeningPorts,
  pickFreeDisplay,
  pickFreePort,
  descendantGroups,
  ownerFilesIn,
  ownsGroup,
  refuseOwnerData,
  refuseWhileRunning
} from './fourScenarios.ts';

test('picks the first display from :140 that is not taken', () => {
  assert.equal(pickFreeDisplay(() => false), 140);
  assert.equal(pickFreeDisplay((n) => n === 140 || n === 141), 142);
  assert.throws(() => pickFreeDisplay(() => true), /No free X display/);
});

test('picks the first port from 5300 that is not listening', () => {
  assert.equal(pickFreePort(new Set()), 5300);
  assert.equal(pickFreePort(new Set([5300, 5301, 5303])), 5302);
});

test('reads listening ports from ss output', () => {
  const ss = 'LISTEN 0 511 127.0.0.1:5300 0.0.0.0:*\nLISTEN 0 4096 [::]:22 [::]:*\n';
  assert.deepEqual([...parseListeningPorts(ss)].sort((a, b) => a - b), [22, 5300]);
});

test('reads the available column from free -g', () => {
  const free = '               total        used        free      shared  buff/cache   available\nMem:              31          17           2           8          19          13\nSwap:              7           7           0\n';
  assert.equal(parseAvailableGb(free), 13);
});

test('refuses the owner data folder and anything under it', () => {
  assert.throws(() => refuseOwnerData('/home/me/.local/share', '/home/me'), /owner/);
  assert.throws(() => refuseOwnerData('/home/me/.local/share/assembly', '/home/me'), /owner/);
  assert.throws(() => refuseOwnerData('/home/me/.local/share/../share', '/home/me'), /owner/);
  assert.doesNotThrow(() => refuseOwnerData('/home/me/assembly-proof/run/data', '/home/me'));
});

test('refuses a data folder that reaches the owner data through a symlink', () => {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'four-scenarios-home-'));
  try {
    fs.mkdirSync(path.join(home, '.local/share/assembly'), { recursive: true });
    fs.symlinkSync(path.join(home, '.local/share'), path.join(home, 'alias'));
    assert.throws(() => refuseOwnerData(path.join(home, 'alias'), home), /owner/);
    assert.throws(() => refuseOwnerData(path.join(home, 'alias/assembly/new'), home), /owner/);
    assert.doesNotThrow(() => refuseOwnerData(path.join(home, 'proof/data'), home));
  } finally {
    fs.rmSync(home, { recursive: true, force: true });
  }
});

test('builds the state file from what start created', () => {
  const state = buildState({
    runDir: '/p/run', display: 140, port: 5300, dataDir: '/p/run/data', worktree: '/w',
    processes: [{ name: 'xvfb', pid: 10, startedAt: 'Mon Oct 19 10:00:00 2026', log: '/p/run/xvfb.log' }]
  });
  assert.equal(state.display, ':140');
  assert.equal(state.port, 5300);
  assert.equal(state.dataDir, '/p/run/data');
  assert.deepEqual(state.processes[0], { name: 'xvfb', pid: 10, pgid: 10, startedAt: 'Mon Oct 19 10:00:00 2026', log: '/p/run/xvfb.log' });
});

test('refuses to start while the previous run still has a live process', () => {
  const state = { runDir: '/proof/old', processes: [{ name: 'xvfb', pid: 10, pgid: 10, startedAt: 'a', log: '' }] };
  assert.throws(() => refuseWhileRunning(state, () => true), /still running in \/proof\/old/);
  assert.doesNotThrow(() => refuseWhileRunning(state, () => false));
  assert.doesNotThrow(() => refuseWhileRunning(undefined, () => true));
});

test('lists only open files under the owner data folders', () => {
  const owner = ['/home/u/.local/share/assembly'];
  const open = ['/home/u/.local/share/assembly/sessions.db', '/home/u/assembly-proof/x/data/assembly/sessions.db', '/home/u/.local/share/assembly-server/x', 'socket:[123]'];
  assert.deepEqual(ownerFilesIn(open, owner), ['/home/u/.local/share/assembly/sessions.db']);
});

test('a live group still belongs to the run after its launcher exits', () => {
  const p = { pid: 10, pgid: 10, startedAt: 'Mon 1' };
  assert.equal(ownsGroup(p, true, 'Mon 1'), true);
  assert.equal(ownsGroup(p, true, ''), true);
  assert.equal(ownsGroup(p, true, 'Tue 2'), false);
  assert.equal(ownsGroup(p, false, 'Mon 1'), false);
});

test('stop reaches descendants that started their own process group', () => {
  const rows = [
    { pid: 10, ppid: 1, pgid: 10 }, // recorded launcher
    { pid: 11, ppid: 10, pgid: 10 }, // the app, same group
    { pid: 12, ppid: 11, pgid: 12 }, // agent sidecar in its own group
    { pid: 13, ppid: 12, pgid: 12 },
    { pid: 14, ppid: 13, pgid: 14 }, // language server, own group, two levels down
    { pid: 20, ppid: 1, pgid: 20 } // unrelated
  ];
  assert.deepEqual(descendantGroups(rows, [10]).sort((a, b) => a - b), [10, 12, 14]);
});

test('two worktrees keep independent run state when one is stopped', () => {
  const linkA = currentRunLink('/proof', '/work/repo-a/review');
  const linkB = currentRunLink('/proof', '/work/repo-b/review');
  assert.notEqual(linkA, linkB, 'equal folder names in different worktrees must stay separate');
  const state = (worktree: string, pid: number) => buildState({
    runDir: `/proof/run-${pid}`, display: 140 + pid, port: 5300 + pid,
    dataDir: `/proof/run-${pid}/data`, worktree,
    processes: [{ name: 'app', pid, startedAt: 'now', log: '' }]
  });
  const runs = new Map([[linkA, state('/work/repo-a/review', 10)], [linkB, state('/work/repo-b/review', 20)]]);
  const live = new Set([10, 20]);
  const isLive = (p: { pid: number }) => live.has(p.pid);
  assert.throws(() => refuseWhileRunning(runs.get(linkA), isLive), /still running/);
  assert.throws(() => refuseWhileRunning(runs.get(linkB), isLive), /still running/);
  for (const p of runs.get(linkA)!.processes) live.delete(p.pid);
  assert.doesNotThrow(() => refuseWhileRunning(runs.get(linkA), isLive));
  assert.throws(() => refuseWhileRunning(runs.get(linkB), isLive), /still running/);
  assert.deepEqual([...live], [20]);
});
