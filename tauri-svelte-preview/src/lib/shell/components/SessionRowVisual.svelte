<script lang="ts">
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessageCircle from '@lucide/svelte/icons/message-circle';
  import type { Snippet } from 'svelte';

  interface Props {
    active?: boolean;
    onSelect?(): void;
    children: Snippet;
  }

  let { active = false, onSelect, children }: Props = $props();
</script>

<div class="row-visual">
  <button
    data-testid="worktree-agent-select"
    type="button"
    class="session-row"
    class:active
    aria-current={active ? 'true' : undefined}
    onclick={onSelect}
  >
    {@render children()}
  </button>

  <span class="row-actions" aria-label="Session shortcuts">
    <button type="button" aria-label="Open session">
      <MessageCircle aria-hidden="true" />
    </button>
    <button type="button" aria-label="Open editor">
      <FileCode2 aria-hidden="true" />
    </button>
    <button type="button" aria-label="Open source control">
      <GitBranch aria-hidden="true" />
    </button>
  </span>
</div>

<style>
  .row-visual {
    position: relative;
  }

  .session-row {
    position: relative;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    column-gap: 10px;
    align-items: center;
    width: 100%;
    min-height: 58px;
    padding: var(--rail-row-content-inset);
    border: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    outline: none;
  }

  .session-row:hover {
    background: var(--color-hover);
  }

  .session-row.active {
    background: color-mix(in srgb, var(--color-elevated) 88%, var(--color-text));
  }

  .session-row.active:hover {
    background: color-mix(in srgb, var(--color-elevated) 84%, var(--color-text));
  }

  .row-actions {
    position: absolute;
    top: 5px;
    right: 11px;
    z-index: 2;
    display: flex;
    gap: 3px;
    opacity: 0;
    pointer-events: none;
  }

  .row-visual:hover .row-actions,
  .row-visual:focus-within .row-actions {
    opacity: 1;
    pointer-events: auto;
  }

  :global(.line-title) {
    padding-right: 82px;
  }

  .row-visual:hover :global(.age),
  .row-visual:focus-within :global(.age) {
    opacity: 0;
    pointer-events: none;
  }

  .row-actions button {
    display: grid;
    width: 25px;
    height: 25px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    place-items: center;
    background: var(--color-elevated);
    color: var(--color-text-2);
    cursor: pointer;
  }

  .row-actions button:hover,
  .row-actions button:focus-visible {
    background: var(--color-hover);
    color: var(--color-text);
    outline: none;
  }

  .row-actions button:focus-visible {
    box-shadow: var(--focus-ring);
  }

  .row-actions :global(svg) {
    width: 15px;
    height: 15px;
  }
</style>
