<script lang="ts">
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';

  let { size = 16 }: { size?: number } = $props();
</script>

<LoaderCircle {size} class="shrink-0 text-[var(--color-accent)]" data-testid="working-spinner" aria-hidden="true" />
