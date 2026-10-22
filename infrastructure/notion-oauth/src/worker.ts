import { decryptToken, encryptToken } from './tokenCrypto.ts';

interface D1RunResult {
  meta: { changes?: number };
}

interface D1FirstRow {
  encrypted_token: string;
}

interface D1Statement {
  bind(...values: unknown[]): D1Statement;
  first<T>(): Promise<T | null>;
  run(): Promise<D1RunResult>;
}

interface D1Database {
  prepare(query: string): D1Statement;
}

interface Environment {
  OAUTH_DB: D1Database;
  NOTION_CLIENT_ID: string;
  NOTION_CLIENT_SECRET: string;
  TOKEN_ENCRYPTION_KEY: string;
}

interface StartRequest {
  state: string;
  verifierHash: string;
}

interface ClaimRequest {
  state: string;
  verifier: string;
  claimCode: string;
}

interface NotionTokenResponse {
  access_token?: string;
}

const CALLBACK_URL = 'https://auth.coblesolutions.com/notion/callback';
const ATTEMPT_LIFETIME_SECONDS = 5 * 60;
const identifierPattern = /^[a-zA-Z0-9_-]{32,128}$/;
const hashPattern = /^[a-f0-9]{64}$/;

function json(body: unknown, status = 200): Response {
  return Response.json(body, {
    status,
    headers: { 'Cache-Control': 'no-store' }
  });
}

