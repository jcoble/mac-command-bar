import assert from 'node:assert/strict';

import { afterFloatingSurfacePaint, type FrameScheduler } from '../src/lib/shell/floatingSurface.ts';

const frames: FrameRequestCallback[] = [];
const schedule: FrameScheduler = (callback) => {
  frames.push(callback);
  return frames.length;
};
let calls = 0;

afterFloatingSurfacePaint(() => calls += 1, schedule);
assert.equal(calls, 0, 'opening does not run optional work synchronously');
frames.shift()?.(0);
assert.equal(calls, 0, 'the first animation frame belongs to rendering the surface');
frames.shift()?.(16);
assert.equal(calls, 1, 'optional work starts only after the surface had a paint opportunity');

console.log('floating surface scheduling tests passed');
