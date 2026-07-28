import assert from 'node:assert/strict';

import {
  codexRolloutHeadIsSubagentThread,
  isAgentLaunchEntrypoint,
  mergeCodexSessionMetadata,
  parseClaudeJsonl,
  parseCodexIndexJsonl,
  parseCodexRolloutJsonl
} from '../src/lib/server/localSourceFs.ts';

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

/**
 * A thread Codex spawned for itself. Both markers are present, as they are on
 * 618 of the 619 helper threads on this machine.
 */
const codexSubagentRolloutJsonl = [
  '{"timestamp":"2026-07-28T17:24:36.972Z","type":"session_meta","payload":{"id":"019fa9c1","parent_thread_id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"subagent","agent_role":"executor","source":{"subagent":{"parent_thread_id":"019fa964","depth":1}}}}',
  '{"timestamp":"2026-07-28T17:25:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Implement the year simulation runner."}]}}'
].join('\n');

/**
 * A session the user started in the terminal. `source` is a plain string, and
 * the first two turns are the ones Codex writes for itself before the user has
 * typed anything.
 */
const codexTopLevelRolloutJsonl = [
  '{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}',
  '{"timestamp":"2026-07-28T16:42:18.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /Users/dev/work/rental-management\\n\\n<INSTRUCTIONS>\\nEvery reply MUST open with a plain-English summary.\\n</INSTRUCTIONS>"}]}}',
  '{"timestamp":"2026-07-28T16:42:19.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\\n  <cwd>/Users/dev/work/rental-management</cwd>\\n</environment_context>"}]}}',
  '{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Please plan out an entire year of scans and entries for the 2027 simulation."}]}}'
].join('\n');

/** An older Codex build that wrote no thread marker at all. */
const codexUnmarkedRolloutJsonl = [
  '{"timestamp":"2026-04-14T13:53:07.000Z","type":"session_meta","payload":{"id":"019d8d20","cwd":"/Users/dev/work/EdiPlatform","originator":"codex_cli_rs","source":"unknown"}}',
  '{"timestamp":"2026-04-14T13:54:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Help me trace where the 810 mapping loses the invoice date."}]}}'
].join('\n');

// Either marker on its own is enough — they disagreed on one real file.
assert.equal(
  codexRolloutHeadIsSubagentThread(
    '{"type":"session_meta","payload":{"thread_source":"subagent","source":"vscode"}}'
  ),
  true
);
assert.equal(
  codexRolloutHeadIsSubagentThread(
    '{"type":"session_meta","payload":{"source":{"subagent":{"depth":1}}}}'
  ),
  true
);

// A session the user started: `source` is a plain string, not an object.
assert.equal(
  codexRolloutHeadIsSubagentThread(
    '{"type":"session_meta","payload":{"thread_source":"user","source":"cli"}}'
  ),
  false
);
// No marker, unreadable, or not a metadata line at all: keep the file.
assert.equal(
  codexRolloutHeadIsSubagentThread('{"type":"session_meta","payload":{"source":"unknown"}}'),
  false
);
assert.equal(codexRolloutHeadIsSubagentThread('{"type":"sessi'), false);
assert.equal(codexRolloutHeadIsSubagentThread(''), false);

assert.deepEqual(parseCodexRolloutJsonl(codexSubagentRolloutJsonl), []);

const codexTopLevel = parseCodexRolloutJsonl(codexTopLevelRolloutJsonl);
assert.equal(codexTopLevel.length, 1);
assert.equal(codexTopLevel[0].id, '019fa964');

// Older rollouts predate the marker. Keep them: losing a session the user wants
// back is worse than listing a helper thread.
const codexUnmarked = parseCodexRolloutJsonl(codexUnmarkedRolloutJsonl);
assert.equal(codexUnmarked.length, 1);
assert.equal(codexUnmarked[0].id, '019d8d20');

// The repository instructions and the environment block are written by Codex,
// not the user, so neither may become the title.
assert.equal(
  codexTopLevel[0].title,
  'Please plan out an entire year of scans and entries for the...'
);

// Nothing typed inside the scanned window leaves the generic label.
assert.equal(
  parseCodexRolloutJsonl(
    '{"type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev","source":"cli"}}'
  )[0].title,
  'Codex session'
);

// The index file names some sessions. That name must survive a merge with the
// rollout file whichever record carries the later timestamp.
assert.equal(
  mergeCodexSessionMetadata(
    parseCodexIndexJsonl(
      '{"id":"019fa964","thread_name":"Year simulation planning","updated_at":"2026-07-28T16:00:00.000000Z"}'
    ),
    parseCodexRolloutJsonl(codexTopLevelRolloutJsonl)
  )[0].title,
  'Year simulation planning'
);

// And an unnamed index row must not overwrite a real rollout title.
assert.equal(
  mergeCodexSessionMetadata(
    parseCodexIndexJsonl('{"id":"019fa964","updated_at":"2026-07-29T16:00:00.000000Z"}'),
    parseCodexRolloutJsonl(codexTopLevelRolloutJsonl)
  )[0].title,
  'Please plan out an entire year of scans and entries for the...'
);

console.log('session scan filter tests passed');
