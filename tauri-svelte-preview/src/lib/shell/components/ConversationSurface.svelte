<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import TerminalSurface from './TerminalSurface.svelte';
  import ConversationTimeline from './conversation/ConversationTimeline.svelte';
  import ConversationComposer from './conversation/ConversationComposer.svelte';
  import ConversationAgentTree from './conversation/ConversationAgentTree.svelte';
  import {
    beginConversationAgentConfigChange,
    confirmConversationAgentConfigChange,
    conversationSessions,
    failConversationAgentConfigChange,
    setConversationAgentConfigError,
    setConversationAgentConfigState,
    setConversationAttachments,
    setConversationDraft,
    setConversationSendError,
    setConversationMode,
    setConversationScrollTop,
    setConversationSelectedChild
  } from '$lib/shell/conversation/conversationStore.svelte';
  import {
    cleanupConversationAttachment,
    clearConversationSessionDraft,
    flushConversationSessionDraft,
    loadConversationCapabilities,
    loadOlderConversationEvents,
    persistConversationSessionDraft,
    readChildConversationTranscript,
    removeConversationAttachment,
    sendPermissionResponse,
    respondToStructuredInput,
    restoreConversationAttachments,
    saveConversationClipboardImage,
    sendStructuredMessage,
    stopStructuredTurn
  } from '$lib/shell/conversation/conversationService';
  import {
    readAgentConversationConfig,
    setAgentConversationConfig,
    type AgentConversationConfigField,
    type AgentConversationConfigRequest
  } from '$lib/shell/conversation/conversationConfig.ts';
  import {
    mergeConversationCommandCatalog,
    type ConversationCommand
  } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import {
    typedConversationTimeline,
    type ConversationDisplayItem
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import { contextMeterState } from '$lib/shell/conversation/composerSlashCommands.ts';
  import { sessionContextUsage } from '$lib/shell/panels/context/sessionContextModel.ts';
  import { requestOpenFile } from '$lib/shell/openFileBus.ts';
  import { clearViewedSession, setViewedSession } from '$lib/shell/conversation/sessionPresence.ts';
  import type { ConversationSendAnchorRequest } from '$lib/shell/conversation/conversationScrollAnchor.ts';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    activeOrigin?: OwnedSession['origin'];
    registerHost(ownedId: string, host: HTMLElement): void;
    onHostLayout?(ownedId: string): void;
    onOpenNativeCli?(ownedId: string): void | Promise<void>;
    onForkNativeCli?(ownedId: string): void | Promise<void>;
    onReturnToStructured?(ownedId: string): void | Promise<void>;
  }
  let {
    owned,
    activeOwnedId,
    activeOrigin,
    registerHost,
    onHostLayout,
    onOpenNativeCli,
    onForkNativeCli,
    onReturnToStructured
  }: Props = $props();
  const active = $derived(owned.find((item) => item.ownedId === activeOwnedId) ?? null);
  const origin = $derived(activeOrigin ?? active?.origin ?? 'external');
  const appOwned = $derived(origin === 'app');
  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude' || active.agent === 'antigravity') && (appOwned || conversation?.mode !== 'raw'));
  const selectedChild = $derived(conversation && conversation.selectedChildId
    ? conversation.children.find((child) => child.childId === conversation.selectedChildId) ?? null
    : null);
  const legacyTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
  let previousTimelineKey = '';
  let previousVisibleTimeline: ConversationDisplayItem[] = [];
  const visibleTimeline = $derived.by((): ConversationDisplayItem[] => {
    if (!conversation) return [];
    const timelineKey = `${conversation.ownedId}:${conversation.selectedChildId ?? 'root'}`;
    if (timelineKey !== previousTimelineKey) {
      previousTimelineKey = timelineKey;
      previousVisibleTimeline = [];
    }
    if (conversation.selectedChildId) {
      previousVisibleTimeline = typedConversationTimeline([], legacyTimeline, {}, previousVisibleTimeline);
      return previousVisibleTimeline;
    }
    const items = typedConversationTimeline(
      conversation.agentItems,
      legacyTimeline,
      {},
      previousVisibleTimeline,
      conversation.sentAttachments
    );
    const now = items.reduce((latest, item) => Math.max(latest, item.timestampMs), 0) + 1;
    const typedKinds = new Set(items.map((item) => item.kind));
    if (conversation.planSteps.length && !typedKinds.has('plan')) {
      items.push({ kind: 'plan', itemId: 'plan:current', turnId: conversation.activeTurnId ?? null, title: 'Plan', steps: conversation.planSteps, timestampMs: now });
    }
    if (conversation.tasks.length && !typedKinds.has('tasks')) {
      items.push({ kind: 'tasks', itemId: 'tasks:current', turnId: conversation.activeTurnId ?? null, title: 'Tasks', tasks: conversation.tasks, timestampMs: now });
    }
    // Live requests render inline above the composer, keeping their response
    // controls attached to the prompt. Resolved requests remain in the
    // normalized transcript returned above.
    previousVisibleTimeline = items.sort((left, right) => left.timestampMs - right.timestampMs);
    return previousVisibleTimeline;
  });
  const pendingApprovals = $derived(conversation ? Object.values(conversation.pendingApprovals) : []);
  const pendingInputs = $derived(conversation ? Object.values(conversation.pendingInputs) : []);
  const commandCatalog = $derived(mergeConversationCommandCatalog(conversation?.availableCommands ?? conversation?.capabilities?.commands ?? []).filter((command) => !appOwned || command.name !== 'terminal'));
  /* The same numbers the Context panel shows. This read only `metadata`, and a
     provider that reports its usage as it goes puts those numbers on `usage` —
     so the panel had a figure and the composer had nothing, from one session. */
  const contextUsage = $derived(
    sessionContextUsage(conversation?.metadata ?? null, conversation?.usage)
  );
  const contextMeter = $derived(
    contextMeterState(contextUsage.usedTokens, contextUsage.contextWindow)
  );

  let attachmentError = $state('');
  // Read from the session rather than held here: this surface is mounted once
  // for the whole shell, so a failure kept in component state was shown under
  // every conversation and survived the send that fixed it.
  const sendError = $derived(conversation?.sendError ?? '');
  let capabilityRequest = $state('');
  let configRequest = $state('');
  /** A request key whose failure has already bought its one retry. The guard
   * above is claimed before the call, so without this a read that lost a
   * start-up race left the composer empty for good: the key still matched, so
   * the effect never asked again. Clearing the guard lets it ask once more —
   * and this remembers that it did, because the effect reads the guard and
   * would otherwise retry forever against a failure that is not going away. */
  let configRetried = $state('');
  let sendAnchorRequest = $state<ConversationSendAnchorRequest | null>(null);
  let sendAnchorRequestId = 0;
  let localTurnActive = $state(false);
  let localTurnStarted = $state(false);
  let composerHeight = $state(0);
  let composer = $state<{ focus(): void } | null>(null);

  /** Put the caret in the prompt box. The page calls this when a panel hands
   * the composer something — an attachment, a line of text — so the reader ends
   * up typing beside it rather than hunting for the box. */
  export function focusComposer(): void {
    composer?.focus();
  }

  $effect(() => {
    if (!localTurnActive) {
      localTurnStarted = false;
      return;
    }
    if (conversation?.sending) {
      localTurnStarted = true;
      return;
    }
    if (localTurnStarted) {
      localTurnActive = false;
      localTurnStarted = false;
      sendAnchorRequest = null;
    }
  });

  $effect(() => {
    const ownedId = activeOwnedId;
    setViewedSession(ownedId);
    if (!ownedId) return;
    return () => clearViewedSession(ownedId);
  });

  $effect(() => {
    if (appOwned && activeOwnedId && conversation?.mode === 'raw') {
      setConversationMode(activeOwnedId, 'structured');
    }
  });

  $effect(() => {
    if (!structured || !active || !conversation || (active.agent !== 'claude' && active.agent !== 'codex' && active.agent !== 'antigravity')) return;
    const ownedId = active.ownedId;
    const generation = conversation.generation;
    const key = `${ownedId}:${generation}:${conversation.connectionState}`;
    if (configRequest === key) return;
    configRequest = key;
    void readAgentConversationConfig(ownedId).then((state) => {
      if (conversationSessions[ownedId]?.generation === generation) {
        setConversationAgentConfigState(ownedId, state);
      }
    }).catch((error) => {
      if (conversationSessions[ownedId]?.generation === generation) {
        setConversationAgentConfigError(ownedId, error instanceof Error ? error.message : String(error));
      }
      // The usual failure here is a race, not a refusal: the session is stored
      // but not yet in the manager's map. Asking a second time is what fills the
      // composer in; asking forever would be a retry storm.
      if (configRequest === key && configRetried !== key) {
        configRetried = key;
        configRequest = '';
      }
    });
  });

  $effect(() => {
    if (!structured || !active || !conversation || (active.agent !== 'claude' && active.agent !== 'codex' && active.agent !== 'antigravity')) return;
    const key = `${active.ownedId}:${conversation.generation}:${conversation.provider}:${conversation.connectionState}`;
    if (capabilityRequest === key) return;
    // A connection re-reads the snapshot: the stored one can predate a provider
    // upgrade, and activation refreshes it from the live handshake.
    if (conversation.capabilities && conversation.connectionState !== 'connected') return;
    capabilityRequest = key;
    void loadConversationCapabilities(active.ownedId, conversation.provider).catch(() => undefined);
  });

  $effect(() => {
    if (!active || !conversation || !structured) return;
    if (!conversation.attachments.length && conversation.attachmentIds.length) {
      void restoreConversationAttachments(active.ownedId).catch(() => undefined);
    }
  });

  async function send(): Promise<void> {
    const ownedId = activeOwnedId;
    if (!ownedId || !conversation || conversation.sending || conversation.selectedChildId) return;
    if (!conversation.draft.trim() && conversation.attachments.length === 0) return;
    const text = conversation.draft;
    const previousUserItemId = visibleTimeline.findLast((item) => item.kind === 'user')?.itemId ?? null;
    sendAnchorRequest = {
      requestId: ++sendAnchorRequestId,
      conversationId: ownedId,
      previousUserItemId
    };
    localTurnActive = true;
    localTurnStarted = false;
    setConversationDraft(ownedId, '');
    setConversationSendError(ownedId, '');
    try {
      await clearConversationSessionDraft(ownedId);
      await sendStructuredMessage(ownedId, text);
    } catch (error) {
      // Restore the draft. The service intentionally leaves attachments in the
      // store on every failure, so the user can retry without data loss. The
      // reason has to be said out loud: a swallowed failure here reads as a
      // composer that silently refuses every Enter.
      setConversationSendError(ownedId, error instanceof Error ? error.message : String(error));
      setConversationDraft(ownedId, text);
      persistConversationSessionDraft(ownedId, text);
      await flushConversationSessionDraft(ownedId).catch(() => undefined);
      localTurnActive = false;
      localTurnStarted = false;
      sendAnchorRequest = null;
    }
  }

  async function selectChild(childId: string | null): Promise<void> {
    if (!active || !conversation || !active.nativeSessionId) return;
    setConversationSelectedChild(active.ownedId, childId);
    if (!childId) return;
    await readChildConversationTranscript({
      ownedId: active.ownedId,
      provider: conversation.provider,
      nativeSessionId: active.nativeSessionId,
      childSessionId: childId
    }).catch(() => undefined);
  }

  /** Cmd+V checks files first; text paste is untouched when there are no files. */
  async function paste(event: ClipboardEvent): Promise<void> {
    if (!active || !conversation || conversation.selectedChildId) return;
    const files = [...(event.clipboardData?.files ?? [])];
    if (files.length === 0) return;
    event.preventDefault();
    await attachImages(files, 'The clipboard file is not a supported image.');
  }

  /** Files dragged onto the composer take the same path as a paste. */
  async function dropFiles(files: File[]): Promise<void> {
    await attachImages(files, 'That file is not a supported image.');
  }

  async function attachImages(files: File[], rejectedMessage: string): Promise<void> {
    if (!active || !conversation || conversation.selectedChildId) return;
    const images = files.filter((file) => file.type.startsWith('image/'));
    attachmentError = '';
    if (images.length === 0) {
      attachmentError = rejectedMessage;
      return;
    }
    const saved = [];
    try {
      for (const file of images) saved.push(await saveConversationClipboardImage(active.ownedId, file));
      setConversationAttachments(active.ownedId, [...conversation.attachments, ...saved]);
    } catch (error) {
      await Promise.all(saved.map((attachment) => cleanupConversationAttachment(active.ownedId, attachment).catch(() => undefined)));
      attachmentError = error instanceof Error ? error.message : String(error);
      // Draft and existing attachments remain untouched after a failed paste.
    }
  }

  async function removeAttachment(id: string): Promise<void> {
    if (!active || !conversation) return;
    const found = conversation.attachments.find((item) => item.id === id);
    if (!found) return;
    try {
      await removeConversationAttachment(active.ownedId, found);
      attachmentError = '';
    } catch (error) {
      attachmentError = error instanceof Error ? error.message : String(error);
    }
  }

  function selectCommand(command: ConversationCommand): void {
    if (!active) return;
    if (appOwned && command.name === 'terminal') return;
    if (command.action === 'insert') {
      setConversationDraft(active.ownedId, `/${command.name} `);
      return;
    }
    if (command.name === 'terminal') setConversationMode(active.ownedId, 'raw');
    else if (command.name === 'conversation') setConversationMode(active.ownedId, 'structured');
    else setConversationDraft(active.ownedId, `/${command.name} `);
  }

  function onApprovalDecision(requestId: string, optionId: string): void {
    if (!active || !optionId) return;
    void sendPermissionResponse(active.ownedId, requestId, optionId).catch((error) => {
      attachmentError = error instanceof Error ? error.message : String(error);
    });
  }

  function onInputSubmit(requestId: string, values: Record<string, AgentConfigValue>, cancelled = false): void {
    if (!active) return;
    void respondToStructuredInput(active.ownedId, { requestId, values, cancelled }).catch((error) => {
      attachmentError = error instanceof Error ? error.message : String(error);
    });
  }

  function openConversationFile(path: string): void {
    const root = (active?.cwd || active?.projectPath || '').replace(/\/+$/, '');
    const candidate = path.trim();
    if (!root || !candidate || candidate.includes('\0') || candidate.split('/').includes('..')) {
      attachmentError = 'The file link is outside the active workspace.';
      return;
    }
    const absolute = candidate.startsWith('/') ? candidate : `${root}/${candidate.replace(/^\.\//, '')}`;
    if (absolute !== root && !absolute.startsWith(`${root}/`)) {
      attachmentError = 'The file link is outside the active workspace.';
      return;
    }
    requestOpenFile({ path: absolute, projectRoot: root });
  }

  async function changeConfig(field: AgentConversationConfigField, value: string): Promise<void> {
    if (!active || !conversation) return;
    const generation = conversation.generation;
    const previous = beginConversationAgentConfigChange(active.ownedId, field, value);
    if (!previous) return;
    const request: AgentConversationConfigRequest = {
      ownedId: active.ownedId,
      generation,
      [field]: value
    };
    try {
      const state = await setAgentConversationConfig(request);
      if (conversationSessions[active.ownedId]?.generation !== generation) throw new Error('Configuration response belongs to a stale conversation generation');
      confirmConversationAgentConfigChange(active.ownedId, field, state);
    } catch (error) {
      if (conversationSessions[active.ownedId]?.generation === generation) {
        failConversationAgentConfigChange(
          active.ownedId,
          field,
          previous,
          error instanceof Error ? error.message : String(error)
        );
      }
    }
  }
