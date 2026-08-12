<script lang="ts">
  import type { ConversationAttachment } from '$lib/shell/conversation/conversationTypes.ts';
  import type {
    AgentConversationConfigField,
    AgentConversationConfigState
  } from '$lib/shell/conversation/conversationConfig.ts';
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import { draftAfterSlashCommand, moveSlashMenuIndex, slashMenuState } from '$lib/shell/conversation/composerSlashCommands.ts';
  import AgentCommandMenu from './AgentCommandMenu.svelte';
  import ComposerConfigMenu from './ComposerConfigMenu.svelte';

  type ComposerAttachment = ConversationAttachment & { byteLength?: number };
  interface Props {
    provider: string;
    draft: string;
    attachments: readonly ComposerAttachment[];
    sending: boolean;
    configState: AgentConversationConfigState;
    pendingConfig: Partial<Record<AgentConversationConfigField, string>>;
    configError?: string | null;
    /** Everything the agent and Assembly advertise; the composer filters it. */
    commands: readonly ConversationCommand[];
    contextRemainingPercent?: number | null;
    attachmentError?: string;
    onDraftChange?(value: string): void;
    onSend?(): void | Promise<void>;
    onStop?(): void | Promise<void>;
    onPaste?(event: ClipboardEvent): void | Promise<void>;
    onDropFiles?(files: File[]): void | Promise<void>;
    onRemoveAttachment?(id: string): void | Promise<void>;
    onAnnotateAttachment?(id: string): void;
    onConfigChange?(field: AgentConversationConfigField, value: string): void | Promise<void>;
    onCommandSelected?(command: ConversationCommand): void;
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
    onDraftChange,
    onSend,
    onStop,
    onPaste,
    onDropFiles,
    onRemoveAttachment,
    onAnnotateAttachment,
    onConfigChange,
    onCommandSelected
  }: Props = $props();

  let activeIndex = $state(0);
  let dismissedDraft = $state<string | null>(null);
  let dragging = $state(false);
  let inputDraft = $state('');
  let observedDraft = $state('');
  $effect(() => {
    if (draft === observedDraft) return;
    observedDraft = draft;
    inputDraft = draft;
    activeIndex = 0;
    dismissedDraft = null;
  });
  const menu = $derived(slashMenuState(inputDraft, commands, {
    activeIndex,
    dismissed: dismissedDraft === inputDraft
  }));

  function chooseCommand(command: ConversationCommand | undefined): void {
    if (!command) return;
    activeIndex = 0;
    inputDraft = draftAfterSlashCommand(inputDraft, command);
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
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void onSend?.();
    }
  }

  function changeDraft(value: string): void {
    inputDraft = value;
    activeIndex = 0;
    dismissedDraft = null;
    onDraftChange?.(value);
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

  function byteLabel(attachment: ComposerAttachment): string {
    const size = attachment.byteLength;
    if (typeof size !== 'number') return attachment.mimeType;
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="composer-area" data-testid="conversation-composer-area">
  {#if attachments.length}<div class="attachments" data-testid="conversation-attachment-previews">{#each attachments as attachment (attachment.id)}<figure data-testid="conversation-attachment-preview"><img src={attachment.previewUrl} alt={attachment.name} /><div class="attachment-actions"><button data-testid="conversation-attachment-remove" aria-label={`Remove ${attachment.name}`} type="button" onclick={() => void onRemoveAttachment?.(attachment.id)}>Remove</button>{#if onAnnotateAttachment}<button data-testid="conversation-attachment-annotate" type="button" onclick={() => onAnnotateAttachment?.(attachment.id)}>Annotate</button>{/if}</div><figcaption><strong>{attachment.name}</strong><small>{byteLabel(attachment)}</small></figcaption></figure>{/each}</div>{/if}
  {#if attachmentError}<div class="attachment-error" data-testid="conversation-attachment-error" role="alert">{attachmentError}</div>{/if}
  {#if menu.open}<AgentCommandMenu commands={menu.commands} activeIndex={menu.activeIndex} emptyText={menu.emptyText} onSelect={chooseCommand} />{/if}
  <div
    class:dragging
    class="composer-box"
    data-testid="conversation-composer-box"
    ondragover={onDragOver}
    ondragleave={() => (dragging = false)}
    ondrop={onDrop}
    role="presentation"
  >
    <div class="composer-row" data-testid="conversation-composer">
      <textarea data-testid="conversation-composer-input" aria-label="Message" placeholder={`Message ${provider}`} value={inputDraft} onpaste={(event) => void onPaste?.(event)} oninput={(event) => changeDraft(event.currentTarget.value)} onkeydown={onKeydown}></textarea>
      {#if sending}<button class="send stop" data-testid="conversation-stop" type="button" onclick={() => void onStop?.()}>Stop</button>{:else}<button class="send" data-testid="conversation-send" type="button" disabled={!inputDraft.trim() && attachments.length === 0} onclick={() => void onSend?.()}>Send</button>{/if}
    </div>
    <ComposerConfigMenu {provider} state={configState} pending={pendingConfig} error={configError} onChange={onConfigChange} />
    {#if dragging}<p class="drop-hint" data-testid="conversation-drop-hint">Drop images to attach them</p>{/if}
  </div>
  <div class="composer-hint" data-testid="conversation-paste-hint"><span>Paste or drop images · type / for commands</span>{#if contextRemainingPercent !== null}<span class="context-remaining" data-testid="conversation-context-remaining" title="Reported context remaining">Context {contextRemainingPercent}% left</span>{/if}</div>
</div>

<style>.composer-area{position:absolute;left:50%;bottom:0;transform:translateX(-50%);z-index:2;width:min(820px,calc(100% - 44px));padding:12px 0 17px;background:linear-gradient(transparent,var(--color-bg) 20%,var(--color-bg))}.composer-box{position:relative;display:flex;flex-direction:column;gap:6px;padding:12px 10px 8px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:var(--radius-lg);background:color-mix(in srgb,var(--color-surface) 82%,var(--color-bg));box-shadow:var(--shadow-md)}.composer-box:focus-within{border-color:color-mix(in srgb,var(--color-text) 28%,var(--color-border));box-shadow:var(--shadow-lg)}.composer-box.dragging{border-color:var(--color-accent);background:color-mix(in srgb,var(--color-accent) 8%,var(--color-surface))}.composer-row{display:flex;align-items:flex-end;gap:12px;min-height:56px;padding-right:2px}textarea{flex:1;min-height:52px;max-height:190px;padding:2px 0;resize:none;border:0!important;outline:0!important;background:transparent;color:inherit;font:14px/1.5 inherit;caret-color:var(--color-accent)}.send{display:inline-flex;align-items:center;align-self:flex-end;min-height:28px;margin-bottom:2px;border:0;border-radius:var(--radius-sm);background:var(--color-accent);color:var(--color-on-accent);padding:6px 12px;font-weight:600}.send:hover:not(:disabled){filter:brightness(1.06)}.send:disabled{opacity:.45}.send:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}.send.stop{background:var(--color-bad);color:var(--color-on-accent)}.attachments{display:flex;flex-wrap:wrap;gap:8px;margin-bottom:8px}.attachments figure{display:grid;grid-template-columns:62px minmax(0,1fr);column-gap:8px;position:relative;min-width:210px;max-width:280px;margin:0;padding:6px;border:1px solid var(--color-border);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 82%,transparent)}.attachments img{grid-row:span 2;width:62px;height:52px;object-fit:cover;border-radius:6px}.attachments figcaption{display:grid;align-content:center;min-width:0}.attachments figcaption strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px}.attachments figcaption small{color:var(--color-text-2);font-size:13px}.attachment-actions{grid-column:2;display:flex;gap:5px}.attachment-actions button{border:0;background:transparent;color:var(--color-text-2);padding:2px 0;font-size:13px}.attachment-actions button:hover{color:var(--color-text)}.attachment-error{margin-bottom:7px;color:var(--color-bad);font-size:13px}.drop-hint{margin:0;padding:2px 0 4px;color:var(--color-accent);font-size:13px}.composer-hint{display:flex;justify-content:flex-end;gap:10px;padding-top:5px;color:var(--color-text-2);font-size:13px}.context-remaining{padding:1px 6px;border:1px solid var(--color-border);border-radius:999px;color:var(--color-text)}@media (prefers-reduced-motion:no-preference){.send{transition:filter .12s ease,opacity .12s ease}.composer-box{transition:border-color .12s ease,background .12s ease}}</style>
