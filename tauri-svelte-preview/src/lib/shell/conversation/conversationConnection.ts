import { EventType, type StreamChunk } from '@tanstack/ai/client';
import {
  createChat,
  type CreateChatReturn,
  type SubscribeConnectionAdapter,
  type UIMessage
} from '@tanstack/ai-svelte';
import {
  discardRestoredAttachments,
  readNewerSelectedConversationItems,
  readOlderSelectedConversationItems,
  readSelectedConversationAttachments,
  restoreAttachmentList,
  sendStructuredMessage,
  subscribeSelectedConversation
} from './conversationService.ts';
import {
  conversationChunksFromEvent,
  conversationMessagesAfterCustom,
  conversationMessagesFromEvents,
  conversationSnapshotChunks,
  displayEventFrom
} from './conversationMessages.ts';
import {
  applySelectedConversationEventState,
  applySelectedConversationHistoryEventState,
  applySelectedConversationLiveWindow,
  applySelectedConversationPageState,
  applySelectedConversationSnapshotState,
  getConversationSession,
  ensureConversationSession,
  evictConversationSession,
  failChildConversationTranscript,
  restoreSelectedConversationAttachments,
  setConversationAttachmentError,
  setConversationSendError,
  setSelectedConversationPageLoading
} from './conversationStore.svelte.ts';
import type {
  AgentConversationEvent,
  AgentConversationItemPage,
  AgentConversationSendReceipt,
  AgentConversationSelectionSnapshot,
  ConversationAttachment
} from './conversationTypes.ts';
import { extendAgentConversationImportFromTauri } from '../../tauriSource.ts';
import { usageDropIsCompaction } from './conversationReducer.ts';

const PAGE_BYTES = 512 * 1024;
const GRAPH_BYTES = 4 * 1024 * 1024;
const GRAPH_ITEMS = 256;
const encoder = new TextEncoder();

type Direction = 'older' | 'newer';
type QueueItem = StreamChunk | { nativeEvent: AgentConversationEvent } | { error: unknown };

interface PendingSend {
  runId: string;
  prompt: string;
  optimisticId: string | null;
  receipt?: AgentConversationSendReceipt;
  events: AgentConversationEvent[];
  bytes: number;
  overflow: boolean;
}

interface PendingAdmission {
  resolve: (receipt: AgentConversationSendReceipt) => void;
  reject: (error: unknown) => void;
}

interface ActiveConversation {
  workspaceOwnedId: string;
  historyOwnedId: string;
  controller: AbortController;
  chat: CreateChatReturn;
  beforeCursor?: number;
  afterCursor?: number;
  pendingSend: PendingSend | null;
  pendingAdmission: PendingAdmission | null;
  messageBytes: Map<string, number>;
  graphBytes: number;
  contentAdmitted: boolean;
  anchorRevision: number;
  anchorAdmission: Promise<void> | null;
  anchorSnapshot: AgentConversationSelectionSnapshot | null;
  resubscribe?: () => Promise<void>;
  push?: (item: QueueItem) => void;
  ready: Promise<void>;
  resolveReady?: () => void;
  rejectReady?: (error: unknown) => void;
  releaseSelectionSignal?: () => void;
}

let active: ActiveConversation | null = null;
let childActive: ActiveConversation | null = null;

function isCurrent(selection: ActiveConversation): boolean {
  return !selection.controller.signal.aborted && (active === selection || childActive === selection);
}

function messageBytes(message: UIMessage): number {
  return encoder.encode(JSON.stringify(message)).byteLength;
}

function boundedMessages(messages: UIMessage[], direction: Direction): UIMessage[] {
  let bytes = messages.reduce((total, message) => total + messageBytes(message), 0);
  while (messages.length > 1 && (bytes > GRAPH_BYTES || messages.length > GRAPH_ITEMS)) {
    const removed = direction === 'older' ? messages.pop() : messages.shift();
    if (removed) bytes -= messageBytes(removed);
  }
  return messages;
}

function resetMessageBytes(selection: ActiveConversation, messages: readonly UIMessage[]): void {
  selection.messageBytes = new Map(messages.map((message) => [message.id, messageBytes(message)]));
  selection.graphBytes = [...selection.messageBytes.values()].reduce((total, bytes) => total + bytes, 0);
}

