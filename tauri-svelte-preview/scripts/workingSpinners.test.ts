import assert from 'node:assert/strict';
import test from 'node:test';

import { pickSpinner, SPINNERS } from '../src/lib/shell/components/conversation/workingSpinners.ts';

test('the set holds the eleven selected spinners with unique names', () => {
  assert.deepEqual(SPINNERS.map((spinner) => spinner.id), [
    'cube', 'facet', 'tide', 'ribbon', 'bloom', 'wave', 'bars', 'halo', 'diamond', 'disc', 'gyro'
  ]);
  assert.equal(new Set(SPINNERS.map((spinner) => spinner.id)).size, 11);
});

test('at least three of the set are drawn in three dimensions', () => {
  assert.ok(SPINNERS.filter((spinner) => spinner.kind === '3d').length >= 3);
});

test('every spinner draws at least one part', () => {
  for (const spinner of SPINNERS) {
    assert.ok(spinner.parts >= 1, `${spinner.id} draws nothing`);
  }
});

test('the same seed always picks the same spinner', () => {
  const first = pickSpinner('turn-1');
  assert.equal(pickSpinner('turn-1'), first);
  assert.equal(pickSpinner('turn-1'), first);
  assert.ok(SPINNERS.some((spinner) => spinner.id === first));
});

test('different turns spread across the set', () => {
  const seeds = Array.from({ length: 64 }, (_, index) => `turn-${index + 1}`);
  const picked = new Set(seeds.map((seed) => pickSpinner(seed)));
  assert.equal(picked.size, SPINNERS.length, `64 turns only reached ${picked.size} spinners`);
});

test('a seed the app can always supply is accepted', () => {
  assert.ok(SPINNERS.some((spinner) => spinner.id === pickSpinner('')));
});
