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
  }

  let { owned, activeOwnedId, registerHost }: Props = $props();

  /**
   * An exited session keeps its host only while its PTY id is still around
   * (so its final scrollback stays readable); once that is gone there is
   * nothing to render.
   */
  const hosted = $derived(owned.filter((session) => session.state !== 'exited' || session.ptySessionId));

  /** Svelte action: publish the host element to the page, once, on mount. */
  function host(node: HTMLElement, ownedId: string) {
    registerHost(ownedId, node);
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
