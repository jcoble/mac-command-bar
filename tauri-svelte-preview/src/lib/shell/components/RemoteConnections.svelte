<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { hydrateOwned, rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions';
  import {
    connectRemoteAssemblyFromTauri, disconnectRemoteAssemblyFromTauri,
    installRemoteAssemblyFromTauri, readRemoteAssemblyEnvironmentFromTauri,
    removeRemoteAssemblyProfileFromTauri,
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
  let editing = $state<RemoteAssemblyProfile | null>(null);
  let environment = $state<RemoteAssemblyEnvironment>({ profiles: [], readyProfileIds: [] });
  let busy = $state(false);
  let status = $state('');
  let error = $state('');
  let operationArea = $state<'saved' | 'install'>('saved');
  let owner = $state<AbortController | null>(null);
  let mounted = false;

  function apply(next: RemoteAssemblyEnvironment) {
    environment = next;
    onChange?.(next);
  }

  onMount(() => {
    mounted = true;
    if (initialProfile) editing = { ...initialProfile };
    void readRemoteAssemblyEnvironmentFromTauri().then((next) => {
      if (mounted) apply(next);
    }).catch((reason: unknown) => { if (mounted) error = String(reason); });
    return () => { mounted = false; owner?.abort(); };
  });

  async function runConnection(selected: RemoteAssemblyProfile, install = false, area: 'saved' | 'install' = 'saved') {
    if (busy) return;
    const attempt = new AbortController();
    owner = attempt;
    busy = true;
    operationArea = area;
    error = '';
    status = install ? 'Preparing installation…' : 'Connecting…';
    try {
      const operation = install ? installRemoteAssemblyFromTauri : connectRemoteAssemblyFromTauri;
      const result = await operation(selected, attempt.signal, (message) => {
        if (mounted && !attempt.signal.aborted) status = message;
      });
      if (!mounted || attempt.signal.aborted) return;
      const ids = new Set(result.sessions.map((session) => session.ownedId));
      hydrateOwned([...rail.owned.filter((session) => !ids.has(session.ownedId)),
        ...result.sessions.map(ownedSessionFromBackend)]);
      const next = await readRemoteAssemblyEnvironmentFromTauri();
      if (!mounted || attempt.signal.aborted) return;
      apply(next);
      if (editing?.id === result.profile.id) editing = null;
      if (profile.id === result.profile.id) profile = emptyProfile();
      status = install
        ? 'Installed and connected. Your conversations are available in Sessions.'
        : 'Connected. Your conversations are available in Sessions.';
      onConnected?.(result.profile);
    } catch (reason) {
      if (mounted) {
        status = '';
        error = attempt.signal.aborted ? `${install ? 'Installation' : 'Connection'} cancelled.` : String(reason);
      }
    } finally {
      if (owner === attempt) { owner = null; busy = false; }
    }
  }

  async function changeConnection(selected: RemoteAssemblyProfile, remove = false) {
    if (busy) return;
    busy = true;
    operationArea = 'saved';
    status = '';
    error = '';
    try {
      const next = remove
        ? await removeRemoteAssemblyProfileFromTauri(selected.id)
        : await disconnectRemoteAssemblyFromTauri(selected.id);
      if (!mounted) return;
      apply(next);
      status = remove ? 'Saved connection removed. The backend is still installed.' : 'Disconnected. The connection is saved for next time.';
      if (remove && editing?.id === selected.id) editing = null;
    } catch (reason) { if (mounted) error = String(reason); }
    finally { if (mounted) busy = false; }
  }
</script>

<div class="remote-connections">
  <section>
    <div class="section-heading"><strong>Saved machines</strong><small>Connect to an installed backend or update it from the latest signed release.</small></div>
    {#if environment.profiles.length === 0}<p>No remote machines saved yet.</p>{/if}
    {#each environment.profiles as saved (saved.id)}
      <div class="connection-row">
        <span><strong>{saved.name}</strong><small><span class="status-dot" class:connected={environment.readyProfileIds.includes(saved.id)} aria-hidden="true"></span>{saved.sshTarget} · {environment.readyProfileIds.includes(saved.id) ? 'Connected' : 'Disconnected'}</small></span>
        {#if environment.readyProfileIds.includes(saved.id)}
          <Button variant="secondary" size="sm" disabled={busy} onclick={() => void changeConnection(saved)}>Disconnect</Button>
        {:else}
          <Button variant="secondary" size="sm" disabled={busy} onclick={() => void runConnection(saved)}>Connect</Button>
        {/if}
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => void runConnection(saved, true, 'saved')}>Update</Button>
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => { editing = { ...saved }; status = ''; error = ''; }}>Edit</Button>
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => void changeConnection(saved, true)}>Remove</Button>
      </div>
    {/each}
    {#if editing}
      <div class="edit-form">
        <div class="connection-heading"><strong>Edit saved machine</strong><Button variant="ghost" size="xs" disabled={busy} onclick={() => editing = null}>Cancel</Button></div>
        <label>Machine name<Input disabled={busy} bind:value={editing.name} autocomplete="off" /></label>
        <label>SSH destination<Input disabled={busy} bind:value={editing.sshTarget} autocomplete="off" /></label>
        <div class="connection-actions"><Button size="sm" disabled={busy || !editing.name.trim() || !editing.sshTarget.trim()} onclick={() => editing && void runConnection(editing)}>Save and connect</Button></div>
      </div>
    {/if}
    {#if operationArea === 'saved' && status}<p role="status">{status}</p>{/if}
    {#if operationArea === 'saved' && error}<p role="alert">{error}</p>{/if}
  </section>
  <section class="install-section">
    <div class="section-heading"><strong>Install a remote machine</strong><small>Uses your signed-in GitHub CLI to download the latest signed Linux x86-64 backend, installs it over SSH, verifies its loopback-only service, then connects.</small></div>
    <label>Machine name<Input disabled={busy} bind:value={profile.name} placeholder="Agent Workbox" autocomplete="off" /></label>
    <label>SSH destination<Input disabled={busy} bind:value={profile.sshTarget} placeholder="user@hostname or SSH alias" autocomplete="off" /></label>
    <div class="connection-actions"><Button size="sm" disabled={busy || !profile.name.trim() || !profile.sshTarget.trim()} onclick={() => void runConnection(profile, true, 'install')}>{busy && operationArea === 'install' ? 'Please wait…' : 'Install and connect'}</Button></div>
    {#if operationArea === 'install' && status}<p role="status">{status}</p>{/if}
    {#if operationArea === 'install' && error}<p role="alert">{error}</p>{/if}
  </section>
  <div class="connection-actions">
    {#if owner}<Button variant="ghost" size="sm" onclick={() => owner?.abort()}>Cancel current operation</Button>
    {:else if onClose}<Button variant="ghost" size="sm" onclick={onClose}>Done</Button>{/if}
  </div>
</div>

<style>
  .remote-connections { display: flex; flex-direction: column; gap: 18px; padding: 20px; text-align: left; color: var(--color-text); }
  section { display: flex; flex-direction: column; gap: 12px; }
  .install-section { padding-top: 18px; border-top: 1px solid var(--color-border); }
  .section-heading { display: flex; flex-direction: column; gap: 3px; }
  p, small { margin: 0; color: var(--color-text-3); font-size: 12px; }
  .connection-row, .connection-heading, .connection-actions { display: flex; align-items: center; gap: 8px; }
  .connection-row { padding: 12px 0; border-bottom: 1px solid var(--color-border); }
  .connection-row > span { display: flex; flex: 1; min-width: 0; flex-direction: column; }
  small { overflow-wrap: anywhere; }
  .status-dot { display: inline-block; width: 7px; height: 7px; margin-right: 6px; border-radius: 50%; background: var(--color-text-3); }
  .status-dot.connected { background: var(--color-accent); }
  .connection-heading { justify-content: space-between; }
  .edit-form { display: flex; flex-direction: column; gap: 10px; padding: 12px 0 4px; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--color-text-3); }
  .connection-actions { justify-content: flex-end; }
</style>
