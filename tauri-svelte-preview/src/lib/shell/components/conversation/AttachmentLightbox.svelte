<script lang="ts">
  import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
  import { onDestroy } from 'svelte';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import {
    createTrackedObjectUrl,
    revokeTrackedObjectUrl
  } from '$lib/shell/resourceDiagnostics.svelte';

  let {
    src,
    fullPath,
    fullSrc,
    name,
    variant
  }: {
    src: string;
    fullPath?: string;
    fullSrc?: string;
    name: string;
    variant: 'composer' | 'timeline';
  } = $props();

  let open = $state(false);
  let fullObjectUrl = $state('');
  let loadGeneration = 0;

  function releaseFullObjectUrl(): void {
    loadGeneration += 1;
    if (fullObjectUrl) revokeTrackedObjectUrl(fullObjectUrl);
    fullObjectUrl = '';
  }

  async function openChanged(next: boolean): Promise<void> {
    open = next;
    if (!next) {
      releaseFullObjectUrl();
      return;
    }
    const source = fullPath
      ? (isTauri() ? convertFileSrc(fullPath) : fullPath)
      : fullSrc;
    if (!source) return;
    const generation = ++loadGeneration;
    try {
      const response = await fetch(source);
      if (!response.ok || generation !== loadGeneration) return;
      const blob = await response.blob();
      if (generation !== loadGeneration) return;
      if (fullObjectUrl) revokeTrackedObjectUrl(fullObjectUrl);
      fullObjectUrl = createTrackedObjectUrl(blob, 'attachment');
    } catch {
      if (generation === loadGeneration) fullObjectUrl = '';
    }
  }

  onDestroy(releaseFullObjectUrl);
</script>

<Dialog.Root {open} onOpenChange={(next) => void openChanged(next)}>
  <Dialog.Trigger class={`attachment-lightbox-thumbnail ${variant}`} aria-label={`Enlarge ${name}`}>
    {#if src}
      <img {src} alt={name} loading="lazy" decoding="async" />
    {:else}
      <span>{name}</span>
    {/if}
  </Dialog.Trigger>
  <Dialog.Content
    class="w-auto max-w-[calc(100vw-48px)] bg-transparent p-0 shadow-none ring-0 sm:max-w-[calc(100vw-48px)]"
    showCloseButton={false}
  >
    <Dialog.Title class="sr-only">{name}</Dialog.Title>
    <Dialog.Close class="attachment-lightbox-full-image" aria-label={`Close enlarged ${name}`}>
      {#if open && fullObjectUrl}
        <img src={fullObjectUrl} alt={name} />
      {/if}
    </Dialog.Close>
  </Dialog.Content>
</Dialog.Root>

<style>
  :global(.attachment-lightbox-thumbnail){display:block;overflow:hidden;padding:0;border:0;background:transparent;cursor:zoom-in}
  :global(.attachment-lightbox-thumbnail img){display:block;width:100%;height:100%;object-fit:cover}
  :global(.attachment-lightbox-thumbnail.composer){width:60px;height:52px;border-radius:7px}
  /* A fixed box, held whether or not the image has landed, so a transcript row
     never changes height when a screenshot finishes loading. The box itself
     paints nothing — the frame belongs to the picture, so a small or narrow
     screenshot is framed at its own size rather than sitting inside an empty
     rectangle. */
  :global(.attachment-lightbox-thumbnail.timeline){display:grid;place-items:center;width:220px;height:160px}
  :global(.attachment-lightbox-thumbnail.timeline img){width:auto;max-width:100%;height:auto;max-height:100%;border:1px solid color-mix(in srgb,var(--color-border) 76%,transparent);border-radius:10px}
  :global(.attachment-lightbox-thumbnail:focus-visible){outline:2px solid var(--color-focus-solid);outline-offset:2px}
  :global(.attachment-lightbox-full-image){display:block;padding:0;border:0;background:transparent;cursor:zoom-out}
  :global(.attachment-lightbox-full-image img){display:block;width:auto;height:auto;max-width:calc(100vw - 48px);max-height:calc(100vh - 48px);object-fit:contain}
</style>
