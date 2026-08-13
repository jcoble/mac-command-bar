<script lang="ts">
  /**
   * RunButton.svelte — the play button and the list behind it.
   *
   * One saved way to start something is a RUN CONFIGURATION. This control is
   * the whole feature in one small widget: press the play button to run the
   * configuration named on it, or open the list to run any of the others.
   *
   * SELF-CONTAINED BY DESIGN. It takes no props and needs no host state, so it
   * can be mounted anywhere — the shell's top bar is where it goes, but nothing
   * here knows that. Everything it draws comes from `stackStore`; everything it
   * does goes through `stackService`, which hands the actual spawning to the
   * page that owns the session rail.
   *
   * NO `$effect` and NO backend call at mount. The one thing it does on mount is
   * `hydrateStacks()`, which reads the saved configurations out of localStorage
   * — storage, not the backend — because otherwise the button would sit empty
   * until somebody opened the tool column. Reading which ports are open IS a
   * backend call, so it only happens when the user asks for it.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Play from '@lucide/svelte/icons/play';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Square from '@lucide/svelte/icons/square';
  import { onMount } from 'svelte';

  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import {
    isStackBusy,
    refreshStacks,
    startStack,
    stopStack
  } from '$lib/shell/stacks/stackService';
  import {
    allStackRows,
    hydrateStacks,
    isInsideRoot,
    runRecordForStack,
    stacks,
    type StackDefinition,
    type StackRow
  } from '$lib/shell/stacks/stackStore.svelte';

  import RunConfigurationDialog from './RunConfigurationDialog.svelte';

  /** The configuration the user last picked here, or `null` to let it choose. */
  let picked = $state<string | null>(null);
  let menuOpen = $state(false);
  let editorOpen = $state(false);
  let menuRows = $state<StackRow[]>([]);
  let menuInProject = $state<StackRow[]>([]);
  let menuElsewhere = $state<StackRow[]>([]);
  let menuCurrent = $state<StackRow | null>(null);
  let menuRoot = $state('');
  let menuLoading = $state(false);
  /** The editor dialog, referred to by the one method this component calls. */
  let editor = $state<{
    openFor: (configuration: StackDefinition | null, defaultFolder?: string) => void;
  } | null>(null);

  onMount(() => {
    hydrateStacks();
  });

  const rows = $derived(allStackRows());
  const root = $derived((stacks.activeRoot ?? '').trim());
  const inProject = $derived(rows.filter((row) => isInsideRoot(row.definition.cwd, root)));
  const elsewhere = $derived(rows.filter((row) => !isInsideRoot(row.definition.cwd, root)));

  /**
   * Which configuration the play button is pointed at.
   *
   * In order: whatever the user last picked here, then anything actually
   * running (that is the one they are most likely to want to stop), then the
   * one started most recently, then the first one belonging to this project.
   * Never a guess that jumps around: each step only applies when the one before
   * it found nothing.
   */
  const current = $derived.by((): StackRow | null => {
    if (rows.length === 0) return null;
    const chosen = rows.find((row) => row.definition.id === picked);
    if (chosen) return chosen;
    const live = rows.find((row) => row.state === 'running' || row.state === 'starting');
    if (live) return live;
    let latest: StackRow | null = null;
    let latestAt = 0;
    for (const row of rows) {
      const startedAt = runRecordForStack(row.definition.id)?.startedAt ?? 0;
      if (startedAt > latestAt) {
        latestAt = startedAt;
        latest = row;
      }
    }
    if (latest) return latest;
    return inProject[0] ?? rows[0];
  });

  const running = $derived(current?.state === 'running' || current?.state === 'starting');
  const busy = $derived(current ? isStackBusy(current.definition.id) : false);

  function changeMenu(next: boolean): void {
    menuOpen = next;
    if (!next) return;
    menuRows = rows.map((row) => ({ ...row, definition: { ...row.definition } }));
    menuRoot = root;
    menuInProject = menuRows.filter((row) => isInsideRoot(row.definition.cwd, menuRoot));
    menuElsewhere = menuRows.filter((row) => !isInsideRoot(row.definition.cwd, menuRoot));
    menuCurrent = current
      ? menuRows.find((row) => row.definition.id === current?.definition.id) ?? null
      : null;
    menuLoading = stacks.loading;
  }

  /** The state as a colour name. The four states, and nothing else. */
  function tone(state: string): string {
    if (state === 'running') return 'running';
    if (state === 'starting') return 'attention';
    if (state === 'failed') return 'failed';
    return 'neutral';
  }

  /** Run one configuration and remember it as the button's pick. */
  function run(row: StackRow): void {
    picked = row.definition.id;
    menuOpen = false;
    void startStack(row.definition.id);
  }

  /** The play button: run what it is pointed at, or stop it if it is going. */
  function pressPlay(): void {
    if (!current) {
      editor?.openFor(null, root);
      return;
    }
    if (running) {
      void stopStack(current.definition.id);
      return;
    }
    void startStack(current.definition.id);
  }

  function addNew(): void {
    menuOpen = false;
    editor?.openFor(null, menuRoot);
  }

  function changeCurrent(): void {
    if (!menuCurrent) return;
    menuOpen = false;
    editor?.openFor(menuCurrent.definition);
  }

  function readPortsAgain(): void {
    menuOpen = false;
    void refreshStacks();
  }
</script>

