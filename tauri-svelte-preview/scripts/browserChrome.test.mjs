import assert from 'node:assert/strict';
import {
  BROWSER_OVERFLOW_ENTRIES,
  BROWSER_TOOLBAR_ACTIONS,
  BROWSER_VIEWPORT_MENU_ENTRIES,
  browserOverflowEntryIds,
  browserToolbarActionIds
} from '../src/lib/shell/browser/browserChrome.ts';

assert.deepEqual(browserToolbarActionIds(), [
  'import',
  'grab',
  'annotate',
  'draw',
  'devtools',
  'external',
  'overflow'
]);
assert.equal(BROWSER_TOOLBAR_ACTIONS.find((action) => action.id === 'import')?.iconOnly, false);
assert.ok(BROWSER_TOOLBAR_ACTIONS.filter((action) => action.iconOnly).length >= 5);

assert.deepEqual(browserOverflowEntryIds(), [
  'profile-default',
  'profile-new',
  'import-cookies',
  'viewport-size',
  'browser-settings'
]);
assert.equal(BROWSER_OVERFLOW_ENTRIES.filter((entry) => entry.section === 'profiles').length, 2);
assert.deepEqual(
  BROWSER_OVERFLOW_ENTRIES.map((entry) => entry.label),
  ['Default', 'New Profile…', 'Import Cookies', 'Viewport Size', 'Browser Settings…']
);

assert.deepEqual(
  BROWSER_VIEWPORT_MENU_ENTRIES.map((entry) => entry.preset),
  ['responsive', 'mobile-s', 'mobile-m', 'mobile-l', 'tablet', 'laptop', 'laptop-l', 'desktop']
);
assert.deepEqual(BROWSER_VIEWPORT_MENU_ENTRIES.map((entry) => entry.dimensions), [
  'full',
  '320 × 568',
  '375 × 667',
  '425 × 812',
  '768 × 1024',
  '1024 × 768',
  '1440 × 900',
  '1920 × 1080'
]);

console.log('browserChrome: all tests passed');
