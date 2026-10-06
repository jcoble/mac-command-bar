<!-- Agents > Session owns the list and one bounded child transcript. -->
<script lang="ts">
  import { createChatUI, UIProvider } from '@tanstack/ai-svelte/ui';
  import ConversationMessageParts from '$lib/shell/components/conversation/ConversationMessageParts.svelte';
  import ConversationToolPart from '$lib/shell/components/conversation/ConversationToolPart.svelte';
  import { setContext, untrack } from 'svelte';
  import Bot from '@lucide/svelte/icons/bot';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import ConversationTimeline from '$lib/shell/components/conversation/ConversationTimeline.svelte';
  import {
    getConversationSession,
    setConversationSelectedChild,
    selectedConversationViewState,
    setSelectedConversationViewState
  } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    readChildConversationHistory,
    stopChildConversationHistory
  } from '$lib/shell/conversation/conversationService.ts';
  import {
    disposeChildConversationChat,
    selectConversationChat,
    selectedConversationChat,
    selectedConversationChatReady,
    pageSelectedConversation,
    jumpSelectedConversationToLatest
  } from '$lib/shell/conversation/conversationConnection.ts';
  import {
    conversationMessagesContext,
    type ConversationMessagesContext
  } from '$lib/shell/conversation/conversationChatUI.ts';
  import { conversationDisplayItems } from '$lib/shell/conversation/conversationMessages.ts';
  import type { ConversationDisplayItem, ConversationFileLinkProvenance } from '$lib/shell/conversation/conversationTimeline.ts';
  import { requestOpenConversationFile } from '$lib/shell/openFileBus.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';

  import AgentRow from './AgentRow.svelte';
  import { agentActivityRows } from './agentActivityModel.ts';
  import WorkflowRuns from './WorkflowRuns.svelte';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let { visible, root, ownedId }: Props = $props();
  let view = $state<'session' | 'workflows'>('session');
  let transcriptState = $state<'loading' | 'unavailable' | 'ready' | 'error'>('unavailable');
  let transcriptError = $state('');
  let fileError = $state('');
  let reader: AbortController | null = null;

  const VIEW_OPTIONS = [
    { value: 'session', label: 'Session' },
    { value: 'workflows', label: 'Workflows' }
  ] as const;
  const STATUS_TONE = { working: 'live', done: 'good', failed: 'bad', idle: 'neutral' } as const;
  const STATUS_WORD = { working: 'Working', done: 'Done', failed: 'Failed', idle: 'Idle' } as const;
  const conversation = $derived(ownedId ? getConversationSession(ownedId) : null);
  const selectedChildId = $derived(conversation?.selectedChildId ?? null);
  // List rows use metadata; returning to the list releases the child body.
  const rows = $derived(agentActivityRows(conversation?.children ?? [], {}));
  const selectedRow = $derived(rows.find((row) => row.childId === selectedChildId) ?? null);
  const historyOwnedId = $derived(conversation?.selectedChildHistoryOwnedId ?? null);
  const childConversation = $derived(historyOwnedId ? getConversationSession(historyOwnedId) : null);
  const transcriptChat = $derived.by(() => {
    if (!ownedId || !historyOwnedId || !childConversation) return null;
    childConversation.timelineRevision;
    return selectedConversationChat(ownedId, historyOwnedId);
  });
  const messages = $derived(transcriptChat?.messages ?? []);
  const messagesById = $derived(new Map(messages.map((message) => [message.id, message])));
  const toolComponents = $state<Record<string, typeof ConversationToolPart>>({});
  const ui = createChatUI({}, {
    components: { layout: ConversationMessageParts, message: ConversationMessageParts },
    partsComponents: {},
    toolsComponents: toolComponents
  });
  $effect.pre(() => {
    const names = new Set(messages.flatMap((message) => message.parts.flatMap((part) =>
      part.type === 'tool-call' ? [part.name] : []
    )));
    untrack(() => {
      for (const name of Object.keys(toolComponents)) if (!names.has(name)) delete toolComponents[name];
      for (const name of names) toolComponents[name] = ConversationToolPart;
    });
  });
  setContext<ConversationMessagesContext>(conversationMessagesContext, {
    ui,
    get messages() { return messagesById; }
  });
  let previousHistoryId: string | null = null;
  let previousItems: ConversationDisplayItem[] = [];
  const items = $derived.by(() => {
    if (historyOwnedId !== previousHistoryId) {
      previousHistoryId = historyOwnedId;
      previousItems = [];
    }
    previousItems = conversationDisplayItems(messages, previousItems, childConversation?.sentAttachments ?? {});
    return previousItems;
  });
  const error = $derived(transcriptError || conversation?.childTranscriptError || '');

  function release(parentId: string): void {
    reader?.abort();
    reader = null;
    stopChildConversationHistory(parentId);
    disposeChildConversationChat(parentId);
    setConversationSelectedChild(parentId, null);
    previousHistoryId = null;
    previousItems = [];
    // These lazy projections must drop their old body even when the list is shown.
    untrack(() => { void messagesById; void items; });
    transcriptState = 'unavailable';
    transcriptError = '';
    fileError = '';
  }

  $effect(() => {
    const parentId = ownedId;
    const active = visible && view === 'session';
    if (!active && parentId) untrack(() => release(parentId));
    return () => { if (parentId) untrack(() => release(parentId)); };
  });

  async function select(childId: string): Promise<void> {
    if (!ownedId || !conversation || !visible || view !== 'session' || selectedChildId === childId) return;
    const parentId = ownedId;
    release(parentId);
    setConversationSelectedChild(parentId, childId);
    const child = conversation.children.find((entry) => entry.childId === childId);
    if (!child?.transcriptAvailable) return;
    const controller = new AbortController();
    reader = controller;
    transcriptState = 'loading';
    const current = (): boolean => !controller.signal.aborted && ownedId === parentId
      && visible && view === 'session' && getConversationSession(parentId)?.selectedChildId === childId;
    try {
      const history = await readChildConversationHistory({
        ownedId: parentId,
        childId,
        childSessionId: child.transcriptId ?? childId,
        signal: controller.signal
      });
      if (!current()) return;
      if (!history) {
        transcriptState = 'unavailable';
        return;
      }
      selectConversationChat(parentId, history.historyOwnedId, controller.signal);
      await selectedConversationChatReady(parentId, history.historyOwnedId);
      if (current()) transcriptState = 'ready';
    } catch (error) {
      if (!current()) return;
      transcriptState = 'error';
      transcriptError = error instanceof Error ? error.message : String(error);
    }
  }

  function openFile(reference: string, provenance?: ConversationFileLinkProvenance): void {
    const active = rail.owned.find((session) => session.ownedId === ownedId);
    fileError = active ? requestOpenConversationFile(active, reference, provenance) ?? ''
      : 'That file link could not be opened.';
  }
