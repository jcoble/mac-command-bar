<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';

  let {
    src,
    name,
    variant
  }: {
    src: string;
    name: string;
    variant: 'composer' | 'timeline';
  } = $props();
</script>

<Dialog.Root>
  <Dialog.Trigger class={`attachment-lightbox-thumbnail ${variant}`} aria-label={`Enlarge ${name}`}>
    <img {src} alt={name} />
  </Dialog.Trigger>
  <Dialog.Content
    class="w-auto max-w-[calc(100vw-48px)] bg-transparent p-0 shadow-none ring-0"
    showCloseButton={false}
  >
    <Dialog.Title class="sr-only">{name}</Dialog.Title>
    <Dialog.Close class="attachment-lightbox-full-image" aria-label={`Close enlarged ${name}`}>
      <img {src} alt={name} />
    </Dialog.Close>
  </Dialog.Content>
</Dialog.Root>

<style>
  :global(.attachment-lightbox-thumbnail){display:block;overflow:hidden;padding:0;border:0;background:transparent;cursor:zoom-in}
  :global(.attachment-lightbox-thumbnail img){display:block;width:100%;height:100%;object-fit:cover}
  :global(.attachment-lightbox-thumbnail.composer){width:60px;height:52px;border-radius:7px}
  :global(.attachment-lightbox-thumbnail.timeline){max-width:220px;max-height:160px;border:1px solid color-mix(in srgb,var(--color-border) 76%,transparent);border-radius:10px}
  :global(.attachment-lightbox-thumbnail.timeline img){width:auto;max-width:220px;height:auto;max-height:160px}
  :global(.attachment-lightbox-thumbnail:focus-visible){outline:2px solid var(--color-focus-solid);outline-offset:2px}
  :global(.attachment-lightbox-full-image){display:block;padding:0;border:0;background:transparent;cursor:zoom-out}
  :global(.attachment-lightbox-full-image img){display:block;width:auto;height:auto;max-width:calc(100vw - 48px);max-height:calc(100vh - 48px);object-fit:contain}
</style>
