<!--
  AgentsPanel.svelte — the Agents tab of the right column.

  The subagents of the session the user is looking at, with what each one is
  doing. Picking one selects it in the conversation and reads its transcript,
  which is also the only way its message count can become known: the record the
  provider gives us for a subagent carries a label, a state, and a time, and
  nothing more.

  The selected running child's transcript refreshes while this panel is visible.
  The child list itself still comes from conversation snapshots.

  Starting or editing workflows is not part of this panel.
-->
<script lang="ts">
  import Bot from '@lucide/svelte/icons/bot';

  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import type { ConversationTimelineEntry } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    getConversationSession,
    setConversationSelectedChild
  } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    cancelChildConversationTranscriptRead,
    readChildConversationTranscript
  } from '$lib/shell/conversation/conversationService.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { showCenterTab } from '$lib/shell/workbenchNavigation.ts';

  import AgentRow from './AgentRow.svelte';
  import { agentActivityRows, agentStatus } from './agentActivityModel.ts';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  let { visible, ownedId }: Props = $props();

  const conversation = $derived(ownedId ? getConversationSession(ownedId) : null);
  const selectedChildId = $derived(conversation?.selectedChildId ?? null);

  /**
   * The store keeps a transcript for the selected child alone, so this map has
   * at most one entry. Everything else in the list has no count and says so.
   */
  const timelineByChild = $derived<Record<string, readonly ConversationTimelineEntry[]>>(
    selectedChildId ? { [selectedChildId]: conversation?.childTimeline ?? [] } : {}
  );

  const rows = $derived(agentActivityRows(conversation?.children ?? [], timelineByChild));
  const selectedChild = $derived(
    conversation?.children.find((child) => child.childId === selectedChildId) ?? null
  );
  const selectedChildRunning = $derived(
    selectedChild !== null && agentStatus(selectedChild.state) === 'working'
  );

  $effect(() => {
    if (!visible || !ownedId || !conversation || !selectedChildId) return;
    if (!selectedChild?.transcriptAvailable) return;
    const parentGeneration = conversation.generation;
    void parentGeneration;
    const nativeSessionId = rail.owned.find((session) => session.ownedId === ownedId)?.nativeSessionId;

    const timer = nativeSessionId && selectedChildRunning
      ? window.setInterval(() => {
          void readChildConversationTranscript({
            ownedId,
            provider: conversation.provider,
            nativeSessionId,
            childSessionId: selectedChildId
          }).catch(() => undefined);
        }, 10_000)
      : null;
    return () => {
      if (timer !== null) window.clearInterval(timer);
      cancelChildConversationTranscriptRead(ownedId);
    };
  });

  async function select(childId: string): Promise<void> {
    if (!ownedId || !conversation) return;
    const next = selectedChildId === childId ? null : childId;
    cancelChildConversationTranscriptRead(ownedId);
    setConversationSelectedChild(ownedId, next);
    if (!next) return;
    const child = conversation.children.find((entry) => entry.childId === next) ?? null;
    if (!child?.transcriptAvailable) return;
    // The conversation in the center switches to the child that was picked, so
    // bring it forward rather than leaving the change somewhere unseen.
    showCenterTab('session');
    const nativeSessionId = rail.owned.find((session) => session.ownedId === ownedId)?.nativeSessionId;
    if (!nativeSessionId) return;
    await readChildConversationTranscript({
      ownedId,
      provider: conversation.provider,
      nativeSessionId,
      childSessionId: next
    }).catch(() => undefined);
  }
</script>

<section class="flex h-full min-h-0 flex-col" data-testid="agents-panel">
  <PanelHeader title="Agents" count={rows.length > 0 ? rows.length : null} />

  {#if rows.length === 0}
    <EmptyState
      title="No agents yet"
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
