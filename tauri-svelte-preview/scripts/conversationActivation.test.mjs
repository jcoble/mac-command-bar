import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { decideConversationActivation } from '../src/lib/shell/conversation/conversationActivation.ts';

const appSession = {
  agent: 'codex',
  origin: 'app',
  state: 'live',
  executionOwner: 'structured',
  ptySessionId: null,
  nativeSessionId: 'thread-app'
};

const matchingConnection = {
  provider: 'codex',
  state: 'connected',
  nativeSessionId: 'thread-app'
};

assert.deepEqual(
  decideConversationActivation(appSession, matchingConnection),
  { kind: 'view' },
  'a connected app session is only brought into view'
);

assert.deepEqual(
  decideConversationActivation(appSession, null),
  { kind: 'structured', nativeSessionMode: 'resume' },
  'a disconnected app session resumes through ACP'
);

assert.deepEqual(
  decideConversationActivation(
    { ...appSession, state: 'exited', executionOwner: 'stopped' },
    matchingConnection
  ),
  { kind: 'structured', nativeSessionMode: 'resume' },
  'a stopped app session resumes even when its retained frontend snapshot still says connected'
);

const stoppedExternal = {
  agent: 'codex',
  origin: 'external',
  state: 'background',
  executionOwner: 'stopped',
  ptySessionId: null,
  nativeSessionId: 'thread-external'
};

assert.deepEqual(
  decideConversationActivation(stoppedExternal, {
    provider: 'codex',
    state: 'connected',
    nativeSessionId: 'thread-external'
  }),
  { kind: 'structured', nativeSessionMode: 'load' },
  'a stopped external Codex session loads by native id instead of trusting a stale connection'
);

assert.deepEqual(
  decideConversationActivation(
    { ...stoppedExternal, state: 'live', executionOwner: 'terminal', ptySessionId: 'pty-live' },
    null
  ),
  { kind: 'terminal' },
  'a running external session stays terminal-owned'
);

assert.deepEqual(
  decideConversationActivation({ ...stoppedExternal, nativeSessionId: null }, null),
  { kind: 'terminal' },
  'external history without a native id keeps the terminal fallback'
);

