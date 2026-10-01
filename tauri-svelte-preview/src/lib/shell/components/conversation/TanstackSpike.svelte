<script lang="ts">
  import { createChat } from '@tanstack/ai-svelte';
  import { onDestroy, untrack } from 'svelte';
  import { assemblySpikeAdapter, spikeWindow, spikeDisplayItems } from '../../conversation/tanstackSpike';
  import { conversationSessions, setConversationSelectedChild } from '../../conversation/conversationStore.svelte';
  import { loadConversationForRead, loadOlderConversationEvents, loadNewerConversationEvents, readChildConversationTranscript } from '../../conversation/conversationService';
  import { typedConversationTimeline, reuseConversationDisplayItems, type ConversationDisplayItem } from '../../conversation/conversationTimeline';
  import ConversationTimeline from './ConversationTimeline.svelte';
  import ConversationAgentTree from './ConversationAgentTree.svelte';
  import type { ConversationSendAnchorRequest } from '../../conversation/conversationScrollAnchor';
  import type { AgentConfigValue } from '../../conversation/conversationTypes';
  import type { ConversationFileLinkProvenance } from '../../conversation/conversationTimeline';
  let { ownedId, composerHeight = 0, anchorRequest = null, onFileLink, onInputSubmit }: {
    ownedId: string; composerHeight?: number; anchorRequest?: ConversationSendAnchorRequest | null;
    onFileLink?(path: string, provenance?: ConversationFileLinkProvenance): void;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
  } = $props();
  const id = untrack(() => ownedId);
  const adapter = assemblySpikeAdapter(id);
  const chat = createChat({
    threadId: id, connection: adapter.connection, live: true,
    initialMessages: spikeWindow(id)
  });
  const childRead = new AbortController();
  let disposed = false;
  onDestroy(() => {
    disposed = true;
    childRead.abort();
    setConversationSelectedChild(id, null);
    chat.dispose();
  });
  const workspace = $derived(conversationSessions[id]);
  let error = $state('');
  let paging = false;
  let previousItems: ConversationDisplayItem[] = [];
  let previousView = '';
  let revision = $state(0);
  const displayItems = $derived.by(() => {
    const childId = workspace?.selectedChildId;
    const view = childId ?? 'root';
    if (view !== previousView) { previousView = view; previousItems = []; }
    const messages = childId
      ? chat.messages.flatMap((message) => message.parts.flatMap((part) =>
        part.type === 'subagent' && part.subagent.id === childId ? part.subagent.messages : []))
      : chat.messages;
    const source = childId ? typedConversationTimeline([], workspace?.childTimeline ?? [])
      : typedConversationTimeline(workspace?.agentItems, workspace?.timeline, {}, [], workspace?.sentAttachments);
    previousItems = reuseConversationDisplayItems(spikeDisplayItems(messages, source), previousItems);
    return previousItems;
  });
  // The library consumes deltas asynchronously, after the Assembly revision changes.
  $effect(() => { displayItems; untrack(() => { revision++; }); });
  // Window replacement/trim is authoritative, including trimming the newer end.
  $effect(() => {
    const oldest = workspace?.oldestLoadedSequence;
    const reachedEnd = workspace?.reachedTranscriptEnd;
    Object.values(workspace?.sentAttachments ?? {}).flatMap((items) => items.map((item) => item.previewUrl));
    untrack(() => { if (oldest !== undefined || reachedEnd !== undefined) adapter.syncWindow(); });
  });
  async function perform(work: Promise<unknown>) {
    try { error = ''; await work; } catch (cause) { error = String(cause); }
  }
  async function page(older: boolean) {
    if (paging) return;
    paging = true;
    try {
      await (older ? loadOlderConversationEvents(id) : loadNewerConversationEvents(id));
      adapter.syncWindow();
    } catch (cause) { error = String(cause); }
    finally { paging = false; }
  }
  async function selectChild(childId: string | null) {
    setConversationSelectedChild(id, childId);
    adapter.syncWindow();
    const state = conversationSessions[id];
    if (!childId || !state?.nativeSessionId) return;
    await readChildConversationTranscript({
      ownedId: id, provider: state.provider, nativeSessionId: state.nativeSessionId,
      childSessionId: childId, signal: childRead.signal
    });
    if (!disposed) adapter.syncWindow();
  }
  export async function sendMessage(text: string, steering: boolean): Promise<void> {
    if (disposed) throw new Error('The selected spike conversation changed before sending.');
    if (steering) { await adapter.steer(text); return; }
    await chat.sendMessage({ content: [
      { type: 'text', content: text },
      ...(workspace?.attachments ?? []).map((attachment) => ({ type: 'image' as const,
        source: { type: 'url' as const, value: attachment.previewUrl, mimeType: attachment.mimeType } }))
    ] });
    if (chat.error) throw chat.error;
  }

</script>

<div class="spike">
  <header>TanStack client spike · SQL window {Math.round((workspace?.loadedEventsBytes ?? 0) / 1024)} KiB · {chat.messages.length} messages · {chat.error ? "error" : chat.sessionGenerating ? "streaming" : chat.isLoading ? "sending" : "ready"}</header>
  <ConversationAgentTree children={workspace?.children ?? []} selectedChildId={workspace?.selectedChildId ?? null}
    onSelect={(childId) => void perform(selectChild(childId))} />
  <ConversationTimeline items={displayItems} conversationId={id} {composerHeight} {anchorRequest} {onFileLink} {onInputSubmit}
    renderWindowId={`${id}:${workspace?.selectedChildId ?? 'root'}`} timelineRevision={revision}
    activeTurnId={workspace?.selectedChildId ? null : workspace?.activeTurnId}
    localTurnActive={!workspace?.selectedChildId && chat.sessionGenerating}
    assistantLabel={workspace?.provider ?? 'Assistant'}
    hasOlder={!workspace?.selectedChildId && !workspace?.reachedTranscriptStart}
    loadingOlder={!workspace?.selectedChildId && workspace?.loadingOlder}
    onLoadOlder={() => void page(true)}
    hasNewer={!workspace?.selectedChildId && !workspace?.reachedTranscriptEnd}
    loadingNewer={!workspace?.selectedChildId && workspace?.loadingNewer}
    onLoadNewer={() => void page(false)}
    onJumpToLatest={async () => { await loadConversationForRead(id, true, childRead.signal); adapter.syncWindow(); }}
    onApprovalDecision={(requestId, decision) => void perform(adapter.approve(requestId, decision))}
  />
  {#if error || chat.error}<p role="alert">{error || chat.error?.message}</p>{/if}
</div>

<style>
  .spike { display:flex; flex-direction:column; min-height:0; height:100%; padding:12px; gap:8px; }
  header { font-size:12px; }
</style>
