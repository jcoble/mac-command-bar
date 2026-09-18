<script lang="ts">
  /**
   * BrowserPanel.svelte — the Browser tab of the right column.
   *
   * The page is not in this document. It is a native child view the shell puts
   * on screen by window-space bounds, which is why this panel spends most of
   * its effort measuring: it hands over the rectangle it wants filled every
   * time that rectangle could have moved — mounting, the panel being resized,
   * the window being resized, and the tab being switched away from and back.
   * When the tab is not the one in front the view is taken off screen entirely,
   * or it would sit over whichever panel replaced it.
   *
   * The page has no width of its own. It is the right column's content, so the
   * column's width IS the page's width: dragging the seam between the center
   * and this column resizes the page. Filling the workspace is the one
   * exception: marking up wants the biggest picture the window can give, so
   * the whole panel — its rows and its page — moves out of the column and lies
   * over everything right of the sessions rail until it is put back. The rail
   * keeps its place, because it is how the reader gets to another session
   * without first putting the page away. It moves out of the document
   * position it had, not just out of the column: the column paints its panels
   * inside its own box (`contain: paint`), so a panel that only positioned
   * itself over the shell would still be clipped to the column.
   *
   * Marking up works on a still of the page rather than the live view, for the
   * same reason: nothing in the document can be drawn over a native view. The
   * first tool armed captures the page, hides the view, and puts that picture
   * under the canvas — so what gets marked is exactly what gets sent, and the
   * still stays up until the turn goes or the marks are thrown away. Nothing
   * about the panel's own rows changes while it is up: the still fills the
   * rectangle the page filled, and everything marking needs floats over it.
   *
   * Annotate asks the page what is under the pointer and outlines it. It does
   * that through the metadata inspector rather than the shell's element picker,
   * because the picker needs the live view in front — and a live view in front
   * takes the still, the marks already made and the half-typed sentence off the
   * screen every time another place is pointed at.
   *
   * Marking a place is a numbered thing with words attached, not just ink. Each
   * element picked and each region drawn gets a circle on the picture and a row
   * in the card that floats at the foot of the page, and Send delivers all of
   * it — the sentence, the numbered list, and the marked-up picture — to the
   * session as one turn. It used to stop at the session's draft, and a message
   * staged in a box the reader was not looking at read as a button that did
   * nothing.
   */
  import { onMount, untrack } from 'svelte';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import Globe from '@lucide/svelte/icons/globe';
  import X from '@lucide/svelte/icons/x';

  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { saveConversationClipboardImage } from '$lib/shell/conversation/conversationService.ts';
  import {
    registerBrowserUrlNavigation,
    sendToSession,
    type OpenUrlRequest
  } from '$lib/shell/workbenchNavigation.ts';
  import {
    createTrackedObjectUrl,
    revokeTrackedObjectUrl
  } from '$lib/shell/resourceDiagnostics.svelte';
  import {
    activateBrowser,
    browser,
    browserModelContext,
    captureBrowserState,
    closeBrowserPageTab,
    createBrowserPageTab,
    hasRestoredBrowserTabs,
    reloadBrowserFrame,
    releaseBrowserWorkspace,
    setBrowserUrl,
    syncBrowserNavigation,
    syncBrowserTab
  } from '$lib/shell/browser/browserStore.svelte.ts';
  import { subscribeToBrowserNavigation } from '$lib/shell/browser/browserBackend.ts';
  import {
    collapseBrowserToControl,
    describeBrowserError,
    setBrowserPresentationMode
  } from '$lib/shell/browser/browserModel.ts';
  import type {
    BrowserElementMetadata,
    BrowserMarkupCapture,
    BrowserPanelTool,
    BrowserRect,
    BrowserTabState
  } from '$lib/shell/browser/browserTypes.ts';
  import type { SessionBrowserWorkspace } from '$lib/shell/sessionWorkspaces.ts';
  import { normalizeBrowserUrl } from '$lib/shell/browser/normalizeBrowserUrl.ts';

  import AnnotationBadges from './AnnotationBadges.svelte';
  import AnnotationCanvas from './AnnotationCanvas.svelte';
  import BrowserMiniComposer from './BrowserMiniComposer.svelte';
  import BrowserTabs from './BrowserTabs.svelte';
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
    hostRectFitsPanel,
    samePlacement,
    usableHostRect,
    HIDDEN_PLACEMENT,
    HOST_MIN_SIZE,
    type HostPlacement
  } from './browserPanelBounds.ts';

  interface Props {
    visible: boolean;
    panelOpen: boolean;
    root: string;
    /** The session whose browser this is. */
    ownedId: string | null;
    onWorkspaceChange?(ownedId: string, state: SessionBrowserWorkspace): void;
  }
  let { visible, panelOpen, root, ownedId, onWorkspaceChange }: Props = $props();

  let pageHost = $state<HTMLDivElement | null>(null);
  /** The panel's own rows above the page — measured, never assumed. */
  let chromeHost = $state<HTMLDivElement | null>(null);
  let tool = $state<BrowserPanelTool>('browse');
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
  /** What the page says is under the pointer while Annotate is armed. */
  let hovered = $state<BrowserRect | null>(null);
  /**
   * One question to the page at a time. The pointer moves far faster than a
   * round trip to the page, so the newest point waits for the answer in flight
   * and the ones behind it are dropped — the outline follows the pointer
   * without a queue of stale questions building up behind it.
   */
  let askingPage = false;
  let pendingPoint: { x: number; y: number } | null = null;
  /** Bumped when the outline is called off, so an answer already in flight for
   *  a pointer that has since left does not put it back. */
  let hoverEpoch = 0;

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
  const browserTabs = $derived.by(() => {
    const tabs: BrowserTabState[] = [];
    for (const id of browser.workspace.tabOrder) {
      const tab = browser.workspace.tabs[id];
      if (tab) tabs.push(tab);
    }
    return tabs;
  });
  const marks = $derived(liveShapes(composeMarks(annotations, strokes), NOTHING_ERASED));
  const numbered = $derived(numberAnnotations(annotations));
  /**
   * The still is up from the moment the first tool is armed until the turn is
   * sent or thrown away. It never steps aside for a tool change: switching from
   * Region to Draw must not re-capture, or the second mark lands on a different
   * picture from the first — and disarming must not take what has been marked
   * and typed off the screen.
   */
  const showsStill = $derived(backdrop !== null);
  /** Lying over the workspace. Only while this tab is the one in front: a
   * panel that is not showing must not be found lying over everything else. */
  const fillsWindow = $derived(expanded && visible);
  /**
   * The rectangle a filled panel lies in: everything right of the sessions
   * rail, out to the right edge of the tool column. The rail stays where it
   * is — a page filling the window used to cover it, and the sessions list is
   * how the reader gets anywhere else.
   *
   * Both edges are measured rather than assumed: the rail can be dragged to
   * any width and folded to a strip, and the tool column's right edge is where
   * the shell's own gutter puts it.
   */
  const fillInset = $derived.by(() => {
    layoutTick;
    if (!fillsWindow || typeof document === 'undefined') return null;
    const rail = document.querySelector('.shell-region-host-sessions');
    const tools = document.querySelector('.shell-region-host-tools');
    if (!rail || !tools) return null;
    const railRect = rail.getBoundingClientRect();
    const toolsRect = tools.getBoundingClientRect();
    return {
      top: railRect.top,
      left: railRect.right,
      right: Math.max(0, window.innerWidth - toolsRect.right),
      bottom: Math.max(0, window.innerHeight - railRect.bottom)
    };
  });
  const markupBounds = $derived.by(() => {
    layoutTick;
    const placement = wantedPlacement(true);
    // Nothing to measure against means no layer this pass. The fallback used
    // to be a zero rect, which as `inset: 0` was a sheet over the whole window
    // — toolbar, address bar and all — whenever the host measured zero.
    if (placement.kind !== 'bounds' || typeof window === 'undefined') return null;
    const { x, y, width, height } = placement.bounds;
    return {
      x,
      y,
      right: Math.max(0, window.innerWidth - x - width),
      bottom: Math.max(0, window.innerHeight - y - height)
    };
  });
  const addressValue = $derived(addressEdited ? address : browser.inputUrl || browser.url);
  /**
   * One place for anything that went wrong, under the page and never over it:
   * the still covers the page area exactly, so a message drawn inside it is a
   * message that comes and goes with the marking.
   */
  const errorText = $derived(failure || activeTab?.error || browser.error);

  function markId(): string {
    nextMarkId += 1;
    return `mark-${nextMarkId}`;
  }

  function say(error: unknown): void {
    failure = describeBrowserError(error);
  }

  function bodyPortal(node: HTMLElement): { destroy(): void } {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  /**
   * Move the panel out to the document body while it fills the window, and
   * back to the place in the column it came from when it stops. The column
   * clips what it contains, so lying over the shell means leaving the column.
   */
  function fillWindow(node: HTMLElement, fills: boolean): { update(fills: boolean): void } {
    const home = node.parentElement;
    const place = (out: boolean): void => {
      if (out) {
        if (node.parentElement !== document.body) document.body.appendChild(node);
      } else if (home && node.parentElement !== home) {
        home.appendChild(node);
      }
    };
    place(fills);
    return { update: place };
  }

  // ── Where the native view goes ─────────────────────────────────────────────

  function hostRect(): { x: number; y: number; width: number; height: number } | null {
    if (!pageHost || typeof window === 'undefined') return null;
    const rect = pageHost.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return null;
    const measured = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
    if (fillsWindow) return measured;

    const toolsHost = pageHost.closest<HTMLElement>('.shell-region-host-tools');
    const centerHost = document.querySelector<HTMLElement>('.shell-region-host-center');
    if (!toolsHost || !centerHost) return null;
    const tools = toolsHost.getBoundingClientRect();
    const center = centerHost.getBoundingClientRect();
    const toolsRect = { x: tools.left, y: tools.top, width: tools.width, height: tools.height };
    const centerRect = { x: center.left, y: center.top, width: center.width, height: center.height };
    return hostRectFitsPanel(measured, toolsRect, centerRect) ? measured : null;
  }

  function windowSize(): { width: number; height: number } {
    return { width: window.innerWidth, height: window.innerHeight };
  }

  /** Where the view belongs right now, measured fresh. Always the page host and
   * nothing else — however wide the column has been made. */
  function wantedPlacement(onScreen: boolean): HostPlacement {
    if (!onScreen || typeof window === 'undefined') return HIDDEN_PLACEMENT;
    const rect = hostRect();
    if (!usableHostRect(rect)) return HIDDEN_PLACEMENT;
    return { kind: 'bounds', bounds: boundsForHost(rect, windowSize()) };
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
        // Nothing has been put on screen, so there is no view to take away —
        // and asking the shell to hide a workspace it never opened comes back
        // as a page error the reader can neither act on nor dismiss.
        if (!browser.workspace.activeTabId || !browser.workspace.activated) {
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
        // A rectangle is only delivered to the native view when a tab is open
        // to receive it — the model moves nothing otherwise. Remembering one
        // that was never delivered is what made Widen dead every other press:
        // the panel believed the view was already there, so the next placement
        // equal to it was skipped and the view never moved. Forget instead, and
        // the following measurement is sent for real.
        if (!browser.workspace.activeTabId) {
          placed = null;
          return;
        }
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
    const wanted = wantedPlacement(true);
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

  function persistWorkspace(): void {
    if (ownedId) onWorkspaceChange?.(ownedId, captureBrowserState());
  }

  function navigate(): void {
    if (!root) return;
    const next = normalizeBrowserUrl(addressValue);
    if (!next) {
      failure = 'Enter an http, https, or file address';
      return;
    }
    failure = '';
    if (ownedId) browser.workspace.ownedId = ownedId;
    placeBeforeOpening();
    activateBrowser();
    setBrowserUrl(next);
    persistWorkspace();
    addressEdited = false;
    dropStill();
    layoutTick += 1;
  }

  function openRequestedUrl(request: OpenUrlRequest): void {
    address = request.url;
    addressEdited = true;
    navigate();
  }

  onMount(() => registerBrowserUrlNavigation(openRequestedUrl));

  function openPageTab(): void {
    if (!root) return;
    failure = '';
    if (ownedId) browser.workspace.ownedId = ownedId;
    placeBeforeOpening();
    if (!createBrowserPageTab()) return;
    persistWorkspace();
    address = '';
    addressEdited = false;
    dropStill();
    layoutTick += 1;
  }

  function selectPageTab(tabId: string): void {
    if (tabId === browser.workspace.activeTabId) return;
    failure = '';
    syncBrowserTab(tabId);
    persistWorkspace();
    address = '';
    addressEdited = false;
    dropStill();
    layoutTick += 1;
  }

  function closePageTab(tabId: string): void {
    failure = '';
    closeBrowserPageTab(tabId);
    persistWorkspace();
    address = '';
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
    if (backdrop) revokeTrackedObjectUrl(backdrop);
    capture = shot;
    backdrop = createTrackedObjectUrl(
      new Blob([Uint8Array.from(shot.bytes)], { type: shot.mimeType || 'image/png' })
    );
  }

  function dropStill(): void {
    stopHover();
    if (backdrop) revokeTrackedObjectUrl(backdrop);
    backdrop = null;
    capture = null;
    annotations = [];
    strokes = [];
    editingId = null;
    listOpen = false;
  }

  /** The cross on the chip: everything marked goes, and the live page is back. */
  function discard(): void {
    tool = 'browse';
    description = '';
    failure = '';
    dropStill();
  }

  // ── The four tools ─────────────────────────────────────────────────────────

  /**
   * Every tool works on the still, so arming one takes the picture if there
   * is not one already. Disarming leaves it up: what has been marked and typed
   * is the turn being written, and it does not go away because the pointer
   * went back to doing nothing.
   */
  async function chooseTool(next: BrowserPanelTool): Promise<void> {
    failure = '';
    stopHover();
    if (next === 'browse') {
      tool = 'browse';
      return;
    }
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
  }

  /**
   * A finished mark from the canvas. A region is a place, so it joins the list
   * and opens for a label straight away — asking what someone meant while they
   * still remember is the whole reason the label sits on the mark rather than
   * in one paragraph about everything. Freehand ink is not a place and does not
   * ask.
   */
  async function addShape(
    shape: AnnotationShape,
    supplied?: BrowserElementMetadata | null,
    pin?: { x: number; y: number }
  ): Promise<void> {
    if (shape.kind === 'stroke') {
      strokes = [...strokes, { id: markId(), shape }];
      return;
    }
    let metadata = supplied ?? null;
    if (!metadata && shape.width > 0 && shape.height > 0 && activeTab) {
      try {
        metadata = await browser.backend.inspect_browser_rect({
          workspaceId: activeTab.workspaceId,
          tabId: activeTab.id,
          generation: activeTab.generation,
          rect: { x: shape.x, y: shape.y, width: shape.width, height: shape.height }
        });
      } catch (error) {
        say(error);
      }
    }
    const id = markId();
    annotations = addAnnotation(annotations, {
      id,
      box: shape as AnnotationBox,
      pin: pin ?? { x: shape.x, y: shape.y },
      label: '',
      tag: shape.kind === 'element' ? shape.tag : elementTagFromSelector(metadata?.selector) ?? 'region',
      selector: metadata?.selector ?? null,
      role: metadata?.role ?? null,
      accessibleName: metadata?.accessibleName ?? null,
      textSnippet: metadata?.textSnippet ?? null,
      classes: metadata?.classes ?? []
    });
    editingId = id;
  }

  // ── Asking the page what is under the pointer ──────────────────────────────

  /**
   * The still is a picture, so the panel cannot read anything off it. What is
   * at a point comes from the page itself, which is still loaded behind the
   * still and answers about the same coordinates the still is drawn in.
   */
  async function elementAt(point: { x: number; y: number }): Promise<BrowserElementMetadata | null> {
    if (!activeTab) return null;
    return await browser.backend.inspect_browser_rect({
      workspaceId: activeTab.workspaceId,
      tabId: activeTab.id,
      generation: activeTab.generation,
      rect: { x: Math.max(0, point.x), y: Math.max(0, point.y), width: 1, height: 1 }
    });
  }

  function stopHover(): void {
    hoverEpoch += 1;
    pendingPoint = null;
    hovered = null;
  }

  function hoverAt(point: { x: number; y: number } | null): void {
    if (!point) {
      stopHover();
      return;
    }
    pendingPoint = point;
    if (askingPage) return;
    void inspectPendingHoverPoints();
  }

  async function inspectPendingHoverPoints(): Promise<void> {
    askingPage = true;
    try {
      while (pendingPoint) {
        const asked = pendingPoint;
        const epoch = hoverEpoch;
        pendingPoint = null;
        // Still inside the box already outlined: the answer cannot change.
        if (hovered && withinRect(hovered, asked)) continue;
        const found = await elementAt(asked);
        if (epoch !== hoverEpoch) return;
        hovered = found?.rect ?? null;
      }
    } catch {
      hovered = null;
    } finally {
      askingPage = false;
    }
  }

  function withinRect(rect: BrowserRect, point: { x: number; y: number }): boolean {
    return (
      point.x >= rect.x
      && point.x <= rect.x + rect.width
      && point.y >= rect.y
      && point.y <= rect.y + rect.height
    );
  }

  /**
   * A click while Annotate is armed. The pin goes where the pointer was, so
   * three marks on one row of buttons are told apart by where they are rather
   * than by three circles stacked in the same corner. The mode stays armed:
   * the next place is another click, not another trip to the toolbar.
   */
  async function pickAt(point: { x: number; y: number }): Promise<void> {
    pendingPoint = null;
    const epoch = hoverEpoch;
    let metadata: BrowserElementMetadata | null = null;
    try {
      metadata = await elementAt(point);
    } catch (error) {
      say(error);
      return;
    }
    const rect = metadata?.rect;
    if (!rect || rect.width < 1 || rect.height < 1) {
      failure = 'Nothing on the page answered for that spot.';
      return;
    }
    if (epoch === hoverEpoch) hovered = rect;
    await addShape(
      {
        kind: 'element',
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        tag: elementTagFromSelector(metadata?.selector) ?? 'element'
      },
      metadata,
      point
    );
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

  // ── Sending it ─────────────────────────────────────────────────────────────

  const canSend = $derived(Boolean(ownedId) && Boolean(browser.url) && Boolean(activeTab));
  const readyToSend = $derived(
    canSend && (description.trim().length > 0 || annotations.length > 0 || strokes.length > 0)
  );
  const annotatingHost = $derived.by(() => {
    try {
      return new URL(browser.url).host;
    } catch {
      return browser.url;
    }
  });

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
            role: item.role,
            label: item.label,
            selector: item.selector,
            accessibleName: item.accessibleName,
            textSnippet: item.textSnippet,
            classes: item.classes
          }))
        }),
        attachments: [saved]
      });
      description = '';
      tool = 'browse';
      dropStill();
    } catch (error) {
      // The still stays up with the marks on it, so a failed send can be tried
      // again rather than drawn again. Discard is always reachable now that
      // freehand marks count, so this is not the trap it was.
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
    // While the panel lies over the workspace its left edge is the sessions
    // rail's right edge, so dragging or folding the rail moves the panel — and
    // nothing else would notice, since the panel is out of the grid by then.
    if (fillsWindow) {
      const rail = document.querySelector('.shell-region-host-sessions');
      if (rail) observer.observe(rail);
    }
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
    const onScreen = visible && Boolean(root) && !showsStill && Boolean(browser.url);
    layoutTick;
    browser.workspace.activeTabId;
    // Placing the view writes to the same shell state this effect reads from,
    // and a rectangle is a new object every time it is measured. Left tracked,
    // the effect would invalidate itself on its own writes and be torn down as
    // a runaway loop — which is what left the view stranded over the shell with
    // nothing able to move or hide it again short of restarting.
    untrack(() => sendPlacement(wantedPlacement(onScreen)));
  });

  /**
   * A session that comes back to a page it had open comes back to the page.
   * Closing the right region or changing sessions releases the native view, so
   * the address survives but the view does not. Opening it again here restores
   * the saved page.
   */
  $effect(() => {
    const wanted = visible
      && Boolean(root)
      && (Boolean(browser.url) || hasRestoredBrowserTabs())
      && !browser.workspace.activeTabId;
    if (!wanted) return;
    untrack(() => {
      placeBeforeOpening();
      activateBrowser();
      layoutTick += 1;
    });
  });

  // Switching right-side tabs only hides the native view through placement.
  // Release it when the whole right region closes or this controller is handed
  // to another session/root.
  $effect(() => {
    const ownsResources = panelOpen;
    ownedId;
    root;
    if (!ownsResources) return;
    return () => untrack(() => {
      releaseBrowserWorkspace();
      discard();
    });
  });

  $effect(() => {
    ownedId;
    root;
    untrack(() => {
      discard();
      expanded = false;
      address = '';
      addressEdited = false;
    });
  });

  $effect(() => {
    if (!visible || !root || !ownedId) return;
    // The diagnostics wrapper increments a reactive counter while subscribing.
    // Keep that bookkeeping outside this effect's dependencies or the counter
    // invalidates the effect that just changed it.
    const stopNavigation = untrack(() => subscribeToBrowserNavigation((event) => {
      syncBrowserNavigation(event);
      persistWorkspace();
    }));
    return () => stopNavigation();
  });

  $effect(() => {
    return () => {
      if (backdrop) revokeTrackedObjectUrl(backdrop);
    };
  });
