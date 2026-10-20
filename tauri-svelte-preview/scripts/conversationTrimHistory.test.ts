/// <reference types="node" />

// Returning to the latest drops the older pages the reader loaded. The real
// connection module runs here against the real store; only the backend reads
// and the TanStack chat are stand-ins.
import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileModule } from 'svelte/compiler';
import { EventType, StreamProcessor } from '@tanstack/ai/client';
import * as conversationMessages from '../src/lib/shell/conversation/conversationMessages.ts';
import { usageDropIsCompaction } from '../src/lib/shell/conversation/conversationReducer.ts';

function compileForTest(path: URL, filename: string): string {
  const stripped = stripTypeScriptTypes(readFileSync(path, 'utf8'), { mode: 'strip' });
  return compileModule(stripped, { generate: 'client', filename }).js.code
    .replaceAll('import.meta.env', '({ DEV: false })');
}

const storeOutput = fileURLToPath(new URL('../src/lib/shell/conversation/.conversationTrimStore.test.mjs', import.meta.url));
const diagnosticsOutput = fileURLToPath(new URL('../src/lib/shell/.conversationTrimDiagnostics.test.mjs', import.meta.url));
writeFileSync(storeOutput, compileForTest(
  new URL('../src/lib/shell/conversation/conversationStore.svelte.ts', import.meta.url), 'conversationStore.svelte.js'
).replace(/\.\.\/resourceDiagnostics\.svelte(\.ts|\.js)/, '../.conversationTrimDiagnostics.test.mjs'));
writeFileSync(diagnosticsOutput, compileForTest(
  new URL('../src/lib/shell/resourceDiagnostics.svelte.ts', import.meta.url), 'resourceDiagnostics.svelte.js'
));
(globalThis as { $state?: unknown }).$state = (value: unknown) => value;
let store: any;
try {
  store = await import(`${storeOutput}?test=${Date.now()}`);
} finally {
  rmSync(storeOutput, { force: true });
  rmSync(diagnosticsOutput, { force: true });
}

const PAGE = 10;
const WITH_ATTACHMENT = new Set([12, 25]);
/** A message number that is a background command still running, or 0 for none. */
let runningToolAt = 0;

/** Messages first..last, one user message per turn; 12 and 25 carry a screenshot. */
function page(first: number, last: number, hasBefore: boolean) {
  const numbers = Array.from({ length: last - first + 1 }, (_, index) => first + index);
  return {
    items: numbers.map((n) => ({ itemId: `u${n}`, stableId: `u${n}`, firstTimestampMs: n,
      firstSequence: n, lastSequence: n, positionKnown: true, prefixComplete: true })),
    events: numbers.map((n) => ({ ownedId: 'chat', provider: 'codex', generation: 1, sequence: n, timestampMs: n,
      payload: n === runningToolAt
        ? { kind: 'tool', itemId: `u${n}`, turnId: `t${n}`, name: 'Run command', state: 'started', rawInput: { command: 'sleep' } }
        : { kind: 'userMessage', itemId: `u${n}`, turnId: `t${n}`, text: `message ${n}`,
          ...(WITH_ATTACHMENT.has(n) ? { attachmentIds: [`a${n}`] } : {}) } })),
    turns: numbers.map((n) => ({ turnId: `t${n}` })),
    watermark: last, transferBytes: 0, oversized: false, hasBefore, hasAfter: false,
    hasEarlierTranscript: false, beforeCursor: first, afterCursor: last
  };
}

