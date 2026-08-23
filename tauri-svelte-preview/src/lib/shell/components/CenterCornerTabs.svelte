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
   * the pane and stays visible without occupying layout height.
   *
   * PRESENTATIONAL ONLY: no state beyond that hold, no IO. The selected tab is
   * handed in and every click is handed back out.
   */
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

  function choose(id: CenterTabId): void {
    onSelect(id);
  }
</script>

<nav class="center-pills" aria-label="Center surfaces">
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
        onclick={() => choose(tab.id)}
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
