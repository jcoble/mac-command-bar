import assert from 'node:assert/strict';

import {
  isRailUtilityRequest,
  isRailUtilityState,
  railUtilityAnchorStyle,
  railUtilityRequest
} from '../src/lib/shell/components/railUtilityEvents.ts';

const request = railUtilityRequest('usage', {
  left: 1184,
  top: 692,
  width: 32,
  height: 32
});

assert.deepEqual(request, {
  id: 'usage',
  anchor: { left: 1184, top: 692, width: 32, height: 32 }
});
assert.equal(isRailUtilityRequest(request), true);
assert.equal(
  railUtilityAnchorStyle(request.anchor),
  'left: 1184px; top: 692px; width: 32px; height: 32px;'
);
assert.equal(isRailUtilityRequest({ ...request, id: 'unknown' }), false);
assert.equal(isRailUtilityRequest({ ...request, anchor: { ...request.anchor, left: Number.NaN } }), false);
assert.equal(isRailUtilityState({ id: 'resources', open: true }), true);
assert.equal(isRailUtilityState({ id: 'usage', open: 'yes' }), false);

console.log('railUtilityEvents: anchored requests and open-state payloads verified');
