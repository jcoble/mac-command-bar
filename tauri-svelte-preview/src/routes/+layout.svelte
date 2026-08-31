<script lang="ts">
  import '../app.css';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';

  let { children } = $props();
</script>

<Tooltip.Provider delayDuration={0}>
  {@render children()}
</Tooltip.Provider>
