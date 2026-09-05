<script lang="ts">
  /**
   * The bottom dock region: lazy workspace Terminal and Problems tabs.
   *
   * This region is on screen from the moment the shell opens, so the panel it
   * hosts must load nothing when it mounts — `ProblemsPanel` reads the language
   * server only when the user presses its Refresh button. See
   * `panelActivation.ts` for the rule and the route that is waiting for a
   * gesture to fire it.
   *
   * Moving the list is a SETTING, not a piece of state this component keeps: the
   * buttons write `settings.panels.problemsLocation` and say so, and the shell
   * reacts to the setting. That way the settings dialog and these buttons cannot
   * disagree, and the choice survives a restart.
   */
  import X from '@lucide/svelte/icons/x';
  import { tick } from 'svelte';

  import { settings, type ProblemsLocation } from '$lib/settingsStore.svelte';

  import ProblemsPanel from './problems/ProblemsPanel.svelte';
  import WorkspaceTerminal from './WorkspaceTerminal.svelte';

  type DockTab = 'problems' | 'terminal';

  interface Props {
    root: string;
    onReset(): void;
    /**
     * The user asked for the Problems list somewhere else. The setting is
     * already written by the time this runs — this is only so the shell can
     * close the bottom strip and open the list in its new home straight away,
     * rather than on the next launch.
     */
    onProblemsLocationChange?(location: ProblemsLocation): void;
  }
  let { root, onReset, onProblemsLocationChange }: Props = $props();
  let activeTab = $state<DockTab>('problems');
  let terminalOpened = $state(false);
  let terminal = $state<WorkspaceTerminal | null>(null);

  async function selectTab(tab: DockTab): Promise<void> {
    activeTab = tab;
    if (tab !== 'terminal') return;
    terminalOpened = true;
    await tick();
    terminal?.refit();
  }

  async function moveProblems(location: ProblemsLocation): Promise<void> {
    await terminal?.close();
    terminalOpened = false;
    activeTab = 'problems';
    settings.panels.problemsLocation = location;
    onProblemsLocationChange?.(location);
  }
</script>

{#snippet tabs()}
  <div class="dock-tabs" role="tablist" aria-label="Bottom dock">
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'problems'}
      class:active={activeTab === 'problems'}
      onclick={() => void selectTab('problems')}
    >Problems</button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'terminal'}
      class:active={activeTab === 'terminal'}
      onclick={() => void selectTab('terminal')}
    >Terminal</button>
  </div>
{/snippet}

{#snippet actions()}
  <button
    class="dock-action"
    type="button"
    title="Hide the bottom dock. Settings or the command palette brings it back."
    onclick={() => void moveProblems('hidden')}
  >
    <X size={13} strokeWidth={1.6} />
    Hide
  </button>
  <button class="dock-action" type="button" onclick={onReset}>Reset layout</button>
{/snippet}

<div class="dock-slot">
  <div class="dock-content" hidden={activeTab !== 'problems'}>
    <ProblemsPanel headerStart={tabs} headerEnd={actions} />
  </div>
  {#if terminalOpened}
    <div class="dock-content terminal-panel" hidden={activeTab !== 'terminal'}>
      <div class="terminal-header">
        {@render tabs()}
        <div class="terminal-actions">{@render actions()}</div>
      </div>
      {#key root}
        <WorkspaceTerminal bind:this={terminal} {root} />
      {/key}
    </div>
  {/if}
</div>

<style>
  .dock-slot {
    position: relative;
    height: 100%;
  }

  .dock-content {
    height: 100%;
    min-height: 0;
  }

  .dock-content[hidden] {
    display: none;
  }

  .terminal-panel {
    display: grid;
    grid-template-rows: 40px minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }

  .terminal-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--color-border);
    padding: 0 8px;
  }

  .terminal-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .dock-tabs {
    display: flex;
    align-self: stretch;
    gap: 14px;
  }

  .dock-tabs button {
    position: relative;
    border: 0;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 13px;
    padding: 0 2px;
    cursor: pointer;
  }

  .dock-tabs button:hover,
  .dock-tabs button.active {
    color: var(--color-text);
  }

  .dock-tabs button.active::after {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 2px;
    background: var(--color-accent);
    content: '';
  }

  .dock-tabs button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  /* On the end of the panel's own header row. These used to float in that
     corner, which put them straight on top of the panel's Refresh button. */
  .dock-action {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
    /* The same 28px the panel's own Refresh button is, so the header row reads
       as one row of controls rather than two sizes of button. */
    height: 28px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--color-text-2);
    font-family: inherit;
    font-size: 12px;
    padding: 0 8px;
    cursor: pointer;
    transition: color 120ms, background-color 120ms;
  }

  .dock-action:hover {
    color: var(--color-text);
    background: var(--color-elevated);
  }

  .dock-action:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
</style>
