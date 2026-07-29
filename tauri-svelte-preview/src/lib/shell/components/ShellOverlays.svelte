<script lang="ts">
  /**
   * ShellOverlays.svelte — everything that floats above the /next shell rather
   * than living in a region: the command palette, the settings dialog, the
   * message strip, and the development-only backend call counter.
   *
   * No backend call and no state of its own beyond the settings handle. The
   * palette is mounted exactly once (two of them would answer the same keyboard
   * shortcut and would fight over the seeded actions), which is why it lives
   * here and not inside a panel.
   */
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import PalettePanel from './PalettePanel.svelte';
  import SettingsHost from './SettingsHost.svelte';
  import { invokeCounts } from '$lib/shell/devInvokeCounter.svelte';

  interface Props {
    /** Put every panel back where it started. */
    onResetLayout: () => void;
    /** Look for agent sessions again. */
    onRescanSessions: () => void | Promise<void>;
    /** One line describing whatever has gone wrong, or null when all is well. */
    message: string | null;
  }
  let { onResetLayout, onRescanSessions, message }: Props = $props();

  let settingsHost: { open: () => void; close: () => void } | null = null;

  /** Open the settings dialog from outside — the gear on the activity bar is
   * over in the left column, and the dialog lives here. Same shape as
   * `SettingsHost`'s own `open()`, one layer out. */
  export function openSettings(): void {
    settingsHost?.open();
  }
</script>

<PalettePanel
  {onResetLayout}
  {onRescanSessions}
  onOpenSettings={() => settingsHost?.open()}
/>
<SettingsHost bind:this={settingsHost} />

{#if message}
  <!-- Something went wrong, said once, along the bottom edge. Announced to
       screen readers, and see-through to the mouse so it can never swallow a
       click meant for the shell underneath. -->
  <footer
    role="alert"
    class="pointer-events-none absolute bottom-2 left-1/2 flex -translate-x-1/2 items-center
           gap-1.5 rounded-md border border-destructive/40 bg-destructive/10 px-2.5 py-1
           text-[12px] leading-[1.4] text-destructive"
  >
    <TriangleAlert class="size-3.5 shrink-0" aria-hidden="true" />
    <span>{message}</span>
  </footer>
{/if}

{#if import.meta.env.DEV}
  <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
{/if}

<style>
  /* Development-only readout of how many backend calls the shell has made.
     Deliberately not part of the shared component set: it is a debugging
     instrument, not chrome, and it never ships to a user. */
  .invoke-counter {
    position: absolute;
    bottom: 8px;
    right: 10px;
    border-radius: 5px;
    background: rgba(16, 16, 20, 0.82);
    color: #6d6d7d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 10px;
    padding: 3px 8px;
    pointer-events: none;
  }
</style>
