<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationViewMode, ConversationWorkspaceState } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import { modelEffortLabel } from '$lib/shell/conversation/agentConfigLabels.ts';
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
  <div class="header-identity">
    <div class="identity-mark" aria-hidden="true">{active.agent === 'codex' ? 'C' : 'A'}</div>
    <div class="identity-copy">
      <div class="breadcrumb"><span>{active.title || 'Untitled session'}</span><span class="separator">/</span><span>{selectedChild ? selectedChild.label : 'Conversation'}</span></div>
      <div class="session-meta">
        <strong data-testid="conversation-title">{selectedChild?.label ?? active.title}</strong>
        <span>{selectedChild ? `${active.agent} child · read-only` : `${active.agent} session`}</span>
        {#if conversation.agentConfig.model || conversation.agentConfig.reasoningEffort}<span class="model-pill" data-testid="conversation-model">{modelEffortLabel(conversation.agentConfig.model, conversation.agentConfig.reasoningEffort)}</span>{/if}
      </div>
    </div>
  </div>
  <div class="header-actions">
    <span class="runtime-state" data-testid="conversation-runtime-state">
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
    {#if active.origin === 'app'}<button class="header-button" data-testid="conversation-inspector-toggle" type="button" onclick={() => onInspectorToggle?.()}>Inspector</button>{:else}<button class="header-button" data-testid="conversation-open-raw" type="button" onclick={() => onModeChange?.('raw')}>Open raw terminal</button>{/if}
  </div>
</header>

<style>
  .conversation-header { display: flex; align-items: center; justify-content: space-between; gap: 18px; min-height: 58px; padding: 9px 18px; border-bottom: 1px solid color-mix(in srgb, var(--color-border) 62%, transparent); background: color-mix(in srgb, var(--color-surface) 54%, var(--color-bg) 46%); }
  .header-identity { display: flex; min-width: 0; align-items: center; gap: 10px; }
  .identity-mark { display: grid; place-items: center; flex: none; width: 26px; height: 26px; border: 1px solid color-mix(in srgb, var(--color-accent) 34%, var(--color-border)); border-radius: 8px; background: color-mix(in srgb, var(--color-accent) 11%, var(--color-surface)); color: var(--color-accent); font-size: 12px; font-weight: 750; }
  .identity-copy { display: grid; min-width: 0; gap: 3px; }
  .breadcrumb { display: flex; min-width: 0; align-items: center; gap: 6px; color: var(--color-text-3); font-size: 12px; }
  .breadcrumb span:first-child, .breadcrumb span:last-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .separator { color: var(--color-border); }
  .session-meta { display: flex; min-width: 0; align-items: center; gap: 8px; }
  .session-meta strong { overflow: hidden; color: var(--color-text); font-size: 14px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
  .session-meta > span:not(.model-pill) { color: var(--color-text-2); font-size: 12px; white-space: nowrap; }
  .model-pill { max-width: 210px; overflow: hidden; padding: 3px 7px; border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent); border-radius: 999px; color: var(--color-text-2); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .header-actions { display: flex; align-items: center; gap: 9px; }
  .runtime-state { display: inline-flex; align-items: center; min-height: 28px; padding: 0 3px; }
  .header-button { min-height: 28px; padding: 5px 10px; border: 1px solid color-mix(in srgb, var(--color-border) 64%, transparent); border-radius: 8px; background: color-mix(in srgb, var(--color-elevated) 84%, transparent); color: var(--color-text); font: 13px/1.2 inherit; cursor: pointer; }
  .header-button:hover { background: var(--color-hover); }
  .header-button:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 2px; }
  @media (prefers-reduced-motion: no-preference) { .header-button { transition: background .14s ease, border-color .14s ease; } }
</style>
