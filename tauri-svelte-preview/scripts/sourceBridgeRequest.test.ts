import assert from 'node:assert/strict';
import test from 'node:test';

import { isOwnPageRequest } from '../src/lib/server/sourceBridgeRequest.ts';

test('the source bridge answers pages served by the dev server itself', () => {
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', origin: 'http://127.0.0.1:5177' }), true);
  assert.equal(isOwnPageRequest({ host: 'localhost:5301', origin: 'http://localhost:5301' }), true);
  assert.equal(isOwnPageRequest({ host: '[::1]:5177', origin: 'http://[::1]:5177' }), true);
  // Without an Origin header, the Referer names the page instead.
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', referer: 'http://127.0.0.1:5177/next?x=1' }), true);
});

test('the source bridge refuses other pages, rebound names and callers that name no page', () => {
  // Another app on localhost, for example one under test with an injected script.
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', origin: 'http://127.0.0.1:3000' }), false);
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', origin: 'https://evil.example' }), false);
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', referer: 'https://evil.example/page' }), false);
  // A website whose DNS name now points at 127.0.0.1 is same-origin with itself, not with us.
  assert.equal(isOwnPageRequest({ host: 'evil.example:5177', origin: 'http://evil.example:5177' }), false);
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177' }), false);
  assert.equal(isOwnPageRequest({ host: '127.0.0.1:5177', origin: 'null' }), false);
  assert.equal(isOwnPageRequest({ origin: 'http://127.0.0.1:5177' }), false);
});
