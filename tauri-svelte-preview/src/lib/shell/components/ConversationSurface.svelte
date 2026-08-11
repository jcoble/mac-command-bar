<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import TerminalSurface from './TerminalSurface.svelte';
  import ConversationHeader from './conversation/ConversationHeader.svelte';
  import ConversationInspector from './conversation/ConversationInspector.svelte';
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
    setConversationMode,
    setConversationScrollTop,
    setConversationSelectedChild
  } from '$lib/shell/conversation/conversationStore.svelte';
  import {
    cleanupConversationAttachment,
    loadConversationCapabilities,
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
    displayItemFromApproval,
    displayItemFromInput,
    typedConversationTimeline,
    type ConversationDisplayItem
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import { requestOpenFile } from '$lib/shell/openFileBus.ts';
  import { clearViewedSession, setViewedSession } from '$lib/shell/conversation/sessionPresence.ts';
  import { requestSessionRestart } from '$lib/shell/conversation/sessionRestart.ts';

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
  const structured = $derived(!!active && (active.agent === 'codex' || active.agent === 'claude') && (appOwned || conversation?.mode !== 'raw'));
  const selectedChild = $derived(conversation && conversation.selectedChildId
    ? conversation.children.find((child) => child.childId === conversation.selectedChildId) ?? null
    : null);
  const legacyTimeline = $derived(conversation?.selectedChildId ? conversation.childTimeline : conversation?.timeline ?? []);
  const visibleTimeline = $derived.by((): ConversationDisplayItem[] => {
    if (!conversation) return [];
    if (conversation.selectedChildId) return typedConversationTimeline([], legacyTimeline);
    const items = typedConversationTimeline(conversation.agentItems, legacyTimeline);
    const now = items.reduce((latest, item) => Math.max(latest, item.timestampMs), 0) + 1;
    const typedKinds = new Set(items.map((item) => item.kind));
    if (conversation.planSteps.length && !typedKinds.has('plan')) {
      items.push({ kind: 'plan', itemId: 'plan:current', title: 'Plan', steps: conversation.planSteps, timestampMs: now });
    }
    if (conversation.tasks.length && !typedKinds.has('tasks')) {
      items.push({ kind: 'tasks', itemId: 'tasks:current', title: 'Tasks', tasks: conversation.tasks, timestampMs: now });
    }
    for (const request of Object.values(conversation.pendingApprovals)) {
      if (!items.some((item) => item.itemId === `approval:${request.requestId}`)) items.push(displayItemFromApproval(request, now));
    }
    for (const request of Object.values(conversation.pendingInputs)) {
      if (!items.some((item) => item.itemId === `input:${request.requestId}`)) items.push(displayItemFromInput(request, now));
    }
    return items.sort((left, right) => left.timestampMs - right.timestampMs);
  });
  const commandCatalog = $derived(mergeConversationCommandCatalog(conversation?.availableCommands ?? conversation?.capabilities?.commands ?? []).filter((command) => !appOwned || command.name !== 'terminal'));
  const remainingContext = $derived.by(() => {
    const used = conversation?.metadata.usedTokens;
    const window = conversation?.metadata.contextWindow;
    if (used == null || window == null || window <= 0) return null;
    return Math.max(0, Math.round(((window - used) / window) * 100));
  });

  let attachmentError = $state('');
  let capabilityRequest = $state('');
  let configRequest = $state('');
  let inspectorOpen = $state(false);

  $effect(() => {
    const ownedId = activeOwnedId;
    setViewedSession(ownedId);
    if (!ownedId) return;
    return () => clearViewedSession(ownedId);
  });

  $effect(() => {
    if (!appOwned) inspectorOpen = false;
    if (appOwned && activeOwnedId && conversation?.mode === 'raw') {
      setConversationMode(activeOwnedId, 'structured');
    }
  });

  $effect(() => {
    if (!structured || !active || !conversation || (active.agent !== 'claude' && active.agent !== 'codex')) return;
    const ownedId = active.ownedId;
    const generation = conversation.generation;
    const key = `${ownedId}:${generation}`;
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
    });
  });

  $effect(() => {
    if (!structured || !active || !conversation || (active.agent !== 'claude' && active.agent !== 'codex')) return;
    const key = `${active.ownedId}:${conversation.generation}:${conversation.provider}`;
    if (conversation.capabilities || capabilityRequest === key) return;
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
    if (!activeOwnedId || !conversation || conversation.sending || conversation.selectedChildId) return;
    if (!conversation.draft.trim() && conversation.attachments.length === 0) return;
    const text = conversation.draft;
    setConversationDraft(activeOwnedId, '');
    try {
      await sendStructuredMessage(activeOwnedId, text, active?.ptySessionId);
    } catch {
      // Restore the draft. The service intentionally leaves attachments in the
      // store on every failure, so the user can retry without data loss.
      setConversationDraft(activeOwnedId, text);
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
      <ConversationHeader
        {active}
        {conversation}
        {selectedChild}
        onRestart={() => requestSessionRestart(active.ownedId, active.state === 'exited')}
        onModeChange={(mode) => { if (!appOwned) setConversationMode(active.ownedId, mode); }}
        onInspectorToggle={() => { if (appOwned) inspectorOpen = !inspectorOpen; }}
      />
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
        </div>
      {/if}
      <ConversationAgentTree children={conversation.children} selectedChildId={conversation.selectedChildId} onSelect={(childId) => void selectChild(childId)} />
      <ConversationTimeline
        items={visibleTimeline}
        assistantLabel={selectedChild?.label ?? active.agent}
        savedScrollTop={conversation.selectedChildId ? conversation.childScrollTopById[conversation.selectedChildId] ?? 0 : conversation.scrollTop}
        emptyText={conversation.selectedChildId ? 'This sub-agent transcript is not available yet.' : 'Start the conversation below.'}
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
          provider={active.agent}
          draft={conversation.draft}
          attachments={conversation.attachments}
          sending={conversation.sending}
          configState={conversation.agentConfig}
          pendingConfig={conversation.pendingAgentConfig}
          configError={conversation.agentConfigError}
          commands={commandCatalog}
          {attachmentError}
          onDraftChange={(value) => setConversationDraft(active.ownedId, value)}
          onSend={send}
          onStop={() => { if (activeOwnedId) void stopStructuredTurn(activeOwnedId).catch(() => undefined); }}
          onPaste={paste}
          onDropFiles={dropFiles}
          onRemoveAttachment={removeAttachment}
          onCommandSelected={selectCommand}
          onConfigChange={(optionId, value) => void changeConfig(optionId, value)}
        />
      {:else}<div class="read-only-note" data-testid="conversation-read-only-note">Read-only sub-agent transcript</div>{/if}
      {#if appOwned && inspectorOpen}<ConversationInspector ownedId={active.ownedId} />{/if}
    </section>
  {:else if active && (active.agent === 'codex' || active.agent === 'claude') && conversation?.mode === 'raw'}
    <div class="raw-actions" aria-label="Conversation handoff actions">
      <button class="structured-toggle" data-testid="conversation-structured-toggle" type="button" onclick={() => void onReturnToStructured?.(active.ownedId)}>
        Return to structured
      </button>
    </div>
  {/if}
</div>

<style>.conversation-shell,.terminal-layer,.structured{position:relative;width:100%;height:100%;min-height:0}.terminal-layer.covered{visibility:hidden}.structured{position:absolute;inset:0;display:flex;flex-direction:column;background:var(--color-bg);color:var(--color-text);font:13px ui-sans-serif,system-ui}.handoff-actions{display:flex;gap:6px;align-items:center;padding:8px 12px;border-bottom:1px solid var(--color-border)}.handoff-actions button,.structured-toggle{border:0;border-radius:7px;background:var(--color-elevated);color:inherit;padding:6px 9px}.handoff-actions button:hover,.structured-toggle:hover{background:var(--color-hover)}.raw-actions{position:absolute;right:12px;top:12px;z-index:2}.read-only-note{padding:10px;text-align:center;border-top:1px solid var(--color-border);color:var(--color-text-2);font-size:12px}</style>
