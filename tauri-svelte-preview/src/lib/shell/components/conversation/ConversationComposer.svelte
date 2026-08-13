<script lang="ts">
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import Paperclip from '@lucide/svelte/icons/paperclip';
  import Square from '@lucide/svelte/icons/square';
  import X from '@lucide/svelte/icons/x';
  import type { ConversationAttachment, AgentConfigValue, AgentPermissionRequest, AgentUserInputRequest } from '$lib/shell/conversation/conversationTypes.ts';
  import type { AgentConversationConfigField, AgentConversationConfigState } from '$lib/shell/conversation/conversationConfig.ts';
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import { draftAfterSlashCommand, moveSlashMenuIndex, slashMenuState } from '$lib/shell/conversation/composerSlashCommands.ts';
  import AgentCommandMenu from './AgentCommandMenu.svelte';
  import ComposerBannerStack, { type ComposerBannerItem } from './ComposerBannerStack.svelte';
  import ComposerConfigMenu from './ComposerConfigMenu.svelte';
  import ComposerPendingApprovalPanel from './ComposerPendingApprovalPanel.svelte';
  import ComposerPendingUserInputPanel from './ComposerPendingUserInputPanel.svelte';
  import CompactComposerControlsMenu from './CompactComposerControlsMenu.svelte';

  type ComposerAttachment = ConversationAttachment & { byteLength?: number };
  interface Props {
    provider: string;
    draft: string;
    attachments: readonly ComposerAttachment[];
    sending: boolean;
    configState: AgentConversationConfigState;
    pendingConfig: Partial<Record<AgentConversationConfigField, string>>;
    configError?: string | null;
    commands: readonly ConversationCommand[];
    contextRemainingPercent?: number | null;
    attachmentError?: string;
    pendingApproval?: AgentPermissionRequest | null;
    pendingApprovalCount?: number;
    pendingInputs?: readonly AgentUserInputRequest[];
    respondingRequestIds?: readonly string[];
    onDraftChange?(value: string): void;
    onSend?(): void | Promise<void>;
    onStop?(): void | Promise<void>;
    onPaste?(event: ClipboardEvent): void | Promise<void>;
    onDropFiles?(files: File[]): void | Promise<void>;
    onRemoveAttachment?(id: string): void | Promise<void>;
    onAnnotateAttachment?(id: string): void;
    onConfigChange?(field: AgentConversationConfigField, value: string): void | Promise<void>;
    onCommandSelected?(command: ConversationCommand): void;
    onApprovalDecision?(requestId: string, optionId: string): void | Promise<void>;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void | Promise<void>;
  }

  let {
    provider,
    draft,
    attachments,
    sending,
    configState,
    pendingConfig,
    configError = null,
    commands,
    contextRemainingPercent = null,
    attachmentError = '',
    pendingApproval = null,
    pendingApprovalCount = 1,
    pendingInputs = [],
    respondingRequestIds = [],
    onDraftChange,
    onSend,
    onStop,
    onPaste,
    onDropFiles,
    onRemoveAttachment,
    onAnnotateAttachment,
    onConfigChange,
    onCommandSelected,
    onApprovalDecision,
    onInputSubmit
  }: Props = $props();

  let activeIndex = $state(0);
  let dismissedDraft = $state<string | null>(null);
  let dragging = $state(false);
  let inputDraft = $state('');
  let observedDraft = $state('');
  let promptHost = $state<HTMLTextAreaElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (draft === observedDraft) return;
    observedDraft = draft;
    inputDraft = draft;
    activeIndex = 0;
    dismissedDraft = null;
    resizePrompt();
  });

  const menu = $derived(slashMenuState(inputDraft, commands, {
    activeIndex,
    dismissed: dismissedDraft === inputDraft
  }));
  const composerLocked = $derived(Boolean(pendingApproval || pendingInputs.length));
  const hasSendableContent = $derived(Boolean(inputDraft.trim() || attachments.length));
  const bannerItems = $derived.by((): ComposerBannerItem[] => {
    const items: ComposerBannerItem[] = [];
    if (attachmentError) items.push({ id: 'attachment-error', variant: 'error', title: 'Attachment unavailable', description: attachmentError });
    if (configError) items.push({ id: 'config-error', variant: 'warning', title: 'Settings unavailable', description: configError });
    return items;
  });

  function resizePrompt(): void {
    if (!promptHost) return;
    promptHost.style.height = 'auto';
    promptHost.style.height = `${Math.min(190, Math.max(52, promptHost.scrollHeight))}px`;
  }

  function chooseCommand(command: ConversationCommand | undefined): void {
    if (!command) return;
    activeIndex = 0;
    const nextDraft = draftAfterSlashCommand(inputDraft, command);
    inputDraft = nextDraft;
    onDraftChange?.(nextDraft);
    resizePrompt();
    onCommandSelected?.(command);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (menu.open) {
      if (event.key === 'Escape') {
        event.preventDefault();
        dismissedDraft = inputDraft;
        return;
      }
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        activeIndex = moveSlashMenuIndex(menu.activeIndex, menu.commands.length, event.key);
        return;
      }
      if (event.key === 'Enter' && !event.shiftKey && menu.commands.length) {
        event.preventDefault();
        chooseCommand(menu.commands[menu.activeIndex]);
        return;
      }
    }
    if (event.key === 'Enter' && !event.shiftKey && !composerLocked) {
      event.preventDefault();
      void onSend?.();
    }
  }

  function changeDraft(value: string): void {
    inputDraft = value;
    activeIndex = 0;
    dismissedDraft = null;
    onDraftChange?.(value);
    resizePrompt();
  }

  function imageFiles(transfer: DataTransfer | null): File[] {
    return [...(transfer?.files ?? [])];
  }

  function onDragOver(event: DragEvent): void {
    if (!onDropFiles || !event.dataTransfer?.types.includes('Files')) return;
    event.preventDefault();
    dragging = true;
  }

  function onDrop(event: DragEvent): void {
    if (!onDropFiles) return;
    const files = imageFiles(event.dataTransfer);
    dragging = false;
    if (!files.length) return;
    event.preventDefault();
    void onDropFiles(files);
  }

  function chooseFiles(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = '';
    if (files.length) void onDropFiles?.(files);
  }

  function byteLabel(attachment: ComposerAttachment): string {
    const size = attachment.byteLength;
    if (typeof size !== 'number') return attachment.mimeType;
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="composer-area" data-testid="conversation-composer-area">
  {#if bannerItems.length}<ComposerBannerStack items={bannerItems} />{/if}
  <form class="composer-form" onsubmit={(event) => { event.preventDefault(); if (!composerLocked) void onSend?.(); }}>
    <div
      class:dragging
      class:locked={composerLocked}
      class="composer-box"
      data-testid="conversation-composer-box"
      role="presentation"
      ondragover={onDragOver}
      ondragleave={() => (dragging = false)}
      ondrop={onDrop}
    >
      {#if pendingApproval}
        <ComposerPendingApprovalPanel
          approval={pendingApproval}
          pendingCount={pendingApprovalCount}
          responding={respondingRequestIds.includes(pendingApproval.requestId)}
          onDecision={onApprovalDecision}
        />
      {:else if pendingInputs.length}
        <ComposerPendingUserInputPanel
          requests={pendingInputs}
          responding={respondingRequestIds.length > 0}
          onSubmit={onInputSubmit}
        />
      {/if}

      <div class="composer-input-zone" data-testid="conversation-composer">
        {#if attachments.length}
          <div class="attachments" data-testid="conversation-attachment-previews">
            {#each attachments as attachment (attachment.id)}
              <figure data-testid="conversation-attachment-preview">
                <div class="attachment-preview">
                  <img src={attachment.previewUrl} alt={attachment.name} />
                  <button class="attachment-remove" data-testid="conversation-attachment-remove" aria-label={`Remove ${attachment.name}`} type="button" onclick={() => void onRemoveAttachment?.(attachment.id)}><X size={13} /></button>
                </div>
                {#if onAnnotateAttachment}<button class="attachment-annotate" data-testid="conversation-attachment-annotate" type="button" onclick={() => onAnnotateAttachment?.(attachment.id)}>Annotate</button>{/if}
                <figcaption><strong>{attachment.name}</strong><small>{byteLabel(attachment)}</small></figcaption>
              </figure>
            {/each}
          </div>
        {/if}
        {#if menu.open && !composerLocked}<AgentCommandMenu commands={menu.commands} activeIndex={menu.activeIndex} emptyText={menu.emptyText} onSelect={chooseCommand} />{/if}
        <div class="prompt-row">
          <textarea
            bind:this={promptHost}
            data-testid="conversation-composer-input"
            aria-label="Message"
            placeholder={pendingApproval ? 'Resolve the approval above to continue' : pendingInputs.length ? 'Complete the requested input above' : `Message ${provider}`}
            value={inputDraft}
            disabled={composerLocked}
            onpaste={(event) => void onPaste?.(event)}
            oninput={(event) => changeDraft(event.currentTarget.value)}
            onkeydown={onKeydown}
          ></textarea>
        </div>
      </div>

      {#if !composerLocked}
        <div class="composer-footer" data-testid="conversation-composer-footer">
          <div class="footer-left">
            <input bind:this={fileInput} class="file-input" type="file" accept="image/*" multiple onchange={chooseFiles} />
            <button class="attach-control" type="button" aria-label="Attach images" onclick={() => fileInput?.click()}><Paperclip size={15} /><span>Attach</span></button>
            <div class="wide-controls"><ComposerConfigMenu {provider} state={configState} pending={pendingConfig} error={configError} onChange={onConfigChange} /></div>
            <div class="compact-controls"><CompactComposerControlsMenu {provider} state={configState} pending={pendingConfig} onChange={onConfigChange} /></div>
          </div>
          <div class="footer-right">
            {#if contextRemainingPercent !== null}<span class="context-remaining" data-testid="conversation-context-remaining" title="Reported context remaining">{contextRemainingPercent}% left</span>{/if}
            {#if sending}
              <button class="send stop" data-testid="conversation-stop" type="button" aria-label="Stop generation" onclick={() => void onStop?.()}><Square size={13} fill="currentColor" /></button>
            {:else}
              <button class="send" data-testid="conversation-send" type="submit" aria-label="Send message" disabled={!hasSendableContent}><ArrowUp size={16} strokeWidth={2.2} /></button>
            {/if}
          </div>
        </div>
      {/if}
      {#if dragging}<p class="drop-hint" data-testid="conversation-drop-hint">Drop images to attach them</p>{/if}
    </div>
  </form>
  <div class="composer-hint" data-testid="conversation-paste-hint"><span>Paste or drop images · type / for commands</span>{#if contextRemainingPercent !== null}<span aria-hidden="true">·</span><span>{contextRemainingPercent}% context left</span>{/if}</div>
</div>

<style>
  .composer-area { position: absolute; left: 50%; bottom: 0; z-index: 2; width: min(100%, 900px); transform: translateX(-50%); padding: 12px 0 17px; container-type: inline-size; container-name: composer; background: linear-gradient(transparent, var(--color-bg) 22%, var(--color-bg)); }
  .composer-form { width: min(820px, calc(100% - 44px)); margin: 0 auto; }
  .composer-box { position: relative; display: flex; flex-direction: column; overflow: visible; border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent); border-radius: 22px; background: color-mix(in srgb, var(--color-surface) 88%, var(--color-bg)); box-shadow: var(--shadow-md); }
  .composer-box:focus-within { border-color: color-mix(in srgb, var(--color-text) 28%, var(--color-border)); box-shadow: var(--shadow-lg); }
  .composer-box.dragging { border-color: var(--color-accent); background: color-mix(in srgb, var(--color-accent) 8%, var(--color-surface)); }
  .composer-box.locked { border-color: color-mix(in srgb, var(--color-attention) 30%, var(--color-border)); }
  .composer-input-zone { position: relative; padding: 14px 16px 8px; }
  .prompt-row { display: flex; min-height: 56px; }
  textarea { width: 100%; min-height: 52px; max-height: 190px; padding: 2px 0; resize: none; border: 0; outline: 0; background: transparent; color: var(--color-text); caret-color: var(--color-accent); font: 14px/1.5 inherit; }
  textarea::placeholder { color: var(--color-text-3); }
  textarea:disabled { cursor: not-allowed; opacity: .6; }
  .composer-footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 40px; padding: 0 12px 12px 14px; }
  .footer-left, .footer-right { display: flex; min-width: 0; align-items: center; gap: 7px; }
  .footer-left { flex: 1; overflow: hidden; }
  .footer-right { flex: none; }
  .file-input { position: absolute; width: 1px; height: 1px; overflow: hidden; opacity: 0; pointer-events: none; }
  .attach-control { display: inline-flex; align-items: center; gap: 5px; flex: none; min-height: 28px; padding: 5px 8px; border: 0; border-radius: 8px; background: transparent; color: var(--color-text-2); font: 13px/1.2 inherit; cursor: pointer; }
  .attach-control:hover { background: var(--color-hover); color: var(--color-text); }
  .attach-control:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 1px; }
  .wide-controls { min-width: 0; overflow: hidden; }
  .compact-controls { display: none; }
  .send { display: inline-grid; place-items: center; flex: none; width: 32px; height: 32px; border: 0; border-radius: 999px; background: var(--color-accent); color: var(--color-on-accent); cursor: pointer; box-shadow: var(--shadow-sm); }
  .send:hover:not(:disabled) { filter: brightness(1.06); transform: translateY(-1px); }
  .send:disabled { cursor: default; opacity: .32; box-shadow: none; }
  .send.stop { background: var(--color-bad); color: var(--color-on-accent); }
  .send:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 2px; }
  .context-remaining { flex: none; padding: 4px 7px; border: 1px solid color-mix(in srgb, var(--color-border) 74%, transparent); border-radius: 999px; color: var(--color-text-2); font-size: 12px; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .attachments { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 10px; }
  .attachments figure { display: grid; grid-template-columns: 60px minmax(0, 1fr); column-gap: 8px; min-width: 210px; max-width: 280px; margin: 0; padding: 6px; border: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent); border-radius: 11px; background: color-mix(in srgb, var(--color-bg) 62%, var(--color-surface)); }
  .attachment-preview { position: relative; grid-row: span 2; width: 60px; height: 52px; }
  .attachment-preview img { width: 100%; height: 100%; object-fit: cover; border-radius: 7px; }
  .attachment-remove { position: absolute; top: 3px; right: 3px; display: grid; place-items: center; width: 20px; height: 20px; border: 0; border-radius: 999px; background: color-mix(in srgb, var(--color-bg) 88%, transparent); color: var(--color-text); cursor: pointer; }
  .attachment-remove:hover { background: var(--color-bad-bg); color: var(--color-bad); }
  .attachment-remove:focus-visible, .attachment-annotate:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: 1px; }
  .attachments figcaption { display: grid; align-content: center; min-width: 0; gap: 2px; }
  .attachments figcaption strong { overflow: hidden; color: var(--color-text); font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .attachments figcaption small { color: var(--color-text-2); font-size: 12px; }
  .attachment-annotate { grid-column: 2; justify-self: start; border: 0; background: transparent; color: var(--color-accent); padding: 1px 0; font-size: 12px; cursor: pointer; }
  .drop-hint { margin: 0; padding: 0 16px 10px; color: var(--color-accent); font-size: 13px; }
  .composer-hint { display: flex; justify-content: flex-end; gap: 8px; width: min(820px, calc(100% - 44px)); margin: 5px auto 0; color: var(--color-text-3); font-size: 12px; }
  @container composer (max-width: 680px) { .wide-controls { display: none; } .compact-controls { display: block; } .attach-control span { display: none; } .composer-footer { gap: 7px; } }
  @media (prefers-reduced-motion: no-preference) { .composer-box, .send { transition: border-color .14s ease, background-color .14s ease, filter .14s ease, transform .14s ease, opacity .14s ease; } }
</style>
