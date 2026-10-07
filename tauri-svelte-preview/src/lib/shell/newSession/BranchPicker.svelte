<!--
  BranchPicker.svelte — the branch menu under a new session's composer.

  Lists the project's branches with a search field. Picking one only tells the
  draft; nothing runs on disk until the first send.
-->
<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import GitBranch from '@lucide/svelte/icons/git-branch';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Command from '$lib/components/ui/command/index.js';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import { filterThreadStartGitRefs, refNote } from '$lib/shell/newSession/threadStartFlow.ts';
  import type { ProjectGitRef } from '$lib/shell/newSession/newSessionBackend.ts';

  interface Props {
    refs: ProjectGitRef[];
    rootPath: string;
    branch: string;
    onPick: (ref: ProjectGitRef) => void;
  }

  let { refs, rootPath, branch, onPick }: Props = $props();

  let open = $state(false);
  let search = $state('');
  const filtered = $derived(filterThreadStartGitRefs(refs, search));

  function pick(ref: ProjectGitRef): void {
    onPick(ref);
    open = false;
  }
</script>

<Popover.Root bind:open onOpenChange={(next) => { if (!next) search = ''; }}>
  <Popover.Trigger>
    {#snippet child({ props })}
      <Button {...props} data-testid="draft-session-branch" variant="ghost" size="xs" class="draft-control draft-branch">
        <GitBranch aria-hidden="true" class="size-3.5" />
        <span class="truncate">{branch || 'Choose a branch'}</span>
        <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
      </Button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Content side="top" align="end" sideOffset={8} collisionPadding={12} class="w-[min(440px,calc(100vw-32px))] gap-0 p-0">
    <Command.Root shouldFilter={false} value={branch}>
      <Command.Input data-testid="draft-session-ref-search" placeholder="Search branches" bind:value={search} />
      <Command.List class="max-h-[360px]">
        <Command.Empty>No matching branches</Command.Empty>
        {#each filtered.visible as ref (ref.name)}
          <Command.Item
            class="min-h-8 px-2.5 text-[13px]"
            value={ref.name}
            data-testid={`draft-session-ref-${ref.name}`}
            title={ref.checkoutPath ? `Checked out in ${ref.checkoutPath}` : undefined}
            onSelect={() => pick(ref)}
          >
            <span class="ref-name">{ref.name}</span>
            <span class="ref-note">{refNote(ref, rootPath)}</span>
          </Command.Item>
        {/each}
      </Command.List>
      <div class="ref-footer">Showing {filtered.visible.length} of {filtered.total} branches</div>
    </Command.Root>
  </Popover.Content>
</Popover.Root>

<style>
  .ref-name {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ref-note { flex: none; color: var(--color-text-3); font-size: 12px; }

  .ref-footer {
    padding: 7px 10px 8px;
    border-top: 1px solid var(--color-border);
    color: var(--color-text-3);
    font-size: 12px;
  }
</style>
