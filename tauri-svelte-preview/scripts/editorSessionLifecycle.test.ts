import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const page = readFileSync(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const selection = readFileSync(
  new URL('../src/lib/shell/controllers/sessionSelectionController.svelte.ts', import.meta.url),
  'utf8'
);
const editor = readFileSync(
  new URL('../src/lib/shell/controllers/editorSessionController.svelte.ts', import.meta.url),
  'utf8'
);
const panel = readFileSync(
  new URL('../src/lib/shell/components/EditorPanel.svelte', import.meta.url),
  'utf8'
);
const draft = readFileSync(
  new URL('../src/lib/shell/newSession/DraftSessionSurface.svelte', import.meta.url),
  'utf8'
);
const startup = readFileSync(
  new URL('../src/lib/shell/controllers/shellStartup.ts', import.meta.url),
  'utf8'
);

assert.match(page, /<EditorPanel[\s\S]*?bind:this=\{editorPanel\}/);
assert.match(page, /rootAvailable=\{selection\.controlledEditorRootAvailable\}/);
assert.match(page, /\$effect\(\(\) => \{\s*selection\.setEditorPanel\(editorPanel\);\s*\}\);/);
assert.doesNotMatch(page, /onMount\(\(\) => \{\s*selection\.setEditorPanel\(editorPanel\)/);
assert.match(page, /async function disposeRoute\(\): Promise<void>[\s\S]*?await selectionDisposal/);
assert.match(selection, /async dispose\(\): Promise<void>[\s\S]*?await editorDisposal/);
assert.doesNotMatch(selection, /void this\.editorSessions\.dispose\(\)/);
assert.match(selection, /await this\.editorSessions\.restoreEditorWorkspaceForSession\([\s\S]*?owner\.signal/);
assert.match(editor, /async restoreEditorWorkspaceForSession\([\s\S]*?stopSignal: AbortSignal/);
assert.match(editor, /await this\.checkpointActiveWorkspace\(stopSignal\);\s*if \(stopSignal\.aborted\) return;/);
assert.match(editor, /this\.releaseActiveEditorResources\(\);\s*if \(stopSignal\.aborted\) return;\s*const snapshot = await readAgentConversationWorkspaceFromTauri\(ownedId\);\s*if \(stopSignal\.aborted\) return;/);
assert.match(editor, /planWorkspaceRestore\(snapshot\)/);
assert.match(editor, /restoreEditorFiles\(plan\.openFiles, plan\.activePath\)/);
assert.match(editor, /await writeAgentConversationWorkspaceFromTauri\(ownedId, snapshot\);\s*if \(stopSignal\.aborted\) return;/);
assert.doesNotMatch(editor, /\.then\(|\.catch\(|\.finally\(/);
assert.doesNotMatch(page, /\{#key\s+selection\.activeOwnedId/);

assert.match(panel, /let sessionStopController = new AbortController\(\);/);
assert.match(panel, /export function releaseSessionResources[\s\S]*?sessionStopController\.abort\(\);[\s\S]*?sessionStopController = new AbortController\(\);/);
assert.match(panel, /destroyed = true;\s*sessionStopController\.abort\(\);/);
assert.match(panel, /const preview = await readSourceFromTauri\(record\);\s*if \(stopSignal\.aborted\) return;/);
assert.match(panel, /const saved = await writeSourceToTauri[\s\S]*?if \(stopSignal\.aborted\) return false;/);
assert.match(panel, /await warmSourceLspForRootFromTauri\(projectRoot\);\s*if \(stopSignal\.aborted\) return;/);

for (const call of [
  'listGitRefs(projectPath)',
  'deployRemoteAssemblyFromTauri(remoteProfile)',
  'pickProjectFolder()',
  'initProjectRepository(draft.projectPath)',
  'loadRefs(draft.projectPath)',
  'onSend(request)',
  'hydrate()',
  'readRemoteAssemblyEnvironmentFromTauri()'
]) {
  const escaped = call.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  assert.match(
    draft,
    new RegExp(`if \\(stopSignal\\.aborted\\) return;\\s*(?:[^;\\n]+ = )?await ${escaped};\\s*if \\(stopSignal\\.aborted\\) return;`),
    `${call} must be guarded before and after`
  );
}

assert.match(startup, /onSelectInitial\(ownedId: string, stopSignal: AbortSignal\): Promise<void>/);
assert.match(startup, /if \(!shellActive\(generation, controller\.signal\)\) return;\s*await options\.onSelectInitial\(initial\.ownedId, controller\.signal\);\s*if \(!shellActive\(generation, controller\.signal\)\) return;/);
assert.match(startup, /listRemoteAgentConversationSessionsFromTauri\(stopSignal\)/);

console.log('editorSessionLifecycle.test.ts passed');
