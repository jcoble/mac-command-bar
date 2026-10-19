import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildState,
  parseAvailableGb,
  parseListeningPorts,
  pickFreeDisplay,
  pickFreePort,
  refuseOwnerData
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
