<script lang="ts">
  /**
   * BrowserPanel.svelte — the Browser tab of the right column.
   *
   * The session's browser is still drawn as a full-window overlay today, and
   * moving it into this column is its own piece of work. Until then this tab is
   * the way in: it says what the browser is and opens it. Nothing is lost — the
   * address bar, the history and the annotations are all the same surface.
   */
  import Globe2 from '@lucide/svelte/icons/globe-2';

  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { openSessionBrowserOverlay } from '$lib/shell/browser/sessionBrowserState.svelte.ts';

  interface Props {
    visible: boolean;
    root: string;
    /** The session whose browser this is. */
    ownedId: string | null;
  }
  let { ownedId }: Props = $props();
</script>

<div class="browser-panel" data-testid="browser-panel">
  <EmptyState
    title="The session's browser"
    body="Open a page, step back and forward through it, and mark it up to send to the session. It opens over the whole window for now."
  >
    {#snippet icon()}<Globe2 strokeWidth={1.5} aria-hidden="true" />{/snippet}
    {#snippet actions()}
      <Button
        variant="secondary"
        size="sm"
        data-testid="browser-panel-open"
        onclick={() => openSessionBrowserOverlay(ownedId)}
      >
        Open the browser
      </Button>
    {/snippet}
  </EmptyState>
</div>

<style>
  .browser-panel {
    display: grid;
    height: 100%;
    width: 100%;
    align-content: center;
    overflow: auto;
    background: var(--color-surface);
  }
</style>
