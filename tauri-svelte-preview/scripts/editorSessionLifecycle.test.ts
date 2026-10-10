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
// 19112edf3: disposeRoute now runs disposeRouteOnce once and shares its promise.
assert.match(page, /async function disposeRouteOnce\(\): Promise<void>[\s\S]*?await selectionDisposal/);
assert.match(selection, /async dispose\(\): Promise<void>[\s\S]*?await editorDisposal/);
assert.doesNotMatch(selection, /void this\.editorSessions\.dispose\(\)/);
assert.match(selection, /await this\.editorSessions\.restoreEditorWorkspaceForSession\([\s\S]*?owner\.signal/);
assert.match(editor, /async restoreEditorWorkspaceForSession\([\s\S]*?stopSignal: AbortSignal/);
// d00de3f79 queued the checkpoint and 2d5702688 wrapped it in try; 725896a73 and 1c889045f wrapped the read for remote sessions.
assert.match(editor, /this\.checkpointActiveWorkspace\(stopSignal\)\);\s*try \{\s*await this\.checkpointQueue;[\s\S]*?\}\s*if \(stopSignal\.aborted\) return null;\s*this\.releaseActiveEditorResources\(\);/);
assert.match(editor, /this\.releaseActiveEditorResources\(\);\s*if \(stopSignal\.aborted\) return null;[\s\S]*?await readAgentConversationWorkspaceFromTauri\(ownedId\);\s*\} catch \(error\) \{\s*if \(stopSignal\.aborted\) return null;[\s\S]*?const snapshot = [^;]+;\s*if \(stopSignal\.aborted\) return null;/);
assert.match(editor, /planWorkspaceRestore\(snapshot\)/);
assert.match(editor, /restoreEditorFiles\(plan\.openFiles, plan\.activePath\)/);
assert.match(editor, /await writeAgentConversationWorkspaceFromTauri\(ownedId, snapshot\);\s*if \(stopSignal\.aborted\) return;/);
assert.doesNotMatch(editor, /\.then\(|\.catch\(|\.finally\(/);
assert.doesNotMatch(page, /\{#key\s+selection\.activeOwnedId/);

assert.match(panel, /let sessionStopController = new AbortController\(\);/);
assert.match(panel, /export function releaseSessionResources[\s\S]*?sessionStopController\.abort\(\);[\s\S]*?sessionStopController = new AbortController\(\);/);
assert.match(panel, /destroyed = true;\s*sessionStopController\.abort\(\);/);
// 41df05d99: images skip the text read, so the read is now one arm of a ternary.
assert.match(panel, /const preview = [^;]*?await readSourceFromTauri\(record\);\s*if \(stopSignal\.aborted\) return;/);
assert.match(panel, /const saved = await writeSourceToTauri[\s\S]*?if \(stopSignal\.aborted\) return false;/);
assert.match(panel, /await warmSourceLspForRootFromTauri\(projectRoot\);\s*if \(stopSignal\.aborted\) return;/);

// 00215d7a7 replaced remote deployment with connecting (deployRemoteAssemblyFromTauri is gone).
// 5a5aca827 moved pickProjectFolder() into AddProjectDialog, dropped loadRefs() from send, renamed
// hydrate() into a Promise.all, and made onSend the last await of its try (the catch checks the signal).
// The guard before a call may now be any earlier abort check with no await in between.
for (const call of [
  'listGitRefs(machine, projectPath)',
  'onSend(request, stagedImages.map((item) => item.file))',
  'Promise.all([hydrateProjects(), hydrateRemoteAssembly(owner)])',
  'readRemoteAssemblyEnvironmentFromTauri()'
]) {
  const escaped = call.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  assert.match(
    draft,
    new RegExp(`if \\(stopSignal\\.aborted(?: \\|\\| [^)]+)?\\) return;(?:(?!await )[\\s\\S])*?(?:[^;\\n]+ = )?await ${escaped};\\s*(?:if \\(stopSignal\\.aborted(?: \\|\\| [^)]+)?\\) return;|\\} catch \\(error\\) \\{\\s*if \\(!stopSignal\\.aborted\\))`),
    `${call} must be guarded before and after`
  );
}

assert.match(startup, /onSelectInitial\(ownedId: string, stopSignal: AbortSignal\): Promise<void>/);
// 8fee02322 opens session loads between the check and the select; a0b9ca6d0 lists remote sessions together with local ones.
assert.match(startup, /if \(!shellActive\(generation, controller\.signal\)\) return;\s*shellPanels\.allowSessionLoads\(\);\s*await options\.onSelectInitial\(initial\.ownedId, controller\.signal\);\s*if \(!shellActive\(generation, controller\.signal\)\) return;/);
assert.match(startup, /listRemoteAgentConversationSessionsFromTauri\(controller\.signal\)/);

console.log('editorSessionLifecycle.test.ts passed');