function pushMessagesSnapshot(
  selection: ActiveConversation,
  messages: readonly UIMessage[],
  push: (item: QueueItem) => void
): void {
  resetMessageBytes(selection, messages);
  for (const chunk of conversationSnapshotChunks(messages)) push(chunk);
}

function messagesFromPage(page: AgentConversationItemPage): UIMessage[] {
  const descriptors = new Map(page.items.map((item) => [item.itemId, item]));
  return conversationMessagesFromEvents(page.events).map((message) => {
    const item = descriptors.get(message.id);
    return item ? {
      ...message,
      metadata: {
        ...message.metadata,
        historyStableId: item.stableId,
        firstSequence: item.firstSequence,
        lastSequence: item.lastSequence,
        positionKnown: item.positionKnown,
        prefixComplete: item.prefixComplete
      }
    } : message;
  });
}

function historyPosition(message: UIMessage | undefined): number | undefined {
  const value = message?.metadata?.firstSequence;
  return typeof value === 'number' ? value : undefined;
}

function hasOlderHistory(page: AgentConversationItemPage): boolean {
  return page.hasBefore
    || page.hasEarlierTranscript
    || !!(page.coverage && !page.coverage.startComplete);
}

function mergeMessages(
  current: readonly UIMessage[],
  incoming: readonly UIMessage[],
  direction: Direction
): UIMessage[] {
  const preferred = direction === 'older' ? current : incoming;
  const byId = new Map([...incoming, ...current].map((message) => [message.id, message]));
  for (const message of preferred) byId.set(message.id, message);
  const order = direction === 'older' ? [...incoming, ...current] : [...current, ...incoming];
  const seen = new Set<string>();
  return boundedMessages(order.flatMap((message) => {
    if (seen.has(message.id)) return [];
    seen.add(message.id);
    return [byId.get(message.id)!];
  }), direction);
}

function boundLiveMessages(
  selection: ActiveConversation,
  changedIds: ReadonlySet<string>,
  replaced: boolean
): StreamChunk[] {
  let messages = selection.chat.messages;
  if (replaced) resetMessageBytes(selection, messages);
  else for (const id of changedIds) {
    const previous = selection.messageBytes.get(id) ?? 0;
    const message = messages.find((candidate) => candidate.id === id);
    const next = message ? messageBytes(message) : 0;
    selection.graphBytes += next - previous;
    if (message) selection.messageBytes.set(id, next);
    else selection.messageBytes.delete(id);
  }
  let evictedOldest = false;
  while (messages.length > 1 && (selection.graphBytes > GRAPH_BYTES || messages.length > GRAPH_ITEMS)) {
    const [removed, ...retained] = messages;
    selection.graphBytes -= selection.messageBytes.get(removed.id) ?? 0;
    selection.messageBytes.delete(removed.id);
    messages = retained;
    evictedOldest = true;
  }
  const replacement = evictedOldest ? conversationSnapshotChunks(messages) : [];
  if (evictedOldest) {
    resetMessageBytes(selection, messages);
    const generation = getConversationSession(selection.historyOwnedId)?.generation ?? 0;
    restoreSelectedConversationAttachments(
      selection.historyOwnedId, {}, generation, messages.map((message) => message.id)
    );
  }
  selection.beforeCursor = historyPosition(messages[0]) ?? selection.beforeCursor;
  applySelectedConversationLiveWindow(
    selection.historyOwnedId,
    messages.flatMap((message) => typeof message.metadata?.turnId === 'string'
      ? [message.metadata.turnId] : []),
    selection.graphBytes,
    evictedOldest
  );
  return replacement;
}

function isTerminal(event: AgentConversationEvent, turnId: string): boolean {
  const payload = displayEventFrom(event).payload as { kind?: unknown; state?: unknown; turnId?: unknown };
  if (payload.turnId !== turnId) return false;
  return payload.kind === 'error'
    || (payload.kind === 'turn' && payload.state !== 'started');
}

function optimisticUserId(messages: readonly UIMessage[]): string | null {
  return messages.findLast((message) => message.role === 'user')?.id ?? null;
}

