import assert from 'node:assert/strict';
import { normalizeProviderUsage } from '../src/lib/shell/usage/usageCurrent.ts';

const unavailable = normalizeProviderUsage({ provider: 'local', state: 'unavailable', reason: 'No quota data' });
assert.equal(unavailable.state, 'unavailable');
assert.equal(unavailable.windows.length, 0);
assert.equal(normalizeProviderUsage({ provider: 'local', inputTokens: 1000 }).state, 'unavailable');
console.log('current usage tests passed');
