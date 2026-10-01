/** Focused spike receipt: exercise the installed client, not a mock processor. */
import assert from 'node:assert/strict';
import { setTimeout } from 'node:timers/promises';
import { ChatClient, type ConnectionAdapter, type UIMessage } from '@tanstack/ai-client';
import { uiMessagesToWire, type StreamChunk } from '@tanstack/ai/client';

let pending: StreamChunk[] = [];
let wake: (() => void) | undefined;
let subscriptions = 0;
const connection: ConnectionAdapter = {
  async send() { throw new Error('This recovery probe must not send provider input'); },
  async *subscribe(signal) {
    subscriptions++;
    const abort = () => wake?.();
    signal?.addEventListener('abort', abort, { once: true });
    try {
      while (!signal?.aborted) {
        if (!pending.length) await new Promise<void>((resolve) => { wake = resolve; });
        wake = undefined;
        while (pending.length && !signal?.aborted) yield pending.shift()!;
      }
    } finally {
      subscriptions--;
      pending = [];
      signal?.removeEventListener('abort', abort);
    }
  }
};
const client = new ChatClient({ threadId: 'tsk1320-probe', connection });
client.subscribe();
const deliver = (...chunks: StreamChunk[]) => {
  pending.push(...chunks);
  wake?.();
};
const waitFor = async (ready: () => boolean) => {
  const deadline = performance.now() + 2000;
  while (!ready()) {
    assert.ok(performance.now() < deadline, 'Timed out waiting for client state');
    await setTimeout(1);
  }
};
const snapshot = (messages: UIMessage[]): StreamChunk => ({
  type: 'MESSAGES_SNAPSHOT', messages: uiMessagesToWire(messages)
});
const message = (id: string, content: string): UIMessage => ({
  id, role: 'assistant', parts: [{ type: 'text', content }]
});
const text = (id: string) => client.getMessages().find((m) => m.id === id)?.parts
  .filter((part) => part.type === 'text').map((part) => part.content).join('');
try {
  deliver({ type: 'RUN_STARTED', threadId: 'tsk1320-probe', runId: 'turn' },
    { type: 'TEXT_MESSAGE_CONTENT', messageId: 'reply', delta: 'before ' });
  await waitFor(() => text('reply') === 'before ' && client.getSessionGenerating());
  // The SQL window includes an update deliberately absent from the stream.
  deliver(snapshot([message('reply', 'before missing ')]),
    { type: 'TEXT_MESSAGE_CONTENT', messageId: 'reply', delta: 'after' });
  await waitFor(() => text('reply') === 'before missing after');
  assert.equal(text('reply'), 'before missing after');
  assert.equal(client.getMessages().filter((m) => m.id === 'reply').length, 1);

  const tool: UIMessage = { id: 'tool-row', role: 'assistant', parts: [{
    type: 'tool-call', id: 'shell', name: 'shell', arguments: '{}',
    state: 'complete', output: 'spike'
  }] };
  deliver(snapshot([message('old', 'trim me'), message('reply', 'before missing after'), tool]));
  await waitFor(() => text('old') === 'trim me');
  deliver(snapshot([message('reply', 'retained '), tool]),
    { type: 'TEXT_MESSAGE_CONTENT', messageId: 'reply', delta: 'tail' });
  await waitFor(() => text('reply') === 'retained tail');
  assert.equal(text('reply'), 'retained tail');
  assert.equal(client.getMessages().some((m) => m.id === 'old'), false);
  assert.ok(client.getMessages().some((m) => m.parts.some((part) =>
    part.type === 'tool-call' && part.id === 'shell' && part.output === 'spike')));

  deliver({ type: 'RUN_FINISHED', threadId: 'tsk1320-probe', runId: 'turn' });
  await waitFor(() => !client.getSessionGenerating());
  assert.equal(client.getIsLoading(), false);
  client.dispose();
  const before = JSON.stringify(client.getMessages());
  deliver({ type: 'TEXT_MESSAGE_CONTENT', messageId: 'reply', delta: 'late' });
  await waitFor(() => subscriptions === 0);
  assert.equal(JSON.stringify(client.getMessages()), before);
  assert.equal(subscriptions, 0);
  console.log('PASS: exact gap recovery, no duplicate or resurrected text, tool output retained, terminal state, disposal');
} finally {
  client.dispose();
}

