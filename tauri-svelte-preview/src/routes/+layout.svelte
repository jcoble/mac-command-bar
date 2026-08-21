<script lang="ts">
  import '../app.css';

  if (typeof window !== 'undefined' && typeof window.requestIdleCallback !== 'function') {
    window.requestIdleCallback = (callback) =>
      window.setTimeout(() => callback({ didTimeout: true, timeRemaining: () => 0 }), 1);
  }

  let { children } = $props();
</script>

{@render children()}