function enqueueFactory(signal: AbortSignal, selection: ActiveConversation): {
  push(item: QueueItem): void;
  stream(): AsyncIterable<StreamChunk>;
} {
  const items: QueueItem[] = [];
  let wake: (() => void) | null = null;
  const abort = (): void => {
    wake?.();
    wake = null;
  };
  const push = (item: QueueItem): void => {
    if (signal.aborted) return;
    items.push(item);
    wake?.();
    wake = null;
  };
  return {
    push,
    async *stream() {
      signal.addEventListener('abort', abort, { once: true });
      try {
        while (!signal.aborted) {
          const item = items.shift();
          if (item) {
            if ('error' in item) {
              selection.pendingSend = null;
              throw item.error;
            } else if ('nativeEvent' in item) {
              const chunks = conversationChunksFromEvent(
                selection.chat.messages,
                item.nativeEvent
              );
              const changedIds = new Set<string>();
              let replaced = false;
              for (const chunk of chunks) {
                if (chunk.type === EventType.MESSAGES_SNAPSHOT) replaced = true;
                else if ('messageId' in chunk && typeof chunk.messageId === 'string') changedIds.add(chunk.messageId);
                else if (chunk.type === EventType.CUSTOM) {
                  const itemId = (chunk.value as { itemId?: unknown })?.itemId;
                  if (typeof itemId === 'string') changedIds.add(itemId);
                }
                yield chunk;
              }
              for (const replacement of boundLiveMessages(selection, changedIds, replaced)) {
                yield replacement;
              }
            } else {
              yield item;
            }
            continue;
          }
          await new Promise<void>((resolve) => { wake = resolve; });
        }
      } finally {
        signal.removeEventListener('abort', abort);
        items.length = 0;
        wake = null;
      }
    }
  };
}

function resetReady(selection: ActiveConversation): void {
  selection.ready = new Promise<void>((resolve, reject) => {
    selection.resolveReady = resolve;
    selection.rejectReady = reject;
  });
  void selection.ready.catch(() => {});
}

function bindSelectionSignal(selection: ActiveConversation, signal?: AbortSignal): void {
  selection.releaseSelectionSignal?.();
  const abort = (): void => {
    if (isCurrent(selection)) {
      if (selection === childActive) disposeChildConversationChat(selection.workspaceOwnedId);
      else disposeSelectedConversationChat(selection.workspaceOwnedId);
    }
  };
  if (signal?.aborted) abort();
  else signal?.addEventListener('abort', abort, { once: true });
  selection.releaseSelectionSignal = () => signal?.removeEventListener('abort', abort);
}

async function restorePageAttachments(
  selection: ActiveConversation,
  wanted: ReadonlyMap<string, readonly string[]>,
  generation: number,
  retainedItemIds: readonly string[]
): Promise<void> {
  try {
    restoreSelectedConversationAttachments(
      selection.historyOwnedId, {}, generation, retainedItemIds
    );
    if (!wanted.size) return;
    const byItem: Record<string, ConversationAttachment[]> = {};
    if (wanted.size) {
      const ids = [...new Set([...wanted.values()].flat())];
      const saved = await readSelectedConversationAttachments(
        selection.historyOwnedId,
        ids,
        selection.controller.signal
      );
      if (!isCurrent(selection) || selection.controller.signal.aborted) return;
      const restored = await restoreAttachmentList(
        selection.historyOwnedId,
        saved,
        selection.controller.signal
      );
      if (!isCurrent(selection) || selection.controller.signal.aborted) {
        discardRestoredAttachments(restored);
        return;
      }
      const byId = new Map(restored.map((attachment) => [attachment.id, attachment]));
      for (const [itemId, attachmentIds] of wanted) {
        byItem[itemId] = attachmentIds.flatMap((id) => byId.get(id) ?? []);
      }
    }
    if (isCurrent(selection)) {
      restoreSelectedConversationAttachments(
        selection.historyOwnedId,
        byItem,
        generation,
        selection.chat.messages.map((message) => message.id)
      );
    }
  } catch (error) {
    if (isCurrent(selection) && !selection.controller.signal.aborted) {
      setConversationAttachmentError(
        selection.historyOwnedId,
        error instanceof Error ? error.message : String(error)
      );
    }
  }
}

function pageAttachments(events: readonly AgentConversationEvent[]): Map<string, readonly string[]> {
  const wanted = new Map<string, readonly string[]>();
  for (const event of events) {
    if (event.payload.kind === 'userMessage' && event.payload.attachmentIds?.length) {
      wanted.set(event.payload.itemId, event.payload.attachmentIds);
    }
  }
  return wanted;
}