// Exercise Assembly's actual queue; replace only its SQL/service boundary.
const { registerHooks } = await import('node:module');
type ProbeEvent = { ownedId: string; generation: number; sequence: number; timestampMs: number; payload: { kind: string; itemId?: string; delta?: string } };
const state = {
  generation: 1, lastSequence: 1, reachedTranscriptEnd: true, sending: true,
  activeTurnId: 'turn' as string | undefined,
  sentAttachments: {}, children: [],
  timeline: [{ kind: 'assistant', itemId: 'reply', text: 'before' }]
};
let observer: ((event: ProbeEvent) => void) | undefined;
let finishRead: (() => void) | undefined;
let sent = false;
const harness = {
  state,
  send: () => { sent = true; },
  observe: (callback: (event: ProbeEvent) => void) => { observer = callback; return () => { observer = undefined; }; },
  read: () => new Promise<void>((resolve) => { finishRead = resolve; })
};
const globals = globalThis as unknown as { spikeAdapterProbe?: typeof harness };
globals.spikeAdapterProbe = harness;
const hooks = registerHooks({
  resolve(specifier, context, nextResolve) {
    if (context.parentURL?.endsWith('/tanstackSpike.ts')) {
      const source = specifier === './conversationStore.svelte'
        ? 'export const getConversationSession = () => globalThis.spikeAdapterProbe.state;'
        : specifier === './conversationService'
          ? 'export const observeConversationForSpike = cb => globalThis.spikeAdapterProbe.observe(cb); export const loadConversationForRead = () => globalThis.spikeAdapterProbe.read(); export const sendStructuredMessage = async () => globalThis.spikeAdapterProbe.send(); export const stopStructuredTurn = async () => {}; export const sendPermissionResponse = async () => {};'
          : null;
      if (source) return { url: `data:text/javascript,${encodeURIComponent(source)}`, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  }
});
const { assemblySpikeAdapter } = await import('../src/lib/shell/conversation/tanstackSpike.ts');
const adapter = assemblySpikeAdapter('owned');
const actual = new ChatClient({ threadId: 'owned', connection: adapter.connection });
const actualText = () => actual.getMessages()[0]?.parts.filter((p) => p.type === 'text').map((p) => p.content).join('');
const emit = (sequence: number, kind: string, delta?: string) => {
  state.lastSequence = sequence;
  if (delta) state.timeline[0].text += delta;
  observer?.({ ownedId: 'owned', generation: state.generation, sequence, timestampMs: Date.now(), payload: { kind, itemId: 'reply', ...(delta ? { delta } : {}) } });
};
try {
  actual.subscribe();
  const sending = actual.sendMessage('probe');
  await waitFor(() => sent);
  adapter.syncWindow();
  await waitFor(() => actual.getSessionGenerating());
  emit(2, 'assistantDelta', ' one');
  emit(3, 'assistantMessage');
  emit(4, 'assistantDelta', ' two');
  await waitFor(() => actualText() === 'before one two');

  state.timeline[0].text += ' missing';
  emit(6, 'assistantDelta'); // Sequence 5 was lost by the adapter.
  assert.ok(finishRead);
  finishRead();
  await waitFor(() => actualText() === 'before one two missing');
  emit(7, 'assistantDelta', ' after');
  await waitFor(() => actualText() === 'before one two missing after');
  state.activeTurnId = undefined;
  state.sending = false;
  emit(8, 'turn');
  emit(9, 'connection'); // Must not discard the queued terminal event.
  await waitFor(() => !actual.getSessionGenerating() && actual.getStatus() === 'ready' && !actual.getIsLoading());
  await sending;

  emit(11, 'assistantDelta'); // Dispose while the SQL read is pending.
  actual.dispose();
  const before = JSON.stringify(actual.getMessages());
  finishRead!();
  await waitFor(() => observer === undefined);
  adapter.syncWindow();
  assert.equal(JSON.stringify(actual.getMessages()), before);
  console.log('PASS: actual adapter snapshot/delta ordering, sequence-gap read, terminal coalescing, teardown during SQL read');
} finally {
  actual.dispose();
  hooks.deregister();
  delete globals.spikeAdapterProbe;
}
