<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationViewMode, ConversationWorkspaceState } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import SessionPresenceIndicator from './SessionPresenceIndicator.svelte';

  interface Props {
    active: OwnedSession;
    conversation: ConversationWorkspaceState;
    selectedChild: ConversationChildAgent | null;
    onRestart?(): void;
    onModeChange?(mode: ConversationViewMode): void;
    onInspectorToggle?(): void;
  }
  let { active, conversation, selectedChild, onRestart, onModeChange, onInspectorToggle }: Props = $props();
</script>

<header class="conversation-header" data-testid="conversation-header">
  <div class="identity"><strong data-testid="conversation-title">{selectedChild?.label ?? active.title}</strong><span>{selectedChild ? `${active.agent} child · read-only` : `${active.agent} session`}</span></div>
  <div class="header-actions">
    <span data-testid="conversation-runtime-state">
      <SessionPresenceIndicator
        ownedId={active.ownedId}
        terminalState={active.state}
        connectionState={conversation.connectionState}
        activeTurnId={conversation.activeTurnId ?? active.activeTurnId}
        sending={conversation.sending}
        pendingApprovalCount={Object.keys(conversation.pendingApprovals).length + conversation.timeline.filter((item) => item.kind === 'approval' && item.state === 'requested').length}
        runtimeState={active.runtimeState === 'starting' ? 'starting' : null}
        {onRestart}
      />
    </span>
    {#if active.origin === 'app'}<button data-testid="conversation-inspector-toggle" type="button" onclick={() => onInspectorToggle?.()}>Inspector</button>{:else}<button data-testid="conversation-open-raw" type="button" onclick={() => onModeChange?.('raw')}>Open raw terminal</button>{/if}
  </div>
</header>

<style>.conversation-header{display:flex;justify-content:space-between;align-items:center;min-height:46px;padding:8px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 75%,transparent)}.identity{display:flex;align-items:baseline;gap:9px;min-width:0}.identity strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px}.identity span{color:var(--color-text-2);font-size:12px}.header-actions{display:flex;align-items:center;gap:9px}.header-actions button{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.header-actions button:hover{background:var(--color-hover)}</style>