</script>

<section class="flex h-full min-h-0 flex-col" data-testid="agents-panel">
  <PanelHeader title="Agents" count={view === 'session' && rows.length > 0 ? rows.length : null} />
  <div class="px-2 pb-2">
    <SegmentedControl
      items={VIEW_OPTIONS}
      value={view}
      size="sm"
      aria-label="Agent view"
      class="w-full"
      onValueChange={(value) => (view = value as typeof view)}
    />
  </div>

  {#if view === 'workflows'}
    <WorkflowRuns visible={visible} {root} />
  {:else if visible && selectedRow && ownedId}
    <div class="flex min-w-0 items-center gap-2 px-2 pb-2" data-testid="agent-transcript-header">
      <Button variant="ghost" size="sm" iconPosition="start" onclick={() => ownedId && release(ownedId)}>
        <ArrowLeft data-icon="inline-start" />Back
      </Button>
      <span class="min-w-0 flex-1 truncate">{selectedRow.label}</span>
      <Chip tone={STATUS_TONE[selectedRow.status]}>{STATUS_WORD[selectedRow.status]}</Chip>
    </div>
    {#if error}
      <EmptyState title="Could not load transcript" body={error} data-testid="agent-transcript-error" />
    {:else if transcriptState === 'loading'}
      <EmptyState title="Loading transcript" body="Reading this agent’s saved conversation." data-testid="agent-transcript-loading" />
    {:else if transcriptState === 'unavailable'}
      <EmptyState title="Transcript unavailable" body="This agent’s transcript is not available yet." data-testid="agent-transcript-unavailable" />
    {:else if historyOwnedId && childConversation}
      {#if fileError}<p role="alert" class="px-2 pb-2 text-sm text-muted-foreground">{fileError}</p>{/if}
      {#if transcriptChat}
        {#key transcriptChat}
          <UIProvider {ui} chat={transcriptChat}>
      <ConversationTimeline
        {items}
        conversationId={ownedId}
        {historyOwnedId}
        renderWindowId={`${ownedId}:${historyOwnedId}`}
        timelineRevision={childConversation.timelineRevision}
        viewState={selectedConversationViewState(ownedId, historyOwnedId)}
        onViewChange={setSelectedConversationViewState}
        itemFirstSequence={(itemId) => {
          const sequence = messagesById.get(itemId)?.metadata?.firstSequence;
          return typeof sequence === 'number' ? sequence : undefined;
        }}
        activeTurnId={childConversation.activeTurnId ?? null}
        turnFacts={childConversation.selectedTurns}
        assistantLabel={selectedRow.label}
        emptyText="This agent’s transcript is empty."
        hasOlder={childConversation.selectedHasBefore}
        loadingOlder={childConversation.selectedLoadingOlder}
        pageError={childConversation.selectedPageError}
        onLoadOlder={() => void pageSelectedConversation('older', historyOwnedId)}
        hasNewer={childConversation.selectedHasAfter}
        loadingNewer={childConversation.selectedLoadingNewer}
        onLoadNewer={() => void pageSelectedConversation('newer', historyOwnedId)}
        oldestSequence={childConversation.selectedBeforeCursor ?? 0}
        newestSequence={childConversation.selectedAfterCursor ?? 0}
        onJumpToLatest={() => jumpSelectedConversationToLatest(ownedId!, historyOwnedId!)}
        onFileLink={openFile}
      />
          </UIProvider>
        {/key}
      {/if}
    {/if}
  {:else if rows.length === 0}
    <EmptyState
      title="No session agents yet"
      body="When this thread spawns subagents or runs a workflow, they show up here with live status, activity, and token usage."
      data-testid="agents-panel-empty"
    >
      {#snippet icon()}<Bot />{/snippet}
    </EmptyState>
  {:else}
    <ScrollArea class="min-h-0 flex-1">
      <ul class="flex flex-col gap-1 p-2">
        {#each rows as row (row.childId)}
          <li class="min-w-0">
            <AgentRow {row} selected={false} onselect={(id) => void select(id)} />
          </li>
        {/each}
      </ul>
    </ScrollArea>
  {/if}
</section>
