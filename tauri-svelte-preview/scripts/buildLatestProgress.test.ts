import assert from 'node:assert/strict';
import test from 'node:test';
import { idleBuildLatest, nextBuildLatest } from '../src/lib/shell/buildLatestProgress.ts';

test('a step line sets the step, the total and the text', () => {
  const state = nextBuildLatest(idleBuildLatest, { line: '[2/4] Compiling serde v1.0.228' });
  assert.deepEqual(state, { phase: 'running', step: 2, total: 4, text: 'Compiling serde v1.0.228', error: '' });
});

test('a line without a step keeps the shown step and text', () => {
  const running = nextBuildLatest(idleBuildLatest, { line: '[1/4] Pulling main' });
  assert.deepEqual(nextBuildLatest(running, { line: 'zsh: login banner' }), running);
});

test('after a page reload the next line brings the progress back', () => {
  const state = nextBuildLatest(idleBuildLatest, { line: '[3/4] box-a: Build backend…' });
  assert.equal(state.phase, 'running');
  assert.equal(state.step, 3);
});

test('a finished run waits for the restart', () => {
  const running = nextBuildLatest(idleBuildLatest, { line: '[4/4] Restarting Assembly on abc1234' });
  const done = nextBuildLatest(running, { done: true, error: null });
  assert.equal(done.phase, 'restarting');
  assert.equal(done.step, 4);
  assert.equal(done.text, 'Restarting Assembly on abc1234');
});

test('a failed run keeps the step it failed on and shows the error', () => {
  const running = nextBuildLatest(idleBuildLatest, { line: '[2/4] Building Assembly' });
  const failed = nextBuildLatest(running, { done: true, error: '[2/4] Failed: cargo exited with 101. Log: /logs/build-latest.log' });
  assert.deepEqual(failed, { phase: 'failed', step: 2, total: 4, text: 'Building Assembly',
    error: 'Failed: cargo exited with 101. Log: /logs/build-latest.log' });
});

test('starting again clears an earlier failure', () => {
  const failed = nextBuildLatest(idleBuildLatest, { done: true, error: 'Failed: no network' });
  const again = nextBuildLatest(failed, { line: '[1/4] Pulling main' });
  assert.equal(again.phase, 'running');
  assert.equal(again.error, '');
});
