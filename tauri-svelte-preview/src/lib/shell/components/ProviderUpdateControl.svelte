<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import {
    checkForProviderUpdates,
    installProviderUpdates,
    providerUpdateState,
    type ProviderUpdateState
  } from '$lib/shell/providerUpdateService.svelte';

  let { profileId, machineName }: { profileId?: string; machineName?: string } = $props();
  const remoteState = $state<ProviderUpdateState>({ phase: 'idle', generation: 0, message: '', status: null });
  const updateState = $derived(profileId ? remoteState : providerUpdateState);
  const owner = new AbortController();
  onMount(() => { if (profileId) void checkForProviderUpdates(owner.signal, updateState, profileId); });
  onDestroy(() => owner.abort());
</script>

<div class="flex flex-col items-end gap-1.5">
  {#if profileId}<span class="text-[12px]">Provider adapters · {machineName}</span>{/if}
  <div class="flex items-center gap-2">
    <Button
      variant="secondary"
      size="sm"
      disabled={updateState.phase === 'checking' || updateState.phase === 'installing'}
      onclick={() => void checkForProviderUpdates(owner.signal, updateState, profileId)}
    >
      {updateState.phase === 'checking' ? 'Checking…' : 'Check now'}
    </Button>
    {#if updateState.phase === 'available' || updateState.phase === 'restart'}
      <Button variant="secondary" size="sm" onclick={() => void installProviderUpdates(owner.signal, updateState, profileId)}>
        {updateState.phase === 'restart' ? (profileId ? 'Restart remote server' : 'Restart Assembly') : (profileId ? 'Install adapters' : 'Install and restart')}
      </Button>
    {/if}
  </div>
  {#if updateState.message}
    <p class="max-w-[360px] text-right text-[12px] leading-[1.4] text-[var(--color-text-2)]">
      {updateState.message}
    </p>
  {/if}
</div>
