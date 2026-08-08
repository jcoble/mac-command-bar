import assert from 'node:assert/strict';
import { resourceOwnerLabel, resourceCanStop } from '../src/lib/shell/resources/resourceViewModel.ts';

assert.equal(resourceOwnerLabel({ owner: 'owned-session', ownerId: 'session-1' }), 'Owned session · session-1');
assert.equal(resourceCanStop({ owner: 'external', canStop: false }), false);
assert.equal(resourceCanStop({ owner: 'owned-session', canStop: true }), true);
console.log('resource view model tests passed');
