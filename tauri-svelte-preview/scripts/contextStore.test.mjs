/**
 * contextStore.test.mjs — the /next context cards' store, run in plain node.
 *
 * `contextStore.svelte.ts` is a runes module, so node cannot import it as it
 * stands: `$state` is compiler syntax, not a function. The test therefore does
 * what vite does — strip the TypeScript types, run the Svelte compiler over the
 * result, and import the compiled JavaScript. The compiled file is written
 * inside `node_modules` so that `svelte/internal/client` still resolves, and it
 * is deleted again at the end.
 *
 * This works only because the store has no runtime imports of its own (its two
 * imports are type-only). Keep it that way, or this test has to grow a bundler.
 */
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath } from 'node:url';
import { compileModule } from 'svelte/compiler';

const storePath = fileURLToPath(
  new URL('../src/lib/shell/context/contextStore.svelte.ts', import.meta.url)
);
const outputDir = fileURLToPath(new URL('../node_modules/.mcb-test/', import.meta.url));
const outputPath = `${outputDir}contextStore.compiled.mjs`;

mkdirSync(outputDir, { recursive: true });
const source = readFileSync(storePath, 'utf8');
const javascript = stripTypeScriptTypes(source, { mode: 'strip' });
const compiled = compileModule(javascript, { generate: 'client', filename: 'contextStore.svelte.js' });
writeFileSync(outputPath, compiled.js.code);

let store;
try {
  store = await import(outputPath);
} finally {
  rmSync(outputDir, { recursive: true, force: true });
}

const {
  applyCardRows,
  beginCardLoad,
  contextCardKeys,
  contextState,
  describeContextInput,
  failCardLoad,
  filterAgentRows,
  folderName,
  isCurrentCardRequest,
  markCardUnavailable,
  markContextActivated,
  processStopUnavailableReason,
  resetContext,
  runStatusGroup,
  setContextInput,
  summarizeAgents,
  summarizeRepositories,
  summarizeRuns,
  summarizeRuntime,
  summarizeWorktrees
} = store;

let passed = 0;
function test(name, run) {
  resetContext();
  run();
  passed += 1;
  console.log(`  ok  ${name}`);
}

// ── Shape ─────────────────────────────────────────────────────────────────────

test('starts inert: nothing activated, every card empty and idle', () => {
  assert.equal(contextState.activated, false);
  assert.deepEqual(contextState.projects, []);
  assert.equal(contextState.activeRoot, null);
  assert.deepEqual(contextCardKeys, ['runs', 'runtime', 'agents', 'worktrees', 'repositories']);
  for (const key of contextCardKeys) {
    assert.deepEqual(contextState[key].rows, [], `${key} rows`);
    assert.equal(contextState[key].loading, false, `${key} loading`);
    assert.equal(contextState[key].error, null, `${key} error`);
    assert.equal(contextState[key].unavailableReason, null, `${key} unavailableReason`);
    assert.equal(contextState[key].requestId, 0, `${key} requestId`);
    assert.equal(contextState[key].loadedAt, null, `${key} loadedAt`);
  }
});

// ── Input ─────────────────────────────────────────────────────────────────────

test('stores the input and names the project from the active root', () => {
  setContextInput({
    projects: [{ id: 'p1', name: 'Command Bar', path: '/dev/work/mac-command-bar' }],
    activeRoot: '/dev/work/mac-command-bar'
  });
  assert.equal(contextState.projects.length, 1);
  assert.equal(contextState.activeRoot, '/dev/work/mac-command-bar');
  assert.equal(contextState.projectName, 'mac-command-bar');
});

test('an explicit project name wins over the folder name', () => {
  setContextInput({
    projects: [],
    activeRoot: '/dev/work/mac-command-bar',
    projectName: 'Command Bar'
  });
  assert.equal(contextState.projectName, 'Command Bar');
});

test('no active root means no project name, and no crash', () => {
  setContextInput({ projects: [], activeRoot: null });
  assert.equal(contextState.projectName, '');
  assert.equal(folderName(null), '');
  assert.equal(folderName('/a/b/c/'), 'c');
  assert.equal(folderName('solo'), 'solo');
});