function loadConnection() {
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationConnection.ts', import.meta.url), 'utf8');
  const body = source.slice(source.indexOf('const PAGE_BYTES')).replace(/^export /gm, '');
  const olderCursors: number[] = [];
  let admitSend: (receipt: unknown) => void = () => undefined;
  let liveEvent: (event: unknown) => void = () => undefined;
  const snapshotState: { activeTurnId?: string } = {};
  const createChat = (options: any) => {
    const processor = new StreamProcessor();
    const chat = {
      get messages() { return processor.getMessages(); },
      dispose: () => undefined,
      setMessages: (messages: any[]) => processor.setMessages(messages),
      async sendMessage(message: any) {
        processor.setMessages([...chat.messages, { id: 'optimistic', role: 'user', parts: message.content, metadata: message.metadata }]);
        await options.connection.send(chat.messages, undefined, undefined, { runId: 'run' });
      }
    };
    void (async () => {
      for await (const chunk of options.connection.subscribe()) {
        if (chunk.type === EventType.CUSTOM) options.onCustomEvent(chunk.name, chunk.value);
        else processor.processChunk(chunk);
      }
    })();
    return chat;
  };
  const dependencies = {
    normalizeProvider: (agent: string) => ({ agent }),
    EventType,
    createChat,
    attachmentDisplayMetadata: (attachment: unknown) => attachment,
    discardRestoredAttachments: () => undefined,
    readNewerSelectedConversationItems: async () => null,
    readOlderSelectedConversationItems: async (_id: string, cursor: number) => {
      olderCursors.push(cursor);
      return page(Math.max(1, cursor - PAGE), cursor - 1, cursor - PAGE > 1);
    },
    // The chat takes each snapshot from its stream a tick later; the read lands after it, as the backend's does.
    readSelectedConversationAttachments: async (_id: string, ids: string[]) => { await settle(); return ids.map((id) => ({ id })); },
    restoreAttachmentList: async (_id: string, saved: { id: string }[]) => saved.map(({ id }) => ({ id, name: id, previewUrl: `data:${id}` })),
    sendStructuredMessage: () => new Promise((resolve) => { admitSend = resolve; }),
    subscribeSelectedConversation: async (options: any) => {
      liveEvent = options.onEvent;
      options.onSnapshot({ page: page(21, 30, true), pendingEvents: [], pendingSequence: 30, suspended: false, ...snapshotState,
        connection: { generation: 1, provider: 'codex', state: 'connected' } });
      return () => undefined;
    },
    ...conversationMessages,
    ...store,
    extendAgentConversationImportFromTauri: async () => null,
    rail: { owned: [] },
    usageDropIsCompaction
  };
  const api = Function(...Object.keys(dependencies), `${stripTypeScriptTypes(body, { mode: 'strip' })}
    return { selectConversationChat, selectedConversationChatReady, pageSelectedConversation,
      trimSelectedConversationToLatest, sendSelectedConversationMessage, jumpSelectedConversationToLatest };
  `)(...Object.values(dependencies));
  return { api, olderCursors, admitSend: (receipt: unknown) => admitSend(receipt), liveEvent: (event: unknown) => liveEvent(event), snapshotState };
}

/** Lets the attachment reads a page started finish. */
const settle = () => new Promise((resolve) => setImmediate(resolve));
const ids = (messages: { id: string }[]) => messages.map((message) => message.id);
const range = (first: number, last: number, prefix: string) =>
  Array.from({ length: last - first + 1 }, (_, index) => `${prefix}${first + index}`);

async function openAndPageTwice(owner: string) {
  const connection = loadConnection();
  const chat = connection.api.selectConversationChat(owner);
  await connection.api.selectedConversationChatReady(owner);
  await connection.api.pageSelectedConversation('older');
  await connection.api.pageSelectedConversation('older');
  await settle();
  assert.deepEqual(ids(chat.messages), range(1, 30, 'u'), 'two older pages are loaded');
  return { ...connection, chat, session: store.getConversationSession(owner) };
}

