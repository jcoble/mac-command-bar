<script lang="ts">
  /**
   * The small figure that means "the agent is working".
   *
   * Which of the eleven it is comes from the seed, so one turn keeps one spinner
   * for its whole life and the next turn brings a different one. Most drawings
   * are a handful of empty spans that the stylesheet below shapes and moves;
   * tide, ribbon and bloom are one small inline SVG each, moved by the same
   * stylesheet. Only transform and opacity are animated (plus the ribbon's
   * dash running along its own outline), so a frame stays cheap.
   *
   * Nothing here loops at rest. The caller mounts this only while a turn is
   * running, so a finished turn removes the element and its animations with it,
   * and while it is mounted it still pauses whenever it scrolls off screen or
   * the window is hidden — an animation nobody can see costs exactly what a
   * visible one costs.
   */
  import { observeElementVisibility } from '$lib/shell/elementVisibility.ts';

  import { pickSpinner, SPINNERS } from './workingSpinners.ts';

  interface Props {
    /** Anything stable for the length of the turn: a turn id, or a session id. */
    seed: string;
    /** Box size in pixels. Everything inside is measured from it. */
    size?: number;
  }

  let { seed, size = 16 }: Props = $props();

  const spinner = $derived(
    SPINNERS.find((candidate) => candidate.id === pickSpinner(seed)) ?? SPINNERS[0]
  );

  let host = $state<HTMLSpanElement | null>(null);
  let onScreen = $state(true);

  $effect(() => {
    const element = host;
    if (!element) return;
    return observeElementVisibility(element, (visible) => {
      onScreen = visible;
    });
  });

  let documentVisible = $state(true);

  $effect(() => {
    const update = () => {
      documentVisible = document.visibilityState === 'visible';
    };
    update();
    document.addEventListener('visibilitychange', update);
    return () => document.removeEventListener('visibilitychange', update);
  });
</script>

<span
  bind:this={host}
  class="working-spinner"
  data-testid="working-spinner"
  data-spinner={spinner.id}
  data-active={onScreen && documentVisible}
  style={`--size:${size}px`}
  aria-hidden="true"
