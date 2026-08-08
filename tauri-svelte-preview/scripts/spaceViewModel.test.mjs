import assert from 'node:assert/strict';
import { diskProtectionLabel, reclaimableBytes } from '../src/lib/shell/resources/workspaceSpaceViewModel.ts';

assert.equal(reclaimableBytes({ bytes: 100, reclaimableBytes: 80, protection: 'safe-candidate' }), 80);
assert.equal(reclaimableBytes({ bytes: 100, reclaimableBytes: 80, protection: 'dirty' }), 0);
assert.equal(diskProtectionLabel('safe-candidate'), 'Safe to review');
console.log('space view-model tests passed');
