<script lang="ts">
  import { onMount } from 'svelte';
  import ProviderUpdateControl from './ProviderUpdateControl.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { hydrateOwned, rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import { ownedSessionFromBackend } from '$lib/shell/ownedSessions';
  import {
    connectRemoteAssemblyFromTauri, disconnectRemoteAssemblyFromTauri,
    installRemoteAssemblyFromTauri, readRemoteAssemblyEnvironmentFromTauri,
    readRemoteBackendStatusesFromTauri, removeRemoteAssemblyProfileFromTauri,
    uninstallRemoteAssemblyFromTauri,
    type RemoteAssemblyEnvironment, type RemoteAssemblyProfile, type RemoteBackendProfileStatus
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
  let backends = $state<Record<string, RemoteBackendProfileStatus>>({});
  let confirmingUninstallId = $state<string | null>(null);
  let mounted = false;

  function apply(next: RemoteAssemblyEnvironment) {
    environment = next;
    onChange?.(next);
  }

  async function readBackends(profiles: RemoteAssemblyProfile[]) {
    try {
      const statuses = await readRemoteBackendStatusesFromTauri(profiles);
      if (mounted) backends = Object.fromEntries(statuses.map((entry) => [entry.profileId, entry]));
    } catch {
      if (mounted) backends = {};
    }
  }

  function backendLabel(profileId: string): string {
    const backend = backends[profileId];
    if (!backend) return 'Backend version unknown';
    if (!backend.installed) return 'Backend not installed';
    const installed = backend.installedVersion ?? 'unknown version';
    if (backend.updateAvailable === null) return `Backend ${installed}`;
    return backend.updateAvailable
      ? `Backend ${installed} · update to ${backend.latestVersion} available`
      : `Backend ${installed} · up to date`;
  }

  onMount(() => {
    mounted = true;
    if (initialProfile) editing = { ...initialProfile };
    void readRemoteAssemblyEnvironmentFromTauri().then(async (next) => {
      if (!mounted) return;
      apply(next);
      await readBackends(next.profiles);
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
      const replacedProfileIds = new Set([result.profile.id, result.replacedProfileId]);
      hydrateOwned([...rail.owned.filter((session) => !replacedProfileIds.has(session.remoteProfileId ?? '')),
        ...result.sessions.map(ownedSessionFromBackend)]);
      const next = await readRemoteAssemblyEnvironmentFromTauri();
      if (!mounted || attempt.signal.aborted) return;
      apply(next);
      await readBackends(next.profiles);
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

  async function uninstallBackend(selected: RemoteAssemblyProfile, deleteData: boolean) {
    if (busy) return;
    busy = true;
    operationArea = 'saved';
    status = deleteData ? 'Uninstalling and deleting data…' : 'Uninstalling…';
    error = '';
    try {
      const result = await uninstallRemoteAssemblyFromTauri(selected, deleteData);
      if (!mounted) return;
      if (deleteData) {
        const replacedProfileIds = new Set([selected.id, result.replacedProfileId]);
        hydrateOwned(rail.owned.filter((session) => !replacedProfileIds.has(session.remoteProfileId ?? '')));
      }
      apply(result.environment);
      confirmingUninstallId = null;
      status = deleteData
        ? `Backend uninstalled from ${selected.name} and its stored data deleted. The saved connection is kept, and project files were left alone.`
        : `Backend uninstalled from ${selected.name}. Its stored data was kept, and the saved connection is kept.`;
      await readBackends(result.environment.profiles);
    } catch (reason) { if (mounted) { status = ''; error = String(reason); } }
    finally { if (mounted) busy = false; }
  }
</script>

<div class="remote-connections">
  <section>
    <div class="section-heading"><strong>Saved machines</strong><small>Connect to an installed backend or update it from the latest signed release.</small></div>
    {#if environment.profiles.length === 0}<p>No remote machines saved yet.</p>{/if}
    {#each environment.profiles as saved (saved.id)}
      {@const backend = backends[saved.id]}
      <div class="connection-row">
        <span><strong>{saved.name}</strong><small><span class="status-dot" class:connected={environment.readyProfileIds.includes(saved.id)} aria-hidden="true"></span>{saved.sshTarget} · {environment.readyProfileIds.includes(saved.id) ? 'Connected' : 'Disconnected'} · {backendLabel(saved.id)}</small></span>
        {#if environment.readyProfileIds.includes(saved.id)}
          <Button variant="secondary" size="sm" disabled={busy} onclick={() => void changeConnection(saved)}>Disconnect</Button>
        {:else}
          <Button variant="secondary" size="sm" disabled={busy} onclick={() => void runConnection(saved)}>Connect</Button>
        {/if}
        {#if backend && !backend.installed}
          <Button variant="ghost" size="xs" disabled={busy} onclick={() => void runConnection(saved, true, 'saved')}>Install</Button>
        {:else if backend?.updateAvailable === true}
          <Button variant="ghost" size="xs" disabled={busy} onclick={() => void runConnection(saved, true, 'saved')}>Update</Button>
        {/if}
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => { editing = { ...saved }; status = ''; error = ''; }}>Edit</Button>
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => { confirmingUninstallId = saved.id; status = ''; error = ''; }}>Uninstall</Button>
        <Button variant="ghost" size="xs" disabled={busy} onclick={() => void changeConnection(saved, true)}>Remove</Button>
      </div>
      {#if environment.readyProfileIds.includes(saved.id)}
        <ProviderUpdateControl profileId={saved.id} machineName={saved.name} />
      {/if}
      {#if confirmingUninstallId === saved.id}
        <div class="uninstall-confirm">
          <strong>Uninstall the backend from {saved.name}?</strong>
          <p>This stops and removes the backend service and its programs on {saved.name}. Any remote work running there right now is interrupted immediately.</p>
          <p>Keeping data leaves the backend's own stored data on the machine — its conversation history, sessions and settings — so reinstalling later picks up where you left off. Deleting that data erases it permanently and cannot be undone. Your project files and code checkouts on the machine are never deleted either way, and the saved connection stays in this list.</p>
          <div class="connection-actions">
            <Button variant="secondary" size="sm" disabled={busy} onclick={() => void uninstallBackend(saved, false)}>Uninstall and keep data</Button>
            <Button variant="destructive" size="sm" disabled={busy} onclick={() => void uninstallBackend(saved, true)}>Uninstall and delete data</Button>
            <Button variant="ghost" size="sm" disabled={busy} onclick={() => confirmingUninstallId = null}>Cancel</Button>
          </div>
        </div>
      {/if}
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
  .uninstall-confirm { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; padding: 12px; border: 1px solid var(--color-border); border-radius: 8px; }
  .uninstall-confirm strong { font-size: 13px; }
  .edit-form { display: flex; flex-direction: column; gap: 10px; padding: 12px 0 4px; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--color-text-3); }
  .connection-actions { justify-content: flex-end; }
</style>
