<script lang="ts">
  /**
   * BrowserPanel.svelte — the Browser tab of the right column.
   *
   * The page is not in this document. It is a native child view the shell puts
   * on screen by window-space bounds, which is why this panel spends most of
   * its effort measuring: it hands over the rectangle it wants filled every
   * time that rectangle could have moved — mounting, the panel being resized,
   * the window being resized, the tab being switched away from and back, and
   * expanding. When the tab is not the one in front the view is taken off
   * screen entirely, or it would sit over whichever panel replaced it.
   *
   * Marking up works on a still of the page rather than the live view, for the
   * same reason: nothing in the document can be drawn over a native view. Arming
   * Region, Draw or Erase captures the page, hides the view, and puts that
   * picture under the canvas — so what gets marked is exactly what gets sent.
   * Select is the one tool that needs the live page, and it uses the shell's own
   * element picker.
   *
   * Attach hands over; it never sends. The picture and a short note go to the
   * session's composer and the session comes forward, and the person sends it.
   */
  import Globe from '@lucide/svelte/icons/globe';

  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { saveConversationClipboardImage } from '$lib/shell/conversation/conversationService.ts';
  import { focusComposerWith } from '$lib/shell/workbenchNavigation.ts';
  import {
    listenToBrowserElementSelected,
    type BrowserElementSelectedEvent
  } from '$lib/shell/browser/browserElementEvents.ts';
  import {
    activateBrowser,
    browser,
    browserModelContext,
    reloadBrowserFrame,
    setBrowserUrl
  } from '$lib/shell/browser/browserStore.svelte.ts';
  import {
    acceptBrowserElementSelection,
    beginBrowserElementPicker,
    cancelBrowserAnnotation,
    collapseBrowserToControl,
    setBrowserPresentationMode
  } from '$lib/shell/browser/browserModel.ts';
  import type {
    BrowserInteractionMode,
    BrowserMarkupCapture,
    BrowserTabState
  } from '$lib/shell/browser/browserTypes.ts';
  import { normalizeBrowserUrl } from '$lib/shell/browser/normalizeBrowserUrl.ts';

  import AnnotationCanvas from './AnnotationCanvas.svelte';
  import BrowserMiniComposer from './BrowserMiniComposer.svelte';
  import BrowserToolbar from './BrowserToolbar.svelte';
  import { compositeAnnotations, liveShapes, type AnnotationShape, type PlacedAnnotationShape } from './annotationComposite.ts';
  import { elementTagFromSelector, formatAttachmentNote } from './browserAttachmentNote.ts';
  import { boundsForHost, expandedBoundsForHost } from './browserPanelBounds.ts';

  interface Props {
    visible: boolean;
    root: string;
    /** The session whose browser this is. */
    ownedId: string | null;
  }
  let { visible, ownedId }: Props = $props();

  let pageHost = $state<HTMLDivElement | null>(null);
  let tool = $state<BrowserInteractionMode>('browse');
  let expanded = $state(false);
  let address = $state('');
  let addressEdited = $state(false);
  let description = $state('');
  let busy = $state(false);
  let failure = $state('');

  let shapes = $state<PlacedAnnotationShape[]>([]);
  let erased = $state<ReadonlySet<string>>(new Set());
  let layerSize = $state({ width: 0, height: 0 });

  let capture = $state<BrowserMarkupCapture | null>(null);
  let backdrop = $state<string | null>(null);
  let pickedSelector = $state<string | null>(null);
  let pickedTag = $state<string | null>(null);

  /** Bumped whenever the host rectangle could have moved. */
  let layoutTick = $state(0);
  let nextMarkId = 0;

  const activeTab = $derived<BrowserTabState | null>(
    browser.workspace.activeTabId ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null : null
  );
  const marks = $derived(liveShapes(shapes, erased));
  const drawingTool = $derived(tool === 'region' || tool === 'drawing' || tool === 'erasing');
  const showsStill = $derived(drawingTool && backdrop !== null);
  const addressValue = $derived(addressEdited ? address : browser.inputUrl || browser.url);
  const errorText = $derived(failure || browser.error);

  function markId(): string {
    nextMarkId += 1;
    return `mark-${nextMarkId}`;
  }

  function say(error: unknown): void {
    failure = error instanceof Error ? error.message : String(error);
  }

  // ── Where the native view goes ─────────────────────────────────────────────

  function hostRect(): { x: number; y: number; width: number; height: number } | null {
    if (!pageHost || typeof window === 'undefined') return null;
    const rect = pageHost.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return null;
    return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
  }

  /** Expanded reaches left to here — the session rail's right edge. */
  function railRightEdge(): number {
    if (typeof document === 'undefined') return 0;
    const rail = document.querySelector('[data-testid="session-rail"]');
    return rail ? rail.getBoundingClientRect().right : 0;
  }

  function placeNativeView(): void {
    const rect = hostRect();
    if (!rect || typeof window === 'undefined') return;
    const size = { width: window.innerWidth, height: window.innerHeight };
    const bounds = expanded
      ? expandedBoundsForHost(rect, railRightEdge(), size)
      : boundsForHost(rect, size);
    try {
      setBrowserPresentationMode(browserModelContext(), 'floating', { window: size, bounds });
    } catch (error) {
      say(error);
    }
  }

  function takeNativeViewOffScreen(): void {
    try {
      collapseBrowserToControl(browserModelContext());
    } catch (error) {
      say(error);
    }
  }

  // ── The page ───────────────────────────────────────────────────────────────

  function navigate(): void {
    const next = normalizeBrowserUrl(addressValue);
    if (!next) {
      failure = 'Enter an address that starts with http or https';
      return;
    }
    failure = '';
    if (ownedId) browser.workspace.ownedId = ownedId;
    activateBrowser();
    setBrowserUrl(next);
    addressEdited = false;
    dropStill();
    layoutTick += 1;
  }

  function step(direction: 'back' | 'forward'): void {
    if (!activeTab) return;
    const target = {
      workspaceId: activeTab.workspaceId,
      tabId: activeTab.id,
      generation: activeTab.generation
    };
    try {
      if (direction === 'back') void browser.backend.go_back_browser_tab(target);
      else void browser.backend.go_forward_browser_tab(target);
    } catch (error) {
      say(error);
    }
  }

  // ── The still under the marks ──────────────────────────────────────────────

  async function captureNow(): Promise<BrowserMarkupCapture> {
    if (!activeTab) throw new Error('Open a page before marking it up.');
    return await browser.backend.capture_browser_viewport({
      workspaceId: activeTab.workspaceId,
      tabId: activeTab.id,
      generation: activeTab.generation
    });
  }

  function showStill(shot: BrowserMarkupCapture): void {
    if (backdrop) URL.revokeObjectURL(backdrop);
    capture = shot;
    backdrop = URL.createObjectURL(
      new Blob([Uint8Array.from(shot.bytes)], { type: shot.mimeType || 'image/png' })
    );
  }

  function dropStill(): void {
    if (backdrop) URL.revokeObjectURL(backdrop);
    backdrop = null;
    capture = null;
    shapes = [];
    erased = new Set();
  }

  // ── The four tools ─────────────────────────────────────────────────────────

  async function chooseTool(next: BrowserInteractionMode): Promise<void> {
    failure = '';
    if (tool === 'picking' && next !== 'picking') {
      try {
        cancelBrowserAnnotation(browserModelContext());
      } catch (error) {
        say(error);
      }
    }

    if (next === 'region' || next === 'drawing' || next === 'erasing') {
      if (!capture) {
        try {
          showStill(await captureNow());
        } catch (error) {
          say(error);
          tool = 'browse';
          return;
        }
      }
      tool = next;
      return;
    }

    if (next === 'picking') {
      try {
        beginBrowserElementPicker(browserModelContext(), 'grab');
        tool = 'picking';
      } catch (error) {
        say(error);
        tool = 'browse';
      }
      return;
    }

    tool = 'browse';
  }

  function addShape(shape: AnnotationShape): void {
    shapes = [...shapes, { id: markId(), shape }];
  }

  function eraseShape(id: string): void {
    erased = new Set([...erased, id]);
  }

  /** The pick the shell sends back after Select armed the picker. */
  function receiveElement(event: BrowserElementSelectedEvent): void {
    if (event.workspaceId !== browser.workspace.workspaceId) return;
    if (event.status !== 'selected') {
      failure = event.reason?.trim() || 'That page would not hand over an element.';
      tool = 'browse';
      return;
    }
    try {
      acceptBrowserElementSelection(browserModelContext(), {
        selector: event.selector,
        accessibleName: event.accessibleName,
        textSnippet: event.textSnippet,
        rect: event.rect,
        classes: event.classes,
        classCount: event.classCount,
        sourceHash: event.sourceHash,
        url: event.url ?? undefined,
        title: event.title ?? undefined,
        generation: event.generation
      });
    } catch (error) {
      say(error);
      tool = 'browse';
      return;
    }
    pickedSelector = event.selector;
    pickedTag = elementTagFromSelector(event.selector);
    if (event.rect) {
      shapes = [
        ...shapes,
        {
          id: markId(),
          shape: {
            kind: 'element',
            x: event.rect.x,
            y: event.rect.y,
            width: event.rect.width,
            height: event.rect.height,
            tag: pickedTag ?? 'element'
          }
        }
      ];
    }
    tool = 'browse';
  }

  // ── Handing it to the composer ─────────────────────────────────────────────

  const canAttach = $derived(Boolean(ownedId) && Boolean(browser.url) && Boolean(activeTab));

  async function attach(): Promise<void> {
    if (!ownedId || busy) return;
    busy = true;
    failure = '';
    try {
      const shot = capture ?? (await captureNow());
      const surface =
        layerSize.width > 0 && layerSize.height > 0
          ? layerSize
          : { width: shot.width, height: shot.height };
      const file = await compositeAnnotations(
        shot,
        marks.map((mark) => mark.shape),
        surface,
        { url: browser.url }
      );
      const saved = await saveConversationClipboardImage(ownedId, file);
      await focusComposerWith({
        ownedId,
        attachments: [saved],
        appendText: formatAttachmentNote({
          url: browser.url,
          selector: pickedSelector,
          tag: pickedTag,
          description
        })
      });
      description = '';
      pickedSelector = null;
      pickedTag = null;
      tool = 'browse';
      dropStill();
    } catch (error) {
      say(error);
    } finally {
      busy = false;
    }
  }

  // ── Keeping the rectangle current ──────────────────────────────────────────

  $effect(() => {
    if (!pageHost) return;
    const observer = new ResizeObserver(() => {
      layoutTick += 1;
    });
    observer.observe(pageHost);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    const onResize = (): void => {
      layoutTick += 1;
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  $effect(() => {
    // Every input that can move the view, read so the effect re-runs.
    visible;
    expanded;
    showsStill;
    layoutTick;
    browser.url;
    browser.workspace.activeTabId;
    if (!visible || showsStill || !browser.url) {
      takeNativeViewOffScreen();
      return;
    }
    placeNativeView();
  });

  $effect(() => {
    let stop: (() => void) | null = null;
    let dropped = false;
    void listenToBrowserElementSelected(receiveElement)
      .then((unsubscribe) => {
        if (dropped) unsubscribe();
        else stop = unsubscribe;
      })
      .catch(() => undefined);
    return () => {
      dropped = true;
      stop?.();
    };
  });

  $effect(() => {
    return () => {
      if (backdrop) URL.revokeObjectURL(backdrop);
    };
  });
</script>

<div class="browser-panel" class:expanded data-testid="browser-panel">
  <BrowserToolbar
    address={addressValue}
    {tool}
    {expanded}
    canGoBack={activeTab?.canGoBack ?? false}
    canGoForward={activeTab?.canGoForward ?? false}
    elementTag={pickedTag}
    onAddressInput={(value) => {
      addressEdited = true;
      address = value;
    }}
    onNavigate={navigate}
    onBack={() => step('back')}
    onForward={() => step('forward')}
    onReload={reloadBrowserFrame}
    onToolChange={(next) => void chooseTool(next)}
    onToggleExpand={() => {
      expanded = !expanded;
      layoutTick += 1;
    }}
  />

  <div class="page" bind:this={pageHost} data-testid="browser-page-host">
    {#if !browser.url}
      <EmptyState
        title="No page open"
        body="Enter an http or https address above to open one here."
      >
        {#snippet icon()}<Globe strokeWidth={1.5} aria-hidden="true" />{/snippet}
      </EmptyState>
    {:else if showsStill}
      <AnnotationCanvas
        tool={tool === 'region' ? 'region' : tool === 'drawing' ? 'drawing' : 'erasing'}
        shapes={marks}
        {backdrop}
        onAdd={addShape}
        onErase={eraseShape}
        onResize={(size) => (layerSize = size)}
      />
    {/if}
  </div>

  {#if errorText}
    <p class="failure" role="alert" data-testid="browser-panel-error">{errorText}</p>
  {/if}

  <BrowserMiniComposer
    {description}
    {busy}
    markCount={marks.length}
    disabled={!canAttach}
    error=""
    onDescriptionChange={(value) => (description = value)}
    onAttach={() => void attach()}
  />
</div>

<style>
  .browser-panel {
    display: grid;
    height: 100%;
    width: 100%;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: var(--color-surface);
  }

  .page {
    position: relative;
    display: grid;
    min-height: 0;
    align-content: center;
    overflow: hidden;
  }

  /* Expanding and collapsing move the native view, which the shell repositions
     in one step. Nothing in this document animates, so both end at rest. */

  .failure {
    margin: 0;
    border-top: 1px solid var(--color-border);
    padding: 8px 12px;
    font-size: 12px;
    color: var(--color-bad);
  }
</style>
