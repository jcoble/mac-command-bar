<script lang="ts">
  import BrowserFeedbackPanel from './BrowserFeedbackPanel.svelte';
  import BrowserTabs from './BrowserTabs.svelte';
  import BrowserToolbar from './BrowserToolbar.svelte';
  import BrowserViewport from './BrowserViewport.svelte';
  import type {
    BrowserFeedbackAttachment,
    BrowserPresentationMode,
    BrowserViewportPreset,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
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

  const expandedMode = $derived(workspace.presentation);
</script>

<section
  class="browser-expanded-overlay"
  class:maximized={expandedMode === 'maximized'}
  class:floating={expandedMode === 'floating'}
  style={`left: ${workspace.floatingBounds.x}px; top: ${workspace.floatingBounds.y}px; width: ${workspace.floatingBounds.width}px; height: ${workspace.floatingBounds.height}px;`}
  aria-label={expandedMode === 'maximized' ? 'Maximized browser' : 'Floating browser'}
  data-testid="browser-expanded-overlay"
>
  <BrowserTabs {workspace} onSelect={onSelectTab} onClose={onCloseTab} onCreate={onCreateTab} />
  <BrowserToolbar
    {workspace}
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
    onMinimize={onMinimize}
    onCancel={onCancelFeedback}
    onMarkupTool={onMarkupTool}
    {devtoolsAvailable}
  />
  <div class="browser-overlay-body">
    <BrowserViewport {workspace} />
    <BrowserFeedbackPanel
      {workspace}
      onRemove={onRemoveFeedback}
      onCopy={onCopyFeedback}
      onStage={onStageFeedback}
    />
  </div>
</section>

<style>
  .browser-expanded-overlay {
    position: fixed;
    z-index: 40;
    display: flex;
    flex-direction: column;
    min-width: 360px;
    min-height: 240px;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    background: var(--color-bg);
    box-shadow: var(--shadow-lg);
    color: var(--color-text);
  }

  .browser-expanded-overlay.maximized {
    inset: 0;
    width: auto !important;
    height: auto !important;
    min-width: 0;
    min-height: 0;
    border-radius: 0;
  }

  .browser-overlay-body {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
  }

  .browser-overlay-body :global(.browser-viewport) {
    min-width: 0;
  }
</style>