const page = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
const selectionController = readFileSync(
  new URL('../src/lib/shell/controllers/sessionSelectionController.svelte.ts', import.meta.url),
  'utf8'
);
const selectionLayers = readFileSync(
  new URL('../src/lib/shell/sessionSelectionLayers.svelte.ts', import.meta.url),
  'utf8'
);
const shellFrame = readFileSync(
  new URL('../src/lib/shell/components/ShellFrame.svelte', import.meta.url),
  'utf8'
);
const shellStartup = readFileSync(
  new URL('../src/lib/shell/controllers/shellStartup.ts', import.meta.url),
  'utf8'
);
const tauriSource = readFileSync(new URL('../src/lib/tauriSource.ts', import.meta.url), 'utf8');
const remoteConversation = readFileSync(
  new URL('../src-tauri/src/agent_conversation/remote.rs', import.meta.url),
  'utf8'
);
assert.match(
  page,
  /async function selectSession\(ownedId: string\): Promise<void> \{\s+await selection\.selectSession\(ownedId\);\s+\}/,
  'rail selection automatically runs the owned conversation activation'
);
assert.match(
  page,
  /selection\.hasChatProjection && selection\.chatOwnedId[\s\S]*?hidden=\{selection\.chatOwnedId !== selection\.activeOwnedId\}[\s\S]*?aria-hidden=\{selection\.chatOwnedId !== selection\.activeOwnedId \? "true" : undefined\}[\s\S]*?inert=\{selection\.chatOwnedId !== selection\.activeOwnedId\}[\s\S]*?activeOwnedId=\{selection\.chatOwnedId\}/,
  'the previous conversation surface stays mounted but is hidden and inert while the next rail selection loads'
);
assert.match(
  page,
  /\.conversation-surface-shell\[hidden\]\s*\{[\s\S]*?display:\s*none/,
  'inactive conversation surfaces are removed from layout and paint while retained'
);
assert.match(
  page,
  /selection\.activeOwnedId && \(!selection\.hasChatProjection \|\| selection\.chatOwnedId !== selection\.activeOwnedId\)[\s\S]*?Loading conversation/,
  'loading state is shown when the active rail session differs from the mounted chat projection'
);
assert.match(
  page,
  /\.conversation-data-isolation \{[\s\S]*?inset:\s*0;[\s\S]*?position:\s*absolute;[\s\S]*?z-index:\s*1;[\s\S]*?background:\s*var\(--color-bg\);/,
  'the loading state is an opaque overlay that covers the retained conversation surface'
);
assert.doesNotMatch(
  page,
  /conversation-surface-shell[\s\S]{0,180}opacity:\s*0/,
  'inactive conversation surfaces do not add an opacity compositor layer'
);
for (const id of ['session', 'editor', 'diff', 'git-history']) {
  assert.match(
    shellFrame,
    new RegExp(`id: '${id}'[\\s\\S]*?renderer: 'onlyWhenVisible'`),
    `${id} center panel only keeps a renderer while it is visible`
  );
}
assert.match(
  shellStartup,
  /const storedSessions = \(await listAgentConversationSessionsFromTauri\(\)\)[\s\S]*?hydrateOwned\(projected\)[\s\S]*?void hydrateRemoteSessionsForOwner\(generation, controller\.signal\)/,
  'local sessions publish before cancellable remote discovery begins'
);
assert.match(
  tauriSource,
  /listRemoteAgentConversationSessionsFromTauri\([\s\S]*?signal\?\.addEventListener\('abort', cancel, \{ once: true \}\)[\s\S]*?finally \{[\s\S]*?signal\?\.removeEventListener\('abort', cancel\)/,
  'remote session discovery removes its abort listener when the invoke settles'
);
assert.match(
  remoteConversation,
  /pub async fn cancel_request[\s\S]*?try_send\(ClientRequest::Cancel \{ id: request_id \}\)/,
  'remote request cancellation never waits behind a saturated request queue'
);
assert.match(
  tauriSource,
  /readAgentConversationCapabilitiesFromTauri\([\s\S]*?requestId[\s\S]*?signal\?\.addEventListener\('abort', cancel, \{ once: true \}\)[\s\S]*?read_agent_conversation_capabilities[\s\S]*?finally \{[\s\S]*?signal\?\.removeEventListener\('abort', cancel\)/,
  'remote capability reads are owned by the surface abort signal'
);
assert.match(
  tauriSource,
  /listAgentConversationEventsBeforeFromTauri\([\s\S]*?requestId[\s\S]*?signal\?\.addEventListener\('abort', cancel, \{ once: true \}\)[\s\S]*?list_agent_conversation_events_before[\s\S]*?finally \{[\s\S]*?signal\?\.removeEventListener\('abort', cancel\)/,
  'older event page reads are owned by the surface abort signal'
);
assert.match(
  tauriSource,
  /listAgentConversationEventsAfterFromTauri\([\s\S]*?requestId[\s\S]*?signal\?\.addEventListener\('abort', cancel, \{ once: true \}\)[\s\S]*?list_agent_conversation_events_after[\s\S]*?finally \{[\s\S]*?signal\?\.removeEventListener\('abort', cancel\)/,
  'newer event page reads are owned by the surface abort signal'
);
assert.match(
  tauriSource,
  /readAgentConversationWorkspaceExpandedPathsFromTauri\([\s\S]*?requestId[\s\S]*?signal\?\.addEventListener\('abort', cancel, \{ once: true \}\)[\s\S]*?read_agent_conversation_workspace_expanded_paths[\s\S]*?finally \{[\s\S]*?signal\?\.removeEventListener\('abort', cancel\)/,
  'expanded-path reads are owned by the selection abort signal'
);
assert.doesNotMatch(
  selectionController,
  /setActiveOwned\(ownedId\);[\s\S]{0,160}clearChatHistory\(\);[\s\S]{0,160}const session = rail\.owned\.find/,
  'rail selection does not clear the rendered chat before the candidate load settles'
);
assert.match(
  selectionLayers,
  /await loadConversationForRead\(session\.ownedId, false, owner\.signal\);[\s\S]*?if \(!this\.isCurrent\(owner\)\) \{[\s\S]*?releaseConversationForRead\(session\.ownedId\);[\s\S]*?return;/,
  'a stale candidate load releases only the stale candidate'
);
assert.match(
  selectionLayers,
  /if \(departingOwnedId && departingOwnedId !== session\.ownedId\) \{[\s\S]*?releaseConversationForRead\(departingOwnedId\);[\s\S]*?this\.hasChatProjection = false;[\s\S]*?this\.chatOwnedId = null;[\s\S]*?evictInactiveConversationSessions\(null\);[\s\S]*?await loadConversationForRead\(session\.ownedId, false, owner\.signal\);/,
  'the departing projection is released before the next conversation snapshot is awaited'
);
assert.doesNotMatch(page, /Open conversation|Conversation paused/, 'conversation activation has no manual workaround gate');

console.log('conversationActivation.test.mjs passed');
