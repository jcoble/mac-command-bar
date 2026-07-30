<script lang="ts">
  /**
   * TerminalSurface.svelte — the stack of xterm hosts for the /next shell.
   *
   * ONE rule, and it is the whole reason this component is separate: a host is
   * NEVER wrapped in `{#if active}`. Every owned session with a (possibly
   * pending) PTY keeps a mounted host for the entire life of the page, and
   * visibility is toggled by the terminal manager through `view.setVisible()`
   * (which sets `display` on the host). Gating the markup instead would destroy
   * the xterm DOM on every switch — the old shell's "my agent restarted when I
   * clicked away" bug, in new clothes.
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

  /**
   * An exited session keeps its host only while its PTY id is still around
   * (so its final scrollback stays readable); once that is gone there is
   * nothing to render.
   */
  const hosted = $derived(owned.filter((session) => session.state !== 'exited' || session.ptySessionId));

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
        frame = framesLeft > 0 ? requestAnimationFrame(step) : 0;
      };
      frame = requestAnimationFrame(step);
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
        if (frame !== 0) cancelAnimationFrame(frame);
        sizes.disconnect();
        shownOrHidden.disconnect();
        fonts?.removeEventListener('loadingdone', onFontsLoaded);
      }
    };
  }
</script>

<div class="surface">
  {#each hosted as session (session.ownedId)}
    <!--
      Hosts start HIDDEN (inline `display: none`) and only the manager turns one
      on, via `view.setVisible(true)` -> `host.style.display = 'block'`. Two bugs
      this closes: a view created while another one is already active is never
      told to hide (so its `inset: 0` host would cover the active terminal), and
      a host that never gets a view at all (an exited session whose PTY died
      while the app was closed) would sit on top as a transparent click-blocker.
      The attribute is static, so Svelte never re-applies it over the manager.
    -->
    <div
      class="term-host"
      style="display: none"
      data-owned-id={session.ownedId}
      use:host={session.ownedId}
    ></div>
  {/each}

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
  {:else if !hosted.some((session) => session.ownedId === activeOwnedId)}
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
    background: #282a36;
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
    background: #101014;
    color: #6d6d7d;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
  }

  .surface-empty p {
    margin: 0;
  }

  .hint {
    color: #4c4c5a;
    font-size: 12px;
  }
</style>
