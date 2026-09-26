<script lang="ts">
  import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
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
    remoteOwnedId,
    attachmentId,
    mimeType,
    originalByteLength,
    name,
    variant
  }: {
    src: string;
    fullPath?: string;
    fullSrc?: string;
    remoteOwnedId?: string;
    attachmentId?: string;
    mimeType?: string;
    originalByteLength?: number;
    name: string;
    variant: 'composer' | 'timeline';
  } = $props();

  let open = $state(false);
  let fullObjectUrl = $state('');
  let loadError = $state('');
  let loadGeneration = 0;
  let activeRead: AbortController | null = null;

  function releaseFullObjectUrl(): void {
    loadGeneration += 1;
    activeRead?.abort();
    activeRead = null;
    if (fullObjectUrl) revokeTrackedObjectUrl(fullObjectUrl);
    fullObjectUrl = '';
    loadError = '';
  }

  async function openChanged(next: boolean): Promise<void> {
    open = next;
    if (!next) {
      releaseFullObjectUrl();
      return;
    }
    const generation = ++loadGeneration;
    if (remoteOwnedId) {
      if (!attachmentId || !mimeType || !originalByteLength
        || !Number.isSafeInteger(originalByteLength) || originalByteLength > 20 * 1024 * 1024) {
        loadError = 'Image metadata is missing or outside the 20 MB limit';
        return;
      }
      const controller = new AbortController();
      activeRead = controller;
      let rejectCancelled: (error: Error) => void = () => {};
      const cancelled = new Promise<never>((_, reject) => { rejectCancelled = reject; });
      const onAbort = () => rejectCancelled(new Error('Attachment read cancelled'));
      controller.signal.addEventListener('abort', onAbort, { once: true });
      try {
        const original = new Uint8Array(originalByteLength);
        let offset = 0;
        while (offset < original.length) {
          const data = await Promise.race([
            invoke<number[]>('read_agent_conversation_attachment_chunk', {
              ownedId: remoteOwnedId, attachmentId, thumbnail: false, offset
            }),
            cancelled
          ]);
          if (controller.signal.aborted || generation !== loadGeneration) return;
          if (!Array.isArray(data) || !data.length || data.length > 128 * 1024 || offset + data.length > original.length) {
            throw new Error('Incomplete image chunk');
          }
          original.set(data, offset);
          offset += data.length;
        }
        if (controller.signal.aborted || generation !== loadGeneration) return;
        fullObjectUrl = createTrackedObjectUrl(new Blob([original.buffer as ArrayBuffer], { type: mimeType }), 'attachment');
      } catch (error) {
        if (generation === loadGeneration && !controller.signal.aborted) {
          const message = error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
          const reason = message.match(/connection closed|not connected|did not answer within \d+ seconds|request queue is unavailable|attachment row was not found|attachment read is outside the 20 MB limit|wrong attachment bytes|incomplete image chunk/i)?.[0];
          loadError = reason ? `Image read failed: ${reason}` : 'Image read failed';
        }
      } finally {
        controller.signal.removeEventListener('abort', onAbort);
        if (activeRead === controller) activeRead = null;
      }
      return;
    }
    const source = fullPath
      ? (isTauri() ? convertFileSrc(fullPath) : fullPath)
      : fullSrc;
    if (!source) return;
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
    showCloseButton={!fullObjectUrl}
  >
    <Dialog.Title class="sr-only">{name}</Dialog.Title>
    {#if open && loadError}<p class="text-destructive text-[13px]" role="alert">{loadError}</p>{/if}
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
