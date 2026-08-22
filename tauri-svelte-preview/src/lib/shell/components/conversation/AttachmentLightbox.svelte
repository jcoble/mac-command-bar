<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';

  let {
    src,
    fullSrc = src,
    name,
    variant
  }: {
    src: string;
    fullSrc?: string;
    name: string;
    variant: 'composer' | 'timeline';
  } = $props();

  let open = $state(false);
</script>

<Dialog.Root bind:open>
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
      {#if open && fullSrc}
        <img src={fullSrc} alt={name} />
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
