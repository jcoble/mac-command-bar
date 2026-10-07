<script lang="ts">
  import { onMount } from 'svelte';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';

  let { size = 16 }: { size?: number } = $props();
  let host = $state<HTMLSpanElement>();
  let visible = $state(false);
  let documentVisible = $state(false);
  let reducedMotion = $state(true);

  onMount(() => {
    const observer = new IntersectionObserver(([entry]) => { visible = entry.isIntersecting; });
    if (host) observer.observe(host);
    const visibilityChanged = () => { documentVisible = document.visibilityState === 'visible'; };
    const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
    const motionChanged = () => { reducedMotion = motion.matches; };
    visibilityChanged();
    motionChanged();
    document.addEventListener('visibilitychange', visibilityChanged);
    motion.addEventListener('change', motionChanged);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', visibilityChanged);
      motion.removeEventListener('change', motionChanged);
    };
  });
</script>

<span bind:this={host} class="spinner" class:spinning={visible && documentVisible && !reducedMotion} data-testid="working-spinner" aria-hidden="true">
  <LoaderCircle {size} />
</span>

<style>
  .spinner{display:inline-flex;flex-shrink:0;color:var(--color-accent)}
  .spinning{animation:spin 1s linear infinite}
  @keyframes spin{to{transform:rotate(360deg)}}
</style>
