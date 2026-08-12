<script lang="ts">
  /**
   * SessionBrowserOverlay.svelte — the session's browser, over the whole window.
   *
   * It covers everything, the sessions rail included, because browsing is what
   * you are doing while you are doing it. What it shows belongs to the active
   * session: the address, the notes drawn on the page, and whether it is open
   * at all are all keyed by session id, so switching sessions swaps the whole
   * overlay and switching back brings it all straight back.
   *
   * The session's own composer floats along the bottom. It is the real
   * component from the conversation surface, placed here rather than copied,
   * so a message typed over a page is the same message typed in the transcript.
   */
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import SquarePen from '@lucide/svelte/icons/square-pen';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import ConversationComposer from '$lib/shell/components/conversation/ConversationComposer.svelte';
  import SessionBrowserAnnotationLayer from './SessionBrowserAnnotationLayer.svelte';
  import {
    annotationCountLabel,
    composeAnnotationMessage,
    overlayHeaderLabel,
    sendButtonLabel,
    type SessionBrowserRect
  } from './sessionBrowserOps.ts';
  import {
    addSessionBrowserAnnotation,
    clearSessionBrowserAnnotations,
    closeSessionBrowserOverlay,
    removeSessionBrowserAnnotation,
    sessionBrowserView,
    setSessionBrowserAddress,
    setSessionBrowserAnnotateMode,
    stepSessionBrowserAddress
  } from './sessionBrowserState.svelte.ts';
  import {
    activateBrowser,
    browser,
    browserModelContext,
    deactivateBrowserWorkspace,
    reloadBrowserFrame,
    setBrowserUrl
  } from './browserStore.svelte.ts';
  import { setBrowserPresentationMode } from './browserModel.ts';
  import { isTauriRuntime } from '../../tauriSource.ts';
  import { normalizeBrowserUrl } from './normalizeBrowserUrl.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import {
    conversationSessions,
    setConversationAttachments,
    setConversationDraft
  } from '$lib/shell/conversation/conversationStore.svelte';
  import {
    cleanupConversationAttachment,
    removeConversationAttachment,
    saveConversationClipboardImage,
    sendStructuredMessage,
    stopStructuredTurn
  } from '$lib/shell/conversation/conversationService';
  import {
    mergeConversationCommandCatalog,
    type ConversationCommand
  } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import type { AgentConversationConfigField } from '$lib/shell/conversation/conversationConfig.ts';
  import { setAgentConversationConfig } from '$lib/shell/conversation/conversationConfig.ts';
  import { remainingContextPercent } from '$lib/shell/conversation/composerSlashCommands.ts';

  interface Props {
    /** Return the center workbench to its previous surface after closing. */
    onClose?: () => void;
  }

  let { onClose }: Props = $props();

  const sessionId = $derived(rail.activeOwnedId);
  const session = $derived(rail.owned.find((item) => item.ownedId === sessionId) ?? null);
  const view = $derived(sessionBrowserView(sessionId));
  const conversation = $derived(sessionId ? conversationSessions[sessionId] ?? null : null);
  const commandCatalog = $derived(
    mergeConversationCommandCatalog(conversation?.availableCommands ?? conversation?.capabilities?.commands ?? [])
  );
  const remainingContext = $derived(
    conversation?.provider === 'codex'
      ? remainingContextPercent(conversation.metadata.usedTokens, conversation.metadata.contextWindow)
      : null
  );
  const noteCount = $derived(view.annotations.length);

  let address = $state('');
  let addressSession = $state<string | null>(null);
  let attachmentError = $state('');
  let reloadVersion = $state(0);
  let lastSyncedBrowserTarget = '';

  const nativeBrowser = isTauriRuntime();

  /** The address box follows whichever session is showing, without an effect:
   * a session change is simply a different key, so the field is re-seeded. */
  const addressValue = $derived.by(() => {
    if (addressSession !== sessionId) return view.url;
    return address;
  });

  function editAddress(value: string): void {
    addressSession = sessionId;
    address = value;
  }

  function navigate(): void {
    const next = normalizeBrowserUrl(addressValue);
    if (!next) return;
    setSessionBrowserAddress(sessionId, next);
    addressSession = null;
  }

  function close(): void {
    closeSessionBrowserOverlay(sessionId);
    lastSyncedBrowserTarget = '';
    if (nativeBrowser) deactivateBrowserWorkspace();
    onClose?.();
  }

  function stepHistory(direction: 'back' | 'forward'): void {
    stepSessionBrowserAddress(sessionId, direction);
  }

  function reload(): void {
    reloadVersion += 1;
    if (nativeBrowser) reloadBrowserFrame();
  }

  function toggleAnnotating(): void {
    setSessionBrowserAnnotateMode(sessionId, !view.annotating);
  }

  function saveAnnotation(rect: SessionBrowserRect, comment: string): void {
    addSessionBrowserAnnotation(sessionId, { rect, comment });
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    close();
  }

  /** One send carries the typed prompt and every note as a single message. */
  async function send(): Promise<void> {
    if (!sessionId || !conversation || conversation.sending) return;
    const message = composeAnnotationMessage(view, conversation.draft);
    if (!message && conversation.attachments.length === 0) return;
    setConversationDraft(sessionId, '');
    try {
      await sendStructuredMessage(sessionId, message, session?.ptySessionId);
      clearSessionBrowserAnnotations(sessionId);
    } catch {
      // Put the draft back so nothing typed over a page is lost. The notes stay
      // where they are, which is what lets the send be tried again.
      setConversationDraft(sessionId, conversation.draft);
    }
  }

  async function paste(event: ClipboardEvent): Promise<void> {
    if (!sessionId || !conversation) return;
    const files = [...(event.clipboardData?.files ?? [])];
    if (files.length === 0) return;
    event.preventDefault();
    const images = files.filter((file) => file.type.startsWith('image/'));
    attachmentError = '';
    if (images.length === 0) {
      attachmentError = 'The clipboard file is not a supported image.';
      return;
    }
    const saved = [];
    try {
      for (const file of images) saved.push(await saveConversationClipboardImage(sessionId, file));
      setConversationAttachments(sessionId, [...conversation.attachments, ...saved]);
    } catch (error) {
      await Promise.all(saved.map((item) => cleanupConversationAttachment(sessionId, item).catch(() => undefined)));
      attachmentError = error instanceof Error ? error.message : String(error);
    }
  }

  async function removeAttachment(id: string): Promise<void> {
    if (!sessionId || !conversation) return;
    const found = conversation.attachments.find((item) => item.id === id);
    if (!found) return;
    try {
      await removeConversationAttachment(sessionId, found);
      attachmentError = '';
    } catch (error) {
      attachmentError = error instanceof Error ? error.message : String(error);
    }
  }

  function editDraft(value: string): void {
    if (sessionId) setConversationDraft(sessionId, value);
  }

  function selectCommand(command: ConversationCommand): void {
    if (!sessionId) return;
    setConversationDraft(sessionId, `/${command.name} `);
  }

  async function changeConfig(field: AgentConversationConfigField, value: string): Promise<void> {
    if (!sessionId || !conversation) return;
    try {
      await setAgentConversationConfig({ ownedId: sessionId, generation: conversation.generation, [field]: value });
    } catch (error) {
      attachmentError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * The browser preview uses the iframe below. In the desktop build the same
   * address is handed to the native child webview and its bounds are kept in
   * the page rectangle. The session store remains authoritative in both
   * cases, so switching sessions never leaks an address or annotation.
   */
  function syncBrowserSurface(): void {
    if (!sessionId || !view.open || !view.url) return;
    const target = `${sessionId}:${view.url}`;
    if (target === lastSyncedBrowserTarget) return;
    lastSyncedBrowserTarget = target;
    browser.workspace.ownedId = sessionId;
    activateBrowser();
    const active = browser.workspace.activeTabId
      ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
      : null;
    if (!active || active.url !== view.url) setBrowserUrl(view.url);
    if (nativeBrowser && typeof window !== 'undefined') {
      const pageTop = 44;
      const composerReserve = 190;
      setBrowserPresentationMode(browserModelContext(), 'maximized', {
        window: { width: window.innerWidth, height: window.innerHeight },
        bounds: {
          x: 0,
          y: pageTop,
          width: window.innerWidth,
          height: Math.max(160, window.innerHeight - pageTop - composerReserve)
        }
      });
    }
  }

  $effect(() => {
    sessionId;
    view.open;
    view.url;
    syncBrowserSurface();
  });
</script>

<svelte:window onkeydown={view.open ? handleKeydown : undefined} />

{#if view.open}
  <section
    class="session-browser-overlay"
    aria-label="Session browser"
    data-testid="session-browser-overlay"
  >
    <header class="overlay-header">
      <Globe2 class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span class="header-label" data-testid="session-browser-header-label">{overlayHeaderLabel(view)}</span>

      <form
        class="address-form"
        onsubmit={(event) => {
          event.preventDefault();
          navigate();
        }}
      >
        <Input
          class="address-input"
          aria-label="Address"
          placeholder="Enter an http or https address"
          value={addressValue}
          oninput={(event) => editAddress(event.currentTarget.value)}
        />
      </form>

      <div class="navigation-controls" aria-label="Page navigation">
        <IconButton
          label="Go back"
          data-testid="session-browser-back"
          disabled={view.historyIndex <= 0}
          onclick={() => stepHistory('back')}
        >
          <ArrowLeft aria-hidden="true" />
        </IconButton>
        <IconButton
          label="Go forward"
          data-testid="session-browser-forward"
          disabled={view.historyIndex < 0 || view.historyIndex >= view.history.length - 1}
          onclick={() => stepHistory('forward')}
        >
          <ArrowRight aria-hidden="true" />
        </IconButton>
        <IconButton label="Reload page" data-testid="session-browser-reload" onclick={reload}>
          <RotateCw aria-hidden="true" />
        </IconButton>
      </div>

      <button
        type="button"
        class="annotate-chip"
        class:on={view.annotating}
        aria-pressed={view.annotating}
        data-testid="session-browser-annotate-chip"
        onclick={toggleAnnotating}
      >
        <SquarePen class="size-3.5" aria-hidden="true" />
        Annotating
      </button>

      {#if noteCount > 0}
        <span class="count-chip" data-testid="session-browser-annotation-count">
          {annotationCountLabel(noteCount)}
        </span>
        <IconButton
          label="Discard every annotation"
          onclick={() => clearSessionBrowserAnnotations(sessionId)}
        >
          <Trash2 aria-hidden="true" />
        </IconButton>
      {/if}

      <Button
        data-testid="session-browser-send"
        disabled={!conversation || conversation.sending || (noteCount === 0 && !conversation?.draft.trim())}
        onclick={() => void send()}
      >
        {sendButtonLabel(noteCount)}
      </Button>

      <IconButton label="Close the browser overlay" onclick={close}>
        <X aria-hidden="true" />
      </IconButton>
    </header>

    {#if browser.error}
      <p class="overlay-error" role="alert">{browser.error}</p>
    {/if}

    <div class="overlay-body">
      <div class="page-region" data-testid="session-browser-page">
        {#if view.url}
          {#if nativeBrowser}
            <div class="native-page-host" data-testid="session-browser-native-page" aria-hidden="true"></div>
          {:else}
            {#key `${view.url}:${reloadVersion}`}
              <iframe
                class="browser-frame"
                data-testid="session-browser-frame"
                title="Session browser page"
                src={view.url}
                sandbox="allow-downloads allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
                referrerpolicy="no-referrer"
              ></iframe>
            {/key}
          {/if}
        {:else}
          <div class="page-placeholder">
            <strong>No address yet</strong>
            <span>Enter an http or https address above to open a page.</span>
          </div>
        {/if}
      </div>
      <SessionBrowserAnnotationLayer
        annotating={view.annotating}
        annotations={view.annotations}
        onSave={saveAnnotation}
        onRemove={(id) => removeSessionBrowserAnnotation(sessionId, id)}
      />
      {#if conversation && session}
        <ConversationComposer
          provider={session.agent}
          draft={conversation.draft}
          attachments={conversation.attachments}
          sending={conversation.sending}
          configState={conversation.agentConfig}
          pendingConfig={conversation.pendingAgentConfig}
          configError={conversation.agentConfigError}
          commands={commandCatalog}
          contextRemainingPercent={remainingContext}
          {attachmentError}
          onDraftChange={editDraft}
          onSend={send}
          onStop={() => { if (sessionId) void stopStructuredTurn(sessionId).catch(() => undefined); }}
          onPaste={paste}
          onRemoveAttachment={removeAttachment}
          onCommandSelected={selectCommand}
          onConfigChange={(field, value) => void changeConfig(field, value)}
        />
      {/if}
    </div>
  </section>
{/if}

<style>
  /* Above every other floating owner in the shell: the rail popovers sit at
     70, and this deliberately covers the sessions column too. */
  .session-browser-overlay {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    flex-direction: column;
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 13px;
  }

  @media (prefers-reduced-motion: no-preference) {
    .session-browser-overlay {
      animation: overlay-in 0.14s ease-out;
    }

    @keyframes overlay-in {
      from {
        opacity: 0;
      }
      to {
        opacity: 1;
      }
    }
  }

  .overlay-header {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .header-label {
    flex: 0 0 auto;
    max-width: 320px;
    overflow: hidden;
    color: var(--color-text-2);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .address-form {
    flex: 1 1 auto;
    min-width: 0;
  }

  .navigation-controls {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 2px;
  }

  .overlay-header :global(.address-input) {
    width: 100%;
    font-family: ui-monospace, Menlo, monospace;
  }

  .annotate-chip {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .annotate-chip:hover {
    background: var(--color-hover);
  }

  .annotate-chip.on {
    border-color: var(--color-accent);
    background: var(--color-selected);
    color: var(--color-text);
  }

  .annotate-chip:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 2px;
  }

  .count-chip {
    flex: 0 0 auto;
    padding: 3px 9px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-elevated);
    color: var(--color-text-2);
    font-size: 13px;
  }

  .overlay-error {
    flex: 0 0 auto;
    margin: 0;
    padding: 7px 10px;
    border-bottom: 1px solid var(--color-bad);
    background: var(--color-bad-bg);
    color: var(--color-bad);
    font-size: 13px;
  }

  /* The composer positions itself against this box, so it has to be the
     positioned ancestor. */
  .overlay-body {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .page-region {
    position: absolute;
    inset: 0;
    background: var(--color-surface);
  }

  .browser-frame,
  .native-page-host {
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
    background: var(--color-surface);
  }

  .native-page-host {
    pointer-events: none;
  }

  .page-placeholder {
    display: flex;
    width: 100%;
    height: 100%;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    max-width: 520px;
    padding: 32px;
    color: var(--color-text-2);
    text-align: center;
    font-size: 13px;
  }

  .page-placeholder strong {
    color: var(--color-text);
    font-family: ui-monospace, Menlo, monospace;
  }
</style>
