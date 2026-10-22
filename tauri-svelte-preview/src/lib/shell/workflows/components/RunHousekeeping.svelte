<!--
  RunHousekeeping.svelte — the owner's housekeeping buttons, each asking first.

  On a run page: "Mark finished" for a run that hasn't ended (with a one-line
  note), or "Remove" for one that has. On the board (no run): "Remove old runs",
  every finished run older than a number of days.
-->
<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { runEnded } from '../workflowBoard.ts';
  import type { WorkflowRunRecord } from '../workflowTypes.ts';

  interface Props {
    /** The open run, or null on the board. */
    run: WorkflowRunRecord | null;
    busy: boolean;
    onMarkFinished?: (note: string) => void;
    /** Removes the open run, or on the board every finished run older than `olderThanDays`. */
    onRemove: (olderThanDays: number | null) => void;
  }

  let { run, busy, onMarkFinished, onRemove }: Props = $props();

  let note = $state('');
  let days = $state(30);
  const trigger = buttonVariants({ variant: 'ghost', size: 'sm' });
  const text = 'text-[13px] leading-[1.5]';
</script>

{#if run && !runEnded(run)}
  <AlertDialog.Root onOpenChange={(open) => { if (open) note = ''; }}>
    <AlertDialog.Trigger class={trigger} disabled={busy}><CircleCheck />Mark finished</AlertDialog.Trigger>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Mark this run finished?</AlertDialog.Title>
        <AlertDialog.Description class={text}>
          The run ends as finished by you, and every agent still working on it stops. A worktree with uncommitted or
          unmerged work is kept. It does not count as merged.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <label class="flex flex-col gap-1 {text} text-muted-foreground">
        Why it is finished, in one line
        <Input class="h-8 text-[13px]" maxlength={200} placeholder="Done by hand in PR #123" bind:value={note} />
      </label>
      <AlertDialog.Footer>
        <AlertDialog.Cancel size="sm">Cancel</AlertDialog.Cancel>
        <AlertDialog.Action size="sm" disabled={!note.trim()} onclick={() => onMarkFinished?.(note.trim())}>Mark finished</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
{:else if run}
  <AlertDialog.Root>
    <AlertDialog.Trigger class={trigger} disabled={busy}><Trash2 />Remove</AlertDialog.Trigger>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Remove this run?</AlertDialog.Title>
        <AlertDialog.Description class={text}>
          Its record and its proof files are deleted. Its worktree and pull request are not touched.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Cancel size="sm">Cancel</AlertDialog.Cancel>
        <AlertDialog.Action size="sm" variant="destructive" onclick={() => onRemove(null)}>Remove</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
{:else}
  <AlertDialog.Root>
    <AlertDialog.Trigger class={trigger} disabled={busy}><Trash2 />Remove old runs</AlertDialog.Trigger>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Remove old finished runs?</AlertDialog.Title>
        <AlertDialog.Description class={text}>
          Completed, stopped and failed runs untouched for that long are deleted with their proof files. Unfinished runs stay.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <label class="flex items-center gap-2 {text} text-muted-foreground">
        Older than
        <Input class="h-8 w-20 text-[13px]" type="number" min={1} bind:value={days} />
        days
      </label>
      <AlertDialog.Footer>
        <AlertDialog.Cancel size="sm">Cancel</AlertDialog.Cancel>
        <AlertDialog.Action size="sm" variant="destructive" disabled={!(days >= 1)} onclick={() => onRemove(Math.floor(days))}>Remove</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
{/if}
