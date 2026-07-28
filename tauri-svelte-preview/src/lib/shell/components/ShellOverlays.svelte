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
  <footer class="next-error">{message}</footer>
{/if}

{#if import.meta.env.DEV}
  <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
{/if}

<style>
  .next-error,
  .invoke-counter {
    position: absolute;
    bottom: 8px;
    border-radius: 5px;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 10px;
    padding: 3px 8px;
    pointer-events: none;
  }

  .next-error {
    left: 50%;
    transform: translateX(-50%);
    background: rgba(255, 85, 85, 0.16);
    color: #ff9d9d;
  }

  .invoke-counter {
    right: 10px;
    background: rgba(16, 16, 20, 0.82);
    color: #6d6d7d;
  }
</style>
