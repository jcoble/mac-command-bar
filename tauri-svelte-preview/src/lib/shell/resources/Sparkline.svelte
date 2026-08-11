<!--
  Sparkline.svelte — the last few minutes of one row, drawn small.

  All the arithmetic lives in `resourceSparkline.ts`; this file only turns the
  path into an element. The line has no axes and no labels on purpose: it is
  there to be read at a glance beside the number, and the number is already
  next to it.
-->
<script lang="ts">
  import { describeSparkline, sparklinePath } from './resourceSparkline';

  interface Props {
    values: readonly number[];
    /** Which reading the line is showing, for the description a reader hears. */
    unit: 'cpu' | 'memory';
    width?: number;
    height?: number;
  }

  let { values, unit, width = 62, height = 16 }: Props = $props();

  const path = $derived(sparklinePath(values, { width, height }));
  const description = $derived(describeSparkline(values, unit));
</script>

{#if path}
  <svg
    class="sparkline"
    class:memory={unit === 'memory'}
    viewBox={`0 0 ${width} ${height}`}
    width={width}
    height={height}
    role="img"
    aria-label={description}
    preserveAspectRatio="none"
  >
    <path d={path} fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round" />
  </svg>
{:else}
  <span class="sparkline-empty" aria-hidden="true"></span>
{/if}

<style>
  .sparkline {
    display: block;
    overflow: visible;
    color: var(--color-text-3);
  }
  .sparkline.memory {
    color: var(--color-accent);
  }
  .sparkline-empty {
    display: block;
    width: 62px;
    height: 16px;
  }
</style>
