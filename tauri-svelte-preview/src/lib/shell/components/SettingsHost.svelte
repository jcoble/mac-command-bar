<!--
  SettingsHost.svelte — puts the settings dialog on the /next shell.

  It owns three things and nothing else:
    1. the /next stylesheets, so the dialog (and everything else rendered
       inside /next) paints in the /next palette;
    2. the open/closed state of the dialog;
    3. loading the dialog itself, which only happens the first time
       someone asks for it.

  The settings surface is `$lib/shell/components/SettingsDialog.svelte` — the
  /next shell's own, built from the shadcn components. It is pulled in on
  demand rather than with the rest of the shell so that starting the app costs
  nothing for a screen most launches never show. Nothing here reads or writes
  the backend.

  Usage:
    <script>
      import SettingsHost from '$lib/shell/components/SettingsHost.svelte';
      let settingsHost: SettingsHost;
    </script>
    <SettingsHost bind:this={settingsHost} />
    <button onclick={() => settingsHost.open()}>Settings</button>
-->
<script lang="ts">
  /* The /next palette, and the Tailwind + shadcn variables built from it.
     Imported here so that anything mounting this host gets the right colors
     even before the page imports the files itself. */
  import '$lib/shell/styles/nextTokens.css';
  import '$lib/shell/styles/next.css';

  /** The real settings surface, referred to by type only — no load yet. */
  type SettingsDialogComponent =
    (typeof import('$lib/shell/components/SettingsDialog.svelte'))['default'];

  let SettingsDialog = $state<SettingsDialogComponent | null>(null);
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
    if (SettingsDialog) {
      dialogOpen = true;
      return;
    }
    if (loading) return;
    loading = true;
    import('$lib/shell/components/SettingsDialog.svelte')
      .then((module) => {
        SettingsDialog = module.default;
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

{#if SettingsDialog}
  <SettingsDialog bind:open={dialogOpen} />
{/if}

{#if loadFailure}
  <!-- Only ever seen if the settings screen fails to load — a silent no-op
       button would be worse than a one-line explanation. -->
  <div
    role="alert"
    class="fixed bottom-3 left-1/2 z-[200] max-w-[480px] -translate-x-1/2 rounded-md border
           border-destructive/40 bg-destructive/10 px-3 py-1.5 text-[12px] text-destructive"
  >
    Settings could not be opened: {loadFailure}
  </div>
{/if}
