<script lang="ts">
  import type { BrowserInteractionMode, BrowserWorkspaceState } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
    onGrab?: () => void;
    onAnnotate?: () => void;
    onDraw?: () => void;
    onCancel?: () => void;
    onMarkupTool?: (tool: 'pen' | 'highlighter' | 'arrow' | 'rectangle' | 'text' | 'undo' | 'clear' | 'crop') => void;
  }

  let {
    workspace = browserWorkspace,
    onGrab,
    onAnnotate,
    onDraw,
    onCancel,
    onMarkupTool
  }: Props = $props();

  const mode = $derived<BrowserInteractionMode>(workspace.interaction);
  const hasActiveTab = $derived(Boolean(workspace.activeTabId));
</script>

<div class="browser-annotation-toolbar" aria-label="Page feedback tools" data-testid="browser-annotation-toolbar">
  <button
    class:active={mode === 'picking'}
    class="feedback-tool"
    type="button"
    disabled={!hasActiveTab}
    aria-pressed={mode === 'picking'}
    title="Grab page metadata without a note"
    data-testid="browser-feedback-grab"
    onclick={() => onGrab?.()}
  >
    Grab element
  </button>
  <button
    class:active={mode === 'annotating'}
    class="feedback-tool"
    type="button"
    disabled={!hasActiveTab}
    aria-pressed={mode === 'annotating'}
    title="Annotate a selected page element"
    data-testid="browser-feedback-annotate"
    onclick={() => onAnnotate?.()}
  >
    Annotate
  </button>
  <button
    class:active={mode === 'drawing'}
    class="feedback-tool"
    type="button"
    disabled={!hasActiveTab}
    aria-pressed={mode === 'drawing'}
    title="Draw on a native viewport capture"
    data-testid="browser-feedback-draw"
    onclick={() => onDraw?.()}
  >
    Draw screenshot
  </button>
  {#if mode !== 'browse'}
    <button class="feedback-tool cancel" type="button" title="Cancel feedback mode" data-testid="browser-feedback-cancel" onclick={() => onCancel?.()}>
      Cancel
    </button>
  {/if}
  {#if mode === 'drawing'}
    <div class="markup-tools" aria-label="Screenshot markup tools">
      {#each ['pen', 'highlighter', 'arrow', 'rectangle', 'text', 'undo', 'clear', 'crop'] as tool}
        <button
          class="markup-tool"
          type="button"
          title={tool === 'highlighter' ? 'Highlighter' : tool[0].toUpperCase() + tool.slice(1)}
          data-testid={`browser-markup-${tool}`}
          onclick={() => onMarkupTool?.(tool as 'pen' | 'highlighter' | 'arrow' | 'rectangle' | 'text' | 'undo' | 'clear' | 'crop')}
        >
          {tool[0].toUpperCase() + tool.slice(1)}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .browser-annotation-toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 5px;
    padding: 7px 9px;
    border-bottom: 1px solid var(--color-border, #858599);
    background: var(--color-bg, #101014);
  }

  .feedback-tool {
    min-height: 30px;
    padding: 0 9px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2, #a7a7b5);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .feedback-tool:hover:not(:disabled),
  .feedback-tool.active {
    border-color: var(--color-selected-border, #4bbd9f);
    background: var(--color-selected, #26302f);
    color: var(--color-text, #eef0f9);
  }

  .feedback-tool.cancel {
    margin-left: auto;
    color: var(--color-attention, #ffd493);
  }

  .feedback-tool:disabled {
    color: var(--color-disabled-text, #858599);
    cursor: not-allowed;
    opacity: 0.65;
  }

  .markup-tools {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-left: auto;
  }

  .markup-tool {
    min-height: 28px;
    padding: 0 7px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 4px;
    background: var(--color-surface, #17171d);
    color: var(--color-text-2, #a7a7b5);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .markup-tool:hover {
    background: var(--color-hover, #25252e);
    color: var(--color-text, #eef0f9);
  }
</style>
