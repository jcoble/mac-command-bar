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
    background: var(--color-bg, #101014);
    color: var(--color-text, #eef0f9);
  }

  .browser-page-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 31px;
    padding: 0 10px;
    border-bottom: 1px solid var(--color-border, #858599);
    color: var(--color-text-2, #a7a7b5);
    font-size: 12px;
  }

  .browser-page-state,
  .browser-page-viewport {
    flex: 0 0 auto;
    padding: 2px 5px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 4px;
    font-size: 12px;
  }

  .browser-page-state[data-state='error'] {
    color: var(--color-bad, #ff6d91);
  }

  .browser-page-state[data-state='loaded'] {
    color: var(--color-good, #8bdc9b);
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
    color: var(--color-text-3, #767687);
  }

  .browser-native-host {
    position: relative;
    display: grid;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    place-items: center;
    overflow: auto;
    background: #ffffff;
    color: #1e2026;
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
    color: #545866;
    font-size: 12px;
  }

  .browser-native-error {
    color: #a3284f;
  }

  .browser-viewport-empty {
    justify-content: center;
    height: 100%;
    color: var(--color-text-2, #a7a7b5);
  }

  .browser-viewport-empty strong {
    color: var(--color-text, #eef0f9);
  }
</style>