</script>

<div
  class="browser-panel"
  class:fills-window={fillsWindow}
  use:fillWindow={fillsWindow}
  style={fillInset
    ? `--fill-top:${fillInset.top}px;--fill-right:${fillInset.right}px;--fill-bottom:${fillInset.bottom}px;--fill-left:${fillInset.left}px`
    : undefined}
  data-testid="browser-panel"
>
  <div class="chrome" bind:this={chromeHost} data-testid="browser-panel-chrome">
    <BrowserTabs
      tabs={browserTabs}
      activeTabId={browser.workspace.activeTabId}
      disabled={!root}
      onSelect={selectPageTab}
      onClose={closePageTab}
      onNew={openPageTab}
    />
    <BrowserToolbar
      address={addressValue}
      {tool}
      {expanded}
      canGoBack={activeTab?.canGoBack ?? false}
      canGoForward={activeTab?.canGoForward ?? false}
      onAddressInput={(value) => {
        addressEdited = true;
        address = value;
      }}
      onNavigate={navigate}
      onBack={() => step('back')}
      onForward={() => step('forward')}
      onReload={reloadBrowserFrame}
      onToolChange={(next) => void chooseTool(next)}
      onToggleExpand={() => (expanded = !expanded)}
    />
    {#if showsStill}
      <div class="annotation-header" data-testid="browser-annotation-header">
        <span>Annotating <span aria-hidden="true">·</span> {annotatingHost}</span>
        <div class="annotation-header-actions">
          <IconButton
            label="Discard annotations"
            size="xs"
            variant="ghost"
            onclick={discard}
          >
            <X aria-hidden="true" />
          </IconButton>
          <Button size="sm" disabled={!readyToSend || busy} onclick={() => void send()}>
            <ArrowUp aria-hidden="true" />
            Send
          </Button>
        </div>
      </div>
    {/if}
  </div>

  <div class="page" bind:this={pageHost} data-testid="browser-page-host">
    {#if !browser.url}
      <EmptyState
        title="No page open"
        body="Enter an http, https, or local file address above to open one here."
      >
        {#snippet icon()}<Globe strokeWidth={1.5} aria-hidden="true" />{/snippet}
      </EmptyState>
    {/if}
  </div>

  {#if showsStill && visible && markupBounds}
    <div
      class="markup-layer"
      use:bodyPortal
      style="top: {markupBounds.y}px; right: {markupBounds.right}px; bottom: {markupBounds.bottom}px; left: {markupBounds.x}px"
    >
      <div class="still">
        <AnnotationCanvas
          {tool}
          shapes={marks}
          {backdrop}
          highlight={tool === 'element' ? hovered : null}
          onAdd={addShape}
          onErase={eraseShape}
          onHover={hoverAt}
          onPick={(point) => void pickAt(point)}
          onResize={(size) => (layerSize = size)}
        />
        <AnnotationBadges
          annotations={numbered}
          {editingId}
          surface={layerSize}
          onLabel={(id, label) => (annotations = labelAnnotation(annotations, id, label))}
          onEdit={(id) => (editingId = id)}
          onDoneEditing={() => (editingId = null)}
        />
        <div class="floating-composer">
          <BrowserMiniComposer
            {description}
            annotations={numbered}
            inkCount={strokes.length}
            {backdrop}
            surface={layerSize}
            {busy}
            {listOpen}
            disabled={!canSend}
            onDescriptionChange={(value) => (description = value)}
            onToggleList={() => (listOpen = !listOpen)}
            onRemove={forgetAnnotation}
            onDiscard={discard}
            onSend={() => void send()}
          />
        </div>
      </div>
    </div>
  {/if}

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

  /* Over the workspace right of the sessions rail — the rail stays readable.
     The four edges come from the measured rectangle above; the zeroes are only
     what a pass with nothing to measure falls back to. Under the markup layer,
     which is the next thing up in the document body; the page host is measured
     wherever it is, so the native view and the still follow the panel out here
     on their own. */
  .browser-panel.fills-window {
    position: fixed;
    top: var(--fill-top, 0);
    right: var(--fill-right, 0);
    bottom: var(--fill-bottom, 0);
    left: var(--fill-left, 0);
    z-index: 1;
  }

  .annotation-header {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid var(--color-border);
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text);
  }

  .annotation-header > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .annotation-header-actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: 4px;
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
  .markup-layer {
    position: fixed;
    z-index: 2;
    display: grid;
    min-height: 0;
    overflow: hidden;
    background: var(--color-surface);
  }

  /* Over the foot of the page, never under it: marking must not move the page
     it is about, and the page is a native view that resizes when it is asked
     to. */
  .floating-composer {
    position: absolute;
    right: 0;
    bottom: 16px;
    left: 0;
    display: flex;
    justify-content: center;
    /* The row is as wide as the page; only the card in it is a thing to click.
       Left solid, it would be a strip across the foot of the page where marks
       cannot be made. */
    pointer-events: none;
  }

  .floating-composer > :global(*) {
    pointer-events: auto;
  }

  /* The canvas and the numbers on it share one box, so a circle lands on the
     mark it belongs to rather than a few pixels off it. */
  .still {
    position: relative;
    min-height: 0;
    height: 100%;
    width: 100%;
  }

  /* Widening moves the seam, which resizes this panel and the native view with
     it. Nothing in this document animates, so both end at rest. */

  .failure {
    margin: 0;
    border-top: 1px solid var(--color-border);
    padding: 8px 12px;
    font-size: 12px;
    color: var(--color-bad);
  }
</style>
