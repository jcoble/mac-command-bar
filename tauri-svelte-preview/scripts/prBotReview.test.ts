import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { openThreads, reviewState } from './prBotReview.ts';

const recorded = JSON.parse(readFileSync(join(import.meta.dirname, 'fixtures/mergeGate/threads.json'), 'utf8')).threads;
const [outdated, agentResolved, open] = recorded;

const summary = (sha: string, status: string) => `<!-- codex-pull-request-review-summary -->
<!-- codex-review-state:v1 {"code_review":{"commit_sha":"${sha}","has_any_findings":true,"has_unresolved_findings":true,"status":"${status}"},"security_review":{"commit_sha":null,"has_any_findings":null,"has_unresolved_findings":null,"status":null}} -->

## Codex Review Summary`;

test('reads the commit the bot last reviewed from its newest summary comment', () => {
  assert.deepEqual(reviewState(['Some other comment', summary('aaa', 'completed'), summary('bbb', 'running')]),
    { commit_sha: 'bbb', has_any_findings: true, has_unresolved_findings: true, status: 'running' });
  assert.equal(reviewState(['no marker here']), null);
});

test('keeps only threads the bot started', () => {
  const byUser = { ...open, id: 'user', comments: { nodes: [{ ...open.comments.nodes[0], author: { __typename: 'User', login: 'jcoble' } }] } };
  const lookalike = { ...open, id: 'lookalike', comments: { nodes: [{ ...open.comments.nodes[0], author: { __typename: 'User', login: 'chatgpt-codex-connector1' } }] } };
  assert.deepEqual(openThreads([open, byUser, lookalike], []).map((item) => item.id), [open.id]);
});

test('an agent-resolved bot thread still counts as open', () => {
  assert.equal(agentResolved.isResolved, true);
  assert.deepEqual(openThreads([agentResolved], []).map((item) => item.id), [agentResolved.id]);
});

test('a thread whose id is in the accepted list is closed', () => {
  assert.deepEqual(openThreads([open, agentResolved], [open.id, agentResolved.id]), []);
});

test('an outdated thread is closed', () => {
  assert.equal(outdated.isOutdated, true);
  assert.deepEqual(openThreads([outdated], []), []);
});

// A fake gh: the bot stays "running" on the head until someone posts "@codex review",
// then reports "completed" (or keeps running when FAKE_STUCK is set). A no-op sleep keeps the waits instant.
const fakeGh = `#!/bin/bash
dir=$(dirname "$0")
state=running
if [ -f "$dir/asked" ] && [ -z "$FAKE_STUCK" ]; then state=completed; fi
case "$1 $2" in
  "repo view") echo o/r ;;
  "pr view") echo '{"number":7,"headRefOid":"abc123def"}' ;;
  "pr comment") echo "$5" >> "$dir/asked" ;;
  "api graphql") echo '[{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":['"$FAKE_THREAD"']}}}}}]' ;;
  "api --paginate") echo '[[{"user":{"login":"'"$FAKE_LOGIN"'"},"body":"<!-- codex-review-state:v1 {\\"code_review\\":{\\"commit_sha\\":\\"abc123def\\",\\"status\\":\\"'$state'\\"}} -->"}]]' ;;
esac
`;

function runWithStalledBot(stuck: boolean, login = 'chatgpt-codex-connector[bot]', thread = '', args: string[] = []): { status: number | null; asks: string[] } {
  const dir = mkdtempSync(join(tmpdir(), 'pr-bot-review-'));
  writeFileSync(join(dir, 'gh'), fakeGh);
  writeFileSync(join(dir, 'sleep'), '#!/bin/sh\n');
  chmodSync(join(dir, 'gh'), 0o755);
  chmodSync(join(dir, 'sleep'), 0o755);
  const run = spawnSync(process.execPath, ['--experimental-strip-types', join(import.meta.dirname, 'prBotReview.ts'), ...args],
    { env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, FAKE_STUCK: stuck ? '1' : '', FAKE_LOGIN: login, FAKE_THREAD: thread }, encoding: 'utf8' });
  let asks: string[] = [];
  try { asks = readFileSync(join(dir, 'asked'), 'utf8').trim().split('\n'); } catch { /* never asked */ }
  return { status: run.status, asks };
}

test('asks the bot once more when it stalls on the head, and passes once it finishes', () => {
  assert.deepEqual(runWithStalledBot(false), { status: 0, asks: ['@codex review'] });
});

test('asks only once and exits 2 when the bot is still stuck after the second wait', () => {
  assert.deepEqual(runWithStalledBot(true), { status: 2, asks: ['@codex review'] });
});

test('ignores a review-state marker posted by an account whose login only starts with the bot name', () => {
  assert.equal(runWithStalledBot(false, 'chatgpt-codex-connector1').status, 2);
});

test('--accepted <file> closes the bot threads listed in it, one id per line', () => {
  const dir = mkdtempSync(join(tmpdir(), 'pr-bot-accepted-'));
  writeFileSync(join(dir, 'accepted'), `\n${open.id}\n`);
  assert.equal(runWithStalledBot(false, undefined, JSON.stringify(open)).status, 1);
  assert.equal(runWithStalledBot(false, undefined, JSON.stringify(open), ['--accepted', join(dir, 'accepted')]).status, 0);
});
