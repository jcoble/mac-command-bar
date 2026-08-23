<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the centre pane's surface picker, as ONE capsule
   * that floats over the pane instead of occupying a bar above it.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file, and the project's language switch leads them while the
   * Editor is showing. One capsule, not four: each control used to carry its own
   * fill and its own shadow, and four separate discs at the top of the pane read
   * as things dropped there rather than as the head of a column. Sharing a
   * surface is what makes them one control, and the head one thing.
   *
   * WHY IT FLOATS. A permanent bar charged every surface the same strip of
   * height to answer a question that is only asked now and then, and on the
   * Editor there was already a bar — its file tabs. So the group is laid OVER
   * the pane, invisible until it is wanted:
   *
   *   the pointer anywhere in the centre pane, or the keyboard focus inside it,
   *   fades it in (the pane marks only this overlay hovered in `ShellFrame`);
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
  import GitBranch from '@lucide/svelte/icons/git-branch';
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
    { id: 'diff', label: 'Diff', icon: GitCompareArrows },
    { id: 'git-history', label: 'Git History', icon: GitBranch }
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
  <!-- The project's language server, at the head of the group, and only while
       the Editor is the surface on screen: it describes the file being edited,
       so on a transcript it has nothing to say. It leads the group rather than
       trailing it precisely because it comes and goes — arriving at the left
       lengthens the group leftwards and the three pills do not move. -->
  {#if activeId === 'editor'}
    <LanguageIntelligenceControls />
  {/if}

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
  /* The head is one capsule laid over the pane, so it is only ever as wide as
     the controls inside it and it lets every click through except its own. The
     surface and the single shadow belong to the capsule rather than to each
     control, which is what makes the controls read as one group.

     ONE offset, on every surface: clear of the line the editor keeps its file
     tabs on. Nothing here reads which surface is showing — an offset that
     changed with the surface made the row jump as you moved between Session,
     Editor and Diff. On Session and Diff that leaves empty space above the
     capsule, which costs nothing, because it floats and takes no layout height
     anywhere. What it does cost is the band it covers, and the transcript
     underneath keeps clear of that band by reading `--center-head-height`. */
  .center-pills {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    /* Tight, because the capsule's own edge is what separates the group from
       the pane now; the controls inside it only need to stay apart. */
    gap: 2px;
    height: var(--center-head-row-height);
    padding: 2px;
    margin: calc(var(--editor-tab-row-height) + 6px) 8px 6px;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
    box-shadow: var(--shadow-sm);
    user-select: none;
    opacity: 0;
    pointer-events: none;
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

  /* Marks on the capsule's own surface: no fill and no shadow of their own, so
     the only filled disc in the group is the one that means something. 28px is
     the kit's floor for an icon button — a smaller square is one a pointer
     misses — and the capsule is built around it rather than the other way
     round. Scoped through `.tab` so the language switch, which is a button too,
     is not shaped like a surface tab. */
  .tab :global(button) {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-2);
  }

  .tab :global(button:hover) {
    background: var(--pill-surface-hover);
    color: var(--color-text);
    /* The kit draws a soft 4px ring around a hovered icon button. On a button
       standing on its own that reads well; inside a capsule with 2px of room
       around it, the ring lands on the capsule's edge and blurs it. */
    box-shadow: none;
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