test('returning_to_latest_drops_older_loaded_pages', async () => {
  const { api, chat, session, olderCursors } = await openAndPageTwice('chat');
  assert.deepEqual(Object.keys(session.sentAttachments).sort(), ['u12', 'u25']);

  await api.trimSelectedConversationToLatest();
  await settle();
  assert.deepEqual(ids(chat.messages), range(21, 30, 'u'), 'only the latest window remains');
  assert.equal(session.selectedBeforeCursor, 21, 'the cursor starts at the first kept message');
  assert.equal(session.selectedHasBefore, true);
  assert.deepEqual(session.selectedTurns.map((turn: { turnId: string }) => turn.turnId), range(21, 30, 't'));
  assert.deepEqual(Object.keys(session.sentAttachments), ['u25'], 'screenshots of dropped messages go');

  await api.pageSelectedConversation('older');
  assert.equal(olderCursors.at(-1), 21, 'scrolling up asks for the dropped range again');
  assert.deepEqual(ids(chat.messages), range(11, 30, 'u'));
});

test('trim_waits_while_a_send_is_pending', async () => {
  const { api, chat, admitSend } = await openAndPageTwice('sending');
  const send = api.sendSelectedConversationMessage('sending', 'Hello');
  await settle();
  await api.trimSelectedConversationToLatest();
  await settle();
  assert.deepEqual(ids(chat.messages), [...range(1, 30, 'u'), 'optimistic'], 'nothing is dropped while a send is pending');
  admitSend({ turnId: 'turn-new', userItemId: 'u31', generation: 1, admittedSequence: 31 });
  await send;
});

test('trim_keeps_live_output_queued_before_it', async () => {
  const { api, chat, liveEvent } = await openAndPageTwice('streaming');
  // A turn started elsewhere streams while the reader returns; the trim must not overwrite its reply.
  liveEvent({ ownedId: 'streaming', provider: 'codex', generation: 1, sequence: 31, timestampMs: 31,
    payload: { kind: 'assistantMessage', itemId: 'live', turnId: 't30', text: 'still streaming', completed: true } });
  await api.trimSelectedConversationToLatest();
  await settle();
  assert.deepEqual(ids(chat.messages), [...range(22, 30, 'u'), 'live'], 'the reply that arrived first is kept');
});

test('trim_waits_while_a_turn_runs', async () => {
  const { api, chat, liveEvent } = await openAndPageTwice('running');
  // A running tool may sit above the latest window; dropping it would lose its output to the next update.
  liveEvent({ ownedId: 'running', provider: 'codex', generation: 1, sequence: 31, timestampMs: 31,
    payload: { kind: 'turn', turnId: 't31', state: 'started' } });
  await api.trimSelectedConversationToLatest();
  await settle();
  assert.deepEqual(ids(chat.messages), range(1, 30, 'u'), 'nothing is dropped while a turn runs');
});

test('jump_to_latest_replaces_history_unless_a_turn_runs', async () => {
  const running = await openAndPageTwice('jump-running');
  // A running tool may sit above the latest page; the jump keeps the earlier pages until the turn ends.
  running.snapshotState.activeTurnId = 't30';
  await running.api.jumpSelectedConversationToLatest('jump-running');
  assert.deepEqual(ids(running.chat.messages), range(1, 30, 'u'), 'a running turn keeps the loaded history');

  const idle = await openAndPageTwice('jump-idle');
  await idle.api.jumpSelectedConversationToLatest('jump-idle');
  assert.deepEqual(ids(idle.chat.messages), range(21, 30, 'u'), 'otherwise the latest page replaces it');
});

test('background_command_still_running_keeps_the_history', async () => {
  // A command can outlive its turn; with no turn running, its row above the latest page must stay.
  runningToolAt = 15;
  try {
    const trim = await openAndPageTwice('background-trim');
    await trim.api.trimSelectedConversationToLatest();
    await settle();
    assert.deepEqual(ids(trim.chat.messages), range(1, 30, 'u'), 'the trim waits for the command');
    const jump = await openAndPageTwice('background-jump');
    await jump.api.jumpSelectedConversationToLatest('background-jump');
    assert.deepEqual(ids(jump.chat.messages), range(1, 30, 'u'), 'the jump keeps the earlier pages');
  } finally {
    runningToolAt = 0;
  }
});
