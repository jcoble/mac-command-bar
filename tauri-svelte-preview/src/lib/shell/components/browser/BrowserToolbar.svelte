<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Code2 from '@lucide/svelte/icons/code-2';
  import Cookie from '@lucide/svelte/icons/cookie';
  import Crop from '@lucide/svelte/icons/crop';
  import Download from '@lucide/svelte/icons/download';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Highlighter from '@lucide/svelte/icons/highlighter';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import Monitor from '@lucide/svelte/icons/monitor';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import Square from '@lucide/svelte/icons/square';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Type from '@lucide/svelte/icons/type';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import X from '@lucide/svelte/icons/x';

  import type {
    BrowserPresentationMode,
    BrowserViewportPreset,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import { BROWSER_VIEWPORT_MENU_ENTRIES } from '$lib/shell/browser/browserChrome.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  type BrowserMarkupTool =
    | 'pen'
    | 'highlighter'
    | 'arrow'
    | 'rectangle'
    | 'text'
    | 'undo'
    | 'clear'
    | 'crop';

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
    onMinimize?: () => void;
    onCancel?: () => void;
    onMarkupTool?: (tool: BrowserMarkupTool) => void;
    onImportCookies?: () => void;
    onOpenSettings?: () => void;
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
    onMinimize,
    onCancel,
    onMarkupTool,
    onImportCookies,
    onOpenSettings,
    devtoolsAvailable = false
  }: Props = $props();

  const active = $derived(
    workspace.activeTabId ? workspace.tabs[workspace.activeTabId] ?? null : null
  );
  const viewport = $derived(active?.viewport.preset ?? 'responsive');
  const interaction = $derived(workspace.interaction);
  const expanded = $derived(workspace.presentation === 'floating' || workspace.presentation === 'maximized');
  const maximized = $derived(workspace.presentation === 'maximized');

  let overflowOpen = $state(false);
  let viewportOpen = $state(false);
  let menuNotice = $state('');

  const markupTools: readonly BrowserMarkupTool[] = [
    'pen',
    'highlighter',
    'arrow',
    'rectangle',
    'text',
    'undo',
    'clear',
    'crop'
  ];

  function updateAddress(event: Event): void {
    const value = (event.currentTarget as HTMLInputElement).value;
    if (active) active.inputUrl = value;
    onAddressInput?.(value);
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    const value = active?.inputUrl.trim();
    if (value) onNavigate?.(value);
  }

  function closeMenu(): void {
    overflowOpen = false;
    viewportOpen = false;
    menuNotice = '';
  }

  function toggleOverflow(event: MouseEvent): void {
    event.stopPropagation();
    overflowOpen = !overflowOpen;
    viewportOpen = false;
    menuNotice = '';
  }

  function toggleViewport(event: MouseEvent): void {
    event.stopPropagation();
    viewportOpen = !viewportOpen;
    menuNotice = '';
  }

  function chooseViewport(preset: BrowserViewportPreset): void {
    onViewport?.(preset);
    closeMenu();
  }

  function importCookies(): void {
    if (onImportCookies) {
      onImportCookies();
      closeMenu();
      return;
    }
    menuNotice = 'Cookie import is reserved for the native profile bridge.';
  }

  function openSettings(): void {
    if (onOpenSettings) {
      onOpenSettings();
      closeMenu();
      return;
    }
    menuNotice = 'Browser settings stub — profile and viewport controls are available here.';
  }

  function toggleMaximize(): void {
    onPresentation?.(maximized ? 'floating' : 'maximized');
  }

  function minimize(): void {
    if (expanded) onMinimize?.();
    else onCollapse?.();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && overflowOpen) closeMenu();
  }

  function handleImport(): void {
    overflowOpen = true;
    viewportOpen = false;
    menuNotice = '';
  }
</script>

<svelte:window onclick={closeMenu} onkeydown={handleKeydown} />

