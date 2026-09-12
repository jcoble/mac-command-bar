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

  let { label, sshTarget, placeholder, disabled = false, value = $bindable('') }: {
    label: string; sshTarget: string; placeholder: string; disabled?: boolean; value?: string;
  } = $props();
  const id = $props.id();
  let open = $state(false);
  let path = $state('');
  let listing = $state<RemoteDirectoryListing | null>(null);
  let loading = $state(false);
  let error = $state('');
  let sequence = 0;
  const parent = $derived(listing?.path.replace(/\/[^/]+\/?$/, '') || '/');

  function close() {
    open = false;
    listing = null;
    error = '';
    sequence += 1;
  }

  // A host change or session teardown must never apply a late folder response.
  $effect(() => { sshTarget; close(); });
  onDestroy(() => { sequence += 1; });

  async function navigate(destination: string) {
    if (loading) return;
    const request = ++sequence;
    const target = sshTarget.trim();
    loading = true;
    listing = null;
    error = '';
    path = destination;
    try {
      const result = await listRemoteDirectoriesFromTauri(target, destination);
      if (request !== sequence || target !== sshTarget.trim() || !open) return;
      listing = result;
      path = result.path;
    } catch (reason) {
      if (request !== sequence || !open) return;
      error = reason instanceof Error ? reason.message
        : typeof reason === 'object' && reason !== null && 'message' in reason ? String(reason.message)
        : String(reason);
    } finally {
      loading = false;
    }
  }

  function browse() {
    open = true;
    void navigate(value || '~');
  }

  function choose() {
    if (!listing || loading || path !== listing.path) return;
    value = listing.path;
    close();
  }
</script>

<div class="directory-field">
  <label for={id}>{label}</label>
  <div class="directory-input">
    <Input id={id} bind:value {placeholder} {disabled} autocomplete="off" />
    <Button variant="ghost" size="sm" disabled={disabled || !sshTarget.trim() || loading} onclick={browse} aria-label={`Browse ${label}`}>
      <Folder aria-hidden="true" class="size-4" /> Browse
    </Button>
  </div>
</div>

<Dialog.Root {open} onOpenChange={(next) => { if (!next) close(); }}>
  <Dialog.Content class="remote-directory-dialog">
    <Dialog.Header>
      <Dialog.Title>Choose a folder</Dialog.Title>
      <Dialog.Description>{label} on {sshTarget}</Dialog.Description>
    </Dialog.Header>
    <form class="directory-navigation" onsubmit={(event) => { event.preventDefault(); void navigate(path); }}>
      <Button type="button" variant="ghost" size="icon-sm" disabled={loading} onclick={() => void navigate('~')} aria-label="Home folder"><House aria-hidden="true" class="size-4" /></Button>
      <Button type="button" variant="ghost" size="icon-sm" disabled={loading || !listing || listing.path === '/'} onclick={() => void navigate(parent)} aria-label="Parent folder"><ArrowUp aria-hidden="true" class="size-4" /></Button>
      <Input aria-label="Remote folder path" bind:value={path} disabled={loading} autocomplete="off" />
      <Button type="submit" variant="ghost" size="sm" disabled={loading}>Go</Button>
    </form>
    <div class="directory-list" aria-label="Remote folders" aria-busy={loading}>
      {#if loading}
        <p role="status">Loading folders…</p>
      {:else if error}
        <p role="alert">{error}</p>
      {:else if listing}
        {#each listing.directories as name (name)}
          <button type="button" class="directory-row" onclick={() => void navigate(`${listing?.path === '/' ? '' : listing?.path}/${name}`)}>
            <Folder aria-hidden="true" class="size-4" /><span>{name}</span><ChevronRight aria-hidden="true" class="size-3.5" />
          </button>
        {:else}
          <p>No subfolders. You can choose this folder.</p>
        {/each}
      {/if}
    </div>
    {#if listing?.truncated}<p class="directory-limit">Showing the first 500 folders. Type a path to open another folder.</p>{/if}
    <Dialog.Footer>
      <Button variant="ghost" size="sm" onclick={close}>Cancel</Button>
      <Button size="sm" disabled={loading || !listing || path !== listing.path} onclick={choose}>Choose this folder</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
  .directory-field { display: grid; gap: 5px; }
  label { color: var(--color-text-3); font-size: 12px; }
  .directory-input, .directory-navigation { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .directory-input :global(input), .directory-navigation :global(input) { min-width: 0; flex: 1; }
  :global(.remote-directory-dialog) { width: 560px; max-width: calc(100vw - 32px); gap: 16px; border-radius: 8px; animation: none; }
  .directory-list { height: min(300px, 45vh); overflow: auto; }
  .directory-list p, .directory-limit { padding: 12px; color: var(--color-text-2); font-size: 13px; overflow-wrap: anywhere; }
  .directory-row { display: flex; align-items: center; gap: 12px; width: 100%; padding: 10px 12px; border: 0; border-radius: 3px; background: transparent; color: var(--color-text); text-align: left; font-size: 13px; content-visibility: auto; contain-intrinsic-size: auto 38px; }
  .directory-row:hover, .directory-row:focus-visible { background: var(--menu-row-hover); outline: none; }
  .directory-row span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .directory-row :global(svg) { flex: none; color: var(--color-text-2); }
</style>
