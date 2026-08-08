<script lang="ts">
  /**
   * StacksPane.svelte — the Run section of the tool column.
   *
   * A RUN CONFIGURATION is one saved way to start something: `pnpm dev`,
   * `docker compose up`, `dotnet watch`. This pane keeps them, starts them,
   * stops them, and says where each one stands right now.
   *
   * The file is still called StacksPane because the shell mounts it by that
   * path and this lane may not edit the file that does the mounting. The word
   * "stack" appears nowhere a user can see it — see the note at the top of
   * `stackStore.svelte.ts` for why the code kept the old name.
   *
   * A running configuration IS a session in the rail — the same list your
   * agents are in — so clicking a row puts its terminal on screen, output and
   * all. There is no second, hidden place for a process to live.
   *
   * NO props, NO IO at mount, NO `$effect`. Everything it draws comes out of
   * `stackStore`; the only things that reach the outside world are the buttons,
   * and they all go through `stackService`. Starting and stopping are handed to
   * the page (which owns the session rail and the terminals) through the three
   * handlers the service registers — see `_(stacks)-INTEGRATION.md`.
   */
  import { Pencil, Play, Plus, RefreshCw, Square, Trash2 } from '@lucide/svelte';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import RunConfigurationDialog from '$lib/shell/components/run/RunConfigurationDialog.svelte';
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
    type StackDefinition
  } from '$lib/shell/stacks/stackStore.svelte';

  /** The editor dialog, referred to by the one method this component calls. */
  let editor = $state<{
    openFor: (configuration: StackDefinition | null, defaultFolder?: string) => void;
  } | null>(null);
  let editorOpen = $state(false);

  /** The configuration whose removal is being confirmed, or `null`. */
  let removing = $state<StackDefinition | null>(null);
  const removeOpen = $derived(removing !== null);

  const rows = $derived(visibleStackRows());

  /** The folder a new configuration starts in: the project the shell is on. */
  const folder = $derived((stacks.activeRoot ?? '').trim());

  /** Which colour a state gets. The four states, and nothing else. */
  function tone(state: string): string {
    if (state === 'running') return 'running';
    if (state === 'starting') return 'attention';
    if (state === 'failed') return 'failed';
    return 'neutral';
  }

  function confirmRemoval(): void {
    if (!removing) return;
    removeStack(removing.id);
    removing = null;
  }
</script>