>
  {#if spinner.id === 'tide'}
    <svg viewBox="0 0 48 48" aria-hidden="true" focusable="false">
      <circle class="as-ring" cx="24" cy="24" r="13" />
      <circle class="as-drop" cx="24" cy="24" r="5" />
      <circle class="as-drop" cx="24" cy="24" r="5" />
      <circle class="as-drop" cx="24" cy="24" r="5" />
    </svg>
  {:else if spinner.id === 'ribbon'}
    <svg viewBox="0 0 48 48" aria-hidden="true" focusable="false">
      <g class="as-twist">
        <path class="as-track" pathLength="100" d="M24 24C30 13 43 13 43 24C43 35 30 35 24 24C18 13 5 13 5 24C5 35 18 35 24 24Z" />
        <path class="as-strand" pathLength="100" d="M24 24C30 13 43 13 43 24C43 35 30 35 24 24C18 13 5 13 5 24C5 35 18 35 24 24Z" />
        <path class="as-strand as-strand-trail" pathLength="100" d="M24 24C30 13 43 13 43 24C43 35 30 35 24 24C18 13 5 13 5 24C5 35 18 35 24 24Z" />
      </g>
    </svg>
  {:else if spinner.id === 'bloom'}
    <svg viewBox="0 0 48 48" aria-hidden="true" focusable="false">
      <g class="as-spin">
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <path class="as-petal" d="M24 24C18.5 18 19.5 9 24 4.5C28.5 9 29.5 18 24 24Z" />
        <circle class="as-core" cx="24" cy="24" r="2.5" />
      </g>
    </svg>
  {:else}
    {#each { length: spinner.parts } as _, index (index)}
      <i style={`--i:${index}`}></i>
    {/each}
  {/if}
  <b class="static-mark"></b>
</span>

<style>
  /* The box every variant is measured from. `--p` is the viewing distance the
     three-dimensional ones project through: close enough that a 16px cube reads
     as a solid, far enough that its near face does not balloon. */
  .working-spinner {
    position: relative;
    display: inline-block;
    flex: 0 0 auto;
    width: var(--size);
    height: var(--size);
    color: var(--color-accent);
    vertical-align: middle;
    --p: calc(var(--size) * 3);
  }

  .working-spinner i,
  .static-mark {
    position: absolute;
    display: block;
    box-sizing: border-box;
  }

  /* cube — six faces on a body that tumbles on two axes. The body is this
     element itself, which is why the perspective rides in the transform. */
  .working-spinner[data-spinner='cube'] {
    transform-style: preserve-3d;
    animation: ws-tumble 3.51s linear infinite;
    --face: calc(var(--size) * 0.34);
  }
  .working-spinner[data-spinner='cube'] i {
    inset: calc(var(--size) * 0.16);
    border: 1.5px solid currentColor;
    background: color-mix(in srgb, currentColor 12%, transparent);
  }
  .working-spinner[data-spinner='cube'] i:nth-child(1) { transform: translateZ(var(--face)); }
  .working-spinner[data-spinner='cube'] i:nth-child(2) { transform: rotateY(180deg) translateZ(var(--face)); }
  .working-spinner[data-spinner='cube'] i:nth-child(3) { transform: rotateY(90deg) translateZ(var(--face)); }
  .working-spinner[data-spinner='cube'] i:nth-child(4) { transform: rotateY(-90deg) translateZ(var(--face)); }
  .working-spinner[data-spinner='cube'] i:nth-child(5) { transform: rotateX(90deg) translateZ(var(--face)); }
  .working-spinner[data-spinner='cube'] i:nth-child(6) { transform: rotateX(-90deg) translateZ(var(--face)); }

  /* facet — four diamond panels hinged on one vertical diagonal. Gathered, they
     lie as a single solid gem; they fan open into an eight-bladed crystal and
     close again while the body turns. The fold runs twice per turn, so the gem
     closes exactly when it faces the reader and never collapses edge-on. */
  .working-spinner[data-spinner='facet'] {
    transform-style: preserve-3d;
    animation: ws-facet-turn 8s linear infinite;
  }
  .working-spinner[data-spinner='facet'] i {
    inset: calc(var(--size) * 0.2);
    border: 1.5px solid currentColor;
    background: color-mix(in srgb, currentColor 12%, transparent);
    animation: ws-facet 4s cubic-bezier(0.45, 0, 0.55, 1) infinite;
  }

  /* wave — three dots riding the same swell a beat apart. */
  .working-spinner[data-spinner='wave'] i {
    top: 50%;
    width: calc(var(--size) * 0.22);
    height: calc(var(--size) * 0.22);
    margin-top: calc(var(--size) * -0.11);
    border-radius: 50%;
    background: currentColor;
    animation: ws-wave 1.22s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.18s);
  }
  .working-spinner[data-spinner='wave'] i:nth-child(1) { left: 0; }
  .working-spinner[data-spinner='wave'] i:nth-child(2) {
    left: 50%;
    margin-left: calc(var(--size) * -0.11);
  }
  .working-spinner[data-spinner='wave'] i:nth-child(3) { right: 0; }

  /* bars — three columns growing off the floor. */
  .working-spinner[data-spinner='bars'] i {
    bottom: 0;
    width: calc(var(--size) * 0.2);
    height: 100%;
    border-radius: calc(var(--size) * 0.1);
    background: currentColor;
    transform-origin: 50% 100%;
    animation: ws-bars 1.05s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.15s);
  }
  .working-spinner[data-spinner='bars'] i:nth-child(1) { left: 0; }
  .working-spinner[data-spinner='bars'] i:nth-child(2) {
    left: 50%;
    margin-left: calc(var(--size) * -0.1);
  }
  .working-spinner[data-spinner='bars'] i:nth-child(3) { right: 0; }

  /* halo — rings leaving the centre, the second half a beat behind. */
  .working-spinner[data-spinner='halo'] i {
    inset: 0;
    border: 1.5px solid currentColor;
    border-radius: 50%;
    animation: ws-halo 1.89s ease-out infinite;
    animation-delay: calc(var(--i) * 0.95s);
  }

  /* diamond — a square that turns and breathes. */
  .working-spinner[data-spinner='diamond'] i {
    inset: calc(var(--size) * 0.16);
    border: 2px solid currentColor;
    border-radius: 2px;
    animation: ws-diamond 1.49s ease-in-out infinite;
  }

  /* disc — a coin turning edge-on and back. */
  .working-spinner[data-spinner='disc'] { transform-style: preserve-3d; }
  .working-spinner[data-spinner='disc'] i {
    inset: calc(var(--size) * 0.06);
    border: 2px solid currentColor;
    border-radius: 50%;
    background: color-mix(in srgb, currentColor 14%, transparent);
    animation: ws-flip 2.03s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  }

  /* gyro — two tilted rings turning about axes at right angles. */
  .working-spinner[data-spinner='gyro'] { transform-style: preserve-3d; }
  .working-spinner[data-spinner='gyro'] i {
    inset: 0;
    border: 1.5px solid currentColor;
    border-radius: 50%;
  }
  .working-spinner[data-spinner='gyro'] i:nth-child(1) { animation: ws-gyro-x 1.76s linear infinite; }
  .working-spinner[data-spinner='gyro'] i:nth-child(2) {
    inset: calc(var(--size) * 0.18);
    border-color: color-mix(in srgb, currentColor 55%, transparent);
    animation: ws-gyro-y 1.49s linear infinite;
  }

  @keyframes ws-wave {
    0%, 100% { transform: translateY(calc(var(--size) * 0.16)); opacity: 0.45; }
    50% { transform: translateY(calc(var(--size) * -0.16)); opacity: 1; }
  }
  @keyframes ws-bars {
    0%, 100% { transform: scaleY(0.32); opacity: 0.5; }
    50% { transform: scaleY(1); opacity: 1; }
  }
  @keyframes ws-halo {
    0% { transform: scale(0.28); opacity: 1; }
    100% { transform: scale(1); opacity: 0; }
  }
  @keyframes ws-diamond {
    0% { transform: rotate(0) scale(0.72); }
    50% { transform: rotate(180deg) scale(1); }
    100% { transform: rotate(360deg) scale(0.72); }
  }
  @keyframes ws-flip {
    0% { transform: perspective(var(--p)) rotateX(8deg) rotateY(0); }
    100% { transform: perspective(var(--p)) rotateX(8deg) rotateY(360deg); }
  }
  @keyframes ws-gyro-x {
    0% { transform: perspective(var(--p)) rotateY(24deg) rotateX(0); }
    100% { transform: perspective(var(--p)) rotateY(24deg) rotateX(360deg); }
  }
  @keyframes ws-gyro-y {
    0% { transform: perspective(var(--p)) rotateX(-18deg) rotateY(0); }
    100% { transform: perspective(var(--p)) rotateX(-18deg) rotateY(360deg); }
  }

  /* ── The three drawn in SVG ──────────────────────────────────────────── */

  /* One 48-unit drawing filling the box; every shape turns about its centre. */
  .working-spinner svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .working-spinner svg * {
    transform-box: view-box;
    transform-origin: 24px 24px;
  }

  /* tide — three drops pool into one bead, then swell out onto a ring. */
  .working-spinner[data-spinner='tide'] .as-ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.25;
    opacity: 0.3;
    animation: as-tide-ring 2.8s cubic-bezier(0.65, 0, 0.35, 1) infinite;
  }
  .working-spinner[data-spinner='tide'] .as-drop {
    --a: 0deg;
    fill: currentColor;
    transform: rotate(var(--a)) translateY(-13px) scale(0.72);
    animation: as-tide 2.8s cubic-bezier(0.65, 0, 0.35, 1) infinite;
  }
  .working-spinner[data-spinner='tide'] .as-drop:nth-child(3) {
    --a: 120deg;
    animation-delay: -0.16s;
  }
  .working-spinner[data-spinner='tide'] .as-drop:nth-child(4) {
    --a: 240deg;
    animation-delay: -0.32s;
  }

  /* ribbon — a figure-eight that twists edge-on into a line and back. */
  .working-spinner[data-spinner='ribbon'] .as-twist {
    animation: as-ribbon-twist 6s cubic-bezier(0.45, 0, 0.55, 1) infinite;
  }
  .working-spinner[data-spinner='ribbon'] path {
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .working-spinner[data-spinner='ribbon'] .as-track {
    opacity: 0.16;
  }
  .working-spinner[data-spinner='ribbon'] .as-strand {
    stroke-dasharray: 28 72;
    animation: as-ribbon-run 2.4s linear infinite;
  }
  .working-spinner[data-spinner='ribbon'] .as-strand-trail {
    opacity: 0.45;
    stroke-dasharray: 14 86;
    animation-delay: -1.2s;
  }

  /* bloom — petals fan open into a flower, then fold around into a bud. */
  .working-spinner[data-spinner='bloom'] .as-spin {
    animation: as-turn 14s linear infinite;
  }
  .working-spinner[data-spinner='bloom'] .as-petal {
    --a: 0deg;
    fill: currentColor;
    opacity: 0.42;
    transform: rotate(var(--a));
    animation: as-bloom 5s cubic-bezier(0.65, 0, 0.35, 1) infinite;
  }
  .working-spinner[data-spinner='bloom'] .as-petal:nth-child(2) { --a: 60deg; }
  .working-spinner[data-spinner='bloom'] .as-petal:nth-child(3) { --a: 120deg; }
  .working-spinner[data-spinner='bloom'] .as-petal:nth-child(4) { --a: 180deg; }
  .working-spinner[data-spinner='bloom'] .as-petal:nth-child(5) { --a: 240deg; }
  .working-spinner[data-spinner='bloom'] .as-petal:nth-child(6) { --a: 300deg; }
  .working-spinner[data-spinner='bloom'] .as-core {
    fill: currentColor;
  }

  @keyframes as-tide {
    0% { transform: rotate(var(--a)) translateY(0) scale(1.45); }
    50% { transform: rotate(calc(var(--a) + 180deg)) translateY(-13px) scale(0.72); }
    100% { transform: rotate(calc(var(--a) + 360deg)) translateY(0) scale(1.45); }
  }
  @keyframes as-tide-ring {
    0%, 100% { transform: scale(0.4); opacity: 0; }
    50% { transform: scale(1); opacity: 0.32; }
  }
  @keyframes as-ribbon-twist {
    0% { transform: rotate(0deg) scaleX(1); }
    50% { transform: rotate(90deg) scaleX(-1); }
    100% { transform: rotate(180deg) scaleX(1); }
  }
  @keyframes as-ribbon-run {
    to { stroke-dashoffset: -100; }
  }
  @keyframes as-bloom {
    0% { transform: rotate(0deg) scale(0.62); }
    50% { transform: rotate(var(--a)) scale(1); }
    100% { transform: rotate(360deg) scale(0.62); }
  }
  @keyframes as-turn {
    to { transform: rotate(360deg); }
  }

  @keyframes ws-tumble {
    0% { transform: perspective(var(--p)) rotateX(0) rotateY(0); }
    50% { transform: perspective(var(--p)) rotateX(180deg) rotateY(180deg); }
    100% { transform: perspective(var(--p)) rotateX(360deg) rotateY(360deg); }
  }
  @keyframes ws-facet-turn {
    0% { transform: perspective(var(--p)) rotateX(-24deg) rotateY(0); }
    100% { transform: perspective(var(--p)) rotateX(-24deg) rotateY(360deg); }
  }
  @keyframes ws-facet {
    0%, 100% { transform: rotateY(0deg) rotateZ(45deg) scale(0.78); }
    50% { transform: rotateY(calc((var(--i) - 1.5) * 45deg)) rotateZ(45deg) scale(1); }
  }

  /* Off screen is off. The attribute is the single switch: it stops the parts,
     the SVG shapes and the tumbling body alike, and it costs nothing to leave
     paused. It sits after every variant so their `animation` shorthands cannot
     reset it; the SVG line also carries `[data-spinner]` to outrank the
     `[data-spinner='…'] .as-…` selectors that animate those shapes. */
  .working-spinner i { animation-play-state: running; }
  .working-spinner[data-active='false'],
  .working-spinner[data-spinner][data-active='false'] i,
  .working-spinner[data-spinner][data-active='false'] svg * { animation-play-state: paused; }

  /* Asked for stillness, the spinner becomes a mark: the moving parts are gone
     and one quiet ring says the same thing. */
  .static-mark { display: none; }

  @media (prefers-reduced-motion: reduce) {
    /* The attribute is carried so this outranks the tumbling body above it,
       which names the same element and would otherwise keep turning. */
    .working-spinner[data-spinner] { animation: none; }
    .working-spinner i,
    .working-spinner svg { display: none; }
    .static-mark {
      display: block;
      inset: calc(var(--size) * 0.2);
      border: 2px solid currentColor;
      border-radius: 50%;
      opacity: 0.7;
    }
  }
</style>
