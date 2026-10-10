import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import ts from 'typescript';

// Execute the production controller with storage and editor I/O replaced.
// These assertions concern persistence and selection, not component structure.
const source = readFileSync(new URL('../src/lib/shell/controllers/editorSessionController.svelte.ts', import.meta.url), 'utf8');
const parsed = ts.createSourceFile('controller.ts', source, ts.ScriptTarget.Latest, true);
let code = source;
for (const statement of [...parsed.statements].reverse()) {
  if (ts.isImportDeclaration(statement)) code = code.slice(0, statement.getStart(parsed)) + code.slice(statement.end);
}
code = stripTypeScriptTypes(code.replace('export class EditorSessionController', 'class EditorSessionController'));
interface Controller {
  restoreEditorWorkspaceForSession(id: string, root: string, available: boolean, signal: AbortSignal): Promise<unknown>;
}
const reads: string[] = [];
const writes: string[] = [];
let writeFailure: unknown;
let readFailure: unknown;
let root: string | null = null;
const dependencies = {
  rail: { owned: [], remoteConnections: {} },
  parseRemoteWorkspacePath: () => null,
  mapWorkspaceSnapshotPaths: (value: unknown) => value,
  editorState: { openFiles: [], activePath: null },
  resetEditorState: () => undefined,
  restoreEditorFiles: () => undefined,
  setEditorProjectRoot: (value: string | null) => { root = value; },
  captureWorkspace: () => ({ openPaths: [], activePath: null }),
  planWorkspaceRestore: () => ({ openFiles: [], activePath: null }),
  captureConversationWorkspace: () => null,
  readAgentConversationWorkspaceFromTauri: async (id: string) => {
    reads.push(id);
    if (readFailure && id === 'other') throw readFailure;
    return { openPaths: [], activePath: null, owner: id };
  },
  writeAgentConversationWorkspaceFromTauri: async (id: string) => {
    writes.push(id);
    if (writeFailure) throw writeFailure;
  },
};
const ControllerClass = new Function(...Object.keys(dependencies), `${code}; return EditorSessionController;`)(...Object.values(dependencies)) as new () => Controller;
const signal = new AbortController().signal;
const missing = 'could not save the workspace because the session does not exist';

for (const failure of [missing, new Error(missing), { code: 'agent-conversation-command-failed', message: missing }]) {
  writeFailure = undefined; readFailure = undefined; reads.length = 0; writes.length = 0;
  const controller = new ControllerClass();
  await controller.restoreEditorWorkspaceForSession('removed', '/old', true, signal);
  writeFailure = failure;
  const restored = await controller.restoreEditorWorkspaceForSession('existing', '/new', true, signal);
  assert.deepEqual(restored, { openPaths: [], activePath: null, owner: 'existing' });
  assert.equal(root, '/new');
  assert.deepEqual(writes, ['removed']);
  writeFailure = undefined;
  await controller.restoreEditorWorkspaceForSession('third', '/third', true, signal);
  await controller.restoreEditorWorkspaceForSession('existing', '/new', true, signal);
  assert.deepEqual(writes, ['removed', 'existing', 'third'], 'subsequent switches checkpoint the correct owner');
  const count = reads.length;
  await controller.restoreEditorWorkspaceForSession('existing', '/new', true, signal);
  assert.equal(reads.length, count, 'same-session selection does not repeat storage work');
}

writeFailure = undefined; readFailure = undefined;
const controller = new ControllerClass();
await controller.restoreEditorWorkspaceForSession('saved', '/saved', true, signal);
const diskError = new Error('disk full');
writeFailure = diskError;
await assert.rejects(controller.restoreEditorWorkspaceForSession('other', '/other', true, signal), (error) => error === diskError);
assert.equal(root, '/saved', 'real save errors retain the outgoing workspace');
writeFailure = undefined;
readFailure = new Error('workspace read failed');
await assert.rejects(controller.restoreEditorWorkspaceForSession('other', '/other', true, signal), (error) => error === readFailure);
readFailure = undefined;
await controller.restoreEditorWorkspaceForSession('other', '/other', true, signal);
assert.equal(root, '/other', 'retry remains possible after storage recovers');
const aborted = AbortSignal.abort();
const count = reads.length;
await controller.restoreEditorWorkspaceForSession('cancelled', '/cancelled', true, aborted);
assert.equal(reads.length, count);
console.log('editorWorkspaceRecovery: deleted outgoing session, repeat switching, save/read failures and cancellation passed');
