<!-- Build latest (dev builds only): pull main, build, deploy the backend to every saved
     machine, then restart. The run itself lives in buildLatestService. -->
<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import WorkingSpinner from '$lib/shell/components/feedback/WorkingSpinner.svelte';
  import { buildLatestState, startBuildLatest, stopBuildLatest } from '$lib/shell/buildLatestService.svelte';
  import { openPathFromTauri } from '$lib/tauriSource';

</script>

<div class="flex flex-col items-end gap-1.5">
  <div class="flex items-center gap-2">
    {#if buildLatestState.logPath}
      <Button variant="ghost" size="sm" onclick={() => void openPathFromTauri(buildLatestState.logPath)}>Open log</Button>
    {/if}
    {#if buildLatestState.phase === 'running'}
      <Button variant="secondary" size="sm" onclick={() => void stopBuildLatest()}><WorkingSpinner size={12} />Stop</Button>
    {:else}
      <Button variant="secondary" size="sm" disabled={buildLatestState.phase === 'restarting'} onclick={() => void startBuildLatest()}>
        {buildLatestState.phase === 'restarting' ? 'Restarting…' : 'Build latest'}
      </Button>
    {/if}
  </div>
  {#if buildLatestState.phase === 'running'}
    <p class="max-w-[260px] truncate text-right text-[12px] leading-[1.4] text-[var(--color-text-2)]" title={buildLatestState.text}>
      Step {buildLatestState.step} of {buildLatestState.total}: {buildLatestState.text}
    </p>
  {:else if buildLatestState.phase === 'restarting'}
    <p class="max-w-[260px] text-right text-[12px] leading-[1.4] text-[var(--color-text-2)]">
      {buildLatestState.text}. The new build opens once Assembly quits.
    </p>
  {:else if buildLatestState.phase === 'failed'}
    <p class="max-w-[260px] text-right text-[12px] leading-[1.4] text-destructive [overflow-wrap:anywhere]" role="alert">
      {buildLatestState.error}
    </p>
  {/if}
</div>
