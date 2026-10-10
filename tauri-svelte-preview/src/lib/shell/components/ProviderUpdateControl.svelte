<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import WorkingSpinner from '$lib/shell/components/conversation/WorkingSpinner.svelte';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import {
    checkForProviderUpdates,
    installProviderUpdates,
    providerUpdateState,
    providerVersionLine,
    restartProviders,
    restartReconnectStep,
    sessionsEndedByRestart,
    startRestartCountdown,
    stepRestartCountdown,
    type ProviderUpdateState,
    type RestartCountdown,
    type RestartReconnect
  } from '$lib/shell/providerUpdateService.svelte';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';

  let { profileId, machineName }: { profileId?: string; machineName?: string } = $props();
  const remoteState = $state<ProviderUpdateState>({ phase: 'idle', generation: 0, installingProvider: null, message: '', status: null });
  const updateState = $derived(profileId ? remoteState : providerUpdateState);
  const owner = new AbortController();
  onMount(() => { if (profileId) void checkForProviderUpdates(owner.signal, updateState, profileId); });
  onDestroy(() => owner.abort());

  // A remote restart ends the sessions running there, so it warns and counts down first.
  let countdown = $state<RestartCountdown | null>(null);
  const counting = $derived(countdown !== null);
  const endingSessions = $derived(profileId ? sessionsEndedByRestart(rail.owned, profileId) : []);

  function requestRestart() {
    if (!profileId || endingSessions.length === 0) void restartProviders(updateState, profileId);
    else countdown = startRestartCountdown();
  }

  function stepCountdown(event: 'tick' | 'cancel' | 'restart-now') {
    if (!countdown) return;
    const next = stepRestartCountdown(countdown, event);
    countdown = next.phase === 'counting' ? next : null;
    if (next.phase === 'finished') void restartProviders(updateState, profileId);
  }

  $effect(() => {
    if (!counting) return;
    // One one-second interval while the warning counts down; cancel, finishing and unmounting clear it.
    const timer = window.setInterval(() => stepCountdown('tick'), 1000);
    return () => window.clearInterval(timer);
  });

  // After a restart, re-check (as Check now does) once the connection drops and returns.
  let reconnect: RestartReconnect = 'waiting-for-drop';
  $effect(() => {
    if (!profileId || updateState.phase !== 'reconnecting') { reconnect = 'waiting-for-drop'; return; }
    reconnect = restartReconnectStep(reconnect, rail.remoteConnections[profileId]);
    if (reconnect === 'recheck') void checkForProviderUpdates(owner.signal, updateState, profileId);
  });
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
      {#if updateState.phase === 'checking'}<WorkingSpinner size={12} />Checking…{:else}Check now{/if}
    </Button>
    {#if updateState.status?.restartRequired}
      <Button variant="secondary" size="sm" disabled={updateState.phase === 'installing'} onclick={requestRestart}>
        {profileId ? 'Restart remote server' : 'Restart Assembly'}
      </Button>
    {/if}
  </div>
  {#if updateState.status}
    {#each updateState.status.providers as provider (provider.provider)}
      <div class="flex items-center gap-2 text-[12px]">
        {#if updateState.installingProvider === provider.provider}<span class="inline-flex" role="status" aria-label={`Downloading ${provider.provider} adapter`}><WorkingSpinner /></span>{/if}
        <span>{providerVersionLine(provider)}</span>
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

<AlertDialog.Root open={countdown !== null} onOpenChange={(open) => { if (!open) stepCountdown('cancel'); }}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Restart the remote server on {machineName}?</AlertDialog.Title>
      <AlertDialog.Description>
        Restarting ends these sessions running on {machineName}. Restarting in {countdown?.remaining ?? 0} seconds.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <ul class="m-0 flex list-disc flex-col gap-1 pl-5 text-[13px]">
      {#each endingSessions as title, index (index)}<li class="break-words">{title}</li>{/each}
    </ul>
    <AlertDialog.Footer>
      <AlertDialog.Cancel size="sm">Cancel</AlertDialog.Cancel>
      <AlertDialog.Action size="sm" variant="destructive" onclick={() => stepCountdown('restart-now')}>Restart now</AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