<div class="browser-toolbar" data-testid="browser-toolbar">
  <div class="browser-navigation" aria-label="Browser navigation">
    <button
      class="browser-icon-button"
      type="button"
      aria-label="Go back"
      title="Back"
      disabled={!active?.canGoBack}
      data-testid="browser-back"
      onclick={() => onBack?.()}
    >
      <ArrowLeft size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      aria-label="Go forward"
      title="Forward"
      disabled={!active?.canGoForward}
      data-testid="browser-forward"
      onclick={() => onForward?.()}
    >
      <ArrowRight size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      aria-label="Reload page"
      title="Reload"
      disabled={!active}
      data-testid="browser-reload"
      onclick={() => onReload?.()}
    >
      <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
  </div>

  <form class="browser-address-form" onsubmit={submit}>
    <label class="sr-only" for="browser-address">Address</label>
    <span class="browser-address-icon" aria-hidden="true">
      <Globe2 size={15} strokeWidth={1.8} />
    </span>
    <input
      id="browser-address"
      class="browser-address"
      value={active?.inputUrl ?? ''}
      oninput={updateAddress}
      autocomplete="off"
      spellcheck="false"
      placeholder="Enter an http or https address"
      data-testid="browser-address"
    />
  </form>

  <div class="browser-toolbar-actions" aria-label="Browser tools">
    <button
      class="browser-import-button"
      type="button"
      title="Import cookies or browser data"
      aria-label="Import browser data"
      data-testid="browser-import"
      onclick={handleImport}
    >
      <Download size={15} strokeWidth={1.8} aria-hidden="true" />
      <span>Import</span>
    </button>
    <button
      class="browser-icon-button"
      type="button"
      disabled={!active}
      title="Pick a page element"
      aria-label="Pick page element"
      data-testid="browser-grab"
      onclick={() => onGrab?.()}
    >
      <MousePointer2 size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      disabled={!active}
      title="Annotate or comment on a page element"
      aria-label="Annotate or comment"
      data-testid="browser-annotate"
      onclick={() => onAnnotate?.()}
    >
      <MessageSquarePlus size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      disabled={!active}
      title="Draw on a screenshot"
      aria-label="Draw screenshot"
      data-testid="browser-draw"
      onclick={() => onDraw?.()}
    >
      <Pencil size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      disabled={!active || !devtoolsAvailable}
      title="Open browser developer tools"
      aria-label="Open developer tools"
      data-testid="browser-devtools"
      onclick={() => onOpenDevtools?.()}
    >
      <Code2 size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-icon-button"
      type="button"
      disabled={!active}
      title="Open in your default browser"
      aria-label="Open in external browser"
      data-testid="browser-external"
      onclick={() => onOpenExternal?.()}
    >
      <ExternalLink size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>

    <div class="browser-overflow-anchor">
      <button
        class="browser-icon-button"
        class:active={overflowOpen}
        type="button"
        aria-label="More browser actions"
        aria-expanded={overflowOpen}
        title="More browser actions"
        data-testid="browser-overflow"
        onclick={toggleOverflow}
      >
        <Ellipsis size={18} strokeWidth={1.8} aria-hidden="true" />
      </button>

      {#if overflowOpen}
        <div class="browser-overflow-menu" role="menu" tabindex="-1" aria-label="Browser menu" data-testid="browser-overflow-menu" onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()}>
          <div class="browser-menu-section-title">Profile</div>
          <button class="browser-menu-item" type="button" role="menuitemradio" aria-checked="true" data-testid="browser-profile-default">
            <Check size={16} strokeWidth={1.8} aria-hidden="true" />
            <span>Default</span>
          </button>
          <button class="browser-menu-item" type="button" role="menuitem" data-testid="browser-profile-new">
            <Plus size={16} strokeWidth={1.8} aria-hidden="true" />
            <span>New Profile…</span>
          </button>
          <div class="browser-menu-divider" role="separator"></div>
          <button class="browser-menu-item" type="button" role="menuitem" onclick={importCookies} data-testid="browser-import-cookies">
            <Cookie size={16} strokeWidth={1.8} aria-hidden="true" />
            <span>Import Cookies</span>
          </button>
          <div class="browser-menu-submenu-anchor">
            <button
              class="browser-menu-item"
              type="button"
              role="menuitem"
              aria-expanded={viewportOpen}
              onclick={toggleViewport}
              data-testid="browser-viewport-menu"
            >
              <Monitor size={16} strokeWidth={1.8} aria-hidden="true" />
              <span>Viewport Size</span>
              <ChevronRight class="browser-menu-chevron" size={16} strokeWidth={1.8} aria-hidden="true" />
            </button>
            {#if viewportOpen}
              <div class="browser-viewport-menu" role="menu" aria-label="Viewport size" data-testid="browser-viewport-submenu">
                {#each BROWSER_VIEWPORT_MENU_ENTRIES as entry (entry.preset)}
                  <button
                    class="browser-viewport-item"
                    class:selected={entry.preset === viewport}
                    type="button"
                    role="menuitemradio"
                    aria-checked={entry.preset === viewport}
                    onclick={() => chooseViewport(entry.preset)}
                    data-testid={`browser-viewport-${entry.preset}`}
                  >
                    <span>{entry.label}</span>
                    <small>{entry.dimensions}</small>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          <button class="browser-menu-item" type="button" role="menuitem" onclick={openSettings} data-testid="browser-settings">
            <Settings2 size={16} strokeWidth={1.8} aria-hidden="true" />
            <span>Browser Settings…</span>
          </button>
          {#if menuNotice}
            <p class="browser-menu-notice" role="status">{menuNotice}</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  {#if interaction === 'drawing' && onMarkupTool}
    <div class="browser-markup-tools" aria-label="Screenshot markup tools" data-testid="browser-markup-tools">
      {#each markupTools as tool}
        <button
          class="browser-markup-button"
          type="button"
          title={tool === 'highlighter' ? 'Highlighter' : tool[0].toUpperCase() + tool.slice(1)}
          aria-label={tool === 'highlighter' ? 'Highlighter' : tool[0].toUpperCase() + tool.slice(1)}
          data-testid={`browser-markup-${tool}`}
          onclick={() => onMarkupTool?.(tool)}
        >
          {#if tool === 'pen'}
            <Pencil size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'highlighter'}
            <Highlighter size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'arrow'}
            <ArrowUpRight size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'rectangle'}
            <Square size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'text'}
            <Type size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'undo'}
            <Undo2 size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else if tool === 'clear'}
            <Trash2 size={14} strokeWidth={1.8} aria-hidden="true" />
          {:else}
            <Crop size={14} strokeWidth={1.8} aria-hidden="true" />
          {/if}
        </button>
      {/each}
      {#if onCancel}
        <button class="browser-markup-button browser-markup-cancel" type="button" title="Cancel feedback mode" aria-label="Cancel feedback mode" data-testid="browser-feedback-cancel" onclick={() => onCancel?.()}>
          <X size={14} strokeWidth={1.8} aria-hidden="true" />
        </button>
      {/if}
    </div>
  {:else if interaction !== 'browse' && onCancel}
    <button class="browser-icon-button browser-toolbar-cancel" type="button" title="Cancel feedback mode" aria-label="Cancel feedback mode" data-testid="browser-feedback-cancel" onclick={() => onCancel?.()}>
      <X size={16} strokeWidth={1.8} aria-hidden="true" />
    </button>
  {/if}

  <div class="browser-window-controls" aria-label="Browser window controls">
    <button
      class="browser-window-button"
      type="button"
      disabled={!active}
      aria-label={maximized ? 'Restore browser size' : 'Maximize browser'}
      title={maximized ? 'Restore browser size' : 'Maximize browser'}
      data-testid="browser-maximize"
      onclick={toggleMaximize}
    >
      <Maximize2 size={15} strokeWidth={1.8} aria-hidden="true" />
    </button>
    <button
      class="browser-window-button"
      type="button"
      disabled={!active}
      aria-label={expanded ? 'Minimize browser' : 'Collapse browser'}
      title={expanded ? 'Minimize browser' : 'Collapse browser'}
      data-testid={expanded ? 'browser-minimize' : 'browser-collapse'}
      onclick={minimize}
    >
      <Minimize2 size={15} strokeWidth={1.8} aria-hidden="true" />
    </button>
  </div>
</div>

<style>
  .browser-toolbar {
    position: relative;
    z-index: 8;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 48px;
    padding: 6px 10px;
    overflow: visible;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-text);
  }

  .browser-navigation,
  .browser-toolbar-actions,
  .browser-window-controls,
  .browser-markup-tools {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    gap: 3px;
  }

  .browser-address-form {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1 1 320px;
    min-width: 150px;
    height: 34px;
    padding: 0 12px 0 32px;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-surface);
    color: var(--color-text-2);
  }

  .browser-address-icon {
    position: absolute;
    left: 11px;
    display: inline-flex;
    color: var(--color-text-3);
    pointer-events: none;
  }

  .browser-address {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--color-text);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .browser-address::placeholder {
    color: var(--color-text-3);
  }

  .browser-address-form:focus-within {
    border-color: var(--color-focus-solid);
    box-shadow: var(--focus-ring);
  }

  .browser-icon-button,
  .browser-window-button,
  .browser-import-button,
  .browser-markup-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    cursor: pointer;
  }

  .browser-icon-button:hover:not(:disabled),
  .browser-icon-button.active,
  .browser-window-button:hover:not(:disabled),
  .browser-markup-button:hover:not(:disabled) {
    background: var(--color-elevated);
    color: var(--color-text);
  }

  .browser-icon-button:focus-visible,
  .browser-window-button:focus-visible,
  .browser-import-button:focus-visible,
  .browser-markup-button:focus-visible,
  .browser-menu-item:focus-visible,
  .browser-viewport-item:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 1px;
  }

  .browser-icon-button:disabled,
  .browser-window-button:disabled,
  .browser-import-button:disabled,
  .browser-markup-button:disabled {
    color: var(--color-disabled-text);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .browser-import-button {
    min-width: 70px;
    gap: 6px;
    padding: 0 9px;
    background: var(--color-elevated);
    color: var(--color-text);
    font-size: 12px;
  }

  .browser-import-button:hover {
    background: var(--color-hover);
  }

  .browser-overflow-anchor,
  .browser-menu-submenu-anchor {
    position: relative;
  }

  .browser-overflow-menu,
  .browser-viewport-menu {
    position: absolute;
    z-index: 30;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border: 1px solid var(--color-border);
    border-radius: 10px;
    background: var(--color-surface);
    box-shadow: var(--shadow-lg);
    color: var(--color-text);
  }

  .browser-overflow-menu {
    top: calc(100% + 8px);
    right: 0;
    width: 232px;
  }

  .browser-viewport-menu {
    top: -8px;
    right: calc(100% + 8px);
    width: 218px;
  }

  .browser-menu-section-title {
    padding: 4px 8px 5px;
    color: var(--color-text-3);
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .browser-menu-divider {
    height: 1px;
    margin: 5px 0;
    background: var(--color-border);
  }

  .browser-menu-item,
  .browser-viewport-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 34px;
    padding: 0 8px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .browser-menu-item:hover,
  .browser-viewport-item:hover,
  .browser-viewport-item.selected {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .browser-menu-item span,
  .browser-viewport-item span {
    min-width: 0;
    flex: 1 1 auto;
  }

  :global(.browser-menu-chevron) {
    margin-left: auto;
    color: var(--color-text-3);
  }

  .browser-viewport-item {
    justify-content: space-between;
  }

  .browser-viewport-item small {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: 12px;
  }

  .browser-menu-notice {
    margin: 7px 8px 2px;
    color: var(--color-text-3);
    font-size: 12px;
    line-height: 1.4;
  }

  .browser-markup-tools {
    padding-left: 3px;
    border-left: 1px solid var(--color-border);
  }

  .browser-markup-button {
    min-width: 28px;
    height: 28px;
    color: var(--color-text-3);
    font-size: 12px;
  }

  .browser-markup-cancel,
  .browser-toolbar-cancel {
    color: var(--color-attention);
  }

  .browser-window-controls {
    margin-left: 2px;
    padding-left: 4px;
    border-left: 1px solid var(--color-border);
  }

  .browser-window-button {
    min-width: 30px;
    height: 30px;
    border-radius: 7px;
    color: var(--color-text-3);
  }

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

  @media (max-width: 840px) {
    .browser-toolbar {
      gap: 4px;
      padding-inline: 6px;
    }

    .browser-address-form {
      flex-basis: 180px;
    }

    .browser-import-button {
      min-width: 32px;
      padding-inline: 0;
    }

    .browser-import-button span {
      display: none;
    }
  }
</style>
