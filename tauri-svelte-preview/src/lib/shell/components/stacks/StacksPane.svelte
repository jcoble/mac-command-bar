<script lang="ts">
  /**
   * StacksPane.svelte — the /next stack runner.
   *
   * A stack is one command you keep re-running in a project: `pnpm dev`,
   * `docker compose up`, `dotnet watch`. This pane keeps them, starts them,
   * stops them, and says where each one stands right now.
   *
   * A running stack IS a session in the rail — the same list your agents are in
   * — so clicking a row puts its terminal on screen, output and all. There is no
   * second, hidden place for a process to live.
   *
   * NO props, NO IO at mount, NO `$effect`. Everything it draws comes out of
   * `stackStore`; the only things that reach the outside world are the buttons,
   * and they all go through `stackService`. Starting and stopping are handed to
   * the page (which owns the session rail and the terminals) through the three
   * handlers the service registers — see `_(stacks)-INTEGRATION.md`.
   */
  import { Play, Plus, RefreshCw, Square, Trash2, X } from '@lucide/svelte';

  import {
    isStackBusy,
    refreshStacks,
    selectStackSession,
    startStack,
    stopStack
  } from '$lib/shell/stacks/stackService';
  import {
    addStack,
    clearStackNotice,
    removeStack,
    stacks,
    visibleStackRows
  } from '$lib/shell/stacks/stackStore.svelte';

  /** Is the "add a stack" form open? */
  let adding = $state(false);
  /** The name being typed. */
  let draftName = $state('');
  /** The command being typed. */
  let draftScript = $state('');
  /** The stack whose Remove button has been pressed once, awaiting a second press. */
  let confirmingRemoval = $state<string | null>(null);

  const rows = $derived(visibleStackRows());

  /** The folder new stacks are saved in: the project the shell is pointed at. */
  const folder = $derived((stacks.activeRoot ?? '').trim());

  function openForm(): void {
    adding = true;
    confirmingRemoval = null;
    clearStackNotice();
  }

  function closeForm(): void {
    adding = false;
    draftName = '';
    draftScript = '';
    clearStackNotice();
  }

  function submitForm(event: SubmitEvent): void {
    event.preventDefault();
    const saved = addStack({ name: draftName, script: draftScript, cwd: folder });
    if (saved) closeForm();
  }

  function pressRemove(stackId: string): void {
    if (confirmingRemoval === stackId) {
      removeStack(stackId);
      confirmingRemoval = null;
      return;
    }
    confirmingRemoval = stackId;
  }

  /** Which colour a state gets. The four states, and nothing else. */
  function tone(state: string): string {
    if (state === 'running') return 'running';
    if (state === 'starting') return 'attention';
    if (state === 'failed') return 'failed';
    return 'neutral';
  }
</script>

