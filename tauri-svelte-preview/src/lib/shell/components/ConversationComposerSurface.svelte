<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import {
    conversationSessions,
    setConversationDraft,
    setConversationSendError
  } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    clearConversationSessionDraft,
    flushConversationSessionDraft,
    persistConversationSessionDraft,
    sendStructuredMessage,
    stopStructuredTurn
  } from '$lib/shell/conversation/conversationService.ts';
  import {
    mergeConversationCommandCatalog,
    type ConversationCommand
  } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import ConversationComposer from './conversation/ConversationComposer.svelte';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    onHeightChange?(height: number): void;
  }

  let { owned, activeOwnedId, onHeightChange }: Props = $props();
  const active = $derived(owned.find((session) => session.ownedId === activeOwnedId) ?? null);
  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
  const commands = $derived(
    mergeConversationCommandCatalog(
      conversation?.availableCommands ?? conversation?.capabilities?.commands ?? []
    )
  );

  async function send(): Promise<void> {
    const ownedId = activeOwnedId;
    if (!ownedId || !conversation || conversation.sending || conversation.selectedChildId) return;
    const text = conversation.draft;
    if (!text.trim()) return;

    setConversationDraft(ownedId, '');
    setConversationSendError(ownedId, '');
    try {
      await clearConversationSessionDraft(ownedId);
      await sendStructuredMessage(ownedId, text);
    } catch (error) {
      setConversationSendError(ownedId, error instanceof Error ? error.message : String(error));
      setConversationDraft(ownedId, text);
      persistConversationSessionDraft(ownedId, text);
      await flushConversationSessionDraft(ownedId).catch(() => undefined);
    }
  }

  function selectCommand(command: ConversationCommand): void {
    if (!active) return;
    setConversationDraft(active.ownedId, `/${command.name} `);
  }
</script>

{#if active && conversation && !conversation.selectedChildId}
  <div class="composer-surface">
    <ConversationComposer
      provider={active.agent}
      draft={conversation.draft}
      attachments={[]}
      sending={conversation.sending}
      configState={conversation.agentConfig}
      pendingConfig={conversation.pendingAgentConfig}
      configError={conversation.agentConfigError}
      {commands}
      sendError={conversation.sendError}
      providerNotice={conversation.providerNotice}
      onDismissSendError={() => setConversationSendError(active.ownedId, '')}
      onDraftChange={(value) => {
        setConversationDraft(active.ownedId, value);
        persistConversationSessionDraft(active.ownedId, value);
      }}
      onDraftBlur={() => flushConversationSessionDraft(active.ownedId)}
      onSend={send}
      onStop={() => void stopStructuredTurn(active.ownedId).catch(() => undefined)}
      onCommandSelected={selectCommand}
      {onHeightChange}
    />
  </div>
{/if}

<style>
  .composer-surface {
    flex: 0 0 auto;
    min-width: 0;
  }
</style>
