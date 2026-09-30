/** Throwaway spike: Assembly owns SQL, permissions and provider context. */
import type { ConnectionAdapter, UIMessage } from '@tanstack/ai-client';
import { EventType, type StreamChunk } from '@tanstack/ai/client';
import { getConversationSession } from './conversationStore.svelte';
import { observeConversationForSpike, loadConversationForRead, sendStructuredMessage, stopStructuredTurn, sendPermissionResponse } from './conversationService';

/** Only the current bounded Assembly projection is copied, never full history. */
export function spikeWindow(ownedId: string): UIMessage[] {
  const state = getConversationSession(ownedId);
  if (!state) return [];
  return state.timeline.flatMap((entry): UIMessage[] => {
    if (entry.kind === 'assistant' || entry.kind === 'user') return [{
      id: entry.itemId, role: entry.kind === 'user' ? 'user' : 'assistant',
      parts: [{ type: 'text', content: entry.text }]
    }];
    if (entry.kind === 'tool') return [{
      id: entry.itemId, role: 'assistant', parts: [{
        type: 'tool-call', id: entry.itemId, name: entry.name,
        arguments: JSON.stringify({ summary: entry.summary ?? '' }),
        state: entry.state === 'completed' ? 'complete' : entry.state === 'failed' ? 'error' : 'input-complete',
        output: entry.output ?? entry.summary ?? ''
      }]
    }];
    return [];
  });
}

export function assemblySpikeAdapter(ownedId: string, restore: () => void) {
  let runId = '';
  let lastSequence = getConversationSession(ownedId)?.lastSequence ?? 0;
  let generation = getConversationSession(ownedId)?.generation ?? 0;
  const stats = { events: 0, deltas: 0, resyncs: 0, maxDeliveryMs: 0 };
  const connection: ConnectionAdapter = {
    async *subscribe(signal) {
      let pending: StreamChunk[] = [];
      let wake: (() => void) | undefined;
      let resync: Promise<void> | null = null;
      const push = (chunk: StreamChunk) => { pending.push(chunk); wake?.(); };
      const recover = () => {
        if (resync || signal?.aborted) return;
        stats.resyncs++;
        resync = loadConversationForRead(ownedId, false, signal).then(() => {
          pending = [];
          lastSequence = getConversationSession(ownedId)?.lastSequence ?? lastSequence;
          generation = getConversationSession(ownedId)?.generation ?? generation;
          restore();
        }).finally(() => { resync = null; });
        void resync.catch((error: unknown) => push({ type: EventType.RUN_ERROR, message: String(error) }));
      };
      const unregister = observeConversationForSpike((event) => {
        if (event.ownedId !== ownedId || signal?.aborted) return;
        if (event.generation < generation || (event.generation === generation && event.sequence <= lastSequence)) return;
        if (resync) return; // SQL snapshot repairs the gap; do not append across it.
        if ((event.generation === generation && event.sequence > lastSequence + 1) || pending.length > 256) {
          recover(); return;
        }
        generation = event.generation;
        lastSequence = event.sequence;
        stats.events++;
        stats.maxDeliveryMs = Math.max(stats.maxDeliveryMs, Date.now() - event.timestampMs);
        const state = getConversationSession(ownedId);
        if (!state?.reachedTranscriptEnd) return;
        const payload = event.payload;
        if (payload.kind === 'assistantDelta') {
          stats.deltas++;
          push({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: payload.itemId, delta: payload.delta });
        } else if (payload.kind === 'turn') {
          if (payload.state === 'started') {
            runId ||= payload.turnId;
            push({ type: EventType.RUN_STARTED, threadId: ownedId, runId });
          } else {
            push({ type: EventType.RUN_FINISHED, threadId: ownedId, runId: runId || payload.turnId });
            runId = '';
          }
          push({ type: 'CUSTOM', name: 'assembly-window', value: null });
        } else {
          // Retain Assembly's normalization for tools, rich ACP parts and snapshots.
          push({ type: 'CUSTOM', name: 'assembly-window', value: null });
        }
      }, recover);
      const abort = () => wake?.();
      signal?.addEventListener('abort', abort, { once: true });
      try {
        while (!signal?.aborted) {
          if (!pending.length) await new Promise<void>((resolve) => { wake = resolve; });
          wake = undefined;
          while (pending.length && !signal?.aborted) yield pending.shift()!;
        }
      } finally {
        signal?.removeEventListener('abort', abort);
        pending = [];
        unregister();
      }
    },
    async send(messages, _data, _signal, context) {
      const last = messages[messages.length - 1];
      if (!last || !('parts' in last)) throw new Error('Spike expects a UI message');
      const text = last.parts.filter((part) => part.type === 'text').map((part) => part.content).join('');
      runId = context?.runId ?? crypto.randomUUID();
      // Send ONLY the new input; provider-native context stays in Rust/provider.
      await sendStructuredMessage(ownedId, text);
    }
  };
  return {
    connection, stats,
    interrupt: () => stopStructuredTurn(ownedId),
    steer: (text: string) => sendStructuredMessage(ownedId, text),
    approve: (requestId: string, optionId: string) => sendPermissionResponse(ownedId, requestId, optionId)
  };
}
