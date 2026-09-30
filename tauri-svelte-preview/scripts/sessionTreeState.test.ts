import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { execFileSync } from 'node:child_process';
import { compile, compileModule } from 'svelte/compiler';
import ts from 'typescript';
import * as runtime from 'svelte/internal/client';

// Execute the production controller and entire FilesPanel script with real Svelte
// effects. Replace I/O, not expansion logic; assert state, never markup or style.
const output = new URL('./.tsk-1311-test/', import.meta.url);
const root = '/same/worktree';
const persisted = new Map<string, readonly string[]>();
const noop = () => undefined;
const deferred = () => {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
};
let pendingWrite: ReturnType<typeof deferred> | null = null;
let pendingRead: ReturnType<typeof deferred> | null = null;
let readStarted = deferred();
const reads: string[] = [];
const writes: string[] = [];
const dependencies: Record<string, unknown> = {
  ...runtime, dev: false, onDestroy: noop,
  rail: { owned: ['A', 'B'].map((ownedId) => ({ ownedId, agent: 'codex', root })), activeOwnedId: null },
  sessionWorkspaceRoot: (session: { root: string }) => session.root,
  canonicalPath: (path: string) => path.replace(/\/+$/, ''),
  shellPanels: { sessionPicked: noop },
  NewSessionController: class { abandonDraftForSessionSwitch = noop; },
  EditorSessionController: class {
    rememberWorkspaceState = noop;
    restoreEditorWorkspaceForSession = async () => null;
  },
  writeAssemblySettingFromTauri: async () => undefined,
  readAssemblySettingFromTauri: async () => undefined,
  validateProjectRootFromTauri: async () => ({ exists: true, isDirectory: true }),
  readAgentConversationWorkspaceExpandedPathsFromTauri: async (id: string) => {
    reads.push(id);
    const value = persisted.get(id) ?? [];
    const gate = pendingRead; pendingRead = null; readStarted.resolve();
    if (gate) await gate.promise;
    return value;
  },
  writeAgentConversationWorkspaceExpandedPathsFromTauri: async (id: string, _root: string, paths: string[]) => {
    writes.push(id);
    const gate = pendingWrite; pendingWrite = null;
    if (gate) await gate.promise;
    persisted.set(id, [...paths]);
  },
  explorer: { root, activated: true, lastScanFinishedAt: 1, unavailable: null },
  explorerNodes: () => [], loadedExplorerDirectories: () => [],
  loadedExplorerDirectoryDepth: () => null,
  activateExplorer: noop, loadDirectory: async () => undefined,
  listRepositoryCheckoutsFromTauri: async () => ({}),
  onWorkspaceFileChange: () => noop,
  visibleFileTreeNodes: () => [], windowFileTreeNodes: () => ({ nodes: [] }),
};
Object.assign(globalThis, { __treeTestDependencies: dependencies });
mkdirSync(output, { recursive: true });
function source(path: string): string {
  return process.env.TREE_TEST_BASE
    ? execFileSync('git', ['show', `${process.env.TREE_TEST_BASE}:tauri-svelte-preview/${path}`], { encoding: 'utf8' })
    : readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
}
async function load(path: string, component = false) {
  let code = source(path);
  if (component) code = code.slice(code.indexOf('>') + 1, code.indexOf('</script>'));
  code = stripTypeScriptTypes(code, { mode: 'strip' });
  const parsed = ts.createSourceFile(path, code, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
  for (const node of [...parsed.statements].reverse()) {
    if (!ts.isImportDeclaration(node)) continue;
    const names: string[] = [];
    if (node.importClause?.name) names.push(node.importClause.name.text);
    const bindings = node.importClause?.namedBindings;
    if (bindings && ts.isNamedImports(bindings)) names.push(...bindings.elements.map((part) => part.name.text));
    for (const name of names) if (!(name in dependencies)) dependencies[name] = noop;
    code = code.slice(0, node.getStart(parsed)) + `const { ${names.join(', ')} } = globalThis.__treeTestDependencies;` + code.slice(node.end);
  }
  const compiled = component
    ? compile(`<script>${code}\nexport function openPaths() { return [...expanded].sort(); }\nexport function toggle(path) { onTreeNodeClicked({ path, isDirectory: true, depth: 0 }); }</script>`, { generate: 'client', filename: 'FilesPanel.svelte' })
    : compileModule(code, { generate: 'client', filename: path });
  const target = new URL(`${component ? 'panel' : path.split('/').at(-1)}.ts`, output);
  writeFileSync(target, compiled.js.code);
  return import(target.href);
}
let destroy = noop;
try {
  dependencies.SessionSelectionLayers = (await load('src/lib/shell/sessionSelectionLayers.svelte.ts')).SessionSelectionLayers;
  const { SessionSelectionController } = await load('src/lib/shell/controllers/sessionSelectionController.svelte.ts');
  const controller = new SessionSelectionController();
  const { default: FilesPanel } = await load('src/lib/shell/panels/files/FilesPanel.svelte', true);
  let panel!: { openPaths(): string[]; toggle(path: string): void };
  destroy = runtime.effect_root(() => {
    panel = FilesPanel(null, {
      visible: true,
      get root() { return controller.filesProjectionRoot; },
      get ownedId() { return controller.filesProjectionOwnedId; },
      get expandedPathsByRoot() { return controller.expandedPathsByRoot; },
      onExpandedPathsChange: (...args: unknown[]) => controller.rememberExpandedPaths(...args),
    });
  });
  const flush = async () => { for (let i = 0; i < 12; i++) { await Promise.resolve(); runtime.flush(); } };
  const select = async (id: string) => { await controller.selectSession(id); await flush(); };
  const expectPaths = (paths: string[]) => assert.deepEqual(panel.openPaths(), paths.map((path) => `${root}/${path}`));
  await select('A'); panel.toggle(`${root}/X`); await flush(); expectPaths(['X']);
  await select('B'); expectPaths([]); panel.toggle(`${root}/Y`); await flush(); expectPaths(['Y']);
  await select('A'); expectPaths(['X']);
  await select('B'); expectPaths(['Y']);
  for (let i = 0; i < 3; i++) {
    await select('A');
    const gate = deferred(); pendingWrite = gate;
    panel.toggle(`${root}/X`); panel.toggle(`${root}/X`);
    const readCount = reads.length;
    const first = controller.selectSession('B'); await flush();
    const last = controller.selectSession('A'); await flush();
    controller.rememberExpandedPaths('B', root, [`${root}/stale`]);
    assert.equal(reads.length, readCount, 'restore must wait for departing writes');
    gate.resolve(); await Promise.all([first, last]); await flush(); expectPaths(['X']);
    const staleRead = deferred(); pendingRead = staleRead; readStarted = deferred();
    const old = controller.selectSession('B'); await readStarted.promise;
    const latest = controller.selectSession('A'); staleRead.resolve();
    await Promise.all([old, latest]); await flush(); expectPaths(['X']);
    await select('B'); expectPaths(['Y']);
  }
  await select('A'); panel.toggle(`${root}/X`); await flush();
  await select('B'); expectPaths(['Y']); await select('A'); expectPaths([]);
  assert.deepEqual(persisted.get('A'), []);
  assert.deepEqual(persisted.get('B'), [`${root}/Y`]);
  assert.equal(writes.filter((id) => id === 'B').length, 1, 'stale callbacks cannot write for B');
  console.log('PASS: actual FilesPanel A/X → B/Y → A → B, rapid queued writes, late reads, and empty collapse');
} finally {
  destroy(); rmSync(output, { recursive: true, force: true });
  Reflect.deleteProperty(globalThis, '__treeTestDependencies');
}
