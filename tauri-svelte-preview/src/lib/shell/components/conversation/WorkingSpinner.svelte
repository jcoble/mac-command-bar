<script lang="ts">
  /**
   * The small figure that means "the agent is working".
   *
   * Which of the three it is comes from the seed, so one turn keeps one spinner
   * for its whole life and the next turn brings a different one. The drawing is
   * a handful of empty spans that the stylesheet below shapes and moves; only
   * transform and opacity are animated, so a frame costs the compositor a
   * matrix and nothing else.
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

  /* ribbon — eight plates ringed into a Möbius band, seen from above. Each
     plate turns half over about the band's own direction, a step behind its
     neighbour, so the twist travels round the loop while the loop stays put.
     A half turn of a plate looks like no turn at all, which is the seam. */
  .working-spinner[data-spinner='ribbon'] {
    transform-style: preserve-3d;
    perspective: var(--p);
    --r: calc(var(--size) * 0.28);
  }
  .working-spinner[data-spinner='ribbon'] i {
    inset: calc(var(--size) * 0.35) calc(var(--size) * 0.38);
    border: 1px solid currentColor;
    background: color-mix(in srgb, currentColor 30%, transparent);
    animation: ws-ribbon 5.4s linear infinite;
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
  @keyframes ws-ribbon {
    0% {
      transform: rotateZ(-24deg) rotateX(52deg) rotateY(calc(var(--i) * 45deg))
        translateZ(var(--r)) rotateX(calc(var(--i) * 22.5deg));
    }
    100% {
      transform: rotateZ(-24deg) rotateX(52deg) rotateY(calc(var(--i) * 45deg))
        translateZ(var(--r)) rotateX(calc(var(--i) * 22.5deg + 180deg));
    }
  }

  /* Off screen is off. The attribute is the single switch: it stops the parts
     and the tumbling body alike, and it costs nothing to leave paused. It sits
     after every variant so their `animation` shorthands cannot reset it. */
  .working-spinner i { animation-play-state: running; }
  .working-spinner[data-active='false'],
  .working-spinner[data-active='false'] i { animation-play-state: paused; }

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