function snapshotChunks(
  selection: ActiveConversation,
  snapshot: AgentConversationSelectionSnapshot,
  push: (item: QueueItem) => void,
  applyControls = selection.historyOwnedId === selection.workspaceOwnedId
): void {
  selection.beforeCursor = snapshot.page.beforeCursor;
  if (selection.beforeCursor === undefined && hasOlderHistory(snapshot.page)) {
    selection.beforeCursor = snapshot.page.watermark + 1;
  }
  selection.afterCursor = snapshot.page.afterCursor;
  const pageMessages = messagesFromPage(snapshot.page);
  let messages = boundedMessages([...pageMessages], 'newer');
  const pending = selection.pendingSend;
  if (pending && !pending.receipt && pending.optimisticId) {
    const optimistic = selection.chat.messages.find((message) => message.id === pending.optimisticId);
    if (optimistic && !messages.some((message) => message.id === optimistic.id)) {
      messages = boundedMessages([...messages, optimistic], 'newer');
    }
  }
  selection.beforeCursor = historyPosition(messages.find((message) => historyPosition(message) !== undefined)) ?? selection.beforeCursor;
  selection.afterCursor = historyPosition(messages.findLast((message) => historyPosition(message) !== undefined)) ?? selection.afterCursor;
  const retainedIds = new Set(messages.map((message) => message.id));
  const evicted = pageMessages.some((message) => !retainedIds.has(message.id));
  resetMessageBytes(selection, messages);
  applySelectedConversationSnapshotState(
    selection.historyOwnedId,
    selection.historyOwnedId,
    snapshot,
    applyControls,
    {
      beforeCursor: selection.beforeCursor,
      afterCursor: selection.afterCursor,
      hasBefore: evicted || hasOlderHistory(snapshot.page),
      hasAfter: snapshot.page.hasAfter,
      retainedTurnIds: messages.flatMap((message) =>
        typeof message.metadata?.turnId === 'string' ? [message.metadata.turnId] : []),
      transferBytes: selection.graphBytes
    }
  );
  if (applyControls) {
    const snapshotError = snapshot.pendingEvents.findLast((event) => event.payload.kind === 'error');
    if (snapshotError?.payload.kind === 'error') {
      setConversationSendError(selection.workspaceOwnedId, snapshotError.payload.message);
    }
  }
  pushMessagesSnapshot(selection, messages, push);
  push({ type: EventType.CUSTOM, name: 'assembly:snapshot-ready', value: selection.historyOwnedId } as StreamChunk);
  void restorePageAttachments(
    selection,
    pageAttachments(snapshot.page.events),
    snapshot.connection.generation,
    messages.map((message) => message.id)
  );
  if (pending?.receipt && snapshot.page.turns.some((turn) =>
    turn.turnId === pending.receipt?.turnId && turn.terminalState
  )) {
    push({
      type: EventType.RUN_FINISHED,
      threadId: selection.historyOwnedId,
      runId: pending.runId
    } as StreamChunk);
    selection.pendingSend = null;
  }
}

function admitInitialSnapshot(
  selection: ActiveConversation,
  snapshot: AgentConversationSelectionSnapshot,
  push: (item: QueueItem) => void
): void {
  if (selection.contentAdmitted) {
    snapshotChunks(selection, snapshot, push);
    return;
  }
  const view = getConversationSession(selection.workspaceOwnedId)
    ?.viewByHistoryId[selection.historyOwnedId];
  const anchor = view?.followLatest === false ? view.anchor : undefined;
  if (!selection.anchorAdmission && !anchor) {
    selection.contentAdmitted = true;
    snapshotChunks(selection, snapshot, push);
    return;
  }
  selection.anchorSnapshot = snapshot;
  applySelectedConversationSnapshotState(
    selection.historyOwnedId,
    selection.historyOwnedId,
    snapshot,
    selection.historyOwnedId === selection.workspaceOwnedId
  );
  const snapshotError = snapshot.pendingEvents.findLast((event) => event.payload.kind === 'error');
  if (snapshotError?.payload.kind === 'error') {
    const childId = getConversationSession(selection.workspaceOwnedId)?.selectedChildId;
    if (childId && selection.historyOwnedId !== selection.workspaceOwnedId) {
      failChildConversationTranscript(selection.workspaceOwnedId, childId, snapshotError.payload.message);
    } else {
      setConversationSendError(selection.workspaceOwnedId, snapshotError.payload.message);
    }
  }
  if (selection.anchorAdmission) return;
  const revision = ++selection.anchorRevision;
  const admission = (async () => {
    const page = await readNewerSelectedConversationItems(
      selection.historyOwnedId,
      anchor!.firstSequence - 1,
      PAGE_BYTES,
      selection.controller.signal
    );
    if (selection.controller.signal.aborted
      || !isCurrent(selection)
      || revision !== selection.anchorRevision) return;
    const authority = selection.anchorSnapshot ?? snapshot;
    selection.anchorAdmission = null;
    selection.anchorSnapshot = null;
    selection.contentAdmitted = true;
    if (page && !page.hasAfter) {
      await selection.resubscribe?.();
      return;
    }
    snapshotChunks(selection, page ? { ...authority, page } : authority, push, false);
  })();
  selection.anchorAdmission = admission;
  void admission.catch((error) => {
    if (!isCurrent(selection)
      || selection.controller.signal.aborted
      || revision !== selection.anchorRevision) return;
    selection.anchorAdmission = null;
    selection.anchorSnapshot = null;
    selection.rejectReady?.(error);
    push({ error });
  });
}

