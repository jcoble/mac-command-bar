<script lang="ts">
  import type { BrowserFeedbackAttachment } from '$lib/shell/browser/browserTypes.ts';

  interface Props {
    attachment: BrowserFeedbackAttachment;
    onRemove?: (id: string) => void;
    onCopy?: (attachment: BrowserFeedbackAttachment) => void;
    onStage?: (attachment: BrowserFeedbackAttachment) => void;
  }

  let { attachment, onRemove, onCopy, onStage }: Props = $props();
</script>

<article class="browser-annotation-card" data-testid={`browser-feedback-card-${attachment.id}`}>
  <header class="card-header">
    <div class="card-heading">
      <strong>{attachment.kind === 'grab' ? 'Grab' : attachment.kind === 'markup' ? 'Draw' : 'Annotation'}</strong>
      <span>{attachment.intent}</span>
    </div>
    <span class="card-target">generation {attachment.generation}</span>
  </header>
  <p class="card-title">{attachment.title || attachment.url || 'Untitled page'}</p>
  {#if attachment.accessibleName}
    <p class="card-detail">Name: {attachment.accessibleName}</p>
  {/if}
  {#if attachment.selector}
    <p class="card-detail selector">{attachment.selector}</p>
  {/if}
  {#if attachment.textSnippet}
    <p class="card-detail">{attachment.textSnippet}</p>
  {/if}
  {#if attachment.note}
    <p class="card-note">{attachment.note}</p>
  {/if}
  <footer class="card-actions">
    <button type="button" data-testid={`browser-feedback-copy-${attachment.id}`} onclick={() => onCopy?.(attachment)}>Copy</button>
    <button type="button" data-testid={`browser-feedback-stage-${attachment.id}`} onclick={() => onStage?.(attachment)}>Add to conversation</button>
    <button type="button" data-testid={`browser-feedback-remove-${attachment.id}`} onclick={() => onRemove?.(attachment.id)}>Remove</button>
  </footer>
</article>

<style>
  .browser-annotation-card {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 13px;
  }

  .card-header,
  .card-heading,
  .card-actions {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .card-header {
    justify-content: space-between;
  }

  .card-heading span,
  .card-target,
  .card-detail {
    color: var(--color-text-3);
    font-size: 12px;
  }

  .card-title,
  .card-detail,
  .card-note {
    margin: 0;
  }

  .card-detail,
  .card-note {
    overflow-wrap: anywhere;
  }

  .card-detail.selector {
    max-height: 64px;
    overflow: auto;
    font-family: ui-monospace, Menlo, monospace;
  }

  .card-note {
    color: var(--color-attention);
  }

  .card-actions {
    flex-wrap: wrap;
    margin-top: 2px;
  }

  .card-actions button {
    min-height: 28px;
    padding: 0 8px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .card-actions button:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }
</style>
