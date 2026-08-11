<script lang="ts">
  /**
   * SessionBrowserButton.svelte — the way into the session's browser.
   *
   * It sits in the strip along the top of the shell, where it is plainly
   * visible at all times, and it opens or closes the overlay for whichever
   * session is showing. When that session has notes waiting to be sent, the
   * count is on the button, so a pile of annotations is never invisible.
   */
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { sessionBrowserView, toggleSessionBrowserOverlay } from './sessionBrowserState.svelte.ts';

  const sessionId = $derived(rail.activeOwnedId);
  const view = $derived(sessionBrowserView(sessionId));
</script>

<button
  type="button"
  class="session-browser-button"
  class:on={view.open}
  data-testid="session-browser-entry"
  aria-pressed={view.open}
  title="Open this session's browser"
  disabled={!sessionId}
  onclick={() => toggleSessionBrowserOverlay(sessionId)}
>
  <Globe2 class="size-4" aria-hidden="true" />
  Browser
  {#if view.annotations.length}
    <span class="count" data-testid="session-browser-entry-count">{view.annotations.length}</span>
  {/if}
</button>

<style>
  .session-browser-button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .session-browser-button:hover:not(:disabled) {
    border-color: var(--color-accent);
    background: var(--color-hover);
  }

  .session-browser-button:disabled {
    color: var(--color-disabled-text);
    cursor: default;
  }

  .session-browser-button.on {
    border-color: var(--color-accent);
    background: var(--color-selected);
  }

  .session-browser-button:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 2px;
  }

  .count {
    display: grid;
    min-width: 18px;
    height: 18px;
    place-items: center;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--color-accent);
    color: var(--color-on-accent);
    font-size: 13px;
  }

  @media (prefers-reduced-motion: no-preference) {
    .session-browser-button {
      transition:
        background 0.12s ease,
        border-color 0.12s ease;
    }
  }
</style>