function emitEvent(
  selection: ActiveConversation,
  event: AgentConversationEvent,
  push: (item: QueueItem) => void,
  updateState = true
): void {
  const previousUsedTokens = getConversationSession(selection.historyOwnedId)?.usage?.usedTokens;
  if (updateState) {
    if (selection.historyOwnedId === selection.workspaceOwnedId) {
      applySelectedConversationEventState(selection.workspaceOwnedId, event);
    } else {
      applySelectedConversationHistoryEventState(selection.historyOwnedId, event);
    }
  }
  if (event.payload.kind === 'error') {
    const childId = getConversationSession(selection.workspaceOwnedId)?.selectedChildId;
    if (childId && selection.historyOwnedId !== selection.workspaceOwnedId) {
      failChildConversationTranscript(selection.workspaceOwnedId, childId, event.payload.message);
    } else {
      setConversationSendError(selection.workspaceOwnedId, event.payload.message);
    }
  }
  const showingLatest = !getConversationSession(selection.historyOwnedId)?.selectedHasAfter;
  const displayEvents = event.payload.kind === 'usage'
    && usageDropIsCompaction(previousUsedTokens, event.payload.usedTokens)
    ? [{
        ...event,
        payload: {
          kind: 'contextCompaction' as const,
          preTokens: previousUsedTokens,
          postTokens: event.payload.usedTokens
        }
      }, event]
    : [event];
  const pending = selection.pendingSend;
  if (pending && !pending.receipt) {
    const bytes = encoder.encode(JSON.stringify(displayEvents)).byteLength;
    if (!pending.overflow && pending.bytes + bytes <= PAGE_BYTES) {
      pending.events.push(...displayEvents);
      pending.bytes += bytes;
    } else {
      pending.events.length = 0;
      pending.bytes = 0;
      pending.overflow = true;
    }
    return;
  }
  if (pending?.receipt && isTerminal(event, pending.receipt.turnId)) {
    push({
      type: EventType.RUN_FINISHED,
      threadId: selection.historyOwnedId,
      runId: pending.runId
    } as StreamChunk);
    selection.pendingSend = null;
  }
  if (!selection.contentAdmitted || !showingLatest) return;
  for (const displayEvent of displayEvents) push({ nativeEvent: displayEvent });
}

function rejectPendingAdmission(selection: ActiveConversation, error: unknown): void {
  const admission = selection.pendingAdmission;
  if (!admission) return;
  selection.pendingAdmission = null;
  admission.reject(error);
}

