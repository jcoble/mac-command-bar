<script lang="ts">
  import { onDestroy } from 'svelte';
  import Folder from '@lucide/svelte/icons/folder';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import House from '@lucide/svelte/icons/house';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { listRemoteDirectoriesFromTauri, type RemoteDirectoryListing } from '$lib/tauriSource';

  let { label, profileId, sshTarget, disabled = false, value = $bindable(''), onChange }: {
    label: string; profileId: string; sshTarget: string; disabled?: boolean; value?: string; onChange?: (path: string) => void;
  } = $props();
  let open = $state(false);
  let path = $state('');
  let listing = $state<RemoteDirectoryListing | null>(null);
  let loading = $state(false);
  let error = $state('');
  let sequence = 0;
  let requestedDirectory = '';
  const matchingDirectories = $derived(listing?.directories.filter((name) => {
    const prefix = listing.path === '/' ? '/' : `${listing.path}/`;
    const query = path === listing.path ? '' : path.startsWith(prefix) ? path.slice(prefix.length) : path;
    return name.toLowerCase().startsWith(query.toLowerCase());
  }) ?? []);
  const parent = $derived(listing?.path.replace(/\/[^/]+\/?$/, '') || '/');
  const currentFolderSelected = $derived((path.replace(/\/+$/, '') || '/') === listing?.path);

  function close() {
    open = false;
    listing = null;
    error = '';
    loading = false;
    sequence += 1;
    requestedDirectory = '';
  }

  // A host change or session teardown must never apply a late folder response.
  $effect(() => { profileId; close(); });
  onDestroy(() => { sequence += 1; });

  async function navigate(destination: string, replaceInput = true) {
    const resolved = destination.startsWith('/') || destination.startsWith('~') || !listing
      ? destination : `${listing.path === '/' ? '' : listing.path}/${destination}`;
    const request = ++sequence;
    const target = profileId;
    requestedDirectory = resolved;
    loading = true;
    listing = null;
    error = '';
    if (replaceInput) path = resolved;
    const inputAtRequest = path;
    try {
      const result = await listRemoteDirectoriesFromTauri(target, resolved);
      if (request !== sequence || target !== profileId || !open) return;
      listing = result;
      if (replaceInput && path === inputAtRequest) path = result.path;
    } catch (reason) {
      if (request !== sequence || !open) return;
      error = reason instanceof Error ? reason.message
        : typeof reason === 'object' && reason !== null && 'message' in reason ? String(reason.message)
        : String(reason);
    } finally {
      if (request === sequence) loading = false;
    }
  }

  function browse() {
    open = true;
    void navigate(value || '~');
  }

  function choose() {
    if (!listing || loading || !currentFolderSelected) return;
    value = listing.path;
    onChange?.(value);
    close();
  }
</script>

<Button variant="ghost" size="xs" disabled={disabled || !sshTarget.trim() || loading} onclick={browse} aria-label={`Choose ${label}`}>
  <Folder aria-hidden="true" class="size-3.5" /> {value.split('/').filter(Boolean).at(-1) || 'Choose a folder'}
</Button>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) close(); }}>
  <Dialog.Content class="remote-directory-dialog">
    <Dialog.Header>
      <Dialog.Title>Choose a folder</Dialog.Title>
      <Dialog.Description>{label} on {sshTarget}</Dialog.Description>
    </Dialog.Header>
    <form class="directory-navigation" onsubmit={(event) => { event.preventDefault(); void navigate(path); }}>
      <Button type="button" variant="ghost" size="icon-sm" disabled={loading} onclick={() => void navigate('~')} aria-label="Home folder"><House aria-hidden="true" class="size-4" /></Button>
      <Button type="button" variant="ghost" size="icon-sm" disabled={loading || !listing || listing.path === '/'} onclick={() => void navigate(parent)} aria-label="Parent folder"><ArrowUp aria-hidden="true" class="size-4" /></Button>
      <Input aria-label="Remote folder path" bind:value={path} autocomplete="off" oninput={(event) => {
        const next = event.currentTarget.value;
        const directory = next.slice(0, next.lastIndexOf('/')) || '/';
        if (next.startsWith('/') && directory !== listing?.path && directory !== requestedDirectory) {
          void navigate(directory, false);
        }
      }} />
      <Button type="submit" variant="ghost" size="sm" disabled={loading}>Go</Button>
    </form>
    <div class="directory-list" aria-label="Remote folders" aria-busy={loading}>
      {#if loading}
        <p role="status">Loading folders…</p>
      {:else if error}
        <p role="alert">{error}</p>
      {:else if listing}
        {#each matchingDirectories as name (name)}
          <button type="button" class="directory-row" onclick={() => void navigate(`${listing?.path === '/' ? '' : listing?.path}/${name}`)}>
            <Folder aria-hidden="true" class="size-4" /><span>{name}</span><ChevronRight aria-hidden="true" class="size-3.5" />
          </button>
        {:else}
          <p>No matching subfolders. Type a path or choose the current folder.</p>
        {/each}
      {/if}
    </div>
    {#if listing?.truncated}<p class="directory-limit">Showing the first 500 folders. Type a path to open another folder.</p>{/if}
    <Dialog.Footer>
      <Button variant="ghost" size="sm" onclick={close}>Cancel</Button>
      <Button size="sm" disabled={loading || !listing || !currentFolderSelected} onclick={choose}>Choose this folder</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
  .directory-navigation { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .directory-navigation :global(input) { min-width: 0; flex: 1; }
  :global(.remote-directory-dialog) { width: 560px; max-width: calc(100vw - 32px); gap: 16px; border-radius: 8px; animation: none; }
  .directory-list { height: min(300px, 45vh); overflow: auto; }
  .directory-list p, .directory-limit { padding: 12px; color: var(--color-text-2); font-size: 13px; overflow-wrap: anywhere; }
  .directory-row { display: flex; align-items: center; gap: 12px; width: 100%; padding: 10px 12px; border: 0; border-radius: 3px; background: transparent; color: var(--color-text); text-align: left; font-size: 13px; content-visibility: auto; contain-intrinsic-size: auto 38px; }
  .directory-row:hover, .directory-row:focus-visible { background: var(--menu-row-hover); outline: none; }
  .directory-row span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .directory-row :global(svg) { flex: none; color: var(--color-text-2); }
</style>
