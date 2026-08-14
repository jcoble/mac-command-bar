<script lang="ts">
  /**
   * AnnotationBadges.svelte — the numbers on the page.
   *
   * Every place the reader pointed at gets a numbered circle where they
   * pointed, and the number matches the row in the card's list. Without them a
   * request naming "the second thing" points at nothing: the picture that gets
   * sent shows three blue boxes and no way to tell which is which.
   *
   * The circles sit over the still, not over the live page — nothing in this
   * document can be drawn over a native view, which is why marking works on a
   * captured picture in the first place. Coordinates are the marking surface's
   * own pixels, the same ones the canvas underneath paints in.
   *
   * The one that was just made is open for a label: an input beside its circle
   * that takes what the reader says about that spot. Enter or the check keeps
   * it, Escape leaves the annotation with no words rather than throwing it
   * away — a box with nothing said about it still shows where to look.
   */
  import Check from '@lucide/svelte/icons/check';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';

  import type { NumberedAnnotation } from './annotationList.ts';

  interface Props {
    annotations: readonly NumberedAnnotation[];
    /** The annotation whose label is being typed, if any. */
    editingId: string | null;
    /** The marking surface's size, so a pill near an edge can be pulled inside. */
    surface: { width: number; height: number };
    onLabel(id: string, label: string): void;
    onDoneEditing(): void;
  }

  let { annotations, editingId, surface, onLabel, onDoneEditing }: Props = $props();

  /** How wide the label input is, and how far a pill may reach past its box. */
  const PILL_WIDTH = 220;

  function left(item: NumberedAnnotation): number {
    return Math.min(item.box.x, item.box.x + item.box.width);
  }

  function top(item: NumberedAnnotation): number {
    return Math.min(item.box.y, item.box.y + item.box.height);
  }

  /** The pill hangs below its box, pulled back inside the surface at the edges. */
  function pillLeft(item: NumberedAnnotation): number {
    const room = surface.width - PILL_WIDTH - 8;
    return Math.max(8, Math.min(left(item), room > 8 ? room : 8));
  }

  function pillTop(item: NumberedAnnotation): number {
    const below = top(item) + Math.abs(item.box.height) + 6;
    return surface.height > 0 ? Math.min(below, Math.max(0, surface.height - 40)) : below;
  }

  function keys(event: KeyboardEvent): void {
    if (event.key === 'Enter' || event.key === 'Escape') {
      event.preventDefault();
      onDoneEditing();
    }
  }
</script>

<div class="badges" data-testid="browser-annotation-badges">
  {#each annotations as item (item.id)}
    <span
      class="badge"
      style:left={`${left(item)}px`}
      style:top={`${top(item)}px`}
      data-testid="browser-annotation-badge"
    >{item.number}</span>

    {#if editingId === item.id}
      <div
        class="pill editing"
        style:left={`${pillLeft(item)}px`}
        style:top={`${pillTop(item)}px`}
        style:width={`${PILL_WIDTH}px`}
        data-testid="browser-annotation-label-editor"
      >
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="label-input"
          autofocus
          aria-label={`What to change about this ${item.tag}`}
          placeholder="What about this one?"
          value={item.label}
          data-testid="browser-annotation-label-input"
          oninput={(event) => onLabel(item.id, event.currentTarget.value)}
          onkeydown={keys}
        />
        <IconButton
          label="Keep this note"
          size="xs"
          variant="ghost"
          data-testid="browser-annotation-label-done"
          onclick={onDoneEditing}
        >
          <Check aria-hidden="true" />
        </IconButton>
      </div>
    {:else if item.label}
      <span
        class="pill"
        style:left={`${pillLeft(item)}px`}
        style:top={`${pillTop(item)}px`}
        data-testid="browser-annotation-label"
      >{item.label}</span>
    {/if}
  {/each}
</div>

<style>
  .badges {
    position: absolute;
    inset: 0;
    /* The canvas underneath takes the pointer for drawing and erasing; only
       the label input inside a pill takes it back. */
    pointer-events: none;
  }

  .badge {
    position: absolute;
    display: inline-flex;
    height: 22px;
    min-width: 22px;
    transform: translate(-50%, -50%);
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.85);
    border-radius: 999px;
    background: rgb(37, 99, 235);
    padding: 0 6px;
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
    color: rgb(255, 255, 255);
  }

  .pill {
    position: absolute;
    display: flex;
    max-width: 220px;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-surface);
    padding: 3px 10px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--color-text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pill.editing {
    padding: 2px 2px 2px 10px;
    pointer-events: auto;
  }

  .label-input {
    min-width: 0;
    flex: 1;
    border: 0;
    background: transparent;
    font-size: 12px;
    color: var(--color-text);
    outline: none;
  }

  .label-input::placeholder {
    color: var(--color-text-2);
  }
</style>
