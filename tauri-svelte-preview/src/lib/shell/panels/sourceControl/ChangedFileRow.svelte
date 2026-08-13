<script lang="ts">
  /**
   * ChangedFileRow.svelte — one changed file in the Source control panel.
   *
   * The row is the kit's `ListRow`: a status letter, the file's name, the folder
   * it sits in, and what changed about it. Clicking it shows the file's changes
   * in the middle of the shell; right-clicking offers the same thing plus the
   * four ways of getting at the file outside this panel.
   *
   * Every action this row can run is read-only. Staging is a single press at the
   * top of the panel, and nothing here throws work away.
   */
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import { absolutePathWithin } from '$lib/shell/git/gitService';
  import { describeGitFileChange, gitFileTitle } from '$lib/shell/git/gitPanelStore.svelte';
  import { openDiffForFile, openFileInEditor } from '$lib/shell/workbenchNavigation';
  import { revealPathFromTauri } from '$lib/tauriSource';
  import type { ProjectGitFileStatus } from '$lib/tauriSource';
  import { splitRepositoryPath } from '$lib/shell/git/gitCommitFilesStore.svelte';

  import {
    sourceControlFileActions,
    type SourceControlFileActionId
  } from './sourceControlFileMenu.ts';

  interface Props {
    file: ProjectGitFileStatus;
    /** The repository folder the panel is pointed at. '' before there is one. */
    root: string;
    selected?: boolean;
  }
  let { file, root, selected = false }: Props = $props();

  const parts = $derived(splitRepositoryPath(file.relativePath));
  const actions = $derived(sourceControlFileActions(file, root));
  const absolutePath = $derived(root === '' ? '' : absolutePathWithin(root, file.relativePath));

  /** The letter's color sorts the list by eye before anyone reads a path. */
  const badgeTone = $derived.by(() => {
    if (file.badge === 'A' || file.badge === '?') return 'text-[var(--color-good)]';
    if (file.badge === 'D') return 'text-[var(--color-bad)]';
    if (file.badge === 'R' || file.badge === 'C') return 'text-[var(--color-accent)]';
    return 'text-[var(--color-attention)]';
  });

  function copy(text: string): void {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    void navigator.clipboard.writeText(text);
  }

  function showChanges(): void {
    if (root === '') return;
    void openDiffForFile({ projectRoot: root, relativePath: file.relativePath });
  }

  function run(id: SourceControlFileActionId): void {
    if (id === 'view') showChanges();
    else if (id === 'copy-path') copy(absolutePath);
    else if (id === 'copy-relative-path') copy(file.relativePath);
    else if (id === 'open-in-editor') openFileInEditor({ path: absolutePath, projectRoot: root });
    else if (id === 'reveal-in-finder') void revealPathFromTauri(absolutePath);
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}
      <div {...props} class="min-w-0">
        <ListRow
          {selected}
          onclick={showChanges}
          data-testid={`source-control-file-${file.relativePath}`}
          class="min-w-0"
        >
          <span
            class={`w-3 shrink-0 text-center font-mono text-sm leading-none ${badgeTone}`}
            aria-hidden="true">{file.badge}</span
          >
          <span class="min-w-0 flex-1 truncate" title={gitFileTitle(file)}>
            {parts.name}
            {#if parts.folder !== ''}
              <span class="text-sm text-muted-foreground">{parts.folder}</span>
            {/if}
          </span>
          <span class="shrink-0 text-sm text-muted-foreground">
            {describeGitFileChange(file)}
          </span>
        </ListRow>
      </div>
    {/snippet}
  </ContextMenu.Trigger>

  <ContextMenu.Content class="w-[220px]" aria-label="File actions">
    {#each actions as item (item.id)}
      <ContextMenu.Item
        data-testid={`source-control-file-menu-${item.id}`}
        disabled={!item.enabled}
        title={item.enabled ? item.label : item.disabledReason}
        onSelect={() => run(item.id)}>{item.label}</ContextMenu.Item
      >
    {/each}
  </ContextMenu.Content>
</ContextMenu.Root>
