/**
 * resourceSparkline.test.mjs — the shape drawn beside each Resource Manager row.
 *
 * A sparkline is easy to get subtly wrong: a series scaled against zero flattens
 * every quiet row, a single reading divides by zero, and a full-height stroke
 * gets clipped by the viewBox. These are the cases that catch that.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  describeSparkline,
  sparklineExtent,
  sparklinePath
} from '../src/lib/shell/resources/resourceSparkline.ts';

const size = { width: 60, height: 20 };

test('an empty series draws nothing at all', () => {
  assert.equal(sparklinePath([], size), '');
  assert.equal(sparklinePath([Number.NaN, Number.POSITIVE_INFINITY], size), '');
});

test('a flat series draws a flat line through the middle', () => {
  assert.equal(sparklinePath([5], size), 'M0,10 L60,10');
  assert.equal(sparklinePath([5, 5, 5], size), 'M0,10 L30,10 L60,10');
});

test('a rising series climbs from the bottom to the top of the box', () => {
  const path = sparklinePath([0, 50, 100], size);

  assert.equal(path, 'M0,19 L30,10 L60,1');
});

test('the line is scaled to its own series, not to zero', () => {
  // Three readings that differ by a little still read as a shape.
  const path = sparklinePath([100, 101, 102], size);

  assert.equal(path, 'M0,19 L30,10 L60,1');
});

test('the stroke stays inside the box', () => {
  const points = sparklinePath([0, 100], { ...size, inset: 2 })
    .split(' ')
    .map((point) => Number(point.split(',')[1]));

  assert.deepEqual(points, [18, 2]);
});

test('the extent ignores readings that are not numbers', () => {
  assert.deepEqual(sparklineExtent([3, Number.NaN, 9, 1]), { min: 1, max: 9 });
  assert.deepEqual(sparklineExtent([]), { min: 0, max: 0 });
});

test('the description says what the line covers', () => {
  assert.equal(describeSparkline([], 'cpu'), 'No history yet');
  assert.equal(describeSparkline([1, 2, 3], 'cpu'), 'CPU over the last 9s');
  assert.equal(describeSparkline(new Array(60).fill(1), 'memory'), 'Memory over the last 3m');
});