<div class="stacks-pane" aria-label="Stacks">
  <header class="toolbar">
    <span class="toolbar-title">
      {stacks.projectName ? stacks.projectName : 'Stacks'}
    </span>
    <button
      type="button"
      class="tool"
      disabled={!folder}
      onclick={() => (adding ? closeForm() : openForm())}
      title={folder
        ? 'Save a command you want to be able to start from here'
        : 'Pick a session first, so a new stack knows which folder to run in'}
    >
      <Plus size={13} />
      <span>add</span>
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

  {#if adding}
    <form class="add-form" onsubmit={submitForm}>
      <label class="field">
        <span class="field-label">Name</span>
        <input
          class="field-input"
          type="text"
          bind:value={draftName}
          placeholder="Web"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <label class="field">
        <span class="field-label">Command</span>
        <input
          class="field-input mono"
          type="text"
          bind:value={draftScript}
          placeholder="pnpm dev"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <p class="field-hint">
        Runs in {folder || 'the project folder'} — the same thing as typing it in a terminal there.
      </p>
      <div class="form-actions">
        <button type="submit" class="primary">Save stack</button>
        <button type="button" class="secondary" onclick={closeForm}>Cancel</button>
      </div>
    </form>
  {/if}

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
        No stacks saved for {stacks.projectName} yet. Add the command you normally type to start it.
      {:else}
        No stacks saved yet. Add the command you normally type to start a project.
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
                title="Show this stack's terminal"
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
                <span>Start</span>
              </button>
            {/if}

            {#if confirmingRemoval === row.definition.id}
              <button
                type="button"
                class="action danger"
                onclick={() => pressRemove(row.definition.id)}
              >
                <span>Really remove?</span>
              </button>
              <button
                type="button"
                class="action quiet"
                title="Keep this stack"
                onclick={() => (confirmingRemoval = null)}
              >
                <X size={11} />
              </button>
            {:else}
              <button
                type="button"
                class="action quiet"
                title="Forget this saved command. The terminal it ran in is left alone."
                onclick={() => pressRemove(row.definition.id)}
              >
                <Trash2 size={11} />
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .stacks-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    background: #101014;
    color: #c9c9d4;
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
    background: #101014;
    border-bottom: 1px solid #22222c;
  }

  .toolbar-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #8a8a9c;
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
    color: #7b7b8c;
    font: inherit;
    font-size: 12px;
    padding: 2px 5px;
    cursor: pointer;
  }

  .tool:hover:not(:disabled) {
    background: #1c1c24;
    color: #d8d8e0;
  }

  .tool:disabled {
    cursor: default;
    opacity: 0.45;
  }

  /* ── Add form ──────────────────────────────────────────────────────── */
  .add-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid #22222c;
    background: #13131a;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .field-label {
    color: #7b7b8c;
    font-size: 12px;
  }

  .field-input {
    width: 100%;
    border: 1px solid #2a2a36;
    border-radius: 4px;
    background: #0c0c10;
    color: #e6e6ee;
    font: inherit;
    font-size: 12px;
    padding: 4px 6px;
  }

  .field-input:focus {
    outline: none;
    border-color: #bd93f9;
  }

  .field-hint {
    margin: 0;
    color: #6d6d7d;
    font-size: 12px;
    line-height: 1.4;
  }

  .form-actions {
    display: flex;
    gap: 6px;
  }

  .primary,
  .secondary {
    border: 1px solid transparent;
    border-radius: 4px;
    font: inherit;
    font-size: 12px;
    padding: 3px 8px;
    cursor: pointer;
  }

  .primary {
    background: #bd93f9;
    color: #16161c;
    font-weight: 600;
  }

  .primary:hover {
    background: #cbaaff;
  }

  .secondary {
    background: transparent;
    border-color: #2a2a36;
    color: #8a8a9c;
  }

  .secondary:hover {
    color: #d8d8e0;
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
    background: #17171d;
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
    background: #22222c;
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
    background: #3a3a48;
  }

  .dot[data-tone='running'] {
    background: #50fa7b;
  }

  .dot[data-tone='attention'] {
    background: #f1fa8c;
  }

  .dot[data-tone='failed'] {
    background: #ff5555;
  }

  .row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    color: #e6e6ee;
    font-size: 12px;
    line-height: 1.35;
  }

  .row-title.link {
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 12px;
    padding: 0;
    cursor: pointer;
  }

  .row-title.link:hover {
    color: #8be9fd;
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
    color: #6d6d7d;
    font-size: 13px;
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
    background: rgba(139, 233, 253, 0.1);
    color: #8be9fd;
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
    background: #24242f;
    color: #9a9aad;
    font-size: 12px;
    padding: 1px 5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip[data-tone='running'] {
    background: rgba(80, 250, 123, 0.12);
    color: #50fa7b;
  }

  .chip[data-tone='attention'] {
    background: rgba(241, 250, 140, 0.12);
    color: #f1fa8c;
  }

  .chip[data-tone='failed'] {
    background: rgba(255, 85, 85, 0.14);
    color: #ff8888;
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
    border: 1px solid #2a2a36;
    border-radius: 4px;
    background: transparent;
    color: #9a9aad;
    font: inherit;
    font-size: 12px;
    padding: 2px 6px;
    cursor: pointer;
  }

  .action:hover:not(:disabled) {
    border-color: #3a3a48;
    color: #e6e6ee;
    background: #1c1c24;
  }

  .action:disabled {
    cursor: default;
    opacity: 0.45;
  }

  .action.quiet {
    border-color: transparent;
    color: #6d6d7d;
  }

  .action.danger {
    border-color: rgba(255, 85, 85, 0.4);
    color: #ff8888;
  }

  /* ── States ────────────────────────────────────────────────────────── */
  .state {
    margin: 0;
    padding: 8px;
    color: #6d6d7d;
    font-size: 13px;
    line-height: 1.45;
  }

  .state.error {
    color: #ff9d9d;
  }

  .state.notice {
    color: #f1fa8c;
  }

  .retry {
    margin-left: 6px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: #bd93f9;
    font: inherit;
    font-size: 12px;
    padding: 0 2px;
    cursor: pointer;
    text-decoration: underline;
  }

  button:focus-visible,
  input:focus-visible {
    outline: 1px solid #bd93f9;
    outline-offset: -1px;
  }
</style>
