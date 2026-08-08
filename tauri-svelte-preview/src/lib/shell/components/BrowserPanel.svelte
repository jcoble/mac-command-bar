<script lang="ts">
  /** Composition-only browser dock.  State and transitions live in the model
   * and compatibility facade; this component only assembles the surfaces. */
  import BrowserFeedbackPanel from './browser/BrowserFeedbackPanel.svelte';
  import BrowserTabs from './browser/BrowserTabs.svelte';
  import BrowserToolbar from './browser/BrowserToolbar.svelte';
  import BrowserViewport from './browser/BrowserViewport.svelte';
  import {
    browser,
    browserBackend,
    browserModelContext,
    browserWorkspace,
    captureBrowserView,
    clearBrowserError,
    reloadBrowserFrame,
    setBrowserPresentation,
    setBrowserUrl,
    setBrowserViewportPreset,
    stageBrowserFeedback,
    syncBrowserTab
  } from '$lib/shell/browser/browserStore.svelte';
  import {
    beginBrowserElementPicker,
    cancelBrowserAnnotation,
    closeBrowserTab,
    collapseBrowserToControl,
    createBrowserTab,
    expandBrowserFrom,
    removeBrowserAnnotation,
    restoreBrowserToDock,
  } from '$lib/shell/browser/browserModel.ts';
  import type {
    BrowserFeedbackAttachment,
    BrowserPresentationMode,
    BrowserViewportPreset
  } from '$lib/shell/browser/browserTypes.ts';

  function newTab(): void {
    createBrowserTab(browserModelContext(), {});
  }

  function chooseTab(tabId: string): void {
    syncBrowserTab(tabId);
  }

  function closeTab(tabId: string): void {
    closeBrowserTab(browserModelContext(), tabId);
  }

  function navigate(value: string): void {
    clearBrowserError();
    setBrowserUrl(value);
  }

  function goBack(): void {
    const active = browserWorkspace.activeTabId
      ? browserWorkspace.tabs[browserWorkspace.activeTabId] ?? null
      : null;
    if (!active) return;
    browserBackend.go_back_browser_tab({ workspaceId: active.workspaceId, tabId: active.id, generation: active.generation });
  }

  function goForward(): void {
    const active = browserWorkspace.activeTabId
      ? browserWorkspace.tabs[browserWorkspace.activeTabId] ?? null
      : null;
    if (!active) return;
    browserBackend.go_forward_browser_tab({ workspaceId: active.workspaceId, tabId: active.id, generation: active.generation });
  }

  function openExternal(): void {
    const active = browserWorkspace.activeTabId
      ? browserWorkspace.tabs[browserWorkspace.activeTabId] ?? null
      : null;
    if (!active?.url) return;
    if (browserBackend.open_browser_tab_external) {
      browserBackend.open_browser_tab_external({ workspaceId: active.workspaceId, tabId: active.id, generation: active.generation });
    } else if (typeof window !== 'undefined') {
      window.open(active.url, '_blank', 'noopener,noreferrer');
    }
  }

  function openDevtools(): void {
    const active = browserWorkspace.activeTabId
      ? browserWorkspace.tabs[browserWorkspace.activeTabId] ?? null
      : null;
    if (active && browserBackend.open_browser_tab_devtools) {
      browserBackend.open_browser_tab_devtools({ workspaceId: active.workspaceId, tabId: active.id, generation: active.generation });
    }
  }

  function grab(): void {
    try {
      beginBrowserElementPicker(browserModelContext(), 'grab');
    } catch (error) {
      browser.workspace.error = error instanceof Error ? error.message : String(error);
    }
  }

  function annotate(): void {
    try {
      beginBrowserElementPicker(browserModelContext(), 'annotation');
    } catch (error) {
      browser.workspace.error = error instanceof Error ? error.message : String(error);
    }
  }

  function draw(): void {
    try {
      void captureBrowserView({ forMarkup: true });
    } catch (error) {
      browser.workspace.error = error instanceof Error ? error.message : String(error);
    }
  }

  function presentation(mode: BrowserPresentationMode): void {
    if (mode === 'docked') restoreBrowserToDock(browserModelContext());
    else if (mode === 'floating') expandBrowserFrom(browserModelContext(), 'docked');
    else setBrowserPresentation(mode);
  }

  function expand(): void {
    expandBrowserFrom(browserModelContext(), 'collapsed');
  }

  function collapse(): void {
    collapseBrowserToControl(browserModelContext());
  }

  function stage(attachment: BrowserFeedbackAttachment): void {
    void stageBrowserFeedback({ attachmentId: attachment.id }).commit();
  }

  function copy(attachment: BrowserFeedbackAttachment): void {
    const preview = stageBrowserFeedback({ attachmentId: attachment.id });
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      void navigator.clipboard.writeText(preview.mergedDraft);
    }
  }

  function viewport(value: BrowserViewportPreset): void {
    setBrowserViewportPreset(value);
  }
</script>

<section class="browser-panel" aria-label="Browser" data-testid="browser-panel">
  {#if browserWorkspace.presentation === 'docked'}
    <BrowserTabs workspace={browserWorkspace} onSelect={chooseTab} onClose={closeTab} onCreate={newTab} />
    <BrowserToolbar
      workspace={browserWorkspace}
      onNavigate={navigate}
      onReload={reloadBrowserFrame}
      onBack={goBack}
      onForward={goForward}
      onGrab={grab}
      onAnnotate={annotate}
      onDraw={draw}
      onOpenDevtools={openDevtools}
      onOpenExternal={openExternal}
      onViewport={viewport}
      onPresentation={presentation}
      onCollapse={collapse}
      onCancel={() => cancelBrowserAnnotation(browserModelContext())}
    />
    {#if browser.error}
      <p class="browser-error" role="alert">{browser.error}</p>
    {/if}
    <div class="browser-panel-body">
      <BrowserViewport workspace={browserWorkspace} />
      <BrowserFeedbackPanel
        workspace={browserWorkspace}
        onRemove={(id) => removeBrowserAnnotation(browserModelContext(), id)}
        onCopy={copy}
        onStage={stage}
      />
    </div>
  {:else}
    <div class="browser-docked-hidden" aria-hidden="true"></div>
  {/if}
</section>

<style>
  .browser-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  .browser-error {
    flex: 0 0 auto;
    margin: 0;
    padding: 7px 10px;
    border-bottom: 1px solid var(--color-bad);
    background: var(--color-bad-bg);
    color: var(--color-bad);
    font-size: 12px;
  }

  .browser-panel-body {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
  }

  .browser-docked-hidden {
    width: 100%;
    height: 100%;
  }
</style>
