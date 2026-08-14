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
   * element picker; the still comes back the moment it has an answer.
   *
   * Marking a place is a numbered thing with words attached, not just ink. Each
   * element picked and each region drawn gets a circle on the picture and a row
   * in the card that floats at the foot of the page, and Send delivers all of
   * it — the sentence, the numbered list, and the marked-up picture — to the
   * session as one turn. It used to stop at the session's draft, and a message
   * staged in a box the reader was not looking at read as a button that did
   * nothing.
   */
  import { untrack } from 'svelte';
  import Globe from '@lucide/svelte/icons/globe';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { saveConversationClipboardImage } from '$lib/shell/conversation/conversationService.ts';
  import { sendToSession } from '$lib/shell/workbenchNavigation.ts';
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
    describeBrowserError,
    setBrowserPresentationMode
  } from '$lib/shell/browser/browserModel.ts';
  import type {
    BrowserInteractionMode,
    BrowserMarkupCapture,
    BrowserTabState
  } from '$lib/shell/browser/browserTypes.ts';
  import { normalizeBrowserUrl } from '$lib/shell/browser/normalizeBrowserUrl.ts';

  import AnnotationBadges from './AnnotationBadges.svelte';
  import AnnotationCanvas from './AnnotationCanvas.svelte';
  import BrowserMiniComposer from './BrowserMiniComposer.svelte';
  import BrowserToolbar from './BrowserToolbar.svelte';
  import { compositeAnnotations, liveShapes, type AnnotationShape, type PlacedAnnotationShape } from './annotationComposite.ts';
  import {
    addAnnotation,
    composeMarks,
    labelAnnotation,
    numberAnnotations,
    removeAnnotation,
    type AnnotationBox,
    type BrowserAnnotation
  } from './annotationList.ts';
  import { elementTagFromSelector, formatAnnotationRequest } from './browserAttachmentNote.ts';
  import {
    boundsForHost,
    expandedBoundsForHost,
    samePlacement,
    usableHostRect,
    HIDDEN_PLACEMENT,
    HOST_MIN_SIZE,
    type HostPlacement
  } from './browserPanelBounds.ts';

  interface Props {
    visible: boolean;
    root: string;
    /** The session whose browser this is. */
    ownedId: string | null;
  }
  let { visible, ownedId }: Props = $props();

  let pageHost = $state<HTMLDivElement | null>(null);
  /** The panel's own rows above the page — measured, never assumed. */
  let chromeHost = $state<HTMLDivElement | null>(null);
  let tool = $state<BrowserInteractionMode>('browse');
  let expanded = $state(false);
  let address = $state('');
  let addressEdited = $state(false);
  let description = $state('');
  let busy = $state(false);
  let failure = $state('');

  /** The places pointed at: a picked element or a drawn region, each with words. */
  let annotations = $state<BrowserAnnotation[]>([]);
  /** Freehand ink. It says what it says by being where it is, so it has no row. */
  let strokes = $state<PlacedAnnotationShape[]>([]);
  let editingId = $state<string | null>(null);
  let listOpen = $state(false);
  let layerSize = $state({ width: 0, height: 0 });

  let capture = $state<BrowserMarkupCapture | null>(null);
  let backdrop = $state<string | null>(null);
  let pickedTag = $state<string | null>(null);

  /** Bumped whenever the host rectangle could have moved. */
  let layoutTick = $state(0);
  let nextMarkId = 0;

  /**
   * Erasing takes a mark out of the list it came from rather than hiding it
   * behind a second one, so there is nothing for `liveShapes` to filter here.
   * It still runs: it is also what drops a mark too small to paint.
   */
  const NOTHING_ERASED: ReadonlySet<string> = new Set();

  const activeTab = $derived<BrowserTabState | null>(
    browser.workspace.activeTabId ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null : null
  );
  const marks = $derived(liveShapes(composeMarks(annotations, strokes), NOTHING_ERASED));
  const numbered = $derived(numberAnnotations(annotations));
  /**
   * The picker works on the live page, so the still steps aside for exactly as
   * long as Select is armed and comes back with the answer. Every other tool
   * marks the still, and it stays up between them: switching from Region to
   * Draw must not re-capture, or the second mark lands on a different picture
   * from the first.
   */
  const showsStill = $derived(backdrop !== null && tool !== 'picking');
  /** Marking is a mode, and the panel says so until it is sent or thrown away. */
  const annotating = $derived(backdrop !== null || tool === 'picking' || annotations.length > 0);
  const addressValue = $derived(addressEdited ? address : browser.inputUrl || browser.url);
  const errorText = $derived(failure || browser.error);
  const pageHostName = $derived(hostName(browser.url));

  function markId(): string {
    nextMarkId += 1;
    return `mark-${nextMarkId}`;
  }

  function say(error: unknown): void {
    failure = describeBrowserError(error);
  }

  /** The address without the scheme or the path — what the strip calls the page. */
  function hostName(url: string): string {
    try {
      return new URL(url).hostname || url;
    } catch {
      return url;
    }
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

  /**
   * Where the panel's own rows end. Measured from the rows themselves and not
   * from the page host, on purpose: the two agree whenever both are current,
   * and it is exactly when they disagree — a rectangle measured a layout ago,
   * before the tool row was there — that the expanded view was handed a top
   * edge above the controls that shrink it again, and painted over them.
   */
  function chromeBottom(): number {
    if (!chromeHost) return 0;
    const rect = chromeHost.getBoundingClientRect();
    return rect.height >= 1 ? rect.bottom : 0;
  }

  function windowSize(): { width: number; height: number } {
    return { width: window.innerWidth, height: window.innerHeight };
  }

  /** Where the view belongs right now, measured fresh. */
  function wantedPlacement(onScreen: boolean, stretched: boolean): HostPlacement {
    if (!onScreen || typeof window === 'undefined') return HIDDEN_PLACEMENT;
    const rect = hostRect();
    if (!usableHostRect(rect)) return HIDDEN_PLACEMENT;
    const size = windowSize();
    return {
      kind: 'bounds',
      bounds: stretched
        ? expandedBoundsForHost(rect, railRightEdge(), size, chromeBottom())
        : boundsForHost(rect, size)
    };
  }

  /**
   * What the shell was last asked for. A plain variable on purpose: the panel
   * re-measures on every layout change, and only a rectangle that actually
   * moved is worth a call.
   */
  let placed: HostPlacement | null = null;

  function sendPlacement(next: HostPlacement): void {
    if (placed && samePlacement(next, placed)) return;
    try {
      if (next.kind === 'hidden') {
        // Nothing has been opened yet, so there is no view to take away.
        if (!browser.workspace.activeTabId) {
          placed = next;
          return;
        }
        collapseBrowserToControl(browserModelContext());
      } else {
        setBrowserPresentationMode(browserModelContext(), 'floating', {
          window: windowSize(),
          bounds: next.bounds,
          minSize: HOST_MIN_SIZE
        });
      }
      placed = next;
    } catch (error) {
      say(error);
    }
  }

  /**
   * The native view is created at whatever rectangle the workspace is holding,
   * so the panel's own has to be in place before a page exists — otherwise the
   * first frame is painted wherever the browser last was, over the middle of
   * the shell.
   */
  function placeBeforeOpening(): void {
    const wanted = wantedPlacement(true, expanded);
    if (wanted.kind !== 'bounds') return;
    try {
      setBrowserPresentationMode(browserModelContext(), 'floating', {
        window: windowSize(),
        bounds: wanted.bounds,
        minSize: HOST_MIN_SIZE
      });
    } catch (error) {
      say(error);
    }
    // The view about to be created still needs its own placement call.
    placed = null;
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
    placeBeforeOpening();
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
    annotations = [];
    strokes = [];
    editingId = null;
    listOpen = false;
    pickedTag = null;
  }

  /** The X on the strip: everything marked goes, and the live page comes back. */
  function discard(): void {
    if (tool === 'picking') {
      try {
        cancelBrowserAnnotation(browserModelContext());
      } catch (error) {
        say(error);
      }
    }
    tool = 'browse';
    description = '';
    failure = '';
    dropStill();
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

  /**
   * A finished mark from the canvas. A region is a place, so it joins the list
   * and opens for a label straight away — asking what someone meant while they
   * still remember is the whole reason the label sits on the mark rather than
   * in one paragraph about everything. Freehand ink is not a place and does not
   * ask.
   */
  function addShape(shape: AnnotationShape): void {
    if (shape.kind === 'stroke') {
      strokes = [...strokes, { id: markId(), shape }];
      return;
    }
    const id = markId();
    annotations = addAnnotation(annotations, {
      id,
      box: shape as AnnotationBox,
      label: '',
      tag: shape.kind === 'element' ? shape.tag : 'region'
    });
    editingId = id;
  }

  /** An erase click, from wherever the mark came from. */
  function eraseShape(id: string): void {
    if (annotations.some((item) => item.id === id)) {
      forgetAnnotation(id);
      return;
    }
    strokes = strokes.filter((item) => item.id !== id);
  }

  function forgetAnnotation(id: string): void {
    annotations = removeAnnotation(annotations, id);
    if (editingId === id) editingId = null;
    if (annotations.length === 0) listOpen = false;
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
    pickedTag = elementTagFromSelector(event.selector);
    const rect = event.rect;
    tool = 'browse';
    if (!rect) return;
    // The picker works on the live page, so the picture the circle goes on has
    // to be taken now — after the click, with the element outlined on it.
    void (async () => {
      try {
        if (!capture) showStill(await captureNow());
      } catch (error) {
        say(error);
        return;
      }
      addShape({
        kind: 'element',
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        tag: pickedTag ?? 'element'
      });
    })();
  }

  // ── Sending it ─────────────────────────────────────────────────────────────

  const canSend = $derived(Boolean(ownedId) && Boolean(browser.url) && Boolean(activeTab));

  /**
   * One turn: the sentence, the numbered list of what was marked, and the
   * picture with the marks burned into it. Then the panel goes back to the live
   * page — the session is what to look at now, and it comes forward on its own.
   */
  async function send(): Promise<void> {
    if (!ownedId || busy || !canSend) return;
    busy = true;
    failure = '';
    editingId = null;
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
      await sendToSession({
        ownedId,
        text: formatAnnotationRequest({
          url: browser.url,
          description,
          annotations: numbered.map((item) => ({
            number: item.number,
            tag: item.tag,
            label: item.label
          }))
        }),
        attachments: [saved]
      });
      description = '';
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
    // The rows above move the page host and set the floor the expanded view is
    // held to, so a row appearing has to be a re-measure in its own right.
    if (chromeHost) observer.observe(chromeHost);
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
    const onScreen = visible && !showsStill && Boolean(browser.url);
    const stretched = expanded;
    layoutTick;
    browser.workspace.activeTabId;
    // Placing the view writes to the same shell state this effect reads from,
    // and a rectangle is a new object every time it is measured. Left tracked,
    // the effect would invalidate itself on its own writes and be torn down as
    // a runaway loop — which is what left the view stranded over the shell with
    // nothing able to move or hide it again short of restarting.
    untrack(() => sendPlacement(wantedPlacement(onScreen, stretched)));
  });

  // Whatever unmounts this panel — a different session, a rebuilt shell — the
  // view must not be left on screen behind it.
  $effect(() => () => untrack(() => sendPlacement(HIDDEN_PLACEMENT)));

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
  <div class="chrome" bind:this={chromeHost} data-testid="browser-panel-chrome">
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

    {#if annotating}
      <div class="strip" data-testid="browser-annotating-strip">
        <IconButton
          label="Discard these annotations"
          size="xs"
          data-testid="browser-annotating-discard"
          onclick={discard}
        >
          <X aria-hidden="true" />
        </IconButton>
        <span class="strip-title">Annotating · {pageHostName}</span>
        <Button
          size="sm"
          disabled={!canSend || busy}
          data-testid="browser-annotating-send"
          onclick={() => void send()}
        >
          {busy ? 'Sending' : 'Send'}
          {#if annotations.length > 0}
            <span class="strip-count">{annotations.length}</span>
          {/if}
        </Button>
      </div>
    {/if}
  </div>

  <div class="page" class:filled={showsStill} bind:this={pageHost} data-testid="browser-page-host">
    {#if !browser.url}
      <EmptyState
        title="No page open"
        body="Enter an http or https address above to open one here."
      >
        {#snippet icon()}<Globe strokeWidth={1.5} aria-hidden="true" />{/snippet}
      </EmptyState>
    {:else if showsStill}
      <div class="still">
        <AnnotationCanvas
          tool={tool === 'region' ? 'region' : tool === 'drawing' ? 'drawing' : 'erasing'}
          shapes={marks}
          {backdrop}
          onAdd={addShape}
          onErase={eraseShape}
          onResize={(size) => (layerSize = size)}
        />
        <AnnotationBadges
          annotations={numbered}
          {editingId}
          surface={layerSize}
          onLabel={(id, label) => (annotations = labelAnnotation(annotations, id, label))}
          onDoneEditing={() => (editingId = null)}
        />
      </div>
    {/if}

    <!-- The card floats inside the page host, so it is only there while the
         still is: over the live view it would be behind a native view, which
         is a card that cannot be read or typed into. Select is the one tool
         that runs on the live page, and the strip above still says so. -->
    {#if annotating && showsStill}
      <div class="floating">
        <BrowserMiniComposer
          {description}
          annotations={numbered}
          {backdrop}
          surface={layerSize}
          {busy}
          {listOpen}
          disabled={!canSend}
          error=""
          onDescriptionChange={(value) => (description = value)}
          onToggleList={() => (listOpen = !listOpen)}
          onRemove={forgetAnnotation}
          onSend={() => void send()}
        />
      </div>
    {/if}
  </div>

  {#if errorText}
    <p class="failure" role="alert" data-testid="browser-panel-error">{errorText}</p>
  {/if}
</div>

<style>
  .browser-panel {
    display: grid;
    height: 100%;
    width: 100%;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: var(--color-surface);
  }

  .strip {
    display: flex;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--color-border);
    padding: 6px 4px 0;
  }

  .strip-title {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    font-size: 12px;
    color: var(--color-text-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .strip-count {
    display: inline-flex;
    height: 16px;
    min-width: 16px;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.25);
    padding: 0 4px;
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
  }

  .page {
    position: relative;
    display: grid;
    min-height: 0;
    align-content: center;
    overflow: hidden;
  }

  /* "No page open" is a small block that belongs in the middle of the panel;
     the still is the panel. A centred row is sized by its content, so a still
     asked to fill one measures no height at all — and marks placed on a
     surface with no height have nowhere to be. */
  .page.filled {
    align-content: stretch;
  }

  /* The canvas and the numbers on it share one box, so a circle lands on the
     mark it belongs to rather than a few pixels off it. */
  .still {
    position: relative;
    min-height: 0;
    height: 100%;
    width: 100%;
  }

  /* The card belongs to the page it is about, so it floats at the foot of the
     picture rather than becoming another row of panel chrome under it. */
  .floating {
    position: absolute;
    right: 0;
    bottom: 12px;
    left: 0;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .floating > :global(*) {
    pointer-events: auto;
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
