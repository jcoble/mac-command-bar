<script lang="ts">
  /**
   * BrowserMiniComposer.svelte — the card that floats over the page being
   * marked up.
   *
   * It sits inside the browser area rather than under it, because what it says
   * is about the picture directly above it: this many places marked, and here
   * is the sentence that goes with them. In the panel's footer it read as
   * another row of chrome and people typed into the session's composer
   * instead.
   *
   * Send is a send. The words and the marked-up picture go to the session as
   * one turn, and the session comes forward — the reader watches it go rather
   * than finding a staged draft somewhere else and pressing Enter again.
   *
   * The chip is the list. Closed it counts; open it shows every annotation
   * with a crop of the place it points at, so a note that ended up on the
   * wrong thing can be taken out before the turn leaves.
   */
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import X from '@lucide/svelte/icons/x';

  import { Chip } from '$lib/components/ui/chip/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';

  import { annotationCountLabel, type NumberedAnnotation } from './annotationList.ts';

  interface Props {
    description: string;
    annotations: readonly NumberedAnnotation[];
    /** The still the annotations were made on, for the list's crops. */
    backdrop: string | null;
    /** The marking surface's size, the coordinates the boxes are in. */
    surface: { width: number; height: number };
    /** True while the picture is being composed and sent. */
    busy: boolean;
    /** Nothing to send: no session, or no page. */
    disabled: boolean;
    error: string;
    listOpen: boolean;
    onDescriptionChange(value: string): void;
    onToggleList(): void;
    onRemove(id: string): void;
    onSend(): void;
  }

  let {
    description,
    annotations,
    backdrop,
    surface,
    busy,
    disabled,
    error,
    listOpen,
    onDescriptionChange,
    onToggleList,
    onRemove,
    onSend
  }: Props = $props();

  /** The crop tile in the list, in CSS pixels. */
  const THUMB_WIDTH = 44;
  const THUMB_HEIGHT = 32;

  const canSend = $derived(!disabled && !busy && (description.trim().length > 0 || annotations.length > 0));

  /**
   * The still, scaled and shifted so one annotation's box fills the tile. A
   * background rather than a second canvas: the crop is a picture of a
   * rectangle of another picture, which is what background-size and
   * background-position already say.
   */
  function crop(item: NumberedAnnotation): string {
    const width = Math.max(1, Math.abs(item.box.width));
    const height = Math.max(1, Math.abs(item.box.height));
    const scale = Math.min(THUMB_WIDTH / width, THUMB_HEIGHT / height);
    const x = Math.min(item.box.x, item.box.x + item.box.width) * scale;
    const y = Math.min(item.box.y, item.box.y + item.box.height) * scale;
    const offsetX = (THUMB_WIDTH - width * scale) / 2;
    const offsetY = (THUMB_HEIGHT - height * scale) / 2;
    return [
      `background-size:${surface.width * scale}px ${surface.height * scale}px`,
      `background-position:${offsetX - x}px ${offsetY - y}px`
    ].join(';');
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (canSend) onSend();
  }
</script>

<div class="card" data-testid="browser-mini-composer">
  {#if listOpen && annotations.length > 0}
    <ul class="list" data-testid="browser-annotation-list">
      {#each annotations as item (item.id)}
        <li class="row">
          <span class="number">{item.number}</span>
          {#if backdrop}
            <span
              class="thumb"
              style={`background-image:url(${backdrop});${crop(item)}`}
              aria-hidden="true"
            ></span>
          {/if}
          <span class="what">
            <Chip tone="neutral">{item.tag}</Chip>
            <span class="said">{item.label || 'No note'}</span>
          </span>
          <IconButton
            label={`Remove annotation ${item.number}`}
            size="xs"
            variant="ghost"
            data-testid="browser-annotation-remove"
            onclick={() => onRemove(item.id)}
          >
            <X aria-hidden="true" />
          </IconButton>
        </li>
      {/each}
    </ul>
  {/if}

  <form class="prompt" onsubmit={submit}>
    {#if annotations.length > 0}
      <button
        type="button"
        class="count"
        aria-expanded={listOpen}
        data-testid="browser-annotation-count"
        onclick={onToggleList}
      >
        <ListChecks aria-hidden="true" />
        {annotationCountLabel(annotations.length)}
      </button>
    {/if}

    <div class="entry">
      <input
        class="say"
        aria-label="Describe the change"
        placeholder="Describe the change…"
        value={description}
        disabled={busy}
        data-testid="browser-mini-composer-input"
        oninput={(event) => onDescriptionChange(event.currentTarget.value)}
      />
      <IconButton
        label={busy ? 'Sending' : 'Send to the session'}
        size="sm"
        variant="default"
        disabled={!canSend}
        data-testid="browser-annotation-send"
        onclick={() => onSend()}
      >
        <ArrowUp aria-hidden="true" />
      </IconButton>
    </div>
  </form>

  {#if error}
    <p class="failure" role="alert" data-testid="browser-mini-composer-error">{error}</p>
  {/if}
</div>

<style>
  .card {
    display: flex;
    width: min(560px, calc(100% - 24px));
    flex-direction: column;
    gap: 6px;
    border: 1px solid var(--color-border);
    border-radius: 14px;
    background: var(--color-surface);
    padding: 8px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
  }

  .list {
    display: flex;
    max-height: 176px;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 10px;
    padding: 4px 6px;
  }

  .row:hover {
    background: var(--color-elevated);
  }

  .number {
    display: inline-flex;
    height: 18px;
    min-width: 18px;
    flex: none;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: rgb(37, 99, 235);
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
    color: rgb(255, 255, 255);
  }

  .thumb {
    display: block;
    height: 32px;
    width: 44px;
    flex: none;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background-color: var(--color-elevated);
    background-repeat: no-repeat;
  }

  .what {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 6px;
  }

  .said {
    overflow: hidden;
    font-size: 12px;
    color: var(--color-text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .prompt {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }

  .count {
    display: inline-flex;
    height: 22px;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-elevated);
    padding: 0 10px;
    font-size: 12px;
    color: var(--color-text);
    cursor: pointer;
  }

  .count :global(svg) {
    height: 12px;
    width: 12px;
  }

  .entry {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 6px;
  }

  .say {
    min-width: 0;
    flex: 1;
    border: 0;
    background: transparent;
    padding: 4px 2px;
    font-size: 13px;
    color: var(--color-text);
    outline: none;
  }

  .say::placeholder {
    color: var(--color-text-2);
  }

  .failure {
    margin: 0;
    padding: 0 4px;
    font-size: 12px;
    color: var(--color-bad);
  }
</style>
