<script lang="ts">
  /**
   * The small figure that means "the agent is working".
   *
   * Which of the ten it is comes from the seed, so one turn keeps one spinner
   * for its whole life and the next turn brings a different one. The drawing is
   * a handful of empty spans that the stylesheet below shapes and moves; only
   * transform and opacity are animated, so a frame costs the compositor a
   * matrix and nothing else.
   *
   * Nothing here loops at rest. The caller mounts this only while a turn is
   * running, so a finished turn removes the element and its animations with it,
   * and while it is mounted it still pauses whenever it scrolls off screen —
   * an animation nobody can see costs exactly what a visible one costs.
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
</script>

<span
  bind:this={host}
  class="working-spinner"
  data-testid="working-spinner"
  data-spinner={spinner.id}
  data-active={onScreen}
  style={`--size:${size}px`}
  aria-hidden="true"
>
  {#each { length: spinner.parts } as _, index (index)}
    <i style={`--i:${index}`}></i>
  {/each}
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

  /* Off screen is off. The attribute is the single switch: it stops the parts
     and the tumbling body alike, and it costs nothing to leave paused. */
  .working-spinner i { animation-play-state: running; }
  .working-spinner[data-active='false'],
  .working-spinner[data-active='false'] i { animation-play-state: paused; }

  /* ── The seven flat ones ─────────────────────────────────────────────── */

  /* arc — a thin ring lit along one quarter, turning. */
  .working-spinner[data-spinner='arc'] i {
    inset: 0;
    border: 2px solid color-mix(in srgb, currentColor 22%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: ws-spin 0.8s linear infinite;
  }

  /* orbit — one dot travelling a track it never leaves. */
  .working-spinner[data-spinner='orbit'] i:nth-child(1) {
    inset: 0;
    border: 1.5px solid color-mix(in srgb, currentColor 26%, transparent);
    border-radius: 50%;
  }
  .working-spinner[data-spinner='orbit'] i:nth-child(2) {
    inset: 0;
    animation: ws-spin 1.1s linear infinite;
  }
  .working-spinner[data-spinner='orbit'] i:nth-child(2)::before {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    width: calc(var(--size) * 0.3);
    height: calc(var(--size) * 0.3);
    margin-left: calc(var(--size) * -0.15);
    border-radius: 50%;
    background: currentColor;
  }

  /* wave — three dots riding the same swell a beat apart. */
  .working-spinner[data-spinner='wave'] i {
    top: 50%;
    width: calc(var(--size) * 0.22);
    height: calc(var(--size) * 0.22);
    margin-top: calc(var(--size) * -0.11);
    border-radius: 50%;
    background: currentColor;
    animation: ws-wave 0.9s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.13s);
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
    animation: ws-bars 0.78s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.11s);
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
    animation: ws-halo 1.4s ease-out infinite;
    animation-delay: calc(var(--i) * 0.7s);
  }

  /* diamond — a square that turns and breathes. */
  .working-spinner[data-spinner='diamond'] i {
    inset: calc(var(--size) * 0.16);
    border: 2px solid currentColor;
    border-radius: 2px;
    animation: ws-diamond 1.1s ease-in-out infinite;
  }

  /* comet — a head and two fading followers on the same circle. */
  .working-spinner[data-spinner='comet'] i {
    inset: 0;
    animation: ws-spin 0.95s linear infinite;
    animation-delay: calc(var(--i) * -0.13s);
  }
  .working-spinner[data-spinner='comet'] i::before {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    width: calc(var(--size) * 0.3);
    height: calc(var(--size) * 0.3);
    margin-left: calc(var(--size) * -0.15);
    border-radius: 50%;
    background: currentColor;
    opacity: calc(1 - var(--i) * 0.28);
    transform: scale(calc(1 - var(--i) * 0.2));
  }

  /* ── The three with depth ────────────────────────────────────────────── */

  /* cube — six faces on a body that tumbles on two axes. The body is this
     element itself, which is why the perspective rides in the transform. */
  .working-spinner[data-spinner='cube'] {
    transform-style: preserve-3d;
    animation: ws-tumble 2.6s linear infinite;
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

  /* disc — a coin turning edge-on and back. */
  .working-spinner[data-spinner='disc'] { transform-style: preserve-3d; }
  .working-spinner[data-spinner='disc'] i {
    inset: calc(var(--size) * 0.06);
    border: 2px solid currentColor;
    border-radius: 50%;
    background: color-mix(in srgb, currentColor 14%, transparent);
    animation: ws-flip 1.5s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  }

  /* gyro — two tilted rings turning about axes at right angles. */
  .working-spinner[data-spinner='gyro'] { transform-style: preserve-3d; }
  .working-spinner[data-spinner='gyro'] i {
    inset: 0;
    border: 1.5px solid currentColor;
    border-radius: 50%;
  }
  .working-spinner[data-spinner='gyro'] i:nth-child(1) { animation: ws-gyro-x 1.3s linear infinite; }
  .working-spinner[data-spinner='gyro'] i:nth-child(2) {
    inset: calc(var(--size) * 0.18);
    border-color: color-mix(in srgb, currentColor 55%, transparent);
    animation: ws-gyro-y 1.1s linear infinite;
  }

  @keyframes ws-spin { to { transform: rotate(360deg); } }
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
  @keyframes ws-tumble {
    0% { transform: perspective(var(--p)) rotateX(0) rotateY(0); }
    50% { transform: perspective(var(--p)) rotateX(180deg) rotateY(180deg); }
    100% { transform: perspective(var(--p)) rotateX(360deg) rotateY(360deg); }
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

  /* Asked for stillness, the spinner becomes a mark: the moving parts are gone
     and one quiet ring says the same thing. */
  .static-mark { display: none; }

  @media (prefers-reduced-motion: reduce) {
    /* The attribute is carried so this outranks the tumbling body above it,
       which names the same element and would otherwise keep turning. */
    .working-spinner[data-spinner] { animation: none; }
    .working-spinner i { display: none; }
    .static-mark {
      display: block;
      inset: calc(var(--size) * 0.2);
      border: 2px solid currentColor;
      border-radius: 50%;
      opacity: 0.7;
    }
  }
</style>
