<script lang="ts">
  /**
   * RemoveWorktreeDialog.svelte — the question asked before ANY worktree is
   * removed, cleared or deleted.
   *
   * There used to be one dialog here, and it only guarded the destructive
   * button. The two quieter paths went straight through on the first click, and
   * one of them — the row whose folder is already gone — turned out to clear
   * every other folder-gone row along with it. Somebody pressed it, watched two
   * rows disappear, and had no way of knowing that would happen. So all three
   * paths now come through this file, and the difference between them is a
   * different set of sentences, not a different amount of asking.
   *
   * What the dialog says is not decided here. `describeRemovalQuestion` in
   * `worktreeManagerRows.ts` works out the title, the sentences, the button
   * words and whether the folder name has to be typed — which is what lets the
   * exact wording, including the older-app-build warning, be read in a test
   * rather than clicked through in an app.
   *
   * No `window.confirm` anywhere: that dialog does not exist in the desktop
   * webview, where it answers "no" without asking anybody.
   */
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import type { WorktreeRemovalQuestion } from '$lib/shell/worktrees/worktreeManagerRows';

  interface Props {
    /** What is being asked, or `null` when the dialog is shut. */
    question: WorktreeRemovalQuestion | null;
    /** The folder's own name — the word that has to be typed, when one does. */
    folderName: string;
    open: boolean;
    onOpenChange(open: boolean): void;
    /** Go through with it. */
    onConfirm(): void;
  }
  let { question, folderName, open, onOpenChange, onConfirm }: Props = $props();

  /** What has been typed into the confirmation box. */
  let typed = $state('');

  const needsName = $derived(question?.requiresTypedName === true);
  /**
   * The folder name is the one thing a person cannot type by accident, so the
   * destructive path waits for it. Every other path is ready as soon as it opens.
   */
  const ready = $derived(!needsName || (typed.trim() === folderName && folderName !== ''));

  function change(next: boolean): void {
    // Every opening starts from an empty box: a name left over from the last
    // worktree would let the next one through without being read.
    if (!next) typed = '';
    onOpenChange(next);
  }

  function confirm(): void {
    if (!ready) return;
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
        {question?.title ?? ''}
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-[1.5] text-[var(--color-text-2)]">
        {question?.intro ?? ''}
      </AlertDialog.Description>
    </AlertDialog.Header>

    <ul class="m-0 flex list-none flex-col gap-1 p-0">
      {#each question?.lines ?? [] as line, index (index)}
        <li class="flex gap-1.5 text-[13px] leading-[1.5] text-[var(--color-text)]">
          <span
            class={question?.destructive ? 'text-[var(--color-bad)]' : 'text-[var(--color-text-3)]'}
            aria-hidden="true">•</span
          >
          <span>{line}</span>
        </li>
      {/each}
    </ul>

    {#if needsName}
      <label class="flex flex-col gap-1 text-[13px] leading-[1.5] text-[var(--color-text-2)]">
        Type <span class="font-medium text-[var(--color-text)]">{folderName}</span> to confirm.
        <Input
          class="h-8 text-[13px]"
          autocomplete="off"
          spellcheck="false"
          placeholder={folderName}
          bind:value={typed}
        />
      </label>
    {/if}

    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">
        {question?.cancelLabel ?? 'Cancel'}
      </AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant={question?.destructive ? 'destructive' : 'default'}
        class="text-[13px]"
        disabled={!ready}
        onclick={confirm}
      >
        {question?.confirmLabel ?? 'Go ahead'}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
