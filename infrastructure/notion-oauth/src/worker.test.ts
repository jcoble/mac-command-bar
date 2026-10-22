// Run with: node --experimental-strip-types infrastructure/notion-oauth/src/worker.test.ts
import assert from 'node:assert/strict';
import { createHash, randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { DatabaseSync } from 'node:sqlite';

import worker from './worker.ts';

// A stand-in for Cloudflare D1 backed by an in-memory SQLite with the real migrations.
const sqlite = new DatabaseSync(':memory:');
for (const migration of ['0001_oauth_attempts.sql', '0002_claim_code.sql']) {
  sqlite.exec(readFileSync(new URL(`../migrations/${migration}`, import.meta.url), 'utf8'));
}
const OAUTH_DB = {
  prepare(query: string) {
    let values: unknown[] = [];
    const statement = {
      bind(...bound: unknown[]) {
        values = bound;
        return statement;
      },
      async first<T>(): Promise<T | null> {
        return (sqlite.prepare(query).get(...(values as never[])) as T | undefined) ?? null;
      },
      async run() {
        return { meta: { changes: Number(sqlite.prepare(query).run(...(values as never[])).changes) } };
      }
    };
    return statement;
  }
};
const env = {
  OAUTH_DB,
  NOTION_CLIENT_ID: 'client',
  NOTION_CLIENT_SECRET: 'secret',
  TOKEN_ENCRYPTION_KEY: randomBytes(32).toString('base64')
};

globalThis.fetch = async () => Response.json({ access_token: 'owner-notion-token' });

const identifier = (): string => randomBytes(24).toString('hex');
const call = (path: string, body?: unknown): Promise<Response> =>
  worker.fetch(
    new Request(`https://auth.example${path}`, body === undefined ? {} : { method: 'POST', body: JSON.stringify(body) }),
    env as never
  );

async function start(state: string, verifier: string): Promise<Response> {
  return await call('/notion/start', { state, verifierHash: createHash('sha256').update(verifier).digest('hex') });
}

// Someone starts a flow and gets the owner to approve it. The owner's browser
// gets the claim code; the starter, who only has the state and verifier, does not.
const state = identifier();
const verifier = identifier();
assert.equal((await start(state, verifier)).status, 200);
const callback = await call(`/notion/callback?code=notion-code&state=${state}`);
assert.equal(callback.status, 200);
const deepLink = (await callback.text()).match(/assembly:\/\/notion-oauth\?state=([\w-]+)&amp;code=([\w-]+)/);
assert.ok(deepLink, 'the callback page links back to the app with a claim code');
assert.equal(deepLink[1], state);
const claimCode = deepLink[2];

assert.equal((await call('/notion/claim', { state, verifier })).status, 400, 'a claim without the code is refused');
assert.equal(
  (await call('/notion/claim', { state, verifier, claimCode: identifier() })).status,
  410,
  'a claim with a wrong code is refused'
);

const claimed = await call('/notion/claim', { state, verifier, claimCode });
assert.equal(claimed.status, 200);
assert.deepEqual(await claimed.json(), { accessToken: 'owner-notion-token' });
assert.equal((await call('/notion/claim', { state, verifier, claimCode })).status, 410, 'a code works once');

// A second start with a state already in use does not replace the first attempt.
const reused = identifier();
assert.equal((await start(reused, verifier)).status, 200);
assert.equal((await start(reused, identifier())).status, 409);

// A second callback for the same attempt cannot swap in another token or code.
const twice = identifier();
assert.equal((await start(twice, verifier)).status, 200);
assert.equal((await call(`/notion/callback?code=one&state=${twice}`)).status, 200);
assert.equal((await call(`/notion/callback?code=two&state=${twice}`)).status, 400);

console.log('notion oauth worker: ok');
