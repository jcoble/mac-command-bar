<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';

  import { Button } from '$lib/components/ui/button/index.js';
  import {
    checkForProviderUpdates,
    installProviderUpdates,
    providerUpdateState,
    restartProviders,
    type ProviderUpdateState
  } from '$lib/shell/providerUpdateService.svelte';

  let { profileId, machineName }: { profileId?: string; machineName?: string } = $props();
  const remoteState = $state<ProviderUpdateState>({ phase: 'idle', generation: 0, installingProvider: null, message: '', status: null });
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
    {#if updateState.status?.restartRequired}
      <Button variant="secondary" size="sm" disabled={updateState.phase === 'installing'} onclick={() => void restartProviders(updateState, profileId)}>
        {profileId ? 'Restart remote server' : 'Restart Assembly'}
      </Button>
    {/if}
  </div>
  {#if updateState.status}
    {#each updateState.status.providers as provider (provider.provider)}
      <div class="flex items-center gap-2 text-[12px]">
        {#if updateState.installingProvider === provider.provider}<LoaderCircle aria-label={`Downloading ${provider.provider} adapter`} class="size-4 animate-spin" />{/if}
        <span>{provider.provider === 'antigravity' ? 'Antigravity' : provider.provider}: {provider.currentVersion ?? 'Not installed'} → {provider.availableVersion}</span>
        {#if provider.updateAvailable}
          <Button variant="secondary" size="sm" disabled={updateState.phase === 'installing'} onclick={() => void installProviderUpdates(provider.provider, owner.signal, updateState, profileId)}>
            {updateState.installingProvider === provider.provider ? 'Installing…' : provider.currentVersion ? 'Update' : 'Install'}
          </Button>
        {/if}
      </div>
    {/each}
  {/if}
  {#if updateState.message}
    <p class="max-w-[360px] text-right text-[12px] leading-[1.4] text-[var(--color-text-2)]">
      {updateState.message}
    </p>
  {/if}
</div>
