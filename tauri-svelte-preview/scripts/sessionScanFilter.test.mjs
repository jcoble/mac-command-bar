import assert from 'node:assert/strict';

import { isAgentLaunchEntrypoint, parseClaudeJsonl } from '../src/lib/server/localSourceFs.ts';

/**
 * Bridge parity for the Rust scanner's "is this a top-level session?" rule
 * (core/src/scanners/sessions.rs). Same fixtures, same expectations — if these
 * two ever disagree, the rail shows different sessions depending on which
 * scanner answered.
 */

const projectPath = '/Users/dev/work/mac-command-bar';

/**
 * A helper-agent transcript as Claude Code actually writes it: flat in the
 * project directory, `isSidechain: false`, `userType: "external"`, no agent
 * name anywhere — identical to a real session except for the entrypoint that
 * launched it.
 */
const helperAgentJsonl = [
  '{"type":"user","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","promptSource":"sdk","sessionId":"S4","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T11:00:00Z","message":{"role":"user","content":"You are implementing Task 1 of the v2-base plan."}}',
  '{"type":"assistant","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","sessionId":"S4","timestamp":"2026-07-28T11:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}'
].join('\n');

/**
 * A REAL session that opens with pasted log output. The user does this often,
 * and it is why the filter reads the entrypoint rather than guessing from the
 * shape of the first message: nothing about this text distinguishes it from a
 * machine-authored dispatch prompt.
 */
const realSessionStartingWithAPastedLogJsonl = [
  '{"type":"user","isSidechain":false,"userType":"external","entrypoint":"cli","promptSource":"typed","sessionId":"S5","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T12:00:00Z","message":{"role":"user","content":"You are seeing this in the console:\\n[vite] hmr update /src/routes/next/+page.svelte\\nERROR  Cannot read properties of undefined (reading \'api\')\\n    at mount (chunk-QK6X.js:14:9)\\nReview this change and tell me why. Your final message should name the file."}}',
  '{"type":"assistant","isSidechain":false,"entrypoint":"cli","sessionId":"S5","timestamp":"2026-07-28T12:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"That mount is running before the store exists."}]}}'
].join('\n');

/** A transcript old enough to predate the entrypoint field entirely. */
const transcriptWithoutAnEntrypointJsonl = [
  '{"type":"user","isSidechain":false,"sessionId":"S1","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T09:00:00Z","message":{"role":"user","content":"Fix the resume rail"}}',
  '{"type":"assistant","isSidechain":false,"sessionId":"S1","timestamp":"2026-07-28T09:02:00Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{}}]}}'
].join('\n');

/** A subagent transcript, still excluded by the older sidechain rule. */
const subagentJsonl =
  '{"parentUuid":null,"isSidechain":true,"agentId":"a0a5","type":"user","sessionId":"S2","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T10:00:00Z","message":{"role":"user","content":"You are implementing Task 6 of the Slice 1 plan."}}';

// The entrypoint is the whole rule: `sdk-*` launched it programmatically.
assert.equal(isAgentLaunchEntrypoint('sdk-cli'), true);
assert.equal(isAgentLaunchEntrypoint('sdk-py'), true);
assert.equal(isAgentLaunchEntrypoint('cli'), false);
assert.equal(isAgentLaunchEntrypoint('claude-vscode'), false);
assert.equal(isAgentLaunchEntrypoint(''), false);

assert.deepEqual(parseClaudeJsonl(helperAgentJsonl, projectPath), []);

// A real session that opens with pasted log output and the very phrases a
// dispatch prompt uses ("You are ", "Review this change", "Your final message")
// must survive. Excluding one of the user's own sessions is strictly worse than
// listing a helper.
const pastedLog = parseClaudeJsonl(realSessionStartingWithAPastedLogJsonl, projectPath);
assert.equal(pastedLog.length, 1);
assert.equal(pastedLog[0].id, 'S5');

// A missing entrypoint is not evidence of a helper, and the cost of guessing
// wrong is losing a session the user wanted.
const noEntrypoint = parseClaudeJsonl(transcriptWithoutAnEntrypointJsonl, projectPath);
assert.equal(noEntrypoint.length, 1);
assert.equal(noEntrypoint[0].id, 'S1');

assert.deepEqual(parseClaudeJsonl(subagentJsonl, projectPath), []);

console.log('session scan filter tests passed');
