import assert from 'node:assert/strict';

import { onOpenFile, type OpenFileRequest } from '../src/lib/shell/openFileBus.ts';
import {
  CENTER_TAB_IDS,
  RIGHT_TAB_IDS,
  clearWorkbenchNavigation,
  focusComposerWith,
  openDiffForFile,
  openFileInEditor,
  openUrlInBrowser,
  registerWorkbenchNavigation,
  showCenterTab,
  showRightTab,
  startWorkbenchSession
} from '../src/lib/shell/workbenchNavigation.ts';

/** Everything each caller did, in the order it happened. */
let trace: string[] = [];
const openedFiles: OpenFileRequest[] = [];
onOpenFile((request) => {
  trace.push(`requestOpenFile:${request.path}`);
  openedFiles.push(request);
});

// ── Nothing registered: every caller is a quiet no-op ────────────────────────

clearWorkbenchNavigation();
showCenterTab('editor');
showRightTab('files');
await openDiffForFile({ projectRoot: '/repo', relativePath: 'src/a.ts' });
await openUrlInBrowser({ url: 'https://example.test' });
await focusComposerWith({ ownedId: 'one' });
assert.equal(
  await startWorkbenchSession({ prompt: 'hello', cwd: '/repo', projectPath: '/repo', title: 'Look' }),
  null,
  'starting a session with nothing registered answers null rather than throwing'
);
assert.deepEqual(
  trace.filter((entry) => !entry.startsWith('requestOpenFile')),
  [],
  'no handler is registered, so nothing was called'
);

// `openFileInEditor` still puts the request on the file bus — that bus is not a
// workbench handler and works on its own.
trace = [];
openFileInEditor({ path: '/repo/src/a.ts' });
assert.deepEqual(trace, ['requestOpenFile:/repo/src/a.ts'], 'the file bus is independent of the handlers');

// ── Registration merges, one field at a time ────────────────────────────────

clearWorkbenchNavigation();
trace = [];
registerWorkbenchNavigation({ showCenterTab: (id) => trace.push(`center:${id}`) });
showCenterTab('diff');
showRightTab('run');
assert.deepEqual(trace, ['center:diff'], 'registering one handler leaves the others unregistered');

registerWorkbenchNavigation({ showRightTab: (id) => trace.push(`right:${id}`) });
showCenterTab('session');
showRightTab('run');
assert.deepEqual(
  trace,
  ['center:diff', 'center:session', 'right:run'],
  'a second registration adds to the first rather than replacing it'
);

// ── The composed callers, and the order they do things in ───────────────────

clearWorkbenchNavigation();
trace = [];
openedFiles.length = 0;
registerWorkbenchNavigation({
  showCenterTab: (id) => trace.push(`center:${id}`),
  showRightTab: (id) => trace.push(`right:${id}`),
  openDiff: (request) => {
    trace.push(`diff:${request.projectRoot}:${request.relativePath}`);
  },
  openUrl: (request) => {
    trace.push(`url:${request.url}`);
  },
  focusComposer: (handoff) => {
    trace.push(`composer:${handoff.ownedId}`);
  },
  startSession: async (request) => {
    trace.push(`session:${request.cwd}`);
    return 'owned-1';
  }
});

openFileInEditor({ path: '/repo/src/a.ts', projectRoot: '/repo' });
assert.deepEqual(
  trace,
  ['requestOpenFile:/repo/src/a.ts', 'center:editor'],
  'a file goes on the bus first, then the editor is brought forward'
);
assert.equal(openedFiles[0]?.projectRoot, '/repo', 'the whole request reaches the bus');

trace = [];
await openDiffForFile({ projectRoot: '/repo', relativePath: 'src/a.ts' });
assert.deepEqual(
  trace,
  ['diff:/repo:src/a.ts', 'center:diff'],
  'the file is selected before the diff tab is brought forward'
);

trace = [];
await openUrlInBrowser({ url: 'https://example.test' });
assert.deepEqual(
  trace,
  ['right:browser', 'url:https://example.test'],
  'the browser panel is on screen before it is asked to navigate'
);

trace = [];
await focusComposerWith({ ownedId: 'one', appendText: 'look at this' });
assert.deepEqual(
  trace,
  ['composer:one', 'center:session'],
  'the composer is handed its content, then the session comes to the front'
);

trace = [];
assert.equal(
  await startWorkbenchSession({ prompt: 'hi', cwd: '/repo', projectPath: '/repo', title: 'Look' }),
  'owned-1',
  'the new session’s id is handed back to the caller'
);
assert.deepEqual(trace, ['session:/repo']);

// ── Clearing unregisters everything ─────────────────────────────────────────

clearWorkbenchNavigation();
trace = [];
showCenterTab('editor');
showRightTab('files');
await openDiffForFile({ projectRoot: '/repo', relativePath: 'src/a.ts' });
await openUrlInBrowser({ url: 'https://example.test' });
await focusComposerWith({ ownedId: 'one' });
assert.equal(
  await startWorkbenchSession({ prompt: 'hi', cwd: '/repo', projectPath: '/repo', title: 'Look' }),
  null
);
assert.deepEqual(trace, [], 'after clearing, every caller is a no-op again');

assert.equal(RIGHT_TAB_IDS.length, 8);
assert.equal(CENTER_TAB_IDS.length, 4);

console.log('workbenchNavigation: no-op safety, partial registration, call order, and clearing passed');