<div class="run-pane" aria-label="Run configurations">
  <header class="toolbar">
    <span class="toolbar-title">
      {stacks.projectName ? stacks.projectName : 'Run'}
    </span>
    <button
      type="button"
      class="tool"
      onclick={() => editor?.openFor(null, folder)}
      title="Save a command you want to be able to start from here"
    >
      <Plus size={13} />
      <span>new</span>
    </button>
    <button
      type="button"
      class="tool"
      disabled={stacks.loading}
      onclick={() => void refreshStacks()}
      title="Look again at which ports are open"
    >
      <RefreshCw size={13} />
      <span>{stacks.loading ? 'reading…' : 'refresh'}</span>
    </button>
  </header>

  {#if stacks.notice}
    <p class="state notice">
      {stacks.notice}
      <button type="button" class="retry" onclick={clearStackNotice}>dismiss</button>
    </p>
  {/if}

  {#if stacks.error}
    <p class="state error">
      {stacks.error}
      <button type="button" class="retry" onclick={() => void refreshStacks()}>try again</button>
    </p>
  {/if}

  {#if stacks.unavailableReason}
    <p class="state">{stacks.unavailableReason}</p>
  {/if}

  {#if rows.length === 0}
    <p class="state">
      {#if !stacks.activated}
        Nothing read yet — press refresh to see what is running.
      {:else if stacks.projectName}
        Nothing saved for {stacks.projectName} yet. A run configuration is one command you keep
        starting — the dev server, the database, the watcher.
      {:else}
        Nothing saved yet. A run configuration is one command you keep starting — the dev server,
        the database, the watcher.
      {/if}
    </p>
  {:else}
    <ul class="rows">
      {#each rows as row (row.definition.id)}
        <li class="row">
          <div class="row-main">
            <span class="dot" data-tone={tone(row.state)} aria-hidden="true"></span>
            {#if row.ownedId}
              <button
                type="button"
                class="row-title link"
                title="Show this configuration's terminal"
                onclick={() => void selectStackSession(row.ownedId as string)}
              >
                {row.definition.name}
              </button>
            {:else}
              <span class="row-title">{row.definition.name}</span>
            {/if}
            <span class="chip" data-tone={tone(row.state)}>{row.statusLabel}</span>
          </div>

          <div class="row-meta">
            <code class="mono script" title={row.definition.script}>{row.definition.script}</code>
          </div>

          {#if row.definition.env && row.definition.env.length > 0}
            <p class="env" title={row.definition.env.map((entry) => entry.key).join(', ')}>
              Sets {row.definition.env.map((entry) => entry.key).join(', ')}
            </p>
          {/if}

          {#if row.processes.length > 0}
            <div class="ports">
              {#each row.processes as process (process.pid)}
                <span class="badge" title={`${process.command} in ${process.cwd}`}>
                  :{process.port} · pid {process.pid}
                </span>
              {/each}
            </div>
          {/if}

          <div class="actions">
            {#if row.state === 'running' || row.state === 'starting'}
              <button
                type="button"
                class="action"
                disabled={isStackBusy(row.definition.id)}
                onclick={() => void stopStack(row.definition.id)}
              >
                <Square size={11} />
                <span>Stop</span>
              </button>
            {:else}
              <button
                type="button"
                class="action"
                disabled={isStackBusy(row.definition.id)}
                onclick={() => void startStack(row.definition.id)}
              >
                <Play size={11} />
                <span>Run</span>
              </button>
            {/if}

            <button
              type="button"
              class="action quiet"
              title="Change the name, command, folder or environment variables"
              aria-label={`Change ${row.definition.name}`}
              onclick={() => editor?.openFor(row.definition)}
            >
              <Pencil size={11} />
            </button>

            <button
              type="button"
              class="action quiet"
              title="Forget this saved command. The terminal it ran in is left alone."
              aria-label={`Remove ${row.definition.name}`}
              onclick={() => (removing = row.definition)}
            >
              <Trash2 size={11} />
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<RunConfigurationDialog bind:this={editor} bind:open={editorOpen} />

<AlertDialog.Root open={removeOpen} onOpenChange={(next) => { if (!next) removing = null; }}>
  <AlertDialog.Content
    class="rounded-lg bg-background text-foreground ring-[var(--color-border)]
           shadow-[var(--shadow-lg)]"
  >
    <AlertDialog.Header>
      <AlertDialog.Title class="text-[14px] leading-[1.4] font-semibold">
        Remove “{removing?.name ?? ''}” from the run list?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-[1.5] text-[var(--color-text-2)]">
        This forgets the saved command and nothing else. If it is running right now, the terminal
        keeps running and stays in your session list — you would stop it there.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Keep it</AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant="destructive"
        class="text-[13px]"
        onclick={confirmRemoval}
      >
        Remove it
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<style>
  .run-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  /* ── Toolbar ───────────────────────────────────────────────────────── */
  .toolbar {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    background: var(--color-bg);
    border-bottom: 1px solid var(--color-border);
  }

  .toolbar-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-2);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .tool {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: 0 0 auto;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    padding: 2px 5px;
    cursor: pointer;
  }

  .tool:hover:not(:disabled) {
    background: var(--color-elevated);
    color: var(--color-text);
  }

  .tool:disabled {
    cursor: default;
    opacity: 0.45;
  }

  /* ── Rows ──────────────────────────────────────────────────────────── */
  .rows {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 4px;
    list-style: none;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    border-radius: 6px;
    transition: background-color 120ms ease;
  }

  .row:hover {
    background: var(--color-surface);
  }

  .row + .row {
    position: relative;
  }

  .row + .row::before {
    content: '';
    position: absolute;
    top: 0;
    left: 6px;
    right: 6px;
    height: 1px;
    background: var(--color-border);
  }

  .row:hover::before,
  .row:hover + .row::before {
    opacity: 0;
  }

  .row-main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
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

  .row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.35;
  }

  .row-title.link {
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 13px;
    padding: 0;
    cursor: pointer;
  }

  .row-title.link:hover {
    color: var(--color-accent);
    text-decoration: underline;
  }

  .row-meta {
    display: flex;
    min-width: 0;
  }

  .script {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-2);
    font-size: 12px;
  }

  .env {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-2);
    font-size: 12px;
  }

  .mono {
    font-family: ui-monospace, Menlo, monospace;
  }

  .ports {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .badge {
    border-radius: 4px;
    background: var(--color-live-bg);
    color: var(--color-live);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 1px 5px;
    white-space: nowrap;
  }

  .chip {
    flex: 0 0 auto;
    max-width: 55%;
    overflow: hidden;
    border-radius: 4px;
    background: var(--color-elevated);
    color: var(--color-text-2);
    font-size: 12px;
    padding: 1px 5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip[data-tone='running'] {
    background: var(--color-live-bg);
    color: var(--color-live);
  }

  .chip[data-tone='attention'] {
    background: var(--color-attention-bg);
    color: var(--color-attention);
  }

  .chip[data-tone='failed'] {
    background: var(--color-bad-bg);
    color: var(--color-bad);
  }

  /* ── Actions ───────────────────────────────────────────────────────── */
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .action {
    display: flex;
    align-items: center;
    gap: 3px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    padding: 2px 6px;
    cursor: pointer;
  }

  .action:hover:not(:disabled) {
    color: var(--color-text);
    background: var(--color-elevated);
  }

  .action:disabled {
    cursor: default;
    opacity: 0.45;
  }

  .action.quiet { background: transparent; }

  /* ── States ────────────────────────────────────────────────────────── */
  .state {
    margin: 0;
    padding: 8px;
    color: var(--color-text-2);
    font-size: 13px;
    line-height: 1.45;
  }

  .state.error {
    color: var(--color-bad);
  }

  .state.notice {
    color: var(--color-attention);
  }

  .retry {
    margin-left: 6px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-accent);
    font: inherit;
    font-size: 12px;
    padding: 0 2px;
    cursor: pointer;
    text-decoration: underline;
  }

  button:focus-visible {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }
</style>