<div class="run-button">
  <button
    type="button"
    class="play"
    class:stop={running}
    disabled={busy}
    title={current
      ? running
        ? `Stop “${current.definition.name}” — ${current.statusLabel}`
        : `Run “${current.definition.name}”: ${current.definition.script}`
      : 'Save a command you want to be able to start from here'}
    onclick={pressPlay}
  >
    {#if !current}
      <Plus size={13} aria-hidden="true" />
      <span class="label">Run</span>
    {:else if running}
      <Square size={12} aria-hidden="true" />
      <span class="label">{current.definition.name}</span>
    {:else}
      <Play size={13} aria-hidden="true" />
      <span class="label">{current.definition.name}</span>
    {/if}
  </button>

  <DropdownMenu.Root open={menuOpen} onOpenChange={changeMenu}>
    <DropdownMenu.Trigger
      class="chevron"
      aria-label="Choose what to run"
      title="Choose what to run"
    >
      <ChevronDown size={13} aria-hidden="true" />
    </DropdownMenu.Trigger>
    <DropdownMenu.Content
      align="end"
      class="w-[min(360px,calc(100vw-2rem))] bg-[var(--color-surface)] text-[var(--color-text)]
             ring-[var(--color-border)]"
    >
      {#if menuRows.length === 0}
        <p class="px-2 py-2 text-[12px] leading-[1.5] text-[var(--color-text-2)]">
          Nothing saved yet. A run configuration is one command you keep starting — the dev server,
          the database, the watcher.
        </p>
      {:else}
        {#each [{ heading: 'This project', list: menuInProject }, { heading: 'Other folders', list: menuElsewhere }] as group (group.heading)}
          {#if group.list.length > 0}
            {#if menuInProject.length > 0 && menuElsewhere.length > 0}
              <DropdownMenu.Label class="text-[12px] text-[var(--color-text-3)]">
                {group.heading}
              </DropdownMenu.Label>
            {/if}
            {#each group.list as row (row.definition.id)}
              <DropdownMenu.Item
                class="items-start gap-2 px-2 py-1.5 text-[13px]"
                onSelect={() => run(row)}
              >
                <span class="dot" data-tone={tone(row.state)} aria-hidden="true"></span>
                <span class="entry">
                  <span class="entry-top">
                    <span class="entry-name">{row.definition.name}</span>
                    <span class="entry-state" data-tone={tone(row.state)}>{row.statusLabel}</span>
                  </span>
                  <span class="entry-command">{row.definition.script}</span>
                </span>
              </DropdownMenu.Item>
            {/each}
          {/if}
        {/each}
        <DropdownMenu.Separator class="bg-[var(--color-border)]" />
      {/if}

      {#if menuCurrent}
        <DropdownMenu.Item class="gap-2 px-2 py-1.5 text-[13px]" onSelect={changeCurrent}>
          <Pencil size={13} aria-hidden="true" />
          Change “{menuCurrent.definition.name}”…
        </DropdownMenu.Item>
      {/if}
      <DropdownMenu.Item class="gap-2 px-2 py-1.5 text-[13px]" onSelect={addNew}>
        <Plus size={13} aria-hidden="true" />
        New run configuration…
      </DropdownMenu.Item>
      {#if menuRows.length > 0}
        <DropdownMenu.Item
          class="gap-2 px-2 py-1.5 text-[13px]"
          disabled={menuLoading}
          onSelect={readPortsAgain}
        >
          <RefreshCw size={13} aria-hidden="true" />
          {menuLoading ? 'Reading…' : 'Look again at which ports are open'}
        </DropdownMenu.Item>
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>

{#if stacks.error}
  <span class="run-error" title={stacks.error}>{stacks.error}</span>
{/if}

<RunConfigurationDialog
  bind:this={editor}
  bind:open={editorOpen}
  onSaved={(stackId) => (picked = stackId)}
/>

<style>
  .run-button {
    display: inline-flex;
    align-items: stretch;
    flex: 0 0 auto;
    max-width: 260px;
    border: 0;
    border-radius: 5px;
    background: var(--color-surface);
    overflow: hidden;
  }

  .play {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
    line-height: 1.3;
    padding: 3px 8px;
    cursor: pointer;
  }

  .play :global(svg) {
    flex: 0 0 auto;
    color: var(--color-good);
  }

  .play.stop :global(svg) {
    color: var(--color-bad);
  }

  .play:hover:not(:disabled) {
    background: var(--color-elevated);
  }

  .play:disabled {
    cursor: default;
    opacity: 0.5;
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The chevron is a bits-ui trigger, so its class lands on the element it
     renders and has to be reached from here with :global. */
  .run-button :global(.chevron) {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    border: 0;
    border-left: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-text-2);
    padding: 0 4px;
    cursor: pointer;
  }

  .run-button :global(.chevron:hover) {
    background: var(--color-elevated);
    color: var(--color-text);
  }

  .run-button :global(.chevron:focus-visible),
  .play:focus-visible {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }

  /* ── Rows in the list ──────────────────────────────────────────────── */
  .dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    margin-top: 6px;
    border-radius: 50%;
    background: var(--color-idle);
  }

  .dot[data-tone='running'] {
    background: var(--color-live);
  }

  .dot[data-tone='attention'] {
    background: var(--color-attention);
  }

  .dot[data-tone='failed'] {
    background: var(--color-bad);
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .entry-top {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }

  .entry-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text);
    font-size: 13px;
  }

  .entry-state {
    flex: 0 0 auto;
    color: var(--color-text-2);
    font-size: 12px;
  }

  .entry-state[data-tone='running'] {
    color: var(--color-live);
  }

  .entry-state[data-tone='attention'] {
    color: var(--color-attention);
  }

  .entry-state[data-tone='failed'] {
    color: var(--color-bad);
  }

  .entry-command {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-2);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .run-error {
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-bad);
    font-size: 12px;
  }
</style>
