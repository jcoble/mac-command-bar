<script lang="ts">
  /**
   * FileTreeRow.svelte — one line of the Files panel's tree.
   *
   * A folder or a file, indented by its depth, with a chevron on folders and a
   * count of what is inside them. A path git ignores is dimmed rather than
   * hidden, and stays clickable — the tree tells you the file is there and that
   * it is outside the repository's history.
   *
   * The row itself is the kit's `ListRow`, so it matches every other list in
   * the app: same height, same hover, same focus ring.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';

  import { Chip } from '$lib/components/ui/chip/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import FileIcon from '$lib/shell/components/explorer/FileIcon.svelte';

  import type { FileTreeNode } from './fileTreeModel.ts';

  interface Props {
    node: FileTreeNode;
    /** Folders only: whether this one is open. */
    expanded: boolean;
    /** True for the file the editor was last asked to open from here. */
    selected: boolean;
    /** Folder rows toggle; file rows open in the editor. */
    onclick: (node: FileTreeNode) => void;
  }
  let { node, expanded, selected, onclick }: Props = $props();

  /** 12px per level, which keeps a six-deep path readable in a narrow panel. */
  const indent = $derived(node.depth * 12);
</script>

<ListRow
  {selected}
  class="h-7"
  onclick={() => onclick(node)}
>
  <span class="shrink-0" style={`width: ${indent}px`} aria-hidden="true"></span>
  <span class="flex size-3 shrink-0 items-center justify-center text-muted-foreground" aria-hidden="true">
    {#if node.isDirectory}
      {#if expanded}
        <ChevronDown size={12} strokeWidth={2} />
      {:else}
        <ChevronRight size={12} strokeWidth={2} />
      {/if}
    {/if}
  </span>
  <span class="flex size-3.5 shrink-0 items-center justify-center text-muted-foreground" aria-hidden="true">
    {#if node.isDirectory}
      {#if expanded}
        <FolderOpen size={14} strokeWidth={1.75} />
      {:else}
        <Folder size={14} strokeWidth={1.75} />
      {/if}
    {:else}
      <FileIcon fileName={node.name} size={14} />
    {/if}
  </span>
  <span
    class="min-w-0 flex-1 truncate {node.ignored || node.isDirectory
      ? 'text-muted-foreground'
      : 'text-foreground'}"
    title={node.ignored ? `${node.path} — ignored by git` : node.path}
  >
    {node.name}
  </span>
  {#if node.isDirectory && node.childCount > 0}
    <Chip tone="count">{node.childCount}</Chip>
  {/if}
</ListRow>
