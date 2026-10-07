<script lang="ts">
  import { onMount } from 'svelte';
  import { DESIGNS, pickSpinner, type Part } from './workingSpinners';

  let { size = 16, seed }: { size?: number; seed?: string } = $props();
  // A session's design follows its seed; an unseeded spinner picks once and keeps it while mounted.
  const randomDesign = pickSpinner();
  const id = $derived(seed === undefined ? randomDesign : pickSpinner(seed));
  const design = $derived(DESIGNS[id]);
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

  const seconds = (value: number | undefined) => (value ? `${Math.abs(value)}s` : undefined);
</script>

<!-- Shapes are painted once; only transform animations run, so frames are composited, not redrawn. -->
{#snippet draw(part: Part, outerPx: number)}
  {@const px = (outerPx * part.size) / 100}
  {@const [x, y] = part.at ?? [50, 50]}
  {@const unit = 24 / px}
  <span class="axis" style:left={`${x - part.size / 2}%`} style:top={`${y - part.size / 2}%`} style:width={`${part.size}%`} style:height={`${part.size}%`} style:transform={part.tilt ? `rotate(${part.tilt}deg)` : undefined}>
    <span
      class={['part', part.shape === 'ring' && 'ring', part.shape === 'dot' && 'dot', part.turn && 'turn', (part.spin ?? 0) > 0 && 'spin', (part.spin ?? 0) < 0 && 'spin-back']}
      style:--c={part.color}
      style:animation-duration={seconds(part.turn ?? part.spin)}
      style:animation-delay={part.delay ? `${part.delay}s` : undefined}
    >
      {#if part.shape !== 'ring' && part.shape !== 'dot'}
        <svg viewBox="0 0 24 24" fill="none" stroke-linecap="round" stroke-linejoin="round">
          <path d={part.shape} stroke={part.color} stroke-opacity="0.25" stroke-width={3 * unit} />
          <path d={part.shape} stroke={part.color} stroke-width={1.5 * unit} />
          <path d={part.shape} stroke="#fff" stroke-width={0.6 * unit} />
        </svg>
      {/if}
    </span>
  </span>
{/snippet}

<span bind:this={host} class="spinner" class:spinning={visible && documentVisible && !reducedMotion} style:width={`${size}px`} style:height={`${size}px`} data-testid="working-spinner" data-spinner={id} aria-hidden="true">
  <span class={['part', (design.spin ?? 0) > 0 && 'spin', (design.spin ?? 0) < 0 && 'spin-back']} style:animation-duration={seconds(design.spin)}>
    {#each design.parts as part}{@render draw(part, size)}{/each}
  </span>
</span>

<style>
  .spinner{display:inline-flex;flex-shrink:0;vertical-align:middle;position:relative}
  .axis,.part{position:absolute}
  .part{inset:0}
  .part svg{display:block;width:100%;height:100%;overflow:visible}
  .ring{border-radius:50%;border:1px solid #fff;box-shadow:0 0 1px 1px var(--c),0 0 3px var(--c),inset 0 0 1px 1px var(--c)}
  .dot{border-radius:50%;background:#fff;box-shadow:0 0 3px 1px var(--c)}
  .turn{animation:turn 1s cubic-bezier(.37,0,.63,1) infinite alternate}
  .spin{animation:spin 1s linear infinite}
  /* A separate keyframe, not animation-direction:reverse, which WebKit redraws every frame. */
  .spin-back{animation:spin-back 1s linear infinite}
  /* Loops run only while the spinner is on screen, the window is visible and motion is allowed. */
  .spinner:not(.spinning) :is(.turn,.spin,.spin-back){animation-play-state:paused}
  @keyframes turn{from{transform:scaleX(1)}to{transform:scaleX(-1)}}
  @keyframes spin{to{transform:rotate(360deg)}}
  @keyframes spin-back{to{transform:rotate(-360deg)}}
</style>
