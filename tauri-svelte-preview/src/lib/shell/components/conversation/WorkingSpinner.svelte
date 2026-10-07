<script module lang="ts">
  // Only the chosen design is fetched; each file becomes its own small chunk.
  const loaders = import.meta.glob<string>('./spinners/*.svg', { query: '?raw', import: 'default' });
  let instances = 0;
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { pickSpinner } from './workingSpinners';

  let { size = 16, seed }: { size?: number; seed?: string } = $props();
  // A session's design follows its seed; an unseeded spinner picks once and keeps it while mounted.
  const randomDesign = pickSpinner();
  const design = $derived(seed === undefined ? randomDesign : pickSpinner(seed));
  const suffix = `-ws${++instances}`;
  let markup = $state('');
  let host = $state<HTMLSpanElement>();
  let visible = $state(false);
  let documentVisible = $state(false);
  let reducedMotion = $state(true);

  // Load only the chosen file; a load that lands after unmount or a seed change is dropped.
  $effect(() => {
    let current = true;
    void loaders[`./spinners/${design}.svg`]().then((raw) => {
      // Several spinners share a page, so each gets its own filter and shape ids.
      if (current) markup = raw.replace(/(id="|href="#|url\(#)([^")]+)/g, `$1$2${suffix}`);
    });
    return () => { current = false; };
  });

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

  // SMIL keeps running off screen, so the inserted SVG is paused unless it can be seen.
  $effect(() => {
    const svg = markup ? host?.querySelector('svg') : null;
    if (!svg) return;
    if (visible && documentVisible && !reducedMotion) svg.unpauseAnimations();
    else svg.pauseAnimations();
  });
</script>

<span bind:this={host} class="spinner" style:width={`${size}px`} style:height={`${size}px`} data-testid="working-spinner" data-spinner={design} aria-hidden="true">{@html markup}</span>

<style>
  .spinner{display:inline-flex;flex-shrink:0;vertical-align:middle}
  .spinner :global(svg){display:block;width:100%;height:100%}
</style>