test('the input description only changes when the selection changes', () => {
  const first = describeContextInput({
    projects: [{ id: 'p1', name: 'One', path: '/one' }],
    activeRoot: '/one'
  });
  const same = describeContextInput({
    projects: [{ id: 'p1', name: 'One renamed', path: '/one' }],
    activeRoot: '/one'
  });
  const other = describeContextInput({
    projects: [{ id: 'p1', name: 'One', path: '/one' }],
    activeRoot: '/two'
  });
  assert.equal(first, same, 'a rename is not a new selection');
  assert.notEqual(first, other, 'a different active root is a new selection');
});

test('activation is recorded', () => {
  assert.equal(contextState.activated, false);
  markContextActivated();
  markContextActivated();
  assert.equal(contextState.activated, true);
});

// ── Load tickets (the superseded-request guard) ───────────────────────────────

test('taking a ticket marks the card loading and clears the last error', () => {
  failCardLoad('runs', beginCardLoad('runs'), 'boom');
  assert.equal(contextState.runs.error, 'boom');

  const ticket = beginCardLoad('runs');
  assert.equal(ticket, 2);
  assert.equal(contextState.runs.loading, true);
  assert.equal(contextState.runs.error, null);
  assert.equal(isCurrentCardRequest('runs', ticket), true);
});

test('a finished load lands rows, clears loading and stamps the time', () => {
  const ticket = beginCardLoad('runtime');
  const landed = applyCardRows('runtime', ticket, [{ pid: 1, port: 5177 }]);
  assert.equal(landed, true);
  assert.equal(contextState.runtime.rows.length, 1);
  assert.equal(contextState.runtime.loading, false);
  assert.equal(typeof contextState.runtime.loadedAt, 'number');
});

test('a superseded load is dropped — the newer scan keeps the card', () => {
  const slow = beginCardLoad('runs');
  const fast = beginCardLoad('runs');
  assert.equal(applyCardRows('runs', fast, [{ id: 'b', status: 'running' }]), true);
  assert.equal(applyCardRows('runs', slow, [{ id: 'a', status: 'failed' }]), false);
  assert.equal(contextState.runs.rows.length, 1);
  assert.equal(contextState.runs.rows[0].id, 'b');
  assert.equal(isCurrentCardRequest('runs', slow), false);
});

test('a superseded failure never overwrites a good newer result', () => {
  const slow = beginCardLoad('agents');
  const fast = beginCardLoad('agents');
  applyCardRows('agents', fast, [{ provider: 'claude', id: '1', title: 'One' }]);
  assert.equal(failCardLoad('agents', slow, 'the old scan blew up'), false);
  assert.equal(contextState.agents.error, null);
  assert.equal(contextState.agents.rows.length, 1);
  assert.equal(markCardUnavailable('agents', slow, 'desktop only'), false);
  assert.equal(contextState.agents.rows.length, 1);
});

test('a failed load stops the spinner and keeps the message', () => {
  const ticket = beginCardLoad('worktrees');
  assert.equal(failCardLoad('worktrees', ticket, 'Could not read worktrees: no such folder'), true);
  assert.equal(contextState.worktrees.loading, false);
  assert.equal(contextState.worktrees.error, 'Could not read worktrees: no such folder');
  assert.equal(contextState.worktrees.unavailableReason, null);
});

test('an unreachable card empties its rows and explains why', () => {
  applyCardRows('repositories', beginCardLoad('repositories'), [{ path: '/one' }]);
  const ticket = beginCardLoad('repositories');
  assert.equal(markCardUnavailable('repositories', ticket, 'This runs in the desktop app only.'), true);
  assert.deepEqual(contextState.repositories.rows, []);
  assert.equal(contextState.repositories.loading, false);
  assert.equal(contextState.repositories.error, null);
  assert.equal(contextState.repositories.unavailableReason, 'This runs in the desktop app only.');
});

test('every card keeps its own ticket counter', () => {
  beginCardLoad('runs');
  beginCardLoad('runs');
  const worktrees = beginCardLoad('worktrees');
  assert.equal(contextState.runs.requestId, 2);
  assert.equal(worktrees, 1);
  assert.equal(applyCardRows('worktrees', worktrees, [{ path: '/w' }]), true);
});

test('reset puts everything back to launch state', () => {
  setContextInput({ projects: [{ id: 'p', name: 'P', path: '/p' }], activeRoot: '/p' });
  markContextActivated();
  applyCardRows('runs', beginCardLoad('runs'), [{ id: 'a', status: 'running' }]);
  resetContext();
  assert.equal(contextState.activated, false);
  assert.equal(contextState.projectName, '');
  assert.deepEqual(contextState.runs.rows, []);
  assert.equal(contextState.runs.requestId, 0);
});

