import assert from 'node:assert/strict';
import { test } from 'node:test';
import { openBotThreads, reviewState } from './prBotReview.ts';

const summary = (sha: string, status: string) => `<!-- codex-pull-request-review-summary -->
<!-- codex-review-state:v1 {"code_review":{"commit_sha":"${sha}","has_any_findings":true,"has_unresolved_findings":true,"status":"${status}"},"security_review":{"commit_sha":null,"has_any_findings":null,"has_unresolved_findings":null,"status":null}} -->

## Codex Review Summary`;

test('reads the commit the bot last reviewed from its newest summary comment', () => {
  assert.deepEqual(reviewState(['Some other comment', summary('aaa', 'completed'), summary('bbb', 'running')]),
    { commit_sha: 'bbb', has_any_findings: true, has_unresolved_findings: true, status: 'running' });
  assert.equal(reviewState(['no marker here']), null);
});

test('keeps only unresolved threads the bot started', () => {
  const thread = (id: string, login: string, isResolved: boolean) => ({ id, isResolved,
    comments: { nodes: [{ databaseId: 1, author: { login }, path: 'a.rs', line: 3, originalLine: 3, body: 'x' }] } });
  const open = openBotThreads([thread('1', 'chatgpt-codex-connector[bot]', false), thread('2', 'chatgpt-codex-connector[bot]', true), thread('3', 'jcoble', false)]);
  assert.deepEqual(open.map((item) => item.id), ['1']);
});
