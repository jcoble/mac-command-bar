<!--
  SettingsHost.svelte — puts the existing settings dialog on the /next shell.

  It owns three things and nothing else:
    1. the /next color file, so the dialog (and every other shared
       component used inside /next) paints in the /next palette;
    2. the open/closed state of the dialog;
    3. loading the dialog itself, which only happens the first time
       someone asks for it.

  The settings surface is `$lib/SettingsPanel.svelte`, reused unchanged.
  It is pulled in on demand rather than with the rest of the shell so
  that starting the app costs nothing for a screen most launches never
  show. Nothing here reads or writes the backend.

  Usage:
    <script>
      import SettingsHost from '$lib/shell/components/SettingsHost.svelte';
      let settingsHost: SettingsHost;
    </script>
    <SettingsHost bind:this={settingsHost} />
    <button onclick={() => settingsHost.open()}>Settings</button>
-->
<script lang="ts">
  /* The /next palette. Imported here so that anything mounting this host
     gets the right colors even before the page imports the file itself. */
  import '$lib/shell/styles/nextTokens.css';

  /** The real settings surface, referred to by type only — no load yet. */
  type SettingsPanelComponent = (typeof import('$lib/SettingsPanel.svelte'))['default'];

  let SettingsPanel = $state<SettingsPanelComponent | null>(null);
  let dialogOpen = $state(false);
  let loadFailure = $state<string | null>(null);
  /** Guards a second open() while the first load is still in flight. */
  let loading = false;

  /**
   * Show the settings dialog. Safe to call repeatedly; the first call
   * loads the panel, later calls just reopen it.
   */
  export function open(): void {
    loadFailure = null;
    if (SettingsPanel) {
      dialogOpen = true;
      return;
    }
    if (loading) return;
    loading = true;
    import('$lib/SettingsPanel.svelte')
      .then((module) => {
        SettingsPanel = module.default;
        dialogOpen = true;
      })
      .catch((error: unknown) => {
        loadFailure = error instanceof Error ? error.message : String(error);
      })
      .finally(() => {
        loading = false;
      });
  }

  /** Hide the settings dialog. */
  export function close(): void {
    dialogOpen = false;
  }

  /** Whether the settings dialog is currently showing. */
  export function isOpen(): boolean {
    return dialogOpen;
  }
</script>

{#if SettingsPanel}
  <SettingsPanel bind:open={dialogOpen} />
{/if}

{#if loadFailure}
  <div class="settings-host-error" role="alert">
    Settings could not be opened: {loadFailure}
  </div>
{/if}

<style>
  /* Only ever seen if the settings screen fails to load — a silent
     no-op button would be worse than a one-line explanation. */
  .settings-host-error {
    position: fixed;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 200;
    max-width: 480px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    background: var(--color-bad-bg);
    color: var(--color-bad);
    font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
    font-size: var(--text-sm);
  }
</style>
