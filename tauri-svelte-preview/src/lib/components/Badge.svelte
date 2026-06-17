<script lang="ts">
  import type { Snippet } from 'svelte';
  import { toneStyle, type Tone } from './tone.js';

  interface Props {
    tone?: Tone;
    children?: Snippet;
  }

  let { tone = 'neutral', children }: Props = $props();
</script>

<span
  class="badge"
  data-tone={tone}
  style={toneStyle(tone)}
>
  {@render children?.()}
</span>

<style>
  .badge {
    display:         inline-flex;
    align-items:     center;
    justify-content: center;

    /* Smaller and more compact than Chip */
    font-size:    var(--text-xs);
    font-weight:  var(--weight-semibold);
    /* Monospace numerals so counts don't shift width */
    font-variant-numeric: tabular-nums;
    line-height:  1;
    min-width:    1.35em;    /* keeps single-digit counts circular */
    padding:      2px calc(var(--space-1) + 1px);
    border-radius: var(--radius-pill);
    white-space:  nowrap;

    /* tone-driven */
    color:            var(--_tone-color);
    background-color: var(--_tone-bg);
  }
</style>