function createConnection(selection: ActiveConversation): SubscribeConnectionAdapter {
  return {
    subscribe(signal) {
      const joined = new AbortController();
      const abort = (): void => joined.abort();
      signal?.addEventListener('abort', abort, { once: true });
      selection.controller.signal.addEventListener('abort', abort, { once: true });
      const queue = enqueueFactory(joined.signal, selection);
      return (async function* () {
        let dispose = (): void => {};
        let opening = Promise.resolve();
        const open = (): Promise<void> => {
          opening = opening.then(async () => {
            dispose();
            dispose = () => {};
            const next = await subscribeSelectedConversation({
              workspaceOwnedId: selection.workspaceOwnedId,
              ownedId: selection.historyOwnedId,
              maxBytes: PAGE_BYTES,
              signal: joined.signal,
              onSnapshot: (snapshot) => admitInitialSnapshot(selection, snapshot, queue.push),
              onEvent: (event) => emitEvent(selection, event, queue.push),
              onError: (error) => queue.push({ error })
            });
            if (joined.signal.aborted) next();
            else dispose = next;
          });
          return opening;
        };
        selection.push = queue.push;
        selection.resubscribe = open;
        try {
          await open();
          yield* queue.stream();
        } catch (error) {
          selection.rejectReady?.(error);
          throw error;
        } finally {
          dispose();
          if (selection.resubscribe === open) selection.resubscribe = undefined;
          if (selection.push === queue.push) selection.push = undefined;
          signal?.removeEventListener('abort', abort);
          selection.controller.signal.removeEventListener('abort', abort);
        }
      })();
    },
    async send(messages, _data, signal, runContext) {
      const user = [...messages].reverse().find((message) =>
        message.role === 'user' && 'parts' in message
      );
      const prompt = user && 'parts' in user
        ? user.parts.map((part: UIMessage['parts'][number]) => part.type === 'text' ? part.content : '').join('')
        : '';
      const pending: PendingSend = {
        runId: runContext?.runId ?? crypto.randomUUID(),
        prompt,
        optimisticId: optimisticUserId(messages as UIMessage[]),
        events: [],
        bytes: 0,
        overflow: false
      };
      selection.pendingSend = pending;
      try {
        if (getConversationSession(selection.historyOwnedId)?.selectedHasAfter) {
          if (!selection.resolveReady) resetReady(selection);
          await selection.resubscribe?.();
          await selection.ready;
        }
        const receipt = await sendStructuredMessage(selection.workspaceOwnedId, prompt);
        if (!receipt) throw new Error('The native conversation did not admit the message.');
        pending.receipt = receipt;
        const admission = selection.pendingAdmission;
        if (admission) {
          selection.pendingAdmission = null;
          admission.resolve(receipt);
        }
        if (signal?.aborted || !isCurrent(selection)) return;
        if (pending.optimisticId && selection.push) {
          pushMessagesSnapshot(selection, selection.chat.messages.map((message) =>
            message.id === pending.optimisticId ? { ...message, id: receipt.userItemId } : message
          ), selection.push);
        }
        if (pending.overflow) {
          await selection.resubscribe?.();
          return;
        }
        const buffered = pending.events.sort((left, right) => left.sequence - right.sequence);
        pending.events = [];
        pending.bytes = 0;
        for (const event of buffered) {
          if (selection.push) emitEvent(selection, event, selection.push, false);
        }
      } catch (error) {
        rejectPendingAdmission(selection, error);
        throw error;
      } finally {
        if (selection.pendingSend === pending && !pending.receipt) selection.pendingSend = null;
      }
    }
  };
}

export function selectConversationChat(
  workspaceOwnedId: string,
  historyOwnedId = workspaceOwnedId,
  signal?: AbortSignal
): CreateChatReturn {
  const child = historyOwnedId !== workspaceOwnedId;
  const current = child ? childActive : active;
  if (current?.workspaceOwnedId === workspaceOwnedId && current.historyOwnedId === historyOwnedId) {
    bindSelectionSignal(current, signal);
    return current.chat;
  }
  if (child) {
    disposeChildConversationChat();
    const parent = getConversationSession(workspaceOwnedId);
    if (!parent) throw new Error('The parent conversation is not selected.');
    ensureConversationSession(historyOwnedId, parent.provider);
    parent.selectedChildHistoryOwnedId = historyOwnedId;
  } else {
    disposeSelectedConversationChat();
  }
  let resolveReady: (() => void) | undefined;
  let rejectReady: ((error: unknown) => void) | undefined;
  const ready = new Promise<void>((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });
  void ready.catch(() => {});
  const selection = {
    workspaceOwnedId,
    historyOwnedId,
    controller: new AbortController(),
    pendingSend: null,
    pendingAdmission: null,
    messageBytes: new Map(),
    graphBytes: 0,
    contentAdmitted: false,
    anchorRevision: 0,
    anchorAdmission: null,
    anchorSnapshot: null,
    ready,
    resolveReady,
    rejectReady
  } as ActiveConversation;
  const connection = createConnection(selection);
  selection.chat = createChat({
    connection,
    live: true,
    queue: 'drop',
    threadId: historyOwnedId,
    onCustomEvent(name, value) {
      if (selection.controller.signal.aborted) return;
      if (name === 'assembly:running-tool' || name === 'assembly:stale-markdown'
        || name === 'assembly:message-metadata') {
        selection.chat.setMessages(conversationMessagesAfterCustom(
          selection.chat.messages, name, value
        ));
      } else if (name === 'assembly:snapshot-ready' && value === selection.historyOwnedId) {
        selection.resolveReady?.();
        selection.resolveReady = undefined;
        selection.rejectReady = undefined;
      }
    },
    onError(error) {
      if (!selection.controller.signal.aborted) {
        const childId = getConversationSession(selection.workspaceOwnedId)?.selectedChildId;
        if (childId && selection.historyOwnedId !== selection.workspaceOwnedId) {
          failChildConversationTranscript(selection.workspaceOwnedId, childId, error.message);
        } else {
          setConversationSendError(selection.workspaceOwnedId, error.message);
        }
      }
    }
  });
  if (child) childActive = selection;
  else active = selection;
  bindSelectionSignal(selection, signal);
  return selection.chat;
}

