import assert from 'node:assert/strict';
import test from 'node:test';

import { pickSpinner, SPINNERS } from '../src/lib/shell/components/conversation/workingSpinners.ts';

test('the set holds ten spinners with unique names', () => {
  assert.equal(SPINNERS.length, 10);
  assert.equal(new Set(SPINNERS.map((spinner) => spinner.id)).size, 10);
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
  const seeds = Array.from({ length: 10 }, (_, index) => `turn-${index + 1}`);
  const picked = new Set(seeds.map((seed) => pickSpinner(seed)));
  assert.ok(picked.size >= 4, `ten turns only reached ${picked.size} spinners`);
});

test('a seed the app can always supply is accepted', () => {
  assert.ok(SPINNERS.some((spinner) => spinner.id === pickSpinner('')));
});
