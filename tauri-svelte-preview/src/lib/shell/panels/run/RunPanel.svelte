<script lang="ts">
  /**
   * RunPanel.svelte — the Run tab of the right column.
   *
   * The project's saved actions: the commands you keep starting, each with the
   * shortcut that starts it, the page it serves, and what it is doing right
   * now. It replaced the play button that used to sit in the top bar, so this
   * panel and the shortcuts are the only two ways an action runs.
   *
   * WHAT IT OWNS AND WHAT IT ASKS FOR. Nothing here spawns or kills a process.
   * The page owns the session rail and the terminals, so starting and stopping
   * go through `stackService`, which calls the handlers the page registered.
   * Reading which ports are open is the service's job too. The one thing this
   * panel does reach out for itself is the output tail under a running action,
   * and that is a service as well (`runOutputService`).
   *
   * NOTHING POLLS, and nothing runs at mount. The panel is mounted for the whole
   * session and parked off-screen; the page tells the service when the tab is
   * shown. The output watch is opened when the tab is on screen and closed the
   * moment it is not, so a panel nobody is looking at costs nothing.
   *
   * The word "stack" in the imports is the old internal name for a run action;
   * renaming it would rename the keys people's saved actions are filed under.
   * See the note at the top of `stackStore.svelte.ts`.
   */
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Play from '@lucide/svelte/icons/play';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { watchRunOutput, type RunOutputWatch } from '$lib/shell/stacks/runOutputService';
  import {
    isStackBusy,
    refreshStacks,
    selectStackSession,
    startStack,
    stopStack
  } from '$lib/shell/stacks/stackService';
  import {
    clearStackNotice,
    removeStack,
    stacks,
    visibleStackRows,
    type StackDefinition,
    type StackRow
  } from '$lib/shell/stacks/stackStore.svelte';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation';

  import AddActionDialog from './AddActionDialog.svelte';
  import RunActionRow from './RunActionRow.svelte';
  import RunningProcessRow from './RunningProcessRow.svelte';
  import { matchKeybinding } from './keybindingCapture.ts';

  interface Props {
    /** True while this is the tab on screen. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's id, or null. Unused: actions belong to the project. */
    ownedId: string | null;
  }
  let { visible }: Props = $props();

  /** The dialog, referred to by the one method this panel calls on it. */
  let dialog = $state<{
    openFor: (action: StackDefinition | null, defaultFolder?: string) => void;
  } | null>(null);
  let dialogOpen = $state(false);

  /** The action whose removal is being confirmed, or `null`. */
  let removing = $state<StackDefinition | null>(null);
  const removeOpen = $derived(removing !== null);

  /** The last lines each running action printed, by session id. */
  let tails = $state<Record<string, string[]>>({});

  const rows = $derived(visibleStackRows());
  const running = $derived(rows.filter((row) => row.state === 'running' || row.state === 'starting'));

  /** The folder a new action starts in: the project the shell is on. */
  const folder = $derived((stacks.activeRoot ?? '').trim());

  /**
   * The sessions to follow, as one string, so the watch below is rebuilt when
   * the set of running actions changes and NOT every time a port is re-read.
   */
  const watchedSessions = $derived(
    running
      .map((row) => row.ownedId)
      .filter((id): id is string => id !== null)
      .join(' ')
  );

  /**
   * Which actions a shortcut may start: this project's first, then the rest.
   * A shortcut is a property of the action, not of the panel, so it keeps
   * working while another tab is in front — but when two projects have given
   * the same combination away, the project on screen is the one meant.
   */
  const shortcutOrder = $derived([
    ...rows.map((row) => row.definition),
    ...stacks.definitions.filter(
      (definition) => !rows.some((row) => row.definition.id === definition.id)
    )
  ]);

  /**
   * Follow the output of everything that is running, while the tab is on
   * screen. Closing the tab stops the subscription — that is the whole of the
   * lifecycle, and there is no timer in it.
   */
  $effect(() => {
    const sessions = watchedSessions.split(' ').filter((id) => id !== '');
    if (!visible || sessions.length === 0) return;
    let watch: RunOutputWatch | null = null;
    let cancelled = false;
    void watchRunOutput(sessions, (sessionOwnedId, tail) => {
      if (!cancelled) tails[sessionOwnedId] = tail;
    }).then((handle) => {
      watch = handle;
      if (cancelled) handle.stop();
    });
    return () => {
      cancelled = true;
      watch?.stop();
    };
  });

  /** Start an action, and show its page when it was set up to do that. */
  async function run(definition: StackDefinition): Promise<void> {
    await startStack(definition.id);
    const url = (definition.previewUrl ?? '').trim();
    if (definition.openPreviewOnRun === true && url) await openUrlInBrowser({ url });
  }

  /** Stop an action, then start it again with whatever it is saved as now. */
  async function restart(row: StackRow): Promise<void> {
    await stopStack(row.definition.id);
    await run(row.definition);
  }

  function confirmRemoval(): void {
    if (!removing) return;
    removeStack(removing.id);
    removing = null;
  }

  /** True when the keypress belongs to something the reader is typing into. */
  function isTyping(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    if (target.isContentEditable) return true;
    const tag = target.tagName;
    return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || tag === 'BUTTON';
  }

  /**
   * A saved shortcut starts its action from anywhere in the app — except while
   * the reader is typing, where the combination belongs to what they are typing
   * in. The capture field in the dialog is a button and swallows its own keys,
   * so it can never reach this.
   */
  function onKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || isTyping(event.target)) return;
    const matched = matchKeybinding(event, shortcutOrder);
    if (!matched) return;
    event.preventDefault();
    void run(matched);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="flex h-full min-h-0 flex-col">
  <PanelHeader title="Run" count={rows.length}>
    {#snippet actions()}
      <IconButton
        label={stacks.loading ? 'Reading which ports are open…' : 'Look again at which ports are open'}
        disabled={stacks.loading}
        onclick={() => void refreshStacks()}
      >
        <RefreshCw />
      </IconButton>
      <Button size="xs" class="text-[13px]" onclick={() => dialog?.openFor(null, folder)}>
        <Plus class="size-3.5" aria-hidden="true" />
        Add action
      </Button>
    {/snippet}
  </PanelHeader>

  {#if stacks.notice}
    <p class="flex items-start gap-2 border-b px-3 py-2 text-sm leading-snug text-[var(--color-attention)]">
      <span class="min-w-0 flex-1">{stacks.notice}</span>
      <Button variant="ghost" size="xs" class="shrink-0 text-sm" onclick={clearStackNotice}>
        Dismiss
      </Button>
    </p>
  {/if}

  {#if stacks.error}
    <p class="flex items-start gap-2 border-b px-3 py-2 text-sm leading-snug text-destructive">
      <span class="min-w-0 flex-1">{stacks.error}</span>
      <Button variant="ghost" size="xs" class="shrink-0 text-sm" onclick={() => void refreshStacks()}>
        Try again
      </Button>
    </p>
  {/if}

  {#if stacks.unavailableReason}
    <p class="border-b px-3 py-2 text-sm leading-snug text-muted-foreground">
      {stacks.unavailableReason}
    </p>
  {/if}

  <ScrollArea class="min-h-0 flex-1">
    {#if rows.length === 0}
      <EmptyState
        title="No actions saved yet"
        body={stacks.projectName
          ? `An action is one command you keep starting in ${stacks.projectName} — the dev server, the database, the watcher. Save it once and it is a click or a shortcut away.`
          : 'An action is one command you keep starting — the dev server, the database, the watcher. Save it once and it is a click or a shortcut away.'}
      >
        {#snippet icon()}<Play />{/snippet}
        {#snippet actions()}
          <Button size="sm" class="text-[13px]" onclick={() => dialog?.openFor(null, folder)}>
            <Plus class="size-3.5" aria-hidden="true" />
            Add action
          </Button>
        {/snippet}
      </EmptyState>
    {:else}
      <ul class="flex flex-col gap-0.5 p-2">
        {#each rows as row (row.definition.id)}
          <li>
            <RunActionRow
              {row}
              busy={isStackBusy(row.definition.id)}
              onRun={() => void run(row.definition)}
              onStop={() => void stopStack(row.definition.id)}
              onEdit={() => dialog?.openFor(row.definition)}
              onRemove={() => (removing = row.definition)}
              onOpenPreview={() =>
                void openUrlInBrowser({ url: (row.definition.previewUrl ?? '').trim() })}
            />
          </li>
        {/each}
      </ul>
    {/if}

    {#if running.length > 0}
      <section class="flex flex-col gap-2 border-t p-2">
        <h3 class="px-1 text-xs leading-tight font-medium tracking-wide text-muted-foreground uppercase">
          Running
        </h3>
        {#each running as row (row.definition.id)}
          <RunningProcessRow
            {row}
            tail={row.ownedId ? (tails[row.ownedId] ?? []) : []}
            busy={isStackBusy(row.definition.id)}
            onStop={() => void stopStack(row.definition.id)}
            onRestart={() => void restart(row)}
            onOpenSession={() => {
              if (row.ownedId) void selectStackSession(row.ownedId);
            }}
          />
        {/each}
      </section>
    {/if}
  </ScrollArea>
</div>

<AddActionDialog bind:this={dialog} bind:open={dialogOpen} />

<AlertDialog.Root
  open={removeOpen}
  onOpenChange={(next) => {
    if (!next) removing = null;
  }}
>
  <AlertDialog.Content
    class="rounded-lg bg-background text-foreground ring-border shadow-[var(--shadow-lg)]"
  >
    <AlertDialog.Header>
      <AlertDialog.Title class="text-[14px] leading-[1.4] font-semibold">
        Forget “{removing?.name ?? ''}”?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-[1.5] text-muted-foreground">
        This forgets the saved command and nothing else. If it is running right now, the terminal
        keeps running and stays in your session list — you would stop it there.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Keep it</AlertDialog.Cancel>
      <AlertDialog.Action size="sm" variant="destructive" class="text-[13px]" onclick={confirmRemoval}>
        Forget it
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
