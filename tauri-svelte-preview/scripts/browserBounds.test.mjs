import assert from 'node:assert/strict';
import {
  BROWSER_VIEWPORT_PRESETS,
  clampBrowserFloatingBounds,
  resolveBrowserViewport
} from '../src/lib/shell/browser/browserBounds.ts';

assert.deepEqual(BROWSER_VIEWPORT_PRESETS['mobile-m'], {
  preset: 'mobile-m',
  width: 375,
  height: 667
});
assert.deepEqual(resolveBrowserViewport('responsive'), {
  preset: 'responsive',
  width: null,
  height: null
});
assert.deepEqual(resolveBrowserViewport({ preset: 'custom', width: 900, height: 700 }), {
  preset: 'custom',
  width: 900,
  height: 700
});
assert.deepEqual(
  resolveBrowserViewport({ preset: 'custom', width: 1, height: 99999 }),
  { preset: 'custom', width: 200, height: 4096 },
  'custom viewports stay within a useful page size'
);

assert.deepEqual(
  clampBrowserFloatingBounds(
    { x: -500, y: 900, width: 1200, height: 900 },
    { width: 800, height: 600 }
  ),
  { x: 0, y: 0, width: 800, height: 600 },
  'a floating browser remains inside a small window'
);
assert.deepEqual(
  clampBrowserFloatingBounds(
    { x: 1000, y: 1000, width: 420, height: 300 },
    { width: 1200, height: 800 }
  ),
  { x: 780, y: 500, width: 420, height: 300 },
  'the title edge remains reachable after a resize'
);

console.log('browserBounds: all tests passed');