export function selectedConversationChat(
  workspaceOwnedId?: string | null,
  historyOwnedId = workspaceOwnedId
): CreateChatReturn | null {
  const selection = historyOwnedId && historyOwnedId !== workspaceOwnedId ? childActive : active;
  return selection && (!workspaceOwnedId || selection.workspaceOwnedId === workspaceOwnedId)
    && (!historyOwnedId || selection.historyOwnedId === historyOwnedId) ? selection.chat : null;
}

function disposeConversationChat(selection: ActiveConversation): void {
  const generation = getConversationSession(selection.historyOwnedId)?.generation ?? 0;
  restoreSelectedConversationAttachments(selection.historyOwnedId, {}, generation, []);
  selection.controller.abort();
  selection.rejectReady?.(new DOMException('Conversation selection disposed', 'AbortError'));
  selection.releaseSelectionSignal?.();
  rejectPendingAdmission(selection, new DOMException('Conversation selection disposed', 'AbortError'));
  selection.chat.dispose();
}

export function disposeChildConversationChat(workspaceOwnedId?: string): void {
  const selection = childActive;
  if (!selection || (workspaceOwnedId && selection.workspaceOwnedId !== workspaceOwnedId)) return;
  childActive = null;
  const parent = getConversationSession(selection.workspaceOwnedId);
  if (parent?.selectedChildHistoryOwnedId === selection.historyOwnedId) parent.selectedChildHistoryOwnedId = null;
  disposeConversationChat(selection);
  evictConversationSession(selection.historyOwnedId);
}

export function disposeSelectedConversationChat(workspaceOwnedId?: string): void {
  disposeChildConversationChat(workspaceOwnedId);
  const selection = active;
  if (!selection || (workspaceOwnedId && selection.workspaceOwnedId !== workspaceOwnedId)) return;
  active = null;
  disposeConversationChat(selection);
}

export async function selectedConversationChatReady(
  workspaceOwnedId: string,
  historyOwnedId = workspaceOwnedId
): Promise<void> {
  const selection = historyOwnedId === workspaceOwnedId ? active : childActive;
  if (!selection || selection.workspaceOwnedId !== workspaceOwnedId || selection.historyOwnedId !== historyOwnedId) return;
  await selection.ready;
}

export function sendSelectedConversationMessage(
  workspaceOwnedId: string,
  text: string
): Promise<AgentConversationSendReceipt> {
  const selection = active;
  if (!selection || selection.workspaceOwnedId !== workspaceOwnedId) {
    return Promise.reject(new Error('The conversation is not ready to send.'));
  }
  if (selection.pendingAdmission) {
    return Promise.reject(new Error('A message is already being admitted.'));
  }
  let resolve!: (receipt: AgentConversationSendReceipt) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<AgentConversationSendReceipt>((accept, decline) => {
    resolve = accept;
    reject = decline;
  });
  const admission = { resolve, reject };
  selection.pendingAdmission = admission;
  void selection.chat.sendMessage({ content: [{ type: 'text', content: text }] })
    .catch((error) => rejectPendingAdmission(selection, error))
    .finally(() => {
      if (selection.pendingAdmission === admission) {
        rejectPendingAdmission(selection, new Error('The message was not admitted.'));
      }
    });
  return promise;
}