// `appLink` is built only from validated identifiers, so it needs no escaping.
function callbackPage(title: string, message: string, success: boolean, appLink = ''): Response {
  const color = success ? '#65d6bd' : '#ff7b72';
  const refresh = appLink ? `<meta http-equiv="refresh" content="0;url=${appLink}">` : '';
  const button = appLink
    ? `<p style="margin:1.25rem 0 0"><a href="${appLink}" style="color:#65d6bd">Open Assembly</a></p>`
    : '';
  return new Response(
    `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width">${refresh}<title>${title}</title><body style="margin:0;background:#111318;color:#f4f5f7;font:16px system-ui;display:grid;min-height:100vh;place-items:center"><main style="max-width:34rem;padding:2rem"><div style="width:.75rem;height:.75rem;border-radius:50%;background:${color};margin-bottom:1rem"></div><h1 style="font-size:1.5rem;margin:0 0 .75rem">${title}</h1><p style="color:#b9bec8;line-height:1.55;margin:0">${message}</p>${button}</main></body></html>`,
    { status: success ? 200 : 400, headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' } }
  );
}

async function sha256Hex(value: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function readJson<T>(request: Request): Promise<T | null> {
  try {
    return await request.json<T>();
  } catch {
    return null;
  }
}

async function discardExpired(env: Environment, now: number): Promise<void> {
  await env.OAUTH_DB.prepare('DELETE FROM oauth_attempts WHERE expires_at < ?').bind(now).run();
}

async function startOAuth(request: Request, env: Environment): Promise<Response> {
  const body = await readJson<StartRequest>(request);
  if (!body || !identifierPattern.test(body.state) || !hashPattern.test(body.verifierHash)) {
    return json({ error: 'Invalid login request' }, 400);
  }

  const now = Math.floor(Date.now() / 1000);
  await discardExpired(env, now);
  // A plain INSERT: a state already in use is refused, never replaced.
  try {
    await env.OAUTH_DB.prepare(
      'INSERT INTO oauth_attempts (state, verifier_hash, encrypted_token, expires_at) VALUES (?, ?, NULL, ?)'
    ).bind(body.state, body.verifierHash, now + ATTEMPT_LIFETIME_SECONDS).run();
  } catch {
    return json({ error: 'This login was already started' }, 409);
  }

  const authorizeUrl = new URL('https://api.notion.com/v1/oauth/authorize');
  authorizeUrl.searchParams.set('client_id', env.NOTION_CLIENT_ID);
  authorizeUrl.searchParams.set('response_type', 'code');
  authorizeUrl.searchParams.set('owner', 'user');
  authorizeUrl.searchParams.set('redirect_uri', CALLBACK_URL);
  authorizeUrl.searchParams.set('state', body.state);
  return json({ authorizationUrl: authorizeUrl.toString() });
}

async function notionCallback(request: Request, env: Environment): Promise<Response> {
  const url = new URL(request.url);
  const code = url.searchParams.get('code');
  const state = url.searchParams.get('state');
  if (!code || !state || !identifierPattern.test(state)) {
    return callbackPage('Notion was not connected', 'The login response was incomplete. Return to Assembly and try again.', false);
  }

  const now = Math.floor(Date.now() / 1000);
  const tokenResponse = await fetch('https://api.notion.com/v1/oauth/token', {
    method: 'POST',
    headers: {
      Authorization: `Basic ${btoa(`${env.NOTION_CLIENT_ID}:${env.NOTION_CLIENT_SECRET}`)}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ grant_type: 'authorization_code', code, redirect_uri: CALLBACK_URL })
  });
  const tokenPayload = await tokenResponse.json<NotionTokenResponse>();
  if (!tokenResponse.ok || !tokenPayload.access_token) {
    return callbackPage('Notion was not connected', 'Notion rejected the login. Return to Assembly and try again.', false);
  }

  // The token is released only to a claim that also carries this code. The
  // code goes to the app through its deep link from this browser, so whoever
  // started the flow, if not this browser's owner, never sees it.
  const claimCode = Array.from(crypto.getRandomValues(new Uint8Array(32)), (byte) => byte.toString(16).padStart(2, '0')).join('');
  const encryptedToken = await encryptToken(tokenPayload.access_token, env.TOKEN_ENCRYPTION_KEY);
  const result = await env.OAUTH_DB.prepare(
    'UPDATE oauth_attempts SET encrypted_token = ?, claim_code_hash = ? WHERE state = ? AND expires_at >= ? AND encrypted_token IS NULL'
  ).bind(encryptedToken, await sha256Hex(claimCode), state, now).run();
  if (result.meta.changes !== 1) {
    return callbackPage('This login expired', 'Return to Assembly and start a new Notion connection.', false);
  }
  return callbackPage(
    'Notion approved',
    'Assembly is opening to finish connecting. If it does not, use the link below.',
    true,
    `assembly://notion-oauth?state=${state}&amp;code=${claimCode}`
  );
}

async function claimOAuth(request: Request, env: Environment): Promise<Response> {
  const body = await readJson<ClaimRequest>(request);
  if (
    !body ||
    !identifierPattern.test(body.state) ||
    !identifierPattern.test(body.verifier) ||
    !identifierPattern.test(body.claimCode)
  ) {
    return json({ error: 'Invalid login claim' }, 400);
  }

  // One statement checks the verifier and the claim code and removes the
  // attempt, so a token can be claimed once.
  const now = Math.floor(Date.now() / 1000);
  const attempt = await env.OAUTH_DB.prepare(
    'DELETE FROM oauth_attempts WHERE state = ? AND verifier_hash = ? AND claim_code_hash = ? AND expires_at >= ? RETURNING encrypted_token'
  ).bind(body.state, await sha256Hex(body.verifier), await sha256Hex(body.claimCode), now).first<D1FirstRow>();
  if (!attempt) return json({ error: 'Login expired' }, 410);

  const token = await decryptToken(attempt.encrypted_token, env.TOKEN_ENCRYPTION_KEY);
  return json({ accessToken: token });
}

export default {
  async fetch(request: Request, env: Environment): Promise<Response> {
    const url = new URL(request.url);
    if (request.method === 'POST' && url.pathname === '/notion/start') return await startOAuth(request, env);
    if (request.method === 'GET' && url.pathname === '/notion/callback') return await notionCallback(request, env);
    if (request.method === 'POST' && url.pathname === '/notion/claim') return await claimOAuth(request, env);
    if (request.method === 'GET' && url.pathname === '/health') return json({ ok: true });
    return json({ error: 'Not found' }, 404);
  }
};
