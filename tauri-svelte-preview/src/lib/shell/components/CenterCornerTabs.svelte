<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the centre pane's surface picker, as a row of
   * pills that floats over the pane instead of occupying a bar above it.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file. The project's language switch travels at the end of the
   * same row, because it is the other thing you change about the middle of the
   * window.
   *
   * WHY IT FLOATS. A permanent bar charged every surface the same strip of
   * height to answer a question that is only asked now and then, and on the
   * Editor there was already a bar — its file tabs. So the group is laid OVER
   * the pane, invisible until it is wanted:
   *
   *   the pointer anywhere in the centre pane, or the keyboard focus inside it,
   *   fades it in (the pane sets `--center-pills-reveal`, see `ShellFrame`);
   *   a tab switch holds it on screen for a moment afterwards, so the surface
   *   you just chose is confirmed before it disappears again.
   *
   * Both endings are definite: the fade is a plain CSS transition and the hold
   * is a single timer that is cancelled when the component goes away.
   *
   * PRESENTATIONAL ONLY: no state beyond that hold, no IO. The selected tab is
   * handed in and every click is handed back out.
   */
  import { onDestroy } from 'svelte';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitCompareArrows from '@lucide/svelte/icons/git-compare-arrows';
  import MessagesSquare from '@lucide/svelte/icons/messages-square';

  import LanguageIntelligenceControls from './LanguageIntelligenceControls.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { CenterTabId } from '$lib/shell/workbenchNavigation';

  interface Props {
    activeId: CenterTabId;
    onSelect(id: CenterTabId): void;
  }
  let { activeId, onSelect }: Props = $props();

  const TABS: ReadonlyArray<{ id: CenterTabId; label: string; icon: typeof FileCode2 }> = [
    { id: 'session', label: 'Session', icon: MessagesSquare },
    { id: 'editor', label: 'Editor', icon: FileCode2 },
    { id: 'diff', label: 'Diff', icon: GitCompareArrows }
  ];

  /** Long enough to read the pill that just filled, short enough that the group
   * is gone before it becomes furniture. */
  const HOLD_AFTER_SWITCH_MS = 1000;

  let heldAfterSwitch = $state(false);
  let holdTimer: ReturnType<typeof setTimeout> | null = null;

  function releaseHold(): void {
    if (holdTimer !== null) clearTimeout(holdTimer);
    holdTimer = null;
  }

  function choose(id: CenterTabId): void {
    onSelect(id);
    releaseHold();
    heldAfterSwitch = true;
    holdTimer = setTimeout(() => {
      heldAfterSwitch = false;
      holdTimer = null;
    }, HOLD_AFTER_SWITCH_MS);
  }

  // The hold is one timer with one ending. Leaving the shell before it fires
  // must not leave it running.
  onDestroy(releaseHold);
</script>

<nav
  class="center-pills"
  class:held={heldAfterSwitch}
  class:below-editor-tabs={activeId === 'editor'}
  aria-label="Center surfaces"
>
  {#each TABS as tab (tab.id)}
    {@const Icon = tab.icon}
    <!-- `text-sm` is 12px in this shell and overrides the xs recipe's 11px,
         which is below the floor for anything a person reads. Rest, hover and
         selected are painted below, off `aria-current`, so the one attribute
         that tells a screen reader which tab this is also tells the eye. -->
    <Button
      size="sm"
      variant="ghost"
      aria-current={tab.id === activeId ? 'page' : undefined}
      class="text-sm"
      data-testid={`center-tab-${tab.id}`}
      onclick={() => choose(tab.id)}
    >
      <Icon class="size-4" strokeWidth={1.6} aria-hidden="true" />
      {tab.label}
    </Button>
  {/each}

  <!-- The project's language server, on the same row. Nothing renders here
       until a project is open. -->
  <LanguageIntelligenceControls />
</nav>

<style>
  /* The group is laid over the pane, so it is only ever as wide as its pills and
     it lets every click through except its own. */
  .center-pills {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 6px;
    margin: 6px 8px;
    user-select: none;
    opacity: var(--center-pills-reveal, 0);
    pointer-events: var(--center-pills-events, none);
    /* Quick, and finished: one transition with a stated duration, nothing that
       keeps running once it has arrived. */
    transition: opacity 120ms ease-out;
  }

  /* Just switched: on screen regardless of where the pointer is, until the one
     timer in the script above lets go. */
  .center-pills.held {
    opacity: 1;
    pointer-events: auto;
  }

  /* On the Editor the pane already has a bar of its own — the row of file tabs
     — and the pills belong under it, not across it. Every other surface starts
     at the top of the pane, so nothing to clear. */
  .center-pills.below-editor-tabs {
    margin-top: calc(var(--editor-tab-row-height) + 6px);
  }

  /* Capsules: 26px tall with real side padding, opaque so they stay readable
     over code and transcript alike, and lifted off the surface underneath.
     The child combinator matters — the language switch is a button too, one
     nested inside its own capsule, and shaping it like a tab flattened it. */
  .center-pills > :global(button) {
    min-height: 26px;
    gap: 6px;
    padding: 4px 12px;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
    box-shadow: var(--shadow-sm);
    color: var(--color-text-2);
  }

  .center-pills > :global(button:hover) {
    background: var(--pill-surface-hover);
    color: var(--color-text);
  }

  /* The one that is filled is the one you are on. */
  .center-pills > :global(button[aria-current='page']) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }

  .center-pills > :global(button[aria-current='page']:hover) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }
</style>
