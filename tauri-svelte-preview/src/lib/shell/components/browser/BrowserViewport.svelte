<script lang="ts">
  import type { BrowserWorkspaceState } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props { workspace?: BrowserWorkspaceState; }

  let { workspace = browserWorkspace }: Props = $props();
  const active = $derived(
    workspace.activeTabId ? workspace.tabs[workspace.activeTabId] ?? null : null
  );

</script>

<section class="browser-viewport" aria-label="Browser page" data-testid="browser-viewport-surface">
  {#if active}
    <div class="browser-page-header">
      <span class="browser-page-state" data-state={active.loadState}>{active.loadState}</span>
      <span class="browser-page-url">{active.url || 'No address'}</span>
      <span class="browser-page-viewport">{active.viewport.preset}</span>
    </div>
    <div class="browser-native-host" data-browser-tab-id={active.id}>
      <div class="browser-native-placeholder" aria-live="polite">
        <strong>{active.title || 'Browser page'}</strong>
        {#if active.error}
          <span class="browser-native-error">{active.error}</span>
        {:else if active.url}
          <span>Native browser surface is ready for this tab.</span>
          <small>Page identity stays attached while the browser moves between dock, floating, maximized and collapsed modes.</small>
        {:else}
          <span>Enter an http or https address to open a page.</span>
        {/if}
      </div>
    </div>
  {:else}
    <div class="browser-viewport-empty">
      <strong>Browser is ready</strong>
      <span>Create a tab or enter an http or https address.</span>
    </div>
  {/if}
</section>

<style>
  .browser-viewport {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
  }

  .browser-page-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 31px;
    padding: 0 10px;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-2);
    font-size: 12px;
  }

  .browser-page-state,
  .browser-page-viewport {
    flex: 0 0 auto;
    padding: 2px 5px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    font-size: 12px;
  }

  .browser-page-state[data-state='error'] {
    color: var(--color-bad);
  }

  .browser-page-state[data-state='loaded'] {
    color: var(--color-good);
  }

  .browser-page-url {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, Menlo, monospace;
  }

  .browser-page-viewport {
    margin-left: auto;
    color: var(--color-text-3);
  }

  .browser-native-host {
    position: relative;
    display: grid;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    place-items: center;
    overflow: auto;
    background: var(--color-surface);
    color: var(--color-text);
  }

  .browser-native-placeholder,
  .browser-viewport-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    max-width: 520px;
    padding: 32px;
    text-align: center;
    font-size: 13px;
  }

  .browser-native-placeholder small {
    max-width: 440px;
    color: var(--color-text-3);
    font-size: 12px;
  }

  .browser-native-error {
    color: var(--color-bad);
  }

  .browser-viewport-empty {
    justify-content: center;
    height: 100%;
    color: var(--color-text-2);
  }

  .browser-viewport-empty strong {
    color: var(--color-text);
  }
</style>
