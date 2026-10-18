<script lang="ts">
  import { formatWorkedFor } from '$lib/shell/conversation/conversationTimeline.ts';

  let { running, completed, elapsedMs }: {
    running: boolean;
    completed: boolean;
    startedAtMs: number | null;
    elapsedMs: number | null;
  } = $props();
  // A running turn shows no live clock; the duration appears once the turn reports it.
  const duration = $derived(running ? null : elapsedMs);
</script>

<span>{running ? 'Working' : completed ? 'Worked' : 'Work'}{duration === null ? '' : ` for ${formatWorkedFor(duration)}`}</span>
