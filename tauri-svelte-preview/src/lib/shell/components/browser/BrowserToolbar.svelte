<script lang="ts">
  import type {
    BrowserPresentationMode,
    BrowserTabState,
    BrowserViewportPreset,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
    onAddressInput?: (value: string) => void;
    onNavigate?: (value: string) => void;
    onReload?: () => void;
    onBack?: () => void;
    onForward?: () => void;
    onGrab?: () => void;
    onAnnotate?: () => void;
    onDraw?: () => void;
    onOpenDevtools?: () => void;
    onOpenExternal?: () => void;
    onViewport?: (preset: BrowserViewportPreset) => void;
    onPresentation?: (mode: BrowserPresentationMode) => void;
    onCollapse?: () => void;
    devtoolsAvailable?: boolean;
  }

  let {
    workspace = browserWorkspace,
    onAddressInput,
    onNavigate,
    onReload,
    onBack,
    onForward,
    onGrab,
    onAnnotate,
    onDraw,
    onOpenDevtools,
    onOpenExternal,
    onViewport,
    onPresentation,
    onCollapse,
    devtoolsAvailable = false
  }: Props = $props();

  const active = $derived(
    workspace.activeTabId ? workspace.tabs[workspace.activeTabId] ?? null : null
  );
  const viewport = $derived(active?.viewport.preset ?? 'responsive');

  function updateAddress(event: Event): void {
    const value = (event.currentTarget as HTMLInputElement).value;
    if (active) active.inputUrl = value;
    onAddressInput?.(value);
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (active) onNavigate?.(active.inputUrl);
  }

  function chooseViewport(event: Event): void {
    onViewport?.((event.currentTarget as HTMLSelectElement).value as BrowserViewportPreset);
  }

  const expanded = $derived(workspace.presentation === 'floating' || workspace.presentation === 'maximized');
</script>

