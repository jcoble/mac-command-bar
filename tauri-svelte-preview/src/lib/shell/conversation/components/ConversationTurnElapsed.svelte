<script lang="ts">
  import { onMount } from 'svelte';
  import { formatWorkedFor } from '$lib/shell/conversation/conversationTimeline.ts';
  import { pageClock, watchPageClock } from '$lib/pageClock.svelte';

  let { running, completed, startedAtMs, elapsedMs }: {
    running: boolean;
    completed: boolean;
    startedAtMs: number | null;
    elapsedMs: number | null;
  } = $props();
  let host = $state<HTMLSpanElement>();
  let visible = $state(false);
  let documentVisible = $state(true);
  const duration = $derived(running && startedAtMs !== null
    ? Math.max(0, pageClock.now - startedAtMs) : elapsedMs);

  onMount(() => {
    const observer = new IntersectionObserver(([entry]) => { visible = entry.isIntersecting; });
    if (host) observer.observe(host);
    const visibilityChanged = () => { documentVisible = document.visibilityState === 'visible'; };
    visibilityChanged();
    document.addEventListener('visibilitychange', visibilityChanged);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', visibilityChanged);
    };
  });
  $effect(() => {
    if (!running || startedAtMs === null || !visible || !documentVisible) return;
    return watchPageClock(1000);
  });
</script>

<span bind:this={host}>{running ? 'Working' : completed ? 'Worked' : 'Work'}{duration === null ? '' : ` for ${formatWorkedFor(duration)}`}</span>
