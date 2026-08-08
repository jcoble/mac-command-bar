<script lang="ts">
  import BrowserAnnotationCard from './BrowserAnnotationCard.svelte';
  import type {
    BrowserFeedbackAttachment,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
    onRemove?: (id: string) => void;
    onCopy?: (attachment: BrowserFeedbackAttachment) => void;
    onStage?: (attachment: BrowserFeedbackAttachment) => void;
  }

  let { workspace = browserWorkspace, onRemove, onCopy, onStage }: Props = $props();
  const queue = $derived(workspace.queue);
</script>

<aside class="browser-feedback-panel" aria-label="Browser feedback queue" data-testid="browser-feedback-panel">
  <header class="feedback-header">
    <div>
      <strong>Feedback queue</strong>
      <span>{queue.length} {queue.length === 1 ? 'item' : 'items'}</span>
    </div>
    <small>Review before adding it to the conversation.</small>
  </header>

  {#if queue.length === 0}
    <div class="feedback-empty">
      <strong>No feedback yet</strong>
      <span>Grab an element, annotate it, or draw on a screenshot.</span>
    </div>
  {:else}
    <div class="feedback-list">
      {#each queue as attachment (attachment.id)}
        <BrowserAnnotationCard {attachment} {onRemove} {onCopy} {onStage} />
      {/each}
    </div>
  {/if}
</aside>

<style>
  .browser-feedback-panel {
    display: flex;
    flex: 0 0 min(340px, 32%);
    flex-direction: column;
    min-width: 240px;
    min-height: 0;
    overflow: hidden;
    border-left: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .feedback-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 13px 14px;
    border-bottom: 1px solid var(--color-border);
  }

  .feedback-header div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .feedback-header span,
  .feedback-header small,
  .feedback-empty span {
    color: var(--color-text-3);
    font-size: 12px;
  }

  .feedback-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    overflow: auto;
    padding: 9px;
  }

  .feedback-empty {
    display: flex;
    flex-direction: column;
    gap: 7px;
    min-height: 150px;
    padding: 28px 16px;
    color: var(--color-text-2);
    text-align: center;
  }
</style>