export async function refreshSelectedConversationChat(workspaceOwnedId: string, historyOwnedId = workspaceOwnedId): Promise<void> {
  const selection = historyOwnedId === workspaceOwnedId ? active : childActive;
  if (!selection || selection.workspaceOwnedId !== workspaceOwnedId || selection.historyOwnedId !== historyOwnedId || !selection.resubscribe) return;
  if (!selection.resolveReady) resetReady(selection);
  await selection.resubscribe();
  await selection.ready;
}

export function jumpSelectedConversationToLatest(workspaceOwnedId: string, historyOwnedId = workspaceOwnedId): Promise<void> {
  const selection = historyOwnedId === workspaceOwnedId ? active : childActive;
  if (selection?.workspaceOwnedId === workspaceOwnedId && selection.historyOwnedId === historyOwnedId) {
    selection.anchorRevision += 1;
    selection.anchorAdmission = null;
    selection.anchorSnapshot = null;
    selection.contentAdmitted = true;
  }
  return refreshSelectedConversationChat(workspaceOwnedId, historyOwnedId);
}

export async function pageSelectedConversation(direction: Direction, historyOwnedId?: string): Promise<void> {
  const selection = historyOwnedId && historyOwnedId !== active?.historyOwnedId ? childActive : active;
  if (!selection || (historyOwnedId && selection.historyOwnedId !== historyOwnedId)) return;
  if (!setSelectedConversationPageLoading(selection.historyOwnedId, direction, true)) return;
  try {
    const cursor = direction === 'older'
      ? selection.beforeCursor
      : selection.afterCursor;
    if (cursor === undefined) return;
    let page = direction === 'older'
      ? await readOlderSelectedConversationItems(
          selection.historyOwnedId, cursor, PAGE_BYTES, selection.controller.signal
        )
      : await readNewerSelectedConversationItems(
          selection.historyOwnedId, cursor, PAGE_BYTES, selection.controller.signal
        );
    if (!page || !isCurrent(selection)) return;
    if (direction === 'newer' && !page.hasAfter) {
      if (!selection.resolveReady) resetReady(selection);
      await selection.resubscribe?.();
      await selection.ready;
      return;
    }
    if (direction === 'older' && !page.hasBefore && page.hasEarlierTranscript) {
      const extended = await extendAgentConversationImportFromTauri(
        selection.historyOwnedId,
        selection.controller.signal
      );
      if (extended && (extended.added > 0 || !extended.reachedStart)) {
        page = await readOlderSelectedConversationItems(
          selection.historyOwnedId, cursor, PAGE_BYTES, selection.controller.signal
        );
        if (!page || !isCurrent(selection)) return;
      }
      if (extended?.reachedStart && page.hasEarlierTranscript) {
        page = { ...page, hasEarlierTranscript: false };
      }
    }
    const previousIds = new Set(selection.chat.messages.map((message) => message.id));
    const messages = mergeMessages(
      selection.chat.messages,
      messagesFromPage(page),
      direction
    );
    if (!selection.push) return;
    pushMessagesSnapshot(selection, messages, selection.push);
    const retainedIds = new Set(messages.map((message) => message.id));
    const evictedPrevious = [...previousIds].some((id) => !retainedIds.has(id));
    selection.beforeCursor = historyPosition(messages[0]) ?? page.beforeCursor;
    selection.afterCursor = historyPosition(messages.at(-1)) ?? page.afterCursor;
    applySelectedConversationPageState(selection.historyOwnedId, page, direction, {
      beforeCursor: selection.beforeCursor,
      afterCursor: selection.afterCursor,
      hasBefore: direction === 'older' ? hasOlderHistory(page) : evictedPrevious || hasOlderHistory(page),
      hasAfter: direction === 'newer' ? page.hasAfter : evictedPrevious || page.hasAfter,
      retainedTurnIds: messages.flatMap((message) =>
        typeof message.metadata?.turnId === 'string' ? [message.metadata.turnId] : []
      ),
      transferBytes: messages.reduce((total, message) => total + messageBytes(message), 0)
    });
    const generation = getConversationSession(selection.historyOwnedId)?.generation ?? 0;
    void restorePageAttachments(
      selection,
      pageAttachments(page.events),
      generation,
      messages.map((message) => message.id)
    );
  } finally {
    if (isCurrent(selection)) {
      setSelectedConversationPageLoading(selection.historyOwnedId, direction, false);
    }
  }
}

export function selectedConversationHistoryOwnedId(): string | null {
  return active?.historyOwnedId ?? null;
}