// ── Plain-English summaries ───────────────────────────────────────────────────

test('run statuses are grouped by what they mean, not by their exact wording', () => {
  assert.equal(runStatusGroup('Running'), 'running');
  assert.equal(runStatusGroup('in progress'), 'running');
  assert.equal(runStatusGroup('waiting for approval'), 'attention');
  assert.equal(runStatusGroup('needs review'), 'attention');
  assert.equal(runStatusGroup('failed'), 'failed');
  assert.equal(runStatusGroup('blocked'), 'failed');
  assert.equal(runStatusGroup('complete'), 'finished');
  assert.equal(runStatusGroup('queued'), 'other');
});

test('the runs summary says what needs a human', () => {
  assert.equal(summarizeRuns([]), '');
  assert.equal(
    summarizeRuns([
      { status: 'running' },
      { status: 'running' },
      { status: 'waiting for review' },
      { status: 'failed' }
    ]),
    '2 running, 1 waiting on you and 1 failed'
  );
  assert.equal(summarizeRuns([{ status: 'done' }]), '1 run, none running');
  assert.equal(summarizeRuns([{ status: 'done' }, { status: 'done' }]), '2 runs, none running');
});

test('the running-processes summary lists the ports in order', () => {
  assert.equal(summarizeRuntime([]), '');
  assert.equal(summarizeRuntime([{ port: 5177 }]), '1 process on port 5177');
  assert.equal(
    summarizeRuntime([{ port: 8080 }, { port: 3000 }, { port: 5177 }]),
    '3 processes on ports 3000, 5177, 8080'
  );
  assert.equal(
    summarizeRuntime([{ port: 1 }, { port: 2 }, { port: 3 }, { port: 4 }, { port: 5 }]),
    '5 processes on ports 1, 2, 3, 4 and 1 more'
  );
});

test('the agent summary names the providers', () => {
  assert.equal(summarizeAgents([]), '');
  assert.equal(summarizeAgents([{ provider: 'claude' }]), '1 session from claude');
  assert.equal(
    summarizeAgents([{ provider: 'claude' }, { provider: 'codex' }, { provider: 'claude' }]),
    '3 sessions from claude and codex'
  );
  assert.equal(summarizeAgents([{}, {}]), '2 sessions');
});

test('the worktree summary counts uncommitted work', () => {
  assert.equal(summarizeWorktrees([]), '');
  assert.equal(summarizeWorktrees([{ isDirty: false }]), '1 worktree, all clean');
  assert.equal(
    summarizeWorktrees([{ isDirty: true }, { isDirty: false }, { isDirty: true }]),
    '3 worktrees, 2 with uncommitted changes'
  );
});

test('the repository summary reports changes and sync state', () => {
  assert.equal(summarizeRepositories([]), '');
  assert.equal(
    summarizeRepositories([{ isDirty: false, ahead: 0, behind: 0 }]),
    '1 repository, all clean and in sync'
  );
  assert.equal(
    summarizeRepositories([
      { isDirty: true, ahead: 2, behind: 0 },
      { isDirty: false, ahead: 0, behind: 1 },
      { isDirty: false, ahead: 0, behind: 0, error: 'not a git repository' }
    ]),
    '3 repositories, 1 with uncommitted changes, 1 ahead of its remote, 1 behind its remote, 1 could not be read'
  );
});

test('the session search looks at name, agent and folder, not just what is on screen', () => {
  const rows = [
    { title: 'Fix the parser', id: 'a1', provider: 'claude', projectPath: '/dev/work/edi' },
    { title: 'Ship the docs', id: 'b2', provider: 'codex', projectPath: '/dev/work/portfolio' }
  ];
  assert.equal(filterAgentRows(rows, '').length, 2);
  assert.equal(filterAgentRows(rows, 'parser')[0].id, 'a1');
  assert.equal(filterAgentRows(rows, 'CODEX')[0].id, 'b2');
  assert.equal(filterAgentRows(rows, 'portfolio')[0].id, 'b2');
  assert.equal(filterAgentRows(rows, 'nothing here').length, 0);
});

test('a button we have not asked about yet is not reported as one the app cannot do', () => {
  assert.equal(processStopUnavailableReason('available'), null);
  assert.match(processStopUnavailableReason('unknown'), /Still asking/);
  assert.match(processStopUnavailableReason('unavailable'), /cannot do this yet/);
});

console.log(`\ncontextStore: ${passed} checks passed`);
