/**
 * offlineRemoteSelection.test.ts — selecting a saved remote session while its
 * machine has gone away still opens the chat from the Mac's copy (TSK-1332).
 *
 * The app can still think the machine is connected for a moment after it
 * stops answering. The folder check, the saved workspace read and the draft
 * read then fail; before this change any one of them cleared the chat. The
 * modules that need Tauri or the whole shell are replaced with stubs below.
 */
import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';

(globalThis as { $state?: unknown }).$state = <T>(value: T): T => value;

const offline = 'throw new Error("Remote machine box is not connected")';
const stubs: Record<string, string> = {
  '/lib/tauriSource.ts': `
    export async function validateProjectRootFromTauri() { ${offline}; }
    export async function readAgentConversationWorkspaceFromTauri() { ${offline}; }
    export async function writeAgentConversationWorkspaceFromTauri() {}`,
  '/conversation/conversationService.ts': `export async function loadConversationSessionDraft() { ${offline}; }`,
  '/conversation/conversationConfig.ts': 'export async function warmAgentConversationConfig() { return null; }',
  '/conversation/conversationConnection.ts': `
    export function selectConversationChat() {}
    export async function selectedConversationChatReady() {}
    export function disposeSelectedConversationChat() {}`,
  '/conversation/conversationStore.svelte.ts': `
    const sessions = {};
    export function ensureConversationSession(ownedId) { return sessions[ownedId] ??= { generation: 1 }; }
    export function getConversationSession(ownedId) { return sessions[ownedId]; }
    export function evictInactiveConversationSessions() {}
    export function setConversationMode() {}
    export function setConversationAgentConfigState() {}
    export function setConversationAgentConfigError() {}
    export function captureConversationWorkspace() { return null; }`,
  '/projects/projectRegistry.svelte.ts': 'export const projectRegistry = { projects: [] };',
  '/devInvokeCounter.svelte.ts': 'export function countInvoke() {}',
  '/files/editor/editorStore.svelte.ts': `
    export const editorState = { openFiles: [], activePath: null };
    export function resetEditorState() {}
    export function restoreEditorFiles() {}
    export function setEditorProjectRoot() {}`
};

registerHooks({
  resolve(specifier, context, nextResolve) {
    const withExtension = specifier.startsWith('.') && !/\.[cm]?[jt]s$/.test(specifier) ? `${specifier}.ts` : specifier;
    const resolved = nextResolve(withExtension, context);
    const stub = Object.keys(stubs).find((suffix) => resolved.url.endsWith(suffix));
    return stub ? { url: `data:text/javascript,${encodeURIComponent(stubs[stub])}`, shortCircuit: true } : resolved;
  }
});

const { rail } = await import('../src/lib/shell/stores/sessionRailStore.svelte.ts');
const { SessionSelectionLayers } = await import('../src/lib/shell/sessionSelectionLayers.svelte.ts');
const { EditorSessionController } = await import('../src/lib/shell/controllers/editorSessionController.svelte.ts');

const session = {
  ownedId: 'saved-remote',
  agent: 'claude',
  cwd: '/home/me/repo',
  projectId: null,
  executionEnvironment: 'remote',
  remoteProfileId: 'box'
} as never;
rail.owned = [session];
// The app has not yet heard that the machine went away.
rail.remoteConnections.box = 'connected';

const layers = new SessionSelectionLayers();
await layers.selectSession(session, null, { generation: 0, signal: new AbortController().signal });
assert.equal(layers.chatOwnedId, 'saved-remote', 'the saved chat opens');
assert.equal(layers.hasChatProjection, true);
assert.equal(layers.treeRoot, '', 'the file tree waits for the machine');

const editor = new EditorSessionController();
const snapshot = await editor.restoreEditorWorkspaceForSession('saved-remote', layers.treeRoot, false, new AbortController().signal);
assert.equal(snapshot, null, 'the saved workspace read fails quietly');

console.log('offlineRemoteSelection: ok');
