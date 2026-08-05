<script lang="ts">
  import type { AgentCapabilities, AgentConfigValue, ConversationAttachment } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import AgentConfigBar from './AgentConfigBar.svelte';
  import AgentCommandMenu from './AgentCommandMenu.svelte';

  type ComposerAttachment = ConversationAttachment & { byteLength?: number };
  interface Props {
    provider: string;
    draft: string;
    attachments: readonly ComposerAttachment[];
    sending: boolean;
    capabilities: AgentCapabilities | null;
    config: Record<string, AgentConfigValue>;
    pendingConfig: Record<string, AgentConfigValue>;
    configErrors: Record<string, string>;
    commands: readonly ConversationCommand[];
    attachmentError?: string;
    onDraftChange?(value: string): void;
    onSend?(): void | Promise<void>;
    onPaste?(event: ClipboardEvent): void | Promise<void>;
    onRemoveAttachment?(id: string): void | Promise<void>;
    onAnnotateAttachment?(id: string): void;
    onConfigChange?(optionId: string, value: AgentConfigValue): void | Promise<void>;
    onCommandSelected?(command: ConversationCommand): void;
  }
  let {
    provider,
    draft,
    attachments,
    sending,
    capabilities,
    config,
    pendingConfig,
    configErrors,
    commands,
    attachmentError = '',
    onDraftChange,
    onSend,
    onPaste,
    onRemoveAttachment,
    onAnnotateAttachment,
    onConfigChange,
    onCommandSelected
  }: Props = $props();
  const commandQuery = $derived(draft.trimStart().startsWith('/') ? draft.trimStart().slice(1) : '');

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
  {#if commandQuery && commands.length}<AgentCommandMenu commands={commands} query={commandQuery} onSelect={onCommandSelected} />{/if}
  <div class="composer-row" data-testid="conversation-composer">
    <textarea data-testid="conversation-composer-input" aria-label="Message" placeholder={`Message ${provider}`} value={draft} onpaste={(event) => void onPaste?.(event)} oninput={(event) => onDraftChange?.(event.currentTarget.value)} onkeydown={(event) => { if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); if (commandQuery && commands.length) onCommandSelected?.(commands[0]); else void onSend?.(); } }}></textarea>
    <button class="send" data-testid="conversation-send" type="button" disabled={sending || (!draft.trim() && attachments.length === 0)} onclick={() => void onSend?.()}>{sending ? 'Working…' : 'Send'}</button>
  </div>
  <AgentConfigBar capabilities={capabilities} {config} pending={pendingConfig} errors={configErrors} onChange={onConfigChange} />
  <div class="composer-hint" data-testid="conversation-paste-hint">Paste screenshots with ⌘V · images stay attached until Send</div>
</div>

<style>.composer-area{position:absolute;left:50%;bottom:0;transform:translateX(-50%);z-index:2;width:min(820px,calc(100% - 44px));padding:12px 0 17px;background:linear-gradient(transparent,var(--color-bg) 20%,var(--color-bg))}.composer-row{display:flex;align-items:flex-end;gap:12px;min-height:76px;padding:12px 12px 10px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:15px;background:color-mix(in srgb,var(--color-surface) 82%,var(--color-bg));box-shadow:0 8px 28px rgba(0,0,0,.18)}.composer-row:focus-within{border-color:color-mix(in srgb,var(--color-text) 28%,var(--color-border));box-shadow:0 8px 30px rgba(0,0,0,.22)}textarea{flex:1;min-height:52px;max-height:190px;padding:2px 0;resize:none;border:0!important;outline:0!important;background:transparent;color:inherit;font:14px/1.5 inherit;caret-color:var(--color-accent)}.send{align-self:flex-end;margin-bottom:2px;border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);color:inherit;padding:6px 9px}.attachments{display:flex;flex-wrap:wrap;gap:8px;margin-bottom:8px}.attachments figure{display:grid;grid-template-columns:62px minmax(0,1fr);column-gap:8px;position:relative;min-width:210px;max-width:280px;margin:0;padding:6px;border:1px solid var(--color-border);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 82%,transparent)}.attachments img{grid-row:span 2;width:62px;height:52px;object-fit:cover;border-radius:6px}.attachments figcaption{display:grid;align-content:center;min-width:0}.attachments figcaption strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:12px}.attachments figcaption small{color:var(--color-text-2);font-size:12px}.attachment-actions{grid-column:2;display:flex;gap:5px}.attachment-actions button{border:0;background:transparent;color:var(--color-text-2);padding:2px 0;font-size:12px}.attachment-error{margin-bottom:7px;color:#ef8b92;font-size:12px}.composer-hint{padding-top:5px;text-align:right;color:var(--color-text-2);font-size:12px}</style>