<div class="browser-toolbar" data-testid="browser-toolbar">
  <div class="browser-navigation" aria-label="Browser navigation">
    <button
      class="browser-tool-button"
      type="button"
      aria-label="Go back"
      title="Back"
      disabled={!active?.canGoBack}
      data-testid="browser-back"
      onclick={() => onBack?.()}
    >
      <span aria-hidden="true">←</span>
    </button>
    <button
      class="browser-tool-button"
      type="button"
      aria-label="Go forward"
      title="Forward"
      disabled={!active?.canGoForward}
      data-testid="browser-forward"
      onclick={() => onForward?.()}
    >
      <span aria-hidden="true">→</span>
    </button>
    <button
      class="browser-tool-button"
      type="button"
      aria-label="Reload page"
      title="Reload"
      disabled={!active}
      data-testid="browser-reload"
      onclick={() => onReload?.()}
    >
      <span aria-hidden="true">↻</span>
    </button>
  </div>

  <form class="browser-address-form" onsubmit={submit}>
    <label class="sr-only" for="browser-address">Address</label>
    <input
      id="browser-address"
      class="browser-address"
      value={active?.inputUrl ?? ''}
      oninput={updateAddress}
      autocomplete="off"
      spellcheck="false"
      placeholder="https://example.com or :5177"
      data-testid="browser-address"
    />
    <button
      class="browser-open-button"
      type="submit"
      disabled={!active?.inputUrl.trim()}
      data-testid="browser-open"
    >
      Open
    </button>
  </form>

  <div class="browser-toolbar-actions" aria-label="Browser tools">
    <button class="browser-tool-button text-button" type="button" disabled title="Import is disabled until a safe import design is approved" data-testid="browser-import">
      Import
    </button>
    <button class="browser-tool-button text-button" type="button" disabled={!active} title="Grab a page element" data-testid="browser-grab" onclick={() => onGrab?.()}>
      Grab
    </button>
    <button class="browser-tool-button text-button" type="button" disabled={!active} title="Annotate a page element" data-testid="browser-annotate" onclick={() => onAnnotate?.()}>
      Annotate
    </button>
    <button class="browser-tool-button text-button" type="button" disabled={!active} title="Draw on a screenshot" data-testid="browser-draw" onclick={() => onDraw?.()}>
      Draw
    </button>
    <button class="browser-tool-button text-button" type="button" disabled={!active || !devtoolsAvailable} title="Open browser developer tools" data-testid="browser-devtools" onclick={() => onOpenDevtools?.()}>
      Devtools
    </button>
    <button class="browser-tool-button text-button" type="button" disabled={!active} title="Open in your default browser" data-testid="browser-external" onclick={() => onOpenExternal?.()}>
      External
    </button>
  </div>

  <div class="browser-toolbar-settings" aria-label="Browser settings">
    <label class="viewport-label" for="browser-viewport">Viewport</label>
    <select id="browser-viewport" value={viewport} onchange={chooseViewport} data-testid="browser-viewport">
      <option value="responsive">Responsive</option>
      <option value="mobile-s">Mobile S</option>
      <option value="mobile-m">Mobile M</option>
      <option value="mobile-l">Mobile L</option>
      <option value="tablet">Tablet</option>
      <option value="laptop">Laptop</option>
      <option value="laptop-l">Laptop L</option>
      <option value="desktop">Desktop</option>
      <option value="custom">Custom</option>
    </select>
    <button class="browser-tool-button text-button" type="button" disabled title="Browser profiles are managed by the workspace" data-testid="browser-profile">
      Profile
    </button>
    <button class="browser-tool-button text-button" type="button" disabled title="Browser settings" data-testid="browser-settings">
      Settings
    </button>
    {#if expanded}
      <button class="browser-tool-button text-button" type="button" title="Restore browser to the dock" data-testid="browser-restore" onclick={() => onPresentation?.('docked')}>
        Restore
      </button>
    {:else}
      <button class="browser-tool-button text-button" type="button" disabled={!active} title="Maximize browser" data-testid="browser-maximize" onclick={() => onPresentation?.('maximized')}>
        Maximize
      </button>
    {/if}
    <button class="browser-tool-button text-button" type="button" disabled={!active} title="Collapse browser to the control" data-testid="browser-collapse" onclick={() => onCollapse?.()}>
      Collapse
    </button>
  </div>
</div>

<style>
  .browser-toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 7px 9px;
    border-bottom: 1px solid var(--color-border, #858599);
    background: var(--color-surface, #17171d);
    color: var(--color-text, #eef0f9);
  }

  .browser-navigation,
  .browser-toolbar-actions,
  .browser-toolbar-settings {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
  }

  .browser-address-form {
    display: flex;
    align-items: center;
    flex: 1 1 260px;
    min-width: 160px;
    gap: 4px;
  }

  .browser-address {
    width: 100%;
    min-width: 0;
    height: 30px;
    padding: 0 9px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 5px;
    background: var(--color-bg, #101014);
    color: var(--color-text, #eef0f9);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .browser-address:focus-visible,
  .browser-toolbar select:focus-visible,
  .browser-tool-button:focus-visible,
  .browser-open-button:focus-visible {
    outline: 2px solid var(--color-focus-solid, #4bf3c8);
    outline-offset: 1px;
  }

  .browser-tool-button,
  .browser-open-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    height: 30px;
    padding: 0 7px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2, #a7a7b5);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .browser-tool-button:hover:not(:disabled) {
    background: var(--color-hover, #25252e);
    color: var(--color-text, #eef0f9);
  }

  .browser-tool-button:disabled,
  .browser-open-button:disabled {
    color: var(--color-disabled-text, #858599);
    cursor: not-allowed;
    opacity: 0.65;
  }

  .browser-tool-button.text-button {
    white-space: nowrap;
  }

  .browser-open-button {
    border-color: var(--color-accent, #4bf3c8);
    background: var(--color-accent, #4bf3c8);
    color: var(--color-on-accent, #0e1013);
    font-weight: 700;
  }

  .browser-toolbar select {
    max-width: 118px;
    height: 30px;
    padding: 0 5px;
    border: 1px solid var(--color-border, #858599);
    border-radius: 5px;
    background: var(--color-bg, #101014);
    color: var(--color-text-2, #a7a7b5);
    font-family: inherit;
    font-size: 12px;
  }

  .viewport-label,
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 1080px) {
    .browser-toolbar-actions .text-button:nth-child(n + 4),
    .browser-toolbar-settings .text-button:nth-child(-n + 2) {
      display: none;
    }
  }
</style>
