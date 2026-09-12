<script lang="ts">
  import { onDestroy } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import {
    checkForProviderUpdates,
    installProviderUpdates,
    providerUpdateState
  } from '$lib/shell/providerUpdateService.svelte';

  const owner = new AbortController();
  onDestroy(() => owner.abort());
</script>

<div class="flex flex-col items-end gap-1.5">
  <div class="flex items-center gap-2">
    <Button
      variant="secondary"
      size="sm"
      disabled={providerUpdateState.phase === 'checking' || providerUpdateState.phase === 'installing'}
      onclick={() => void checkForProviderUpdates(owner.signal)}
    >
      {providerUpdateState.phase === 'checking' ? 'Checking…' : 'Check now'}
    </Button>
    {#if providerUpdateState.phase === 'available'}
      <Button variant="secondary" size="sm" onclick={() => void installProviderUpdates(owner.signal)}>
        Install and restart
      </Button>
    {/if}
  </div>
  {#if providerUpdateState.message}
    <p class="max-w-[360px] text-right text-[12px] leading-[1.4] text-[var(--color-text-2)]">
      {providerUpdateState.message}
    </p>
  {/if}
</div>
