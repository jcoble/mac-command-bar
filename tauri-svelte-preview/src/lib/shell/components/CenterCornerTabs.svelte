<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the centre pane's surface picker, as a row of
   * pills that floats over the pane instead of occupying a bar above it.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file. Three pills and nothing else: the language switch used
   * to ride along here, and on a 630px centre pane the four capsules took most
   * of the width the editor's file tabs needed. It lives in the editor's own
   * status band now, which is where the readout about the current file already
   * is.
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
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
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

  function choose(id: CenterTabId, event: MouseEvent): void {
    onSelect(id);
    // A pointer click leaves focus sitting on the button it hit, and focus is
    // what keeps the group on screen — so a mouse user would never see it fade
    // again, however far away the pointer went. Hand focus back and let hover
    // plus the hold below do the work. `detail` is 0 when a click came from the
    // keyboard, so Tab-and-Enter keeps both its focus and its reveal.
    if (event.detail > 0 && event.currentTarget instanceof HTMLElement) {
      event.currentTarget.blur();
    }
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

<nav class="center-pills" class:held={heldAfterSwitch} aria-label="Center surfaces">
  <!-- The project's language server, at the head of the group. -->
  <LanguageIntelligenceControls />

  {#each TABS as tab (tab.id)}
    {@const Icon = tab.icon}
    <!-- Icon only. The word lives on `label`, which `IconButton` makes both the
         accessible name and the tooltip, so nothing is lost to a screen reader
         or to anyone who pauses on one. `aria-current` sits on the wrapper
         because the kit button does not take it — the same arrangement the
         right panel's tab strip uses — and it is what the fill is painted from,
         so the one attribute that names the current surface also shows it. -->
    <span class="tab" aria-current={tab.id === activeId ? 'page' : undefined}>
      <IconButton
        label={tab.label}
        size="sm"
        side="bottom"
        variant="ghost"
        data-testid={`center-tab-${tab.id}`}
        onclick={(event) => choose(tab.id, event)}
      >
        <Icon class="size-4" strokeWidth={1.6} aria-hidden="true" />
      </IconButton>
    </span>
  {/each}
</nav>

<style>
  /* The group is laid over the pane, so it is only ever as wide as its pills and
     it lets every click through except its own.

     ONE offset, on every surface: clear of the line the editor keeps its file
     tabs on. Nothing here reads which surface is showing — an offset that
     changed with the surface made the row jump as you moved between Session,
     Editor and Diff. On Session and Diff that leaves empty space above the
     group, which costs nothing, because the row floats and takes no layout
     height anywhere. */
  .center-pills {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 6px;
    margin: calc(var(--editor-tab-row-height) + 6px) 8px 6px;
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

  /* The group's OWN keyboard focus, and only its own. The pane used to answer
     for this, which meant typing in the composer or the editor held the group
     open the whole time. Scoped here it does what it should: reach the pills by
     Tab and they appear; focus anything else and they do not. */
  .center-pills:focus-within {
    opacity: 1;
    pointer-events: auto;
  }

  .tab {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
  }

  /* Round capsules, opaque so they stay readable over code and transcript
     alike, and lifted off the surface underneath. Scoped through `.tab` so the
     language switch — which is a button too, inside its own capsule — is not
     shaped like a surface tab. */
  .tab :global(button) {
    width: 30px;
    height: 30px;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
    box-shadow: var(--shadow-sm);
    color: var(--color-text-2);
  }

  .tab :global(button:hover) {
    background: var(--pill-surface-hover);
    color: var(--color-text);
  }

  /* The one that is filled is the one you are on. With the words gone this fill
     is the only thing saying so, which is why it is the accent and not a tint. */
  .tab[aria-current='page'] :global(button) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }

  .tab[aria-current='page'] :global(button:hover) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }
</style>