</script>

<div class="conversation-shell" data-testid="conversation-shell">
  <div class:covered={structured} class="terminal-layer"><TerminalSurface {owned} {activeOwnedId} {registerHost} {onHostLayout} /></div>
  {#if structured && active && conversation}
    <section class="structured" data-testid="structured-conversation" aria-label={`${active.agent} conversation`}>
      {#if !appOwned}
        <div class="handoff-actions" aria-label="Conversation handoff actions">
          <button type="button" data-testid="open-native-cli" onclick={() => void onOpenNativeCli?.(active.ownedId)}>
            Open in native CLI
          </button>
          {#if conversation.capabilities?.session.fork}
            <button type="button" data-testid="fork-native-cli" onclick={() => void onForkNativeCli?.(active.ownedId)}>
              Fork to native CLI
            </button>
          {/if}
          <button type="button" data-testid="conversation-open-raw" onclick={() => setConversationMode(active.ownedId, 'raw')}>
            Open raw terminal
          </button>
        </div>
      {/if}
      <ConversationAgentTree children={conversation.children} selectedChildId={conversation.selectedChildId} onSelect={(childId) => void selectChild(childId)} />
      <ConversationTimeline
        items={visibleTimeline}
        conversationId={active.ownedId}
        renderWindowId={`${active.ownedId}:${conversation.selectedChildId ?? 'root'}`}
        timelineRevision={conversation.timelineRevision}
        anchorRequest={sendAnchorRequest}
        activeTurnId={conversation.activeTurnId ?? null}
        {localTurnActive}
        {composerHeight}
        assistantLabel={selectedChild?.label ?? active.agent}
        savedScrollTop={conversation.selectedChildId ? conversation.childScrollTopById[conversation.selectedChildId] ?? 0 : conversation.scrollTop}
        emptyText={conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}
        hasOlder={!conversation.selectedChildId && !conversation.reachedTranscriptStart}
        loadingOlder={!conversation.selectedChildId && conversation.loadingOlder}
        onLoadOlder={() => void loadOlderConversationEvents(active.ownedId)}
        onScroll={(scrollTop) => {
          if (conversation.selectedChildId) conversation.childScrollTopById[conversation.selectedChildId] = scrollTop;
          else setConversationScrollTop(active.ownedId, scrollTop);
        }}
        onApprovalDecision={onApprovalDecision}
        onInputSubmit={onInputSubmit}
        onFileLink={openConversationFile}
      />
      {#if !conversation.selectedChildId}
        <ConversationComposer
          bind:this={composer}
          provider={active.agent}
          draft={conversation.draft}
          attachments={conversation.attachments}
          sending={conversation.sending}
          configState={conversation.agentConfig}
          pendingConfig={conversation.pendingAgentConfig}
          configError={conversation.agentConfigError}
          commands={commandCatalog}
          contextMeter={contextMeter}
          pendingApproval={pendingApprovals[0] ?? null}
          pendingApprovalCount={pendingApprovals.length}
          pendingInputs={pendingInputs}
          {attachmentError}
          {sendError}
          onDismissSendError={() => setConversationSendError(active.ownedId, '')}
          onDraftChange={(value) => {
            setConversationDraft(active.ownedId, value);
            persistConversationSessionDraft(active.ownedId, value);
          }}
          onDraftBlur={() => flushConversationSessionDraft(active.ownedId)}
          onSend={send}
          onStop={() => { if (activeOwnedId) void stopStructuredTurn(activeOwnedId).catch(() => undefined); }}
          onPaste={paste}
          onDropFiles={dropFiles}
          onRemoveAttachment={removeAttachment}
          onCommandSelected={selectCommand}
          onApprovalDecision={onApprovalDecision}
          onInputSubmit={onInputSubmit}
          onConfigChange={(optionId, value) => void changeConfig(optionId, value)}
          onHeightChange={(height) => (composerHeight = height)}
        />
      {/if}
    </section>
  {:else if active && (active.agent === 'codex' || active.agent === 'claude' || active.agent === 'antigravity') && conversation?.mode === 'raw'}
    <div class="raw-actions" aria-label="Conversation handoff actions">
      <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => void onReturnToStructured?.(active.ownedId)}>
        Return to structured
      </button>
    </div>
  {/if}
</div>

<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:transparent;color:var(--color-text);font:13px ui-sans-serif,system-ui}.handoff-actions{display:flex;gap:6px;align-items:center;padding:8px 12px;border-bottom:1px solid var(--color-border)}.handoff-actions button,.structured-toggle{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.handoff-actions button:hover,.structured-toggle:hover{background:var(--color-hover)}.raw-actions{position:absolute;right:12px;top:12px;z-index:2}</style>
