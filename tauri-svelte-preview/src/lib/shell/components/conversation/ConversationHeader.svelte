<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationViewMode, ConversationWorkspaceState } from '$lib/shell/conversation/conversationStore.svelte.ts';

  interface Props {
    active: OwnedSession;
    conversation: ConversationWorkspaceState;
    selectedChild: ConversationChildAgent | null;
    onModeChange?(mode: ConversationViewMode): void;
  }
  let { active, conversation, selectedChild, onModeChange }: Props = $props();
</script>

<header class="conversation-header" data-testid="conversation-header">
  <div class="identity"><strong data-testid="conversation-title">{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} child · read-only` : `${active.agent} session`}</span>{#if conversation.connectionState !== 'connected'}<small data-testid="conversation-connection-state">{conversation.connectionState}</small>{/if}</div>
  <div class="header-actions"><span class="runtime-state" data-testid="conversation-runtime-state">{conversation.executionOwner}</span><button data-testid="conversation-open-raw" type="button" onclick={() => onModeChange?.('raw')}>Open raw terminal</button></div>
</header>

<style>.conversation-header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:9px;min-width:0}.identity strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px}.identity span,.identity small,.runtime-state{color:var(--color-text-2);font-size:12px}.header-actions{display:flex;align-items:center;gap:9px}.header-actions button{border:1px solid color-mix(in srgb,var(--color-border) 82%,transparent);border-radius:7px;background:color-mix(in srgb,var(--color-surface) 80%,transparent);color:inherit;padding:6px 9px}.runtime-state{padding:3px 6px}</style>
