/** Throwaway spike: Assembly owns SQL, permissions and provider context. */
import type { ConnectionAdapter, UIMessage } from '@tanstack/ai-client';
import { EventType, uiMessagesToWire, type StreamChunk } from '@tanstack/ai/client';
import { getConversationSession } from './conversationStore.svelte';
import { observeConversationForSpike, loadConversationForRead, sendStructuredMessage, stopStructuredTurn, sendPermissionResponse } from './conversationService';

/** Only the current bounded Assembly projection is copied, never full history. */
export function spikeWindow(ownedId: string): UIMessage[] {
  const state = getConversationSession(ownedId);
  if (!state) return [];
  const messages = state.timeline.flatMap((entry): UIMessage[] => {
    if (entry.kind === 'assistant' || entry.kind === 'user') return [{
      id: entry.itemId, role: entry.kind === 'user' ? 'user' : 'assistant',
      parts: [{ type: 'text', content: entry.text }, ...(state.sentAttachments[entry.itemId] ?? []).map((attachment) => ({
        type: 'image' as const, id: attachment.id,
        source: { type: 'url' as const, value: attachment.previewUrl, mimeType: attachment.mimeType }
      }))]
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
  for (const child of state.children) {
    messages.push({ id: `child:${child.childId}`, role: 'assistant', parts: [{
      type: 'subagent', subagent: {
        id: child.childId, name: child.title, description: child.latestActivity,
        status: child.state === 'failed' ? 'error' : (child.state === 'completed' || child.state === 'finished') ? 'finished'
          : child.state === 'running' ? 'running' : 'suspended',
        messages: state.selectedChildId === child.childId ? state.childTimeline.flatMap((entry): UIMessage[] =>
          entry.kind === 'assistant' || entry.kind === 'user' ? [{
            id: entry.itemId, role: entry.kind, parts: [{ type: 'text', content: entry.text }]
          }] : []) : []
      }
    }] });
  }
  return messages;
}

export function assemblySpikeAdapter(ownedId: string) {
  let runId = '';
  let syncWindow = () => {};
  let lastSequence = getConversationSession(ownedId)?.lastSequence ?? 0;
  let generation = getConversationSession(ownedId)?.generation ?? 0;
  const stats = { events: 0, deltas: 0, resyncs: 0, maxDeliveryMs: 0 };
  const connection: ConnectionAdapter = {
    async *subscribe(signal) {
      let pending: StreamChunk[] = [];
      let wake: (() => void) | undefined;
      let resync: Promise<void> | null = null;
      const push = (chunk: StreamChunk) => { if (!signal?.aborted) { pending.push(chunk); wake?.(); } };
      const queueSnapshot = () => {
        if (signal?.aborted) return;
        const state = getConversationSession(ownedId);
        if (!state) return;
        // The snapshot already includes all applied events. Discard their queued
        // deltas and reset TanStack's accumulators in the same ordered stream.
        // A following snapshot must not swallow a queued turn completion.
        pending = pending.filter((chunk) => chunk.type === EventType.RUN_FINISHED || chunk.type === EventType.RUN_ERROR);
        lastSequence = state.lastSequence;
        generation = state.generation;
        push({ type: EventType.MESSAGES_SNAPSHOT, messages: uiMessagesToWire(spikeWindow(ownedId)) });
        if (state.activeTurnId) {
          runId ||= state.activeTurnId;
          push({ type: EventType.RUN_STARTED, threadId: ownedId, runId });
        } else if (runId && !state.sending) {
          push({ type: EventType.RUN_FINISHED, threadId: ownedId, runId });
          runId = '';
        }
      };
      syncWindow = () => { if (!resync) queueSnapshot(); };
      const recover = () => {
        if (resync || signal?.aborted) return;
        stats.resyncs++;
        pending = pending.filter((chunk) => chunk.type === EventType.RUN_FINISHED || chunk.type === EventType.RUN_ERROR);
        resync = loadConversationForRead(ownedId, false, signal).then(() => {
          queueSnapshot();
        }).finally(() => { resync = null; });
        void resync.catch((error: unknown) => push({ type: EventType.RUN_ERROR, message: String(error) }));
      };
      const unregister = observeConversationForSpike((event) => {
        if (event.ownedId !== ownedId || signal?.aborted) return;
        if (event.generation < generation || (event.generation === generation && event.sequence <= lastSequence)) return;
        if (resync) return; // SQL snapshot repairs the gap; do not append across it.
        if (event.generation > generation || event.sequence > lastSequence + 1 || pending.length > 256) {
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
        } else {
          // Tools, turn state and SQL replacement share the same reset boundary.
          queueSnapshot();
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
        syncWindow = () => {};
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
    syncWindow: () => syncWindow(),
    interrupt: () => stopStructuredTurn(ownedId),
    steer: (text: string) => sendStructuredMessage(ownedId, text),
    approve: (requestId: string, optionId: string) => sendPermissionResponse(ownedId, requestId, optionId)
  };
}
