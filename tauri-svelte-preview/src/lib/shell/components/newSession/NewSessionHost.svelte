<!--
  NewSessionHost.svelte — puts the new-session dialog on the /next shell.

  Exactly the shape of `SettingsHost.svelte`, and for the same reason: the
  dialog is a screen most launches never show, so it is loaded the first time
  somebody asks for it rather than with the rest of the shell.

  It owns three things and nothing else:
    1. the open/closed state of the dialog;
    2. loading the dialog itself, which only happens on the first open;
    3. handing the dialog the folders the running sessions are in, so the
       project picker knows about projects nobody added by hand.

  It starts nothing. `onStart` is passed straight through to whoever mounted
  this host — the page that owns the session list. See
  `_(new-session)-INTEGRATION.md` next to this file.

  Usage:
    <script>
      import NewSessionHost from '$lib/shell/components/newSession/NewSessionHost.svelte';
      let newSessionHost: { open: (input?: { sessionRoots?: string[] }) => void };
    </script>
    <NewSessionHost bind:this={newSessionHost} onStart={startNewSession} />
    <button onclick={() => newSessionHost.open({ sessionRoots: rail.owned.map((s) => s.cwd) })}>
      New session
    </button>
-->
<script lang="ts">
  import { tick } from 'svelte';

  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';

  interface Props {
    /** Start the session the user described. See `NewSessionDialog`'s own prop. */
    onStart: (request: NewSessionRequest) => void | Promise<void>;
  }

  let { onStart }: Props = $props();

  /** The real dialog, referred to by type only — no load yet. */
  type NewSessionDialogComponent =
    (typeof import('$lib/shell/components/newSession/NewSessionDialog.svelte'))['default'];

  let NewSessionDialog = $state<NewSessionDialogComponent | null>(null);
  let dialogOpen = $state(false);
  let loadFailure = $state<string | null>(null);
  /** Guards a second open() while the first load is still in flight. */
  let loading = false;
  let dialog = $state<{ reset: (input?: { sessionRoots?: string[] }) => void } | null>(null);

  /**
   * Show the new-session dialog, cleared and pointed at the current projects.
   *
   * Safe to call repeatedly; the first call loads the dialog, later calls
   * reopen it. `sessionRoots` are the folders the sessions on the rail are
   * running in — the picker offers those alongside the folders someone added,
   * so a shell with no saved projects still has somewhere to start.
   */
  export function open(input: { sessionRoots?: string[] } = {}): void {
    loadFailure = null;
    if (NewSessionDialog) {
      dialogOpen = true;
      void tick().then(() => dialog?.reset(input));
      return;
    }
    if (loading) return;
    loading = true;
    import('$lib/shell/components/newSession/NewSessionDialog.svelte')
      .then(async (module) => {
        NewSessionDialog = module.default;
        dialogOpen = true;
        // The dialog only exists after the next render, and `reset` is what
        // loads its checkouts — so wait for it rather than opening on nothing.
        await tick();
        dialog?.reset(input);
      })
      .catch((error: unknown) => {
        loadFailure = error instanceof Error ? error.message : String(error);
      })
      .finally(() => {
        loading = false;
      });
  }

  /** Hide the dialog. */
  export function close(): void {
    dialogOpen = false;
  }

  /** Whether the dialog is currently showing. */
  export function isOpen(): boolean {
    return dialogOpen;
  }
</script>

{#if NewSessionDialog}
  <NewSessionDialog bind:this={dialog} bind:open={dialogOpen} {onStart} />
{/if}

{#if loadFailure}
  <!-- Only ever seen if the dialog itself fails to load. A button that quietly
       does nothing would be worse than a one-line explanation. -->
  <div
    role="alert"
    class="fixed bottom-3 left-1/2 z-[200] max-w-[480px] -translate-x-1/2 rounded-md border
           border-destructive/40 bg-destructive/10 px-3 py-1.5 text-[13px] text-destructive"
  >
    The new-session dialog could not be opened: {loadFailure}
  </div>
{/if}
