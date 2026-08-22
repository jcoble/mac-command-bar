<script lang="ts">
  /**
   * TerminalSurface.svelte — the active xterm host for the /next shell.
   *
   * Only the active session owns a host and an xterm view. Switching sessions
   * destroys that frontend view while the PTY keeps running in the desktop
   * backend; returning rebuilds the view from the backend scrollback ring.
   *
   * The component does no IO and holds no state: it hands each host element to
   * the page through `registerHost` exactly once, on mount.
   *
   * It does own one other thing, and it is the only place that can: WHEN the
   * terminal has to be measured again. See `watchHost` below for what goes wrong
   * without it.
   */
  import '@xterm/xterm/css/xterm.css';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import {
    cancelTrackedAnimationFrame,
    requestTrackedAnimationFrame
  } from '$lib/shell/resourceDiagnostics.svelte';

  interface Props {
    /** Sessions CommandBar owns (rail.owned). */
    owned: OwnedSession[];
    /** `ownedId` of the session the manager is showing, if any. */
    activeOwnedId: string | null;
    /** Called once per host element, as soon as it is in the DOM. */
    registerHost(ownedId: string, host: HTMLElement): void;
    /**
     * This session's terminal needs measuring again — it just appeared, its box
     * changed size, or the font it draws with finished loading.
     *
     * Safe to call often: measuring re-reports the grid through the same path
     * every other measurement uses, and a grid that has not actually changed
     * costs nothing (the terminal service drops a resize that matches the size
     * the session is already at).
     */
    onHostLayout?(ownedId: string): void;
  }

  let { owned, activeOwnedId, registerHost, onHostLayout }: Props = $props();

  const active = $derived(owned.find((session) => session.ownedId === activeOwnedId) ?? null);
  const hosted = $derived(active && (active.state !== 'exited' || active.ptySessionId) ? active : null);

  /**
   * How many animation frames to keep re-measuring for after a terminal appears.
   *
   * It is not one, and that is the whole fix. xterm works out how big one
   * character is by measuring it on screen, and a terminal built inside a hidden
   * host cannot do that — so it holds a character size of zero. Showing the host
   * does start a re-measure, but xterm starts it from a browser notification
   * ("this element is on screen now") that is not delivered until the end of the
   * frame, well after the code that made the host visible has finished. Measuring
   * the grid in that gap is the bug in screenshots 46 and 52: xterm still has no
   * character size, so the fit does nothing and quietly reports the OLD grid,
   * which is then what the session is resized to. Nothing measures it again
   * afterwards, so the terminal stays wrong — the prompt sits above the line you
   * type on, the top of the input box is cut off, and the bottom of the output
   * runs past the end of the panel — until a divider is dragged.
   *
   * Four frames is comfortably past that first one, and the extra measurements
   * are free: a grid that has not changed sends nothing to the session.
   */
  const FRAMES_AFTER_APPEARING = 4;

  /**
   * Svelte action: publish the host element to the page, and from then on say
   * when this session's terminal needs measuring again.
   *
   * Three things ask for that, and before this the shell listened for none of
   * them — the only re-measure in the whole shell came from the middle tab area
   * reporting a layout, which a newly created session never triggers:
   *
   *  - the host appeared (the terminal manager sets `display` on it directly,
   *    which is why this watches the element's own style rather than any state);
   *  - its box changed size (a dragged divider, a resized window, a folded
   *    column);
   *  - the font finished loading, which changes how wide a character is after
   *    xterm has already measured one — the same wrong-grid symptom, arriving a
   *    beat later.
   */
  function host(node: HTMLElement, ownedId: string) {
    registerHost(ownedId, node);

    /** Is this host the one on screen? A hidden host has nothing to measure. */
    const shown = (): boolean => node.style.display !== 'none';

    let framesLeft = 0;
    let frame = 0;

    /** Ask for a measurement now and on the next few frames. */
    function remeasure(): void {
      if (!shown()) return;
      framesLeft = FRAMES_AFTER_APPEARING;
      if (frame !== 0) return;
      const step = (): void => {
        onHostLayout?.(ownedId);
        framesLeft -= 1;
        frame = framesLeft > 0 ? requestTrackedAnimationFrame(step) : 0;
      };
      frame = requestTrackedAnimationFrame(step);
    }

    const sizes = new ResizeObserver(() => remeasure());
    sizes.observe(node);

    const shownOrHidden = new MutationObserver(() => remeasure());
    shownOrHidden.observe(node, { attributes: true, attributeFilter: ['style'] });

    const fonts = document.fonts ?? null;
    const onFontsLoaded = (): void => remeasure();
    fonts?.addEventListener('loadingdone', onFontsLoaded);

    return {
      destroy(): void {
        if (frame !== 0) cancelTrackedAnimationFrame(frame);
        sizes.disconnect();
        shownOrHidden.disconnect();
        fonts?.removeEventListener('loadingdone', onFontsLoaded);
      }
    };
  }
</script>

<div class="surface">
  {#if hosted}
    {#key hosted.ownedId}
      <div
        class="term-host"
        style="display: none"
        data-owned-id={hosted.ownedId}
        use:host={hosted.ownedId}
      ></div>
    {/key}
  {/if}

  <!--
    The second case is the one that matters: a session whose terminal was closed
    has no host left, and the manager has no view to show for it, so whichever
    terminal was last on screen would simply stay there — live, focused, and
    belonging to a DIFFERENT session. The overlay is `inset: 0`, so it covers it
    and says what actually happened.
  -->
  {#if activeOwnedId === null}
    <div class="surface-empty">
      <p>No session selected.</p>
      <p class="hint">Pick a session in the rail, or resume one to start a terminal.</p>
    </div>
  {:else if !hosted}
    <div class="surface-empty">
      <p>This session’s terminal is closed.</p>
      <p class="hint">The transcript is still on disk; the session stays on your list.</p>
    </div>
  {/if}
</div>

<style>
  .surface {
    position: relative;
    height: 100%;
    width: 100%;
    overflow: hidden;
    background: var(--color-surface);
  }

  .term-host {
    position: absolute;
    inset: 0;
    padding: 6px 8px;
  }

  .surface-empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    /* The card's own surface colour, not the backdrop: this overlay reads as
       part of the panel rather than a hole in it. It stays opaque on purpose —
       covering a terminal that is still on screen is the whole reason it
       exists (see the note above) — so it cannot simply be dropped. */
    background: var(--color-surface);
    color: var(--color-text-2);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
  }

  .surface-empty p {
    margin: 0;
  }

  .hint {
    color: var(--color-text-3);
    font-size: 12px;
  }
</style>
