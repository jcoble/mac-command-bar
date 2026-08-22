<script lang="ts">
  import '../app.css';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';

  if (typeof window !== 'undefined' && typeof window.requestIdleCallback !== 'function') {
    window.requestIdleCallback = (callback) =>
      window.setTimeout(() => callback({ didTimeout: true, timeRemaining: () => 0 }), 1);
    window.cancelIdleCallback = (handle) => window.clearTimeout(handle);
  }

  let { children } = $props();
</script>

<Tooltip.Provider delayDuration={400}>
  {@render children()}
</Tooltip.Provider>
