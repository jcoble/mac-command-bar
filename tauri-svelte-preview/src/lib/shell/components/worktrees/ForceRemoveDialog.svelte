<script lang="ts">
  /**
   * ForceRemoveDialog.svelte — the question asked before a worktree is deleted
   * along with whatever is still in it.
   *
   * This is the only place in the pane that can lose work, so it is written to
   * be read, not clicked through:
   *
   *  - it lists, one sentence per item, exactly what will be gone afterwards —
   *    the uncommitted changes, the commits that reached no remote, the lock it
   *    will undo, the sessions still pointed at the folder;
   *  - it will not go through until the folder's own name is typed, because the
   *    name is the one thing a person cannot type by accident;
   *  - it says the name of the folder on disk, not the branch. The branch
   *    survives; the folder does not.
   *
   * No `window.confirm` anywhere: that dialog does not exist in the desktop
   * webview, where it answers "no" without asking anybody.
   */
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { describeForcedRemoval, type WorktreeManagerRow } from '$lib/shell/worktrees/worktreeManagerRows';

  interface Props {
    /** The row being asked about, or `null` when the dialog is shut. */
    row: WorktreeManagerRow | null;
    open: boolean;
    onOpenChange(open: boolean): void;
    /** Go through with it. Only ever reachable once the name has been typed. */
    onConfirm(): void;
  }
  let { row, open, onOpenChange, onConfirm }: Props = $props();

  /** What has been typed into the confirmation box. */
  let typed = $state('');

  const losses = $derived(row ? describeForcedRemoval(row) : []);
  const folder = $derived(row?.folderName ?? '');
  const matches = $derived(typed.trim() === folder && folder !== '');

  function change(next: boolean): void {
    // Every opening starts from an empty box: a name left over from the last
    // worktree would let the next one through without being read.
    if (!next) typed = '';
    onOpenChange(next);
  }

  function confirm(): void {
    if (!matches) return;
    typed = '';
    onConfirm();
  }
</script>

<AlertDialog.Root {open} onOpenChange={change}>
  <AlertDialog.Content
    class="rounded-lg bg-background text-foreground ring-[var(--color-border)]
           shadow-[var(--shadow-lg)]"
  >
    <AlertDialog.Header>
      <AlertDialog.Title class="text-[14px] leading-[1.4] font-semibold">
        Delete the folder “{folder}” and everything left in it?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-[1.5] text-[var(--color-text-2)]">
        The branch “{row?.branch ?? ''}” stays in the repository. The folder on disk does not, and
        neither does anything below.
      </AlertDialog.Description>
    </AlertDialog.Header>

    <ul class="m-0 flex list-none flex-col gap-1 p-0">
      {#each losses as line, index (index)}
        <li class="flex gap-1.5 text-[13px] leading-[1.5] text-[var(--color-text)]">
          <span class="text-[var(--color-bad)]" aria-hidden="true">•</span>
          <span>{line}</span>
        </li>
      {/each}
    </ul>

    <label class="flex flex-col gap-1 text-[13px] leading-[1.5] text-[var(--color-text-2)]">
      Type <span class="font-medium text-[var(--color-text)]">{folder}</span> to confirm.
      <Input
        class="h-8 text-[13px]"
        autocomplete="off"
        spellcheck="false"
        placeholder={folder}
        bind:value={typed}
      />
    </label>

    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Keep it</AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant="destructive"
        class="text-[13px]"
        disabled={!matches}
        onclick={confirm}
      >
        Delete it
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
