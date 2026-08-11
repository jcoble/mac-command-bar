<script lang="ts">
  /**
   * SessionBrowserAnnotationLayer.svelte — the sheet the notes are drawn on.
   *
   * It sits above the page region. When annotate mode is off it lets every
   * pointer event through to the page underneath; when it is on, dragging
   * boxes a region and a comment field opens beside it. Saved notes stay as
   * numbered markers.
   *
   * Rectangles are recorded relative to this layer, not to the page's own
   * scroll position, because the page is a separate view the shell cannot
   * measure into. That is what the note carries, and it is what the message
   * says.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import X from '@lucide/svelte/icons/x';
  import {
    isAnnotatableRect,
    rectFromDrag,
    type SessionBrowserAnnotation,
    type SessionBrowserPoint,
    type SessionBrowserRect
  } from './sessionBrowserOps.ts';

  interface Props {
    annotating: boolean;
    annotations: readonly SessionBrowserAnnotation[];
    onSave(rect: SessionBrowserRect, comment: string): void;
    onRemove(id: string): void;
  }

  let { annotating, annotations, onSave, onRemove }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let dragStart = $state<SessionBrowserPoint | null>(null);
  let dragRect = $state<SessionBrowserRect | null>(null);
  let pendingRect = $state<SessionBrowserRect | null>(null);
  let comment = $state('');
  let commentField = $state<HTMLTextAreaElement | null>(null);

  function pointFrom(event: PointerEvent): SessionBrowserPoint {
    const box = host?.getBoundingClientRect();
    return { x: event.clientX - (box?.left ?? 0), y: event.clientY - (box?.top ?? 0) };
  }

  /** Holding the pointer keeps a drag that leaves the layer from being lost.
   * A pointer the browser has already let go of cannot be held, and that is
   * not a failure worth stopping the drag for. */
  function capture(pointerId: number): void {
    try {
      host?.setPointerCapture(pointerId);
    } catch {
      // The drag still works from the events themselves.
    }
  }

  function release(pointerId: number): void {
    try {
      if (host?.hasPointerCapture(pointerId)) host.releasePointerCapture(pointerId);
    } catch {
      // Nothing to give back.
    }
  }

  function startDrag(event: PointerEvent): void {
    if (!annotating || event.button !== 0 || pendingRect) return;
    event.preventDefault();
    capture(event.pointerId);
    dragStart = pointFrom(event);
    dragRect = { ...dragStart, width: 0, height: 0 };
  }

  function moveDrag(event: PointerEvent): void {
    if (!dragStart) return;
    dragRect = rectFromDrag(dragStart, pointFrom(event));
  }

  function endDrag(event: PointerEvent): void {
    if (!dragStart) return;
    const rect = rectFromDrag(dragStart, pointFrom(event));
    dragStart = null;
    dragRect = null;
    release(event.pointerId);
    if (!isAnnotatableRect(rect)) return;
    pendingRect = rect;
    comment = '';
    queueMicrotask(() => commentField?.focus());
  }

  function save(): void {
    if (!pendingRect || !comment.trim()) return;
    onSave(pendingRect, comment);
    pendingRect = null;
    comment = '';
  }

  function cancel(): void {
    pendingRect = null;
    comment = '';
  }

  /** Keep the comment card inside the overlay when the box is near an edge. */
  function cardStyle(rect: SessionBrowserRect): string {
    const width = host?.clientWidth ?? 0;
    const left = Math.max(8, Math.min(rect.x, Math.max(8, width - 300)));
    return `left:${left}px;top:${rect.y + rect.height + 8}px`;
  }
</script>

<div
  bind:this={host}
  class="annotation-layer"
  class:armed={annotating}
  role="group"
  aria-label="Page annotations"
  data-testid="session-browser-annotation-layer"
  onpointerdown={startDrag}
  onpointermove={moveDrag}
  onpointerup={endDrag}
  onpointercancel={endDrag}
>
  {#each annotations as note (note.id)}
    <div
      class="marker-box"
      data-testid="session-browser-marker"
      style={`left:${note.rect.x}px;top:${note.rect.y}px;width:${note.rect.width}px;height:${note.rect.height}px`}
    >
      <span class="marker-number">{note.marker}</span>
      <span class="marker-comment">{note.comment}</span>
      <span class="marker-remove">
        <IconButton
          label={`Remove annotation ${note.marker}`}
          size="xs"
          variant="ghost"
          onclick={() => onRemove(note.id)}
        >
          <X aria-hidden="true" />
        </IconButton>
      </span>
    </div>
  {/each}

  {#if dragRect}
    <div
      class="drag-box"
      data-testid="session-browser-drag-box"
      style={`left:${dragRect.x}px;top:${dragRect.y}px;width:${dragRect.width}px;height:${dragRect.height}px`}
    ></div>
  {/if}

  {#if pendingRect}
    <div
      class="pending-box"
      style={`left:${pendingRect.x}px;top:${pendingRect.y}px;width:${pendingRect.width}px;height:${pendingRect.height}px`}
    ></div>
    <div class="comment-card" data-testid="session-browser-comment-card" style={cardStyle(pendingRect)}>
      <textarea
        bind:this={commentField}
        bind:value={comment}
        aria-label="What should change in this region"
        placeholder="What should change here?"
        onkeydown={(event) => {
          if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            save();
          }
          if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            cancel();
          }
        }}
      ></textarea>
      <div class="comment-actions">
        <Button variant="ghost" onclick={cancel}>Cancel</Button>
        <Button
          data-testid="session-browser-save-annotation"
          disabled={!comment.trim()}
          onclick={save}
        >
          Save note
        </Button>
      </div>
    </div>
  {/if}
</div>

<style>
  .annotation-layer {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
  }

  /* Only annotate mode takes the pointer; otherwise the page keeps it. */
  .annotation-layer.armed {
    cursor: crosshair;
    pointer-events: auto;
  }

  .marker-box,
  .drag-box,
  .pending-box {
    position: absolute;
    border-radius: var(--radius-sm);
  }

  .drag-box,
  .pending-box {
    border: 2px dashed var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  }

  .marker-box {
    border: 2px solid var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
    pointer-events: auto;
  }

  .marker-number {
    position: absolute;
    top: -11px;
    left: -11px;
    display: grid;
    width: 22px;
    height: 22px;
    place-items: center;
    border-radius: 999px;
    background: var(--color-accent);
    color: var(--color-on-accent);
    font-size: 13px;
    font-weight: 600;
  }

  .marker-comment {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    max-width: 280px;
    overflow: hidden;
    padding: 4px 7px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .marker-remove {
    position: absolute;
    top: -11px;
    right: -11px;
    border-radius: 999px;
    background: var(--color-surface);
  }

  .comment-card {
    position: absolute;
    z-index: 2;
    display: grid;
    gap: 8px;
    width: 292px;
    padding: 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-lg);
    pointer-events: auto;
  }

  .comment-card textarea {
    min-height: 66px;
    padding: 7px 8px;
    resize: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    font: 13px/1.45 inherit;
  }

  .comment-card textarea:focus-visible {
    border-color: var(--color-focus-solid);
    outline: none;
  }

  .comment-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  @media (prefers-reduced-motion: no-preference) {
    .marker-box,
    .comment-card {
      transition: opacity 0.12s ease;
    }
  }
</style>
