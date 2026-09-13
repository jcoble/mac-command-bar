<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { hydrateOwned, rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions';
  import {
    connectRemoteAssemblyFromTauri, disconnectRemoteAssemblyFromTauri,
    readRemoteAssemblyEnvironmentFromTauri, removeRemoteAssemblyProfileFromTauri,
    type RemoteAssemblyEnvironment, type RemoteAssemblyProfile
  } from '$lib/tauriSource';

  let { initialProfile = null, onConnected, onChange, onClose }: {
    initialProfile?: RemoteAssemblyProfile | null;
    onConnected?: (profile: RemoteAssemblyProfile) => void;
    onChange?: (environment: RemoteAssemblyEnvironment) => void;
    onClose?: () => void;
  } = $props();
  const emptyProfile = (): RemoteAssemblyProfile => ({
    id: crypto.randomUUID(), name: '', sshTarget: '', sourceRoot: '', defaultCwd: ''
  });
  let profile = $state<RemoteAssemblyProfile>(emptyProfile());
  let environment = $state<RemoteAssemblyEnvironment>({ profiles: [], readyProfileIds: [] });
  let busy = $state(false);
  let status = $state('');
  let error = $state('');
  let owner = $state<AbortController | null>(null);
  let mounted = false;

  function apply(next: RemoteAssemblyEnvironment) {
    environment = next;
    onChange?.(next);
  }

  onMount(() => {
    mounted = true;
    profile = initialProfile ? { ...initialProfile } : emptyProfile();
    void readRemoteAssemblyEnvironmentFromTauri().then((next) => {
      if (mounted) apply(next);
    }).catch((reason: unknown) => { if (mounted) error = String(reason); });
    return () => { mounted = false; owner?.abort(); };
  });

  async function connect(selected = profile) {
    if (busy) return;
    const attempt = new AbortController();
    owner = attempt;
    busy = true;
    error = '';
    status = 'Connecting…';
    try {
      const result = await connectRemoteAssemblyFromTauri(selected, attempt.signal, (message) => {
        if (mounted && !attempt.signal.aborted) status = message;
      });
      if (!mounted || attempt.signal.aborted) return;
      const ids = new Set(result.sessions.map((session) => session.ownedId));
      hydrateOwned([...rail.owned.filter((session) => !ids.has(session.ownedId)),
        ...result.sessions.map(ownedSessionFromBackend)]);
      const next = await readRemoteAssemblyEnvironmentFromTauri();
      if (!mounted || attempt.signal.aborted) return;
      apply(next);
      profile = { ...result.profile };
      status = 'Connected. Your conversations are available in Sessions.';
      onConnected?.(result.profile);
    } catch (reason) {
      if (mounted) {
        status = '';
        error = attempt.signal.aborted ? 'Connection cancelled.' : String(reason);
      }
    } finally {
      if (owner === attempt) { owner = null; busy = false; }
    }
  }

  async function changeConnection(selected: RemoteAssemblyProfile, remove = false) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const next = remove
        ? await removeRemoteAssemblyProfileFromTauri(selected.id)
        : await disconnectRemoteAssemblyFromTauri(selected.id);
      if (!mounted) return;
      apply(next);
      status = remove ? 'Saved connection removed. The backend is still installed.' : 'Disconnected. The connection is saved for next time.';
      if (remove && profile.id === selected.id) profile = emptyProfile();
    } catch (reason) { if (mounted) error = String(reason); }
    finally { if (mounted) busy = false; }
  }
</script>

<div class="remote-connections">
  <p>Connect to an installed Assembly backend to resume its conversations. Choose a project when starting a new chat.</p>
  {#each environment.profiles as saved (saved.id)}
    <div class="connection-row">
      <span><strong>{saved.name}</strong><small>{saved.sshTarget} · {environment.readyProfileIds.includes(saved.id) ? 'Connected' : 'Not connected'}</small></span>
      {#if environment.readyProfileIds.includes(saved.id)}
        <Button variant="secondary" size="sm" disabled={busy} onclick={() => void changeConnection(saved)}>Disconnect</Button>
      {:else}
        <Button variant="secondary" size="sm" disabled={busy} onclick={() => void connect(saved)}>Connect</Button>
      {/if}
      <Button variant="ghost" size="xs" disabled={busy} onclick={() => { profile = { ...saved }; status = ''; error = ''; }}>Edit</Button>
      <Button variant="ghost" size="xs" disabled={busy} onclick={() => void changeConnection(saved, true)}>Remove</Button>
    </div>
  {/each}
  <div class="connection-heading"><strong>{environment.profiles.some((saved) => saved.id === profile.id) ? 'Edit connection' : 'Add a remote machine'}</strong>
    <Button variant="ghost" size="xs" disabled={busy} onclick={() => { profile = emptyProfile(); status = ''; error = ''; }}>Add new</Button>
  </div>
  <label>Machine name<Input disabled={busy} bind:value={profile.name} placeholder="Agent Workbox" autocomplete="off" /></label>
  <label>SSH destination<Input disabled={busy} bind:value={profile.sshTarget} placeholder="user@hostname or SSH alias" autocomplete="off" /></label>
  {#if status}<p role="status">{status}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  <div class="connection-actions">
    {#if owner}<Button variant="ghost" size="sm" onclick={() => owner?.abort()}>Cancel connection</Button>
    {:else if onClose}<Button variant="ghost" size="sm" onclick={onClose}>Done</Button>{/if}
    <Button size="sm" disabled={busy || !profile.name.trim() || !profile.sshTarget.trim()} onclick={() => void connect()}>{busy ? 'Please wait…' : 'Connect'}</Button>
  </div>
</div>

<style>
  .remote-connections { display: flex; flex-direction: column; gap: 14px; padding: 20px; text-align: left; color: var(--color-text); }
  p, small { margin: 0; color: var(--color-text-3); font-size: 12px; }
  .connection-row, .connection-heading, .connection-actions { display: flex; align-items: center; gap: 8px; }
  .connection-row { padding: 12px 0; border-bottom: 1px solid var(--color-border); }
  .connection-row > span { display: flex; flex: 1; min-width: 0; flex-direction: column; }
  small { overflow-wrap: anywhere; }
  .connection-heading { justify-content: space-between; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--color-text-3); }
  .connection-actions { justify-content: flex-end; }
</style>
