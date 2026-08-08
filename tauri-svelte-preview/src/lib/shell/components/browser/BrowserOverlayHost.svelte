<script lang="ts">
  import BrowserExpandedOverlay from './BrowserExpandedOverlay.svelte';
  import type {
    BrowserFeedbackAttachment,
    BrowserPresentationMode,
    BrowserViewportPreset,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
    onExpand?: () => void;
    onSelectTab?: (id: string) => void;
    onCloseTab?: (id: string) => void;
    onCreateTab?: () => void;
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
    onCancelFeedback?: () => void;
    onMarkupTool?: (tool: 'pen' | 'highlighter' | 'arrow' | 'rectangle' | 'text' | 'undo' | 'clear' | 'crop') => void;
    onRemoveFeedback?: (id: string) => void;
    onCopyFeedback?: (attachment: BrowserFeedbackAttachment) => void;
    onStageFeedback?: (attachment: BrowserFeedbackAttachment) => void;
    onMinimize?: () => void;
    devtoolsAvailable?: boolean;
  }

  let {
    workspace = browserWorkspace,
    onExpand,
    onSelectTab,
    onCloseTab,
    onCreateTab,
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
    onCancelFeedback,
    onMarkupTool,
    onRemoveFeedback,
    onCopyFeedback,
    onStageFeedback,
    onMinimize,
    devtoolsAvailable = false
  }: Props = $props();

  const visible = $derived(workspace.presentation !== 'docked');
  const collapsed = $derived(workspace.presentation === 'collapsed');
</script>

{#if visible && collapsed}
  <div class="browser-collapsed-control" data-testid="browser-collapsed-control">
    <button type="button" aria-label="Expand browser" title="Expand browser" data-testid="browser-expand" onclick={() => onExpand?.()}>
      Browser
    </button>
  </div>
{:else if visible}
  <BrowserExpandedOverlay
    {workspace}
    {onSelectTab}
    {onCloseTab}
    {onCreateTab}
    {onAddressInput}
    {onNavigate}
    {onReload}
    {onBack}
    {onForward}
    {onGrab}
    {onAnnotate}
    {onDraw}
    {onOpenDevtools}
    {onOpenExternal}
    {onViewport}
    {onPresentation}
    {onCollapse}
    {onCancelFeedback}
    {onMarkupTool}
    {onRemoveFeedback}
    {onCopyFeedback}
    {onStageFeedback}
    {onMinimize}
    {devtoolsAvailable}
  />
{/if}

<style>
  .browser-collapsed-control {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 40;
  }

  .browser-collapsed-control button {
    min-width: 64px;
    min-height: 44px;
    padding: 0 13px;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: var(--color-surface);
    box-shadow: var(--shadow-md);
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .browser-collapsed-control button:hover {
    border-color: var(--color-accent);
    background: var(--color-hover);
  }
</style>
