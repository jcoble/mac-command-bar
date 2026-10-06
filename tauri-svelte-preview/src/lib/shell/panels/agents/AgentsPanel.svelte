<!--
  AgentsPanel.svelte — the Agents tab of the right column.

  The subagents of the session the user is looking at, with what each one is
  doing. Picking one selects it in the conversation and reads its transcript,
  which is also the only way its message count can become known: the record the
  provider gives us for a subagent carries a label, a state, and a time, and
  nothing more.

  The selected child uses the same bounded conversation graph as its parent view.
  Native watches publish durable updates without a panel polling loop.

  The Workflows view hosts the app-owned, cross-provider handoff loop. It stays
  separate from provider-native child agents so either surface can be removed
  without disturbing the other.
-->
<script lang="ts">
  import Bot from '@lucide/svelte/icons/bot';

  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import type { UIMessage } from '@tanstack/ai/client';
  import {
    getConversationSession,
    setConversationSelectedChild
  } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    readChildConversationHistory,
    stopChildConversationHistory
  } from '$lib/shell/conversation/conversationService.ts';
  import {
    disposeSelectedConversationChat,
    selectConversationChat,
    selectedConversationChat,
    selectedConversationChatReady
  } from '$lib/shell/conversation/conversationConnection.ts';
  import { showCenterTab } from '$lib/shell/workbenchNavigation.ts';

  import AgentRow from './AgentRow.svelte';
  import { agentActivityRows } from './agentActivityModel.ts';
  import WorkflowRuns from './WorkflowRuns.svelte';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  let { visible, root, ownedId }: Props = $props();
  let view = $state<'session' | 'workflows'>('session');

  const VIEW_OPTIONS = [
    { value: 'session', label: 'Session' },
    { value: 'workflows', label: 'Workflows' }
  ] as const;

  const conversation = $derived(ownedId ? getConversationSession(ownedId) : null);
  const selectedChildId = $derived(conversation?.selectedChildId ?? null);

  /**
   * The sole selected chat graph supplies one child's count. Everything else
   * in the list has no loaded body and says so.
   */
  const timelineByChild = $derived.by((): Record<string, readonly UIMessage[]> => {
    if (!selectedChildId || !conversation || conversation.selectedHistoryOwnedId === ownedId) return {};
    conversation.timelineRevision;
    return { [selectedChildId]: selectedConversationChat(ownedId)?.messages ?? [] };
  });

  const rows = $derived(agentActivityRows(conversation?.children ?? [], timelineByChild));
  async function select(childId: string): Promise<void> {
    if (!ownedId || !conversation) return;
    const next = selectedChildId === childId ? null : childId;
    stopChildConversationHistory(ownedId);
    setConversationSelectedChild(ownedId, next);
    if (!next) {
      selectConversationChat(ownedId, ownedId);
      await selectedConversationChatReady(ownedId);
      return;
    }
    disposeSelectedConversationChat(ownedId);
    const child = conversation.children.find((entry) => entry.childId === next) ?? null;
    if (!child?.transcriptAvailable) return;
    // The conversation in the center switches to the child that was picked, so
    // bring it forward rather than leaving the change somewhere unseen.
    showCenterTab('session');
    try {
      const history = await readChildConversationHistory({
        ownedId,
        childId: next,
        childSessionId: child.transcriptId ?? next
      });
      if (!history || getConversationSession(ownedId)?.selectedChildId !== next) return;
      selectConversationChat(ownedId, history.historyOwnedId);
      await selectedConversationChatReady(ownedId);
    } catch {
      // The child-specific store error remains visible without replacing saved content.
    }
  }
</script>

<section class="flex h-full min-h-0 flex-col" data-testid="agents-panel">
  <PanelHeader title="Agents" count={view === 'session' && rows.length > 0 ? rows.length : null} />
  <div class="border-b border-border px-3 pb-2">
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
            <AgentRow {row} selected={selectedChildId === row.childId} onselect={(id) => void select(id)} />
          </li>
        {/each}
      </ul>
    </ScrollArea>
  {/if}
</section>
